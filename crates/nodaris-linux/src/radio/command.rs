use std::{
    io::ErrorKind,
    path::Path,
    process::{Command, Output},
};

use nodaris_core::models::measurement::UnavailableReason;

pub(super) fn run(program: &str, args: &[&str]) -> Result<Output, UnavailableReason> {
    // GNU timeout terminates the process group and escalates to SIGKILL after the grace period.
    let output = Command::new("timeout")
        .env("LC_ALL", "C")
        .args(["--kill-after=0.25s", "2s", program])
        .args(args)
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => UnavailableReason::NotSupported,
            ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
            _ => UnavailableReason::TemporarilyUnavailable,
        })?;

    match output.status.code() {
        Some(127) => Err(UnavailableReason::NotSupported),
        Some(126) => Err(UnavailableReason::PermissionDenied),
        None | Some(124 | 137) => Err(UnavailableReason::TemporarilyUnavailable),
        _ => Ok(output),
    }
}

pub(super) fn read_iw(args: &[&str]) -> Result<String, UnavailableReason> {
    let iw = ["/usr/sbin/iw", "/sbin/iw"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .unwrap_or("iw");

    let output = run(iw, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        return Err(
            if stderr.contains("operation not permitted") || stderr.contains("permission denied") {
                UnavailableReason::PermissionDenied
            } else if stderr.contains("operation not supported") {
                UnavailableReason::NotSupported
            } else if stderr.contains("no such device") {
                UnavailableReason::HardwareUnavailable
            } else if stderr.contains("no such file or directory") {
                UnavailableReason::NotMeasured
            } else {
                UnavailableReason::TemporarilyUnavailable
            },
        );
    }

    String::from_utf8(output.stdout).map_err(|_| UnavailableReason::InvalidMeasurement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_the_lifetime_of_a_blocking_radio_command() {
        assert!(matches!(
            run("/bin/sleep", &["20"]),
            Err(UnavailableReason::TemporarilyUnavailable)
        ));
    }
}
