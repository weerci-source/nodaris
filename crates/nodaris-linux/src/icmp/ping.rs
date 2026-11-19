use std::{net::Ipv4Addr, process::Output, time::Duration};

use nodaris_core::models::icmp::{IcmpOutcome, ProbeUnavailable};

use super::process;

const PING_TIMEOUT: Duration = Duration::from_millis(2500);

pub(super) async fn probe(target: Ipv4Addr) -> IcmpOutcome {
    let result = process::run(
        "ping",
        &[
            "-4",
            "-n",
            "-c",
            "1",
            "-W",
            "1",
            "-w",
            "2",
            "--",
            &target.to_string(),
        ],
        PING_TIMEOUT,
    )
    .await
    .and_then(|output| {
        if process::permission_error(&output) {
            return Err(ProbeUnavailable::PermissionDenied);
        }
        if !output.stderr.is_empty() {
            return Err(ProbeUnavailable::MeasurementFailed);
        }
        parse_output(&output)
    });
    result.unwrap_or_else(|reason| IcmpOutcome::Unavailable { reason })
}

fn parse_output(output: &Output) -> Result<IcmpOutcome, ProbeUnavailable> {
    let text = std::str::from_utf8(&output.stdout).map_err(|_| ProbeUnavailable::InvalidOutput)?;
    parse_ping(output.status.code(), text)
}

fn parse_ping(exit_code: Option<i32>, text: &str) -> Result<IcmpOutcome, ProbeUnavailable> {
    let Some(summary) = text
        .lines()
        .find_map(|line| line.strip_prefix("1 packets transmitted, "))
    else {
        return Err(match exit_code {
            Some(0 | 1) => ProbeUnavailable::InvalidOutput,
            _ => ProbeUnavailable::MeasurementFailed,
        });
    };
    match exit_code {
        Some(0) if summary.starts_with("1 received,") => {
            let rtt_ms = text
                .lines()
                .filter(|line| line.contains("icmp_seq=1 "))
                .find_map(|line| line.split_once(" time=").map(|(_, tail)| tail))
                .and_then(|tail| tail.split_once(" ms").map(|(rtt, _)| rtt))
                .and_then(|rtt| rtt.parse::<f64>().ok())
                .filter(|rtt| rtt.is_finite() && *rtt >= 0.0)
                .ok_or(ProbeUnavailable::InvalidOutput)?;
            Ok(IcmpOutcome::Reply { rtt_ms })
        }
        Some(1) if summary.starts_with("0 received,") => {
            if summary.split(',').any(|field| field.trim() == "+1 errors") {
                Ok(IcmpOutcome::IcmpError)
            } else {
                Ok(IcmpOutcome::NoReply)
            }
        }
        _ => Err(ProbeUnavailable::InvalidOutput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_float_rtt_and_timeout_without_rtt() {
        let reply = "64 bytes from 192.0.2.1: icmp_seq=1 ttl=64 time=0.231 ms\n\
                     1 packets transmitted, 1 received, 0% packet loss, time 0ms";
        assert_eq!(
            parse_ping(Some(0), reply),
            Ok(IcmpOutcome::Reply { rtt_ms: 0.231 })
        );
        let no_reply = "1 packets transmitted, 0 received, 100% packet loss, time 0ms";
        assert_eq!(parse_ping(Some(1), no_reply), Ok(IcmpOutcome::NoReply));
        let icmp_error = "1 packets transmitted, 0 received, +1 errors, 100% packet loss, time 0ms";
        assert_eq!(parse_ping(Some(1), icmp_error), Ok(IcmpOutcome::IcmpError));
    }

    #[test]
    fn does_not_count_a_startup_failure_as_packet_loss() {
        assert_eq!(
            parse_ping(Some(2), ""),
            Err(ProbeUnavailable::MeasurementFailed)
        );
        assert_eq!(
            parse_ping(Some(1), "0 packets transmitted, 0 received"),
            Err(ProbeUnavailable::InvalidOutput)
        );
    }
}
