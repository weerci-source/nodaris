use nodaris_core::models::{
    measurement::{MeasurementSessionId, MeasurementState, ObservationTime, UnavailableReason},
    radio::{RssiDbm, SignalQualityPercent},
    radio_observation::{Bssid, FrequencyMhz, InterfaceId, RadioObservation},
};

use super::{iw_link::IwLink, nmcli::ConnectedAp, report::to_observation};

fn access_point() -> ConnectedAp {
    ConnectedAp {
        bssid: Bssid("AA:BB:CC:DD:EE:FF".into()),
        frequency: Some(FrequencyMhz(5260)),
        quality: Some(SignalQualityPercent::try_from(58).expect("valid signal quality")),
    }
}

fn link(bssid: &str, frequency_mhz: u32) -> IwLink {
    IwLink {
        bssid: bssid.into(),
        frequency_mhz,
        rssi_dbm: Some(-57),
    }
}

fn observe(ap: ConnectedAp, link: Result<IwLink, UnavailableReason>) -> RadioObservation {
    to_observation(
        &InterfaceId("wlan0".into()),
        ap,
        link,
        ObservationTime {
            unix_time_ms: None,
            monotonic_time_ns: 123,
            session_id: MeasurementSessionId("radio-test".into()),
        },
    )
}

fn assert_rssi_unavailable(observation: &RadioObservation, expected: UnavailableReason) {
    let rssi = observation.rssi.as_ref().expect("RSSI state");
    assert!(matches!(&rssi.state, MeasurementState::Unavailable(reason) if *reason == expected));
}

fn quality(observation: &RadioObservation) -> Option<u8> {
    observation
        .signal_quality
        .as_ref()
        .and_then(|measurement| measurement.value())
        .map(|value| value.value())
}

#[test]
fn accepts_rssi_from_the_same_ap_and_frequency() {
    let observation = observe(access_point(), Ok(link("aa:bb:cc:dd:ee:ff", 5260)));

    let rssi = observation.rssi.as_ref().expect("RSSI state");
    assert_eq!(rssi.value().copied().map(RssiDbm::as_dbm), Some(-57));
    assert_eq!(rssi.source.0, "linux.iw");
    assert_eq!(rssi.observed_at.monotonic_time_ns, 123);
    assert_eq!(quality(&observation), Some(58));

    let signal_quality = observation.signal_quality.as_ref().expect("quality state");
    assert_eq!(signal_quality.source.0, "linux.network_manager");

    let noise = observation.noise.as_ref().expect("noise state");
    assert!(matches!(
        &noise.state,
        MeasurementState::Unavailable(UnavailableReason::NotSupported)
    ));
}

#[test]
fn rejects_rssi_from_a_different_ap_on_the_same_frequency() {
    let observation = observe(access_point(), Ok(link("aa:bb:cc:dd:ee:01", 5260)));
    assert_rssi_unavailable(&observation, UnavailableReason::TemporarilyUnavailable);
    assert_eq!(quality(&observation), Some(58));
}

#[test]
fn rejects_rssi_from_the_same_ap_on_a_different_frequency() {
    let observation = observe(access_point(), Ok(link("aa:bb:cc:dd:ee:ff", 5180)));
    assert_rssi_unavailable(&observation, UnavailableReason::TemporarilyUnavailable);
    assert_eq!(observation.frequency, Some(FrequencyMhz(5260)));
}

#[test]
fn rejects_rssi_when_nmcli_did_not_report_a_frequency() {
    let mut ap = access_point();
    ap.frequency = None;
    let observation = observe(ap, Ok(link("aa:bb:cc:dd:ee:ff", 5260)));
    assert_rssi_unavailable(&observation, UnavailableReason::TemporarilyUnavailable);
    assert_eq!(observation.frequency, None);
    assert_eq!(quality(&observation), Some(58));
}

#[test]
fn preserves_nmcli_data_when_iw_is_unavailable() {
    let observation = observe(access_point(), Err(UnavailableReason::NotSupported));
    assert_rssi_unavailable(&observation, UnavailableReason::NotSupported);
    assert_eq!(observation.frequency, Some(FrequencyMhz(5260)));
    assert_eq!(quality(&observation), Some(58));
}

#[test]
fn keeps_rssi_when_nmcli_quality_is_invalid() {
    let mut ap = access_point();
    ap.quality = None;
    let observation = observe(ap, Ok(link("aa:bb:cc:dd:ee:ff", 5260)));
    assert_eq!(quality(&observation), None);
    assert_eq!(
        observation
            .rssi
            .as_ref()
            .and_then(|measurement| measurement.value())
            .copied()
            .map(RssiDbm::as_dbm),
        Some(-57)
    );
}
