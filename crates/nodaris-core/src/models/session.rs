use serde::Serialize;

use crate::radio_api::RadioResponse;

use super::{
    connection_timeline::ConnectionRun,
    icmp::IcmpReport,
    measurement::{MeasurementSessionId, ObservationTime, UnavailableReason},
    sticky_client::{StickyClientAssessment, StickyClientEvidence, StickyClientReport},
};

#[derive(Debug, Serialize)]
#[serde(tag = "key", rename_all = "snake_case")]
pub enum DiagnosticHypothesis {
    BeyondGateway {
        gateway_avg_rtt_ms: f64,
        external_avg_rtt_ms: f64,
        started_at_ms: u64,
        finished_at_ms: u64,
    },
    StickyClient {
        evidence: StickyClientEvidence,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPlatform {
    Linux,
    Windows,
    Android,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SessionSection<T> {
    Available { data: T },
    Unavailable { reason: UnavailableReason },
    NotCollected,
}

#[derive(Serialize)]
pub struct DiagnosticSession {
    pub id: MeasurementSessionId,
    pub interface: String,
    pub platform: DiagnosticPlatform,
    pub started_at: ObservationTime,
    pub finished_at: ObservationTime,
    pub rules_version: u8,
    pub radio: SessionSection<RadioResponse>,
    pub probes: SessionSection<IcmpReport>,
    pub connection: SessionSection<ConnectionRun>,
    pub hypotheses: Vec<DiagnosticHypothesis>,
}

pub fn hypotheses(
    probes: Option<&IcmpReport>,
    sticky: Option<&StickyClientReport>,
) -> Vec<DiagnosticHypothesis> {
    let mut findings = Vec::new();
    if let Some(finding) = probes.and_then(beyond_gateway) {
        findings.push(finding);
    }
    if let Some(StickyClientAssessment::Suspected(evidence)) =
        sticky.map(|report| &report.assessment)
    {
        findings.push(DiagnosticHypothesis::StickyClient {
            evidence: evidence.clone(),
        });
    }
    findings
}

fn beyond_gateway(report: &IcmpReport) -> Option<DiagnosticHypothesis> {
    let gateway = &report.gateway;
    let external = &report.external;
    let gateway_route = gateway.route.as_ref()?;
    let external_route = external.route.as_ref()?;
    if gateway_route.interface != report.interface
        || external_route.interface != report.interface
        || gateway.target.is_none()
        || external_route.gateway != gateway.target
        || gateway.unavailable_reason.is_some()
        || external.unavailable_reason.is_some()
    {
        return None;
    }
    let local = gateway.statistics.as_ref()?;
    let remote = external.statistics.as_ref()?;
    let gateway_avg_rtt_ms = local.avg_rtt_ms?;
    let external_avg_rtt_ms = remote.avg_rtt_ms?;
    if local.sent < 3
        || local.received != local.sent
        || remote.received < 3
        || local.jitter_ms? > 10.0
        || external_avg_rtt_ms - gateway_avg_rtt_ms < 50.0
    {
        return None;
    }
    Some(DiagnosticHypothesis::BeyondGateway {
        gateway_avg_rtt_ms,
        external_avg_rtt_ms,
        started_at_ms: report
            .started_at_unix_ms
            .saturating_add(gateway.started_offset_ms),
        finished_at_ms: report
            .started_at_unix_ms
            .saturating_add(external.finished_offset_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::icmp::{IcmpSeries, IcmpStatistics, ProbeRoute},
        providers::network_probe::ProbeMethod,
    };

    #[test]
    fn compares_stable_series_only_when_their_routes_are_compatible() {
        let router = "192.0.2.1".parse().unwrap();
        let series = |target, gateway, avg| IcmpSeries {
            target: Some(target),
            route: Some(ProbeRoute {
                interface: "wlan0".into(),
                gateway,
                source: None,
                table: None,
            }),
            started_offset_ms: 0,
            finished_offset_ms: 150,
            attempts: vec![],
            statistics: Some(IcmpStatistics {
                sent: 3,
                received: 3,
                min_rtt_ms: Some(avg),
                avg_rtt_ms: Some(avg),
                max_rtt_ms: Some(avg),
                jitter_ms: Some(0.0),
                packet_loss_percent: 0.0,
            }),
            unavailable_reason: None,
        };
        let mut report = IcmpReport {
            method: ProbeMethod::Icmp,
            interface: "wlan0".into(),
            started_at_unix_ms: 1_000,
            gateway: series(router, None, 4.0),
            external: series("198.51.100.1".parse().unwrap(), Some(router), 64.0),
        };
        assert!(matches!(
            hypotheses(Some(&report), None).as_slice(),
            [DiagnosticHypothesis::BeyondGateway { .. }]
        ));
        report.external.route.as_mut().unwrap().interface = "wg0".into();
        assert!(hypotheses(Some(&report), None).is_empty());
        report.external.route.as_mut().unwrap().interface = "wlan0".into();
        report.external.statistics = None;
        assert!(hypotheses(Some(&report), None).is_empty());
    }
}
