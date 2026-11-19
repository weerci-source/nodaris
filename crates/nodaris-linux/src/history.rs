use std::{
    env,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, BufReader, ErrorKind},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use nodaris_core::{
    error::{CoreError, ErrorSeverity},
    keys,
    models::{
        ethernet::EthernetReport,
        history::{HistoryEntry, HistoryKind, RecordedReport},
        session::DiagnosticSession,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const FILE_VERSION: u8 = 1;
const LIMIT: usize = 10;

#[derive(Deserialize, Serialize)]
struct HistoryFile {
    version: u8,
    entries: Vec<HistoryEntry<Value>>,
}

struct Store(PathBuf);

pub async fn run_session(
    interface: String,
    target: Option<String>,
) -> Result<RecordedReport<DiagnosticSession>, CoreError> {
    let report = crate::collect_session(interface, target.clone()).await?;
    let id = report.id.0.clone();
    Ok(record(id, HistoryKind::Session, target, report).await)
}

pub async fn run_ethernet(
    interface: String,
    target: String,
) -> Result<RecordedReport<EthernetReport>, CoreError> {
    let report = crate::collect_ethernet_report(interface, target.clone()).await?;
    let id = report.link.state.observed_at.session_id.0.clone();
    Ok(record(id, HistoryKind::Ethernet, Some(target), report).await)
}

async fn record<T: Serialize>(
    id: String,
    kind: HistoryKind,
    target: Option<String>,
    report: T,
) -> RecordedReport<T> {
    let result = match serde_json::to_value(&report) {
        Ok(report) => {
            let entry = HistoryEntry {
                id,
                kind,
                target,
                report,
            };
            run_store(keys::history::SAVE_FAILED, move |store| store.save(entry)).await
        }
        Err(error) => {
            Err(CoreError::io_err(keys::history::SAVE_FAILED).with_tech(error.to_string()))
        }
    };
    RecordedReport {
        report,
        history_error: result.err().map(|mut error| {
            error.severity = ErrorSeverity::Warning;
            error.into_api()
        }),
    }
}

pub async fn list() -> Result<Vec<HistoryEntry<Value>>, CoreError> {
    run_store(keys::history::READ_FAILED, |store| store.list()).await
}

pub async fn delete(id: String) -> Result<Vec<HistoryEntry<Value>>, CoreError> {
    if !valid_id(&id) {
        return Err(CoreError::validation(keys::history::INVALID_ID));
    }
    run_store(keys::history::DELETE_FAILED, move |store| store.delete(&id)).await
}

fn valid_id(id: &str) -> bool {
    id.len() <= 96
        && id.strip_prefix("linux-").is_some_and(|suffix| {
            let mut parts = suffix.split('-');
            (0..3).all(|_| {
                parts.next().is_some_and(|part| {
                    !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
                })
            }) && parts.next().is_none()
        })
}

async fn run_store<T: Send + 'static>(
    key: &'static str,
    action: impl FnOnce(Store) -> io::Result<T> + Send + 'static,
) -> Result<T, CoreError> {
    tokio::task::spawn_blocking(move || action(Store::open()?))
        .await
        .map_err(|error| CoreError::io_err(key).with_tech(format!("History worker: {error}")))?
        .map_err(|error| CoreError::io_err(key).with_tech(format!("History store: {error}")))
}

impl Store {
    fn open() -> io::Result<Self> {
        let dir = if let Some(dir) = env::var_os("NODARIS_HISTORY_DIR") {
            PathBuf::from(dir)
        } else if let Some(dir) = absolute_env("XDG_STATE_HOME") {
            dir.join("nodaris")
        } else if let Some(dir) = absolute_env("HOME") {
            dir.join(".local/state/nodaris")
        } else {
            return Err(io::Error::new(ErrorKind::NotFound, "No state directory"));
        };
        Self::open_at(dir)
    }

    fn open_at(dir: PathBuf) -> io::Result<Self> {
        if !dir.is_absolute() {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "State path must be absolute",
            ));
        }
        DirBuilder::new().recursive(true).mode(0o700).create(&dir)?;
        let metadata = fs::symlink_metadata(&dir)?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "State directory must be private",
            ));
        }
        Ok(Self(dir))
    }

    fn lock(&self) -> io::Result<File> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(self.0.join("history.lock"))?;
        lock.lock()?;
        Ok(lock)
    }

    fn list(&self) -> io::Result<Vec<HistoryEntry<Value>>> {
        let _lock = self.lock()?;
        Ok(self.read()?.entries)
    }

    fn save(&self, entry: HistoryEntry<Value>) -> io::Result<()> {
        let _lock = self.lock()?;
        let mut file = self.read()?;
        if file.entries.iter().any(|item| item.id == entry.id) {
            return Ok(());
        }
        file.entries.insert(0, entry);
        file.entries.truncate(LIMIT);
        self.write(&file)
    }

    fn delete(&self, id: &str) -> io::Result<Vec<HistoryEntry<Value>>> {
        let _lock = self.lock()?;
        let mut file = self.read()?;
        if let Some(index) = file.entries.iter().position(|entry| entry.id == id) {
            file.entries.remove(index);
            self.write(&file)?;
        }
        Ok(file.entries)
    }

    fn read(&self) -> io::Result<HistoryFile> {
        let file = match File::open(self.0.join("history.json")) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(HistoryFile {
                    version: FILE_VERSION,
                    entries: Vec::new(),
                });
            }
            Err(error) => return Err(error),
        };
        let history: HistoryFile = serde_json::from_reader(BufReader::new(file))?;
        if history.version != FILE_VERSION {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Unsupported history version",
            ));
        }
        Ok(history)
    }

    fn write(&self, history: &HistoryFile) -> io::Result<()> {
        // The separate lock file stays in place when the history file is replaced.
        let temp = self.0.join("history.json.tmp");
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .mode(0o600)
                .open(&temp)?;
            serde_json::to_writer(&file, history)?;
            file.sync_all()?;
            fs::rename(&temp, self.0.join("history.json"))?;
            File::open(&self.0)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}

