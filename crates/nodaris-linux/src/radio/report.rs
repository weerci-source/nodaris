use nodaris_core::models::{
    diagnostic_report::{DiagnosticReport, ReportSection},
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio::RssiDbm,
    radio_observation::{InterfaceId, RadioObservation},
    retries::{RetryCalculation, RetryUnavailableReason},
};

use super::{
    iw_link::{self, IwLink},
    nmcli::{self, ConnectedAp},
    station,
    survey::{self, SurveyReport},
};

/// Читает сведения об активной точке, RSSI, survey и TX-счётчики.
/// read_time фиксирует окончание чтения, а не момент измерения драйвером.
pub fn collect_radio_report(
    interface: &InterfaceId,
    read_time: impl Fn() -> ObservationTime,
    max_snr_time_difference_ns: u64,
) -> DiagnosticReport {
    let ap = match nmcli::read_connected_ap(interface) {
        Ok(ap) => ap,
        Err(reason) => {
            let mut report = DiagnosticReport::from_radio(
                ReportSection::Unavailable(reason),
                max_snr_time_difference_ns,
            );
            report.retries = ReportSection::Unavailable(reason);
            report.airtime = ReportSection::Unavailable(reason);
            return report;
        }
    };
    let link = iw_link::read_link(&interface.0);
    let observed_at = read_time();
    let survey = match &link {
        Ok(link) if matches_ap(&ap, link) => survey::collect(interface, link, &read_time),
        Ok(_) => SurveyReport::unavailable(UnavailableReason::ContextChanged, observed_at.clone()),
        Err(reason) => SurveyReport::unavailable(*reason, observed_at.clone()),
    };
    let retries = match &link {
        Ok(link) if matches_ap(&ap, link) => station::collect_retries(interface, link, &read_time),
        Ok(_) => ReportSection::Available(RetryCalculation::Unavailable(
            RetryUnavailableReason::ConnectionChanged,
        )),
        Err(reason) => ReportSection::Unavailable(*reason),
    };
    let mut observation = to_observation(interface, ap, link, observed_at);
    observation.noise = Some(survey.noise);
    let mut report = DiagnosticReport::from_radio(
        ReportSection::Available(observation),
        max_snr_time_difference_ns,
    );
    report.airtime = survey.airtime;
    report.retries = retries;
    report
}

pub(super) fn to_observation(
    interface: &InterfaceId,
    ap: ConnectedAp,
    link: Result<IwLink, UnavailableReason>,
    observed_at: ObservationTime,
) -> RadioObservation {
    let nmcli_source = SourceId("linux.network_manager".into());
    let iw_source = SourceId("linux.iw".into());

    let signal_quality = match ap.quality {
        Some(value) => Measurement::available(value, nmcli_source.clone(), observed_at.clone()),
        None => Measurement::unavailable(
            UnavailableReason::InvalidMeasurement,
            nmcli_source.clone(),
            observed_at.clone(),
        ),
    };

    let rssi = match link.and_then(|link| matched_rssi(&ap, &link)) {
        Ok(value) => Measurement::available(value, iw_source, observed_at.clone()),
        Err(reason) => Measurement::unavailable(reason, iw_source, observed_at.clone()),
    };

    RadioObservation {
        interface: interface.clone(),
        bssid: Some(ap.bssid),
        frequency: ap.frequency,
        rssi: Some(rssi),
        noise: None,
        signal_quality: Some(signal_quality),
    }
}

fn matched_rssi(ap: &ConnectedAp, link: &IwLink) -> Result<RssiDbm, UnavailableReason> {
    if !matches_ap(ap, link) {
        return Err(UnavailableReason::TemporarilyUnavailable);
    }
    link.rssi_dbm
        .map(RssiDbm::new)
        .ok_or(UnavailableReason::NotMeasured)
}

fn matches_ap(ap: &ConnectedAp, link: &IwLink) -> bool {
    ap.bssid.0.eq_ignore_ascii_case(&link.bssid)
        && ap
            .frequency
            .is_some_and(|frequency| frequency.0 == link.frequency_mhz)
}
