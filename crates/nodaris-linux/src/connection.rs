use std::time::{Duration, Instant};

use nodaris_core::{
    error::CoreError,
    keys::diagnostics,
    models::{
        connection_timeline::{ConnectionRun, summarize_connection},
        radio_observation::InterfaceId,
    },
};
use tokio::sync::Semaphore;

use crate::{observe_connection, session::RunClock};

static SLOT: Semaphore = Semaphore::const_new(1);
const SAMPLE_COUNT: usize = 21;
const SAMPLE_SPACING: Duration = Duration::from_secs(2);

pub async fn monitor_connection(interface: String) -> Result<ConnectionRun, CoreError> {
    monitor(interface, RunClock::new()).await
}

pub(super) async fn monitor(
    interface: String,
    clock: RunClock,
) -> Result<ConnectionRun, CoreError> {
    let mut permit = SLOT
        .try_acquire()
        .map_err(|_| CoreError::network_err(diagnostics::CONNECTION_BUSY))?;
    let started = Instant::now();
    let mut observations = Vec::with_capacity(SAMPLE_COUNT);
    let mut fresh_since_ns = None;

    for index in 0..SAMPLE_COUNT {
        if index > 0 {
            tokio::time::sleep(SAMPLE_SPACING).await;
        }
        let id = InterfaceId(interface.clone());
        let clock = clock.clone();
        let (observation, slot) = tokio::task::spawn_blocking(move || {
            let observation = observe_connection(id, fresh_since_ns, || clock.now());
            (observation, permit)
        })
        .await
        .map_err(|error| CoreError::internal(format!("Connection worker: {error}")))?;
        permit = slot;
        fresh_since_ns = Some(observation.neighbors.observed_at.monotonic_time_ns);
        observations.push(observation);
    }

    Ok(ConnectionRun {
        interface,
        elapsed_ms: started.elapsed().as_millis() as u64,
        timeline: summarize_connection(&observations),
    })
}
