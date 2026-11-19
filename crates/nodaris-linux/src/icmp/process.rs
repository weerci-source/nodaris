use std::{
    io::ErrorKind,
    process::{Output, Stdio},
    time::Duration,
};

use nodaris_core::models::icmp::ProbeUnavailable;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

const MAX_OUTPUT_BYTES: usize = 16 * 1024;

pub(super) async fn run(
    program: &str,
    args: &[&str],
    deadline: Duration,
) -> Result<Output, ProbeUnavailable> {
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => ProbeUnavailable::ToolUnavailable,
            ErrorKind::PermissionDenied => ProbeUnavailable::PermissionDenied,
            _ => ProbeUnavailable::MeasurementFailed,
        })?;

    let limit = MAX_OUTPUT_BYTES as u64 + 1;
    let mut out = child.stdout.take().expect("piped stdout").take(limit);
    let mut err = child.stderr.take().expect("piped stderr").take(limit);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let result = timeout(deadline, async {
        tokio::try_join!(out.read_to_end(&mut stdout), err.read_to_end(&mut stderr))
            .map_err(|_| ProbeUnavailable::MeasurementFailed)?;
        if stdout.len() > MAX_OUTPUT_BYTES || stderr.len() > MAX_OUTPUT_BYTES {
            return Err(ProbeUnavailable::InvalidOutput);
        }
        let status = child
            .wait()
            .await
            .map_err(|_| ProbeUnavailable::MeasurementFailed)?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    })
    .await;

    match result {
        Ok(Ok(output)) => Ok(output),
        outcome => {
            // kill().await also waits for the child. Cancellation drops and kills it via Tokio.
            let _ = child.kill().await;
            outcome.unwrap_or(Err(ProbeUnavailable::ProcessTimeout))
        }
    }
}

pub(super) fn permission_error(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    stderr.contains("operation not permitted") || stderr.contains("permission denied")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kills_a_hung_command_on_deadline() {
        let result = run("/bin/sleep", &["20"], Duration::from_millis(50)).await;
        assert!(matches!(result, Err(ProbeUnavailable::ProcessTimeout)));
    }
}