fn absolute_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| Path::is_absolute(path))
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;
    use crate::session::RunClock;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            Self(env::temp_dir().join(RunClock::new().now().session_id.0))
        }

        fn open(&self) -> Store {
            Store::open_at(self.0.clone()).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(sequence: usize) -> HistoryEntry<Value> {
        HistoryEntry {
            id: format!("linux-1-1-{sequence}"),
            kind: HistoryKind::Ethernet,
            target: Some("1.1.1.1".to_owned()),
            report: serde_json::json!({
                "measurement": sequence,
                "avg_rtt_ms": 109.333_333_333_333_33_f64,
            }),
        }
    }

    #[test]
    fn reopens_latest_reports_and_keeps_deletion_on_disk() {
        let fixture = Fixture::new();
        let store = fixture.open();
        assert!(store.list().unwrap().is_empty());
        for sequence in 0..=LIMIT {
            store.save(entry(sequence)).unwrap();
        }
        let store = fixture.open();
        let entries = store.list().unwrap();
        assert_eq!(entries.len(), LIMIT);
        assert_eq!(entries[0].report, entry(LIMIT).report);
        assert_eq!(entries[LIMIT - 1].id, entry(1).id);
        assert_eq!(store.delete(&entry(LIMIT).id).unwrap().len(), LIMIT - 1);
        assert_eq!(
            fixture.open().delete(&entry(LIMIT).id).unwrap().len(),
            LIMIT - 1
        );
        assert_eq!(
            fs::metadata(fixture.0.join("history.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn parallel_writers_preserve_reports() {
        let fixture = Fixture::new();
        thread::scope(|threads| {
            for sequence in 0..LIMIT {
                let fixture = &fixture;
                threads.spawn(move || fixture.open().save(entry(sequence)).unwrap());
            }
        });
        assert_eq!(fixture.open().list().unwrap().len(), LIMIT);
    }

    #[test]
    fn refuses_to_overwrite_unreadable_or_unsupported_history() {
        let fixture = Fixture::new();
        let store = fixture.open();
        let path = fixture.0.join("history.json");
        for content in ["{", r#"{"version":2,"entries":[]}"#] {
            fs::write(&path, content).unwrap();
            assert!(store.list().is_err());
            assert!(store.save(entry(1)).is_err());
            assert!(store.delete(&entry(1).id).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }
}
