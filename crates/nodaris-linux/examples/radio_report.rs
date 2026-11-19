use std::{
    env,
    process::ExitCode,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use nodaris_core::models::{
    diagnostic_report::ReportSection,
    measurement::{MeasurementSessionId, ObservationTime},
    radio_observation::InterfaceId,
};
use nodaris_linux::collect_radio_report;

fn main() -> ExitCode {
    let Some(interface_name) = env::args().nth(1) else {
        eprintln!("Usage: radio_report <wifi-interface>");
        return ExitCode::from(2);
    };

    let interface = InterfaceId(interface_name);
    let started_at = Instant::now();
    let session_start_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());

    let report = collect_radio_report(
        &interface,
        || ObservationTime {
            unix_time_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok()),
            monotonic_time_ns: u64::try_from(started_at.elapsed().as_nanos()).unwrap_or(u64::MAX),
            session_id: MeasurementSessionId(format!(
                "radio-example-{}-{session_start_ns}",
                std::process::id()
            )),
        },
        250_000_000,
    );

    println!("Interface: {}", interface.0);

    match report.radio {
        ReportSection::Available(radio) => {
            let quality = radio
                .observation
                .signal_quality
                .as_ref()
                .and_then(|measurement| measurement.value());

            println!(
                "Frequency (MHz): {:?}",
                radio.observation.frequency.map(|f| f.0)
            );
            match quality {
                Some(value) => println!("Signal quality: {}%", value.value()),
                None => println!("Signal quality: unavailable"),
            }
            println!("Noise: {:?}", radio.observation.noise);
            println!("SNR: {:?}", radio.snr);
        }
        ReportSection::Unavailable(reason) => {
            println!("Radio: unavailable ({reason:?})");
        }
        ReportSection::NotCollected => {
            println!("Radio: not collected");
        }
    }

    println!("Sticky Client: {:?}", report.sticky_client);
    println!("Airtime: {:?}", report.airtime);
    println!("Retries: {:?}", report.retries);

    ExitCode::SUCCESS
}
