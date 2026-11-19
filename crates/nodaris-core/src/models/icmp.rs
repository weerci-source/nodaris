use std::net::Ipv4Addr;

use serde::Serialize;

use crate::providers::network_probe::ProbeMethod;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ProbeRoute {
    pub interface: String,
    pub gateway: Option<Ipv4Addr>,
    pub source: Option<Ipv4Addr>,
    pub table: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeUnavailable {
    MissingGateway,
    RouteUnavailable,
    RouteMismatch,
    RouteChanged,
    ToolUnavailable,
    PermissionDenied,
    ProcessTimeout,
    MeasurementFailed,
    InvalidOutput,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IcmpOutcome {
    Reply { rtt_ms: f64 },
    NoReply,
    IcmpError,
    Unavailable { reason: ProbeUnavailable },
}

#[derive(Debug, PartialEq, Serialize)]
pub struct IcmpAttempt {
    pub started_offset_ms: u64,
    pub finished_offset_ms: u64,
    pub outcome: IcmpOutcome,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct IcmpStatistics {
    pub sent: u32,
    pub received: u32,
    pub min_rtt_ms: Option<f64>,
    pub avg_rtt_ms: Option<f64>,
    pub max_rtt_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub packet_loss_percent: f64,
}

impl IcmpStatistics {
    pub fn from_attempts(attempts: &[IcmpAttempt]) -> Option<Self> {
        let mut sent = 0;
        let mut received = 0;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut total = 0.0;
        let mut jitter = 0.0;
        let mut pairs = 0;
        let mut previous: Option<f64> = None;

        for attempt in attempts {
            match attempt.outcome {
                IcmpOutcome::Reply { rtt_ms } => {
                    sent += 1;
                    received += 1;
                    min = min.min(rtt_ms);
                    max = max.max(rtt_ms);
                    total += rtt_ms;
                    if let Some(previous) = previous {
                        jitter += (rtt_ms - previous).abs();
                        pairs += 1;
                    }
                    previous = Some(rtt_ms);
                }
                IcmpOutcome::NoReply | IcmpOutcome::IcmpError => {
                    sent += 1;
                    previous = None;
                }
                IcmpOutcome::Unavailable { .. } => previous = None,
            }
        }

        if sent == 0 {
            return None;
        }

        Some(Self {
            sent,
            received,
            min_rtt_ms: (received > 0).then_some(min),
            avg_rtt_ms: (received > 0).then(|| total / f64::from(received)),
            max_rtt_ms: (received > 0).then_some(max),
            jitter_ms: (pairs > 0).then(|| jitter / f64::from(pairs)),
            packet_loss_percent: 100.0 * f64::from(sent - received) / f64::from(sent),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct IcmpSeries {
    pub target: Option<Ipv4Addr>,
    pub route: Option<ProbeRoute>,
    pub started_offset_ms: u64,
    pub finished_offset_ms: u64,
    pub attempts: Vec<IcmpAttempt>,
    pub statistics: Option<IcmpStatistics>,
    pub unavailable_reason: Option<ProbeUnavailable>,
}

#[derive(Debug, Serialize)]
pub struct IcmpReport {
    pub method: ProbeMethod,
    pub interface: String,
    pub started_at_unix_ms: u64,
    pub gateway: IcmpSeries,
    pub external: IcmpSeries,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempts(outcomes: Vec<IcmpOutcome>) -> Vec<IcmpAttempt> {
        outcomes
            .into_iter()
            .map(|outcome| IcmpAttempt {
                started_offset_ms: 0,
                finished_offset_ms: 1,
                outcome,
            })
            .collect()
    }

    #[test]
    fn computes_jitter_only_between_adjacent_replies() {
        let statistics = IcmpStatistics::from_attempts(&attempts(vec![
            IcmpOutcome::Reply { rtt_ms: 2.0 },
            IcmpOutcome::Reply { rtt_ms: 5.0 },
            IcmpOutcome::NoReply,
            IcmpOutcome::Reply { rtt_ms: 20.0 },
        ]))
        .unwrap();
        assert_eq!(statistics.sent, 4);
        assert_eq!(statistics.received, 3);
        assert_eq!(statistics.min_rtt_ms, Some(2.0));
        assert_eq!(statistics.avg_rtt_ms, Some(9.0));
        assert_eq!(statistics.max_rtt_ms, Some(20.0));
        assert_eq!(statistics.jitter_ms, Some(3.0));
        assert_eq!(statistics.packet_loss_percent, 25.0);
    }

    #[test]
    fn distinguishes_no_reply_from_no_packets_sent() {
        let statistics = IcmpStatistics::from_attempts(&attempts(vec![IcmpOutcome::NoReply]))
            .expect("sent a packet");
        assert_eq!(statistics.packet_loss_percent, 100.0);
        assert_eq!(statistics.avg_rtt_ms, None);

        assert_eq!(
            IcmpStatistics::from_attempts(&attempts(vec![IcmpOutcome::Unavailable {
                reason: ProbeUnavailable::PermissionDenied,
            }])),
            None,
        );
    }
}
