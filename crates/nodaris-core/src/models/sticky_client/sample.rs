use super::super::connection::{ConnectionObservation, ConnectionState, Ssid};
use super::super::measurement::{Measurement, MeasurementSessionId, MeasurementState, SourceId};
use super::super::radio_observation::{Bssid, InterfaceId};

use super::{NeighborMeasurementIssue, StickyClientConfig, StickyClientUnavailableReason};

#[derive(Debug, Clone)]
pub(in super::super) struct NeighborSignal {
    pub(in super::super) bssid: Bssid,
    pub(in super::super) rssi_dbm: i16,
}

#[derive(Debug, Clone)]
pub(in super::super) struct Sample {
    pub(in super::super) interface: InterfaceId,
    pub(in super::super) ssid: Ssid,
    pub(in super::super) connected_bssid: Bssid,
    pub(in super::super) current_rssi_dbm: i16,
    pub(in super::super) neighbors: Vec<NeighborSignal>,
    pub(in super::super) monotonic_time_ns: u64,
    pub(in super::super) neighbor_issues: Vec<NeighborMeasurementIssue>,
}

/// Получает значение измерения и проверяет его происхождение и актуальность.
fn measurement_value<'a, T>(
    measurement: &'a Measurement<T>,
    reference_source: &SourceId,
    reference_session: &MeasurementSessionId,
    reference_time_ns: u64,
    config: &StickyClientConfig,
) -> Result<&'a T, StickyClientUnavailableReason> {
    if &measurement.source != reference_source {
        return Err(StickyClientUnavailableReason::DifferentSources);
    }

    if &measurement.observed_at.session_id != reference_session {
        return Err(StickyClientUnavailableReason::DifferentSessions);
    }

    let measured_at = measurement.observed_at.monotonic_time_ns;

    if measured_at <= reference_time_ns {
        let age = reference_time_ns - measured_at;

        if age > config.max_measurement_age_ns {
            return Err(StickyClientUnavailableReason::StaleMeasurement);
        }
    } else {
        let future_offset = measured_at - reference_time_ns;

        if future_offset > config.max_measurement_age_ns {
            return Err(StickyClientUnavailableReason::FutureMeasurement);
        }
    }

    match &measurement.state {
        MeasurementState::Available(value) => Ok(value),
        MeasurementState::Unavailable(reason) => Err(
            StickyClientUnavailableReason::MeasurementUnavailable(*reason),
        ),
    }
}

/// Преобразует снимок подключения в пригодное для анализа наблюдение.
pub(super) fn extract_sample(
    observation: &ConnectionObservation,
    config: &StickyClientConfig,
) -> Result<Sample, StickyClientUnavailableReason> {
    let reference_source = &observation.state.source;
    let reference_session = &observation.state.observed_at.session_id;
    let reference_time_ns = observation.state.observed_at.monotonic_time_ns;

    let connected = measurement_value(
        &observation.state,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let connected = match connected {
        ConnectionState::Connected(access_point) => access_point,
        ConnectionState::Disconnected => {
            return Err(StickyClientUnavailableReason::NotConnected);
        }
    };

    let current_rssi = connected
        .rssi
        .as_ref()
        .ok_or(StickyClientUnavailableReason::CurrentRssiUnavailable)?;

    let current_rssi = measurement_value(
        current_rssi,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let neighbors = measurement_value(
        &observation.neighbors,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let mut neighbor_signals = Vec::new();
    let mut neighbor_issues = Vec::new();

    for neighbor in neighbors {
        if neighbor.ssid != connected.ssid || neighbor.bssid == connected.bssid {
            continue;
        }

        let Some(rssi_measurement) = neighbor.rssi.as_ref() else {
            neighbor_issues.push(NeighborMeasurementIssue {
                bssid: neighbor.bssid.clone(),
                reason: StickyClientUnavailableReason::NeighborRssiUnavailable,
            });
            continue;
        };

        let rssi = match measurement_value(
            rssi_measurement,
            reference_source,
            reference_session,
            reference_time_ns,
            config,
        ) {
            Ok(rssi) => rssi,
            Err(reason) => {
                neighbor_issues.push(NeighborMeasurementIssue {
                    bssid: neighbor.bssid.clone(),
                    reason,
                });
                continue;
            }
        };

        neighbor_signals.push(NeighborSignal {
            bssid: neighbor.bssid.clone(),
            rssi_dbm: rssi.as_dbm(),
        });
    }

    Ok(Sample {
        interface: observation.interface.clone(),
        ssid: connected.ssid.clone(),
        connected_bssid: connected.bssid.clone(),
        current_rssi_dbm: current_rssi.as_dbm(),
        neighbors: neighbor_signals,
        monotonic_time_ns: reference_time_ns,
        neighbor_issues,
    })
}
