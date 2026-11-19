use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use nodaris_core::{
    error::CoreError,
    input::{external_unicast, valid_interface_name},
    keys::diagnostics,
    models::{
        measurement::{MeasurementSessionId, ObservationTime, UnavailableReason},
        radio_observation::InterfaceId,
        session::{DiagnosticPlatform, DiagnosticSession, SessionSection, hypotheses},
    },
    radio_api::encode_radio,
};
use tokio::sync::Semaphore;

use crate::{collect_icmp_report, collect_radio_report, connection};

static RADIO_SLOT: Semaphore = Semaphore::const_new(1);
static SLOT: Semaphore = Semaphore::const_new(1);
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub fn try_radio_slot() -> Result<tokio::sync::SemaphorePermit<'static>, CoreError> {
    RADIO_SLOT
        .try_acquire()
        .map_err(|_| CoreError::network_err(diagnostics::RADIO_BUSY))
}

#[derive(Clone)]
pub(super) struct RunClock {
    start: Instant,
    id: MeasurementSessionId,
}

impl RunClock {
    pub(super) fn new() -> Self {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |value| value.as_millis());
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            start: Instant::now(),
            id: MeasurementSessionId(format!("linux-{}-{time}-{sequence}", std::process::id())),
        }
    }

    pub(super) fn now(&self) -> ObservationTime {
        ObservationTime {
            unix_time_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|value| i64::try_from(value.as_millis()).ok()),
            monotonic_time_ns: u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX),
            session_id: self.id.clone(),
        }
    }
}

pub async fn collect_session(
    interface: String,
    target: Option<String>,
) -> Result<DiagnosticSession, CoreError> {
    if !valid_interface_name(&interface) {
        return Err(CoreError::validation(diagnostics::INVALID_INTERFACE));
    }
    let target = target
        .map(|address| address.parse::<Ipv4Addr>())
        .transpose()
        .map_err(|_| CoreError::validation(diagnostics::SESSION_INVALID_TARGET))?;
    if target.is_some_and(|address| !external_unicast(address)) {
        return Err(CoreError::validation(diagnostics::SESSION_INVALID_TARGET));
    }
    let _permit = SLOT
        .try_acquire()
        .map_err(|_| CoreError::network_err(diagnostics::SESSION_BUSY))?;
    let clock = RunClock::new();
    let started_at = clock.now();
    let id = InterfaceId(interface.clone());
    let radio_clock = clock.clone();
    let id_for_response = interface.clone();
    let radio_job = async {
        let Ok(permit) = try_radio_slot() else {
            return SessionSection::Unavailable {
                reason: UnavailableReason::TemporarilyUnavailable,
            };
        };
        match tokio::task::spawn_blocking(move || {
            let _permit = permit;
            collect_radio_report(&id, || radio_clock.now(), 250_000_000)
        })
        .await
        {
            Ok(report) => SessionSection::Available {
                data: encode_radio(id_for_response, report),
            },
            Err(error) => {
                let _ = CoreError::internal(format!("Session radio worker: {error}")).into_api();
                SessionSection::Unavailable {
                    reason: UnavailableReason::TemporarilyUnavailable,
                }
            }
        }
    };

    let (radio, probes, connection) = tokio::join!(
        radio_job,
        async {
            match target {
                Some(target) => collect_icmp_report(interface.clone(), target).await.map_or(
                    SessionSection::Unavailable {
                        reason: UnavailableReason::TemporarilyUnavailable,
                    },
                    |data| SessionSection::Available { data },
                ),
                None => SessionSection::NotCollected,
            }
        },
        connection::monitor(interface.clone(), clock.clone())
    );

    let connection = match connection {
        Ok(data) => SessionSection::Available { data },
        Err(error) => {
            let _ = error.into_api();
            SessionSection::Unavailable {
                reason: UnavailableReason::TemporarilyUnavailable,
            }
        }
    };
    let findings = hypotheses(
        match &probes {
            SessionSection::Available { data } => Some(data),
            _ => None,
        },
        match &connection {
            SessionSection::Available { data } => Some(&data.timeline.sticky_client),
            _ => None,
        },
    );
    Ok(DiagnosticSession {
        id: clock.id.clone(),
        interface: interface.clone(),
        platform: DiagnosticPlatform::Linux,
        started_at,
        finished_at: clock.now(),
        rules_version: 1,
        radio,
        probes,
        connection,
        hypotheses: findings,
    })
}
