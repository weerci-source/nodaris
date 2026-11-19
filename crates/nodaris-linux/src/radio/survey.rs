use std::{thread, time::Duration};

use nodaris_core::models::{
    airtime::{
        AirtimeCalculation, ChannelSurveyCounters, ChannelSurveySnapshot, calculate_airtime,
    },
    diagnostic_report::ReportSection,
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio::NoiseDbm,
    radio_observation::{FrequencyMhz, InterfaceId},
};

use super::{
    command,
    iw_link::{self, IwLink},
};

const SAMPLE_GAP: Duration = Duration::from_secs(1);

struct Survey {
    noise: Result<NoiseDbm, UnavailableReason>,
    counters: Result<ChannelSurveyCounters, UnavailableReason>,
}

pub(super) struct SurveyReport {
    pub(super) noise: Measurement<NoiseDbm>,
    pub(super) airtime: ReportSection<AirtimeCalculation>,
}

impl SurveyReport {
    pub(super) fn unavailable(reason: UnavailableReason, observed_at: ObservationTime) -> Self {
        Self {
            noise: Measurement::unavailable(reason, source(), observed_at),
            airtime: ReportSection::Unavailable(reason),
        }
    }
}

pub(super) fn collect(
    interface: &InterfaceId,
    link: &IwLink,
    read_time: impl Fn() -> ObservationTime,
) -> SurveyReport {
    let first = read(interface, link.frequency_mhz);
    let first_time = read_time();
    let first = match first {
        Ok(survey) => survey,
        Err(reason) => return SurveyReport::unavailable(reason, first_time),
    };
    let needs_confirmation = first.noise.is_ok() || first.counters.is_ok();

    let airtime = match first.counters {
        Ok(counters) => {
            let first = snapshot(interface, link, counters, first_time.clone());
            thread::sleep(SAMPLE_GAP);
            let last = read(interface, link.frequency_mhz);
            let last_time = read_time();
            match last.and_then(|survey| survey.counters) {
                Ok(counters) => ReportSection::Available(calculate_airtime(
                    &first,
                    &snapshot(interface, link, counters, last_time),
                )),
                Err(reason) => ReportSection::Unavailable(reason),
            }
        }
        Err(reason) => ReportSection::Unavailable(reason),
    };

    // The records identify the interface and in-use channel. Bracket the
    // readings with link observations to catch a disconnect or change of AP.
    if needs_confirmation {
        match iw_link::read_link(&interface.0) {
            Ok(current) if link.same_context(&current) => {}
            Ok(_) | Err(UnavailableReason::NotMeasured) => {
                return SurveyReport::unavailable(UnavailableReason::ContextChanged, first_time);
            }
            Err(reason) => return SurveyReport::unavailable(reason, first_time),
        }
    }

    let noise = match first.noise {
        Ok(value) => Measurement::available(value, source(), first_time),
        Err(reason) => Measurement::unavailable(reason, source(), first_time),
    };
    SurveyReport { noise, airtime }
}

fn source() -> SourceId {
    SourceId("linux.iw.survey".into())
}

fn snapshot(
    interface: &InterfaceId,
    link: &IwLink,
    counters: ChannelSurveyCounters,
    observed_at: ObservationTime,
) -> ChannelSurveySnapshot {
    ChannelSurveySnapshot {
        interface: interface.clone(),
        frequency: FrequencyMhz(link.frequency_mhz),
        counters: Measurement::available(counters, source(), observed_at),
    }
}

fn read(interface: &InterfaceId, frequency: u32) -> Result<Survey, UnavailableReason> {
    let output = command::read_iw(&["dev", &interface.0, "survey", "dump"])?;
    parse(&output, &interface.0, frequency)
}

fn parse(output: &str, interface: &str, frequency: u32) -> Result<Survey, UnavailableReason> {
    let frequency = frequency.to_string();
    let mut active = output
        .split("Survey data from ")
        .skip(1)
        .filter_map(|record| {
            let (device, fields) = record.split_once('\n')?;
            (device.trim() == interface).then_some(fields)
        })
        .filter(|fields| {
            fields.lines().any(|line| {
                line.split_ascii_whitespace()
                    .eq(["frequency:", &frequency, "MHz", "[in", "use]"])
            })
        });

    let record = active.next().ok_or(UnavailableReason::NotMeasured)?;
    if active.next().is_some() {
        return Err(UnavailableReason::InvalidMeasurement);
    }

    Ok(Survey {
        noise: field(record, "noise:", "dBm", |value| {
            let dbm = value.parse::<i8>().ok()?;
            (dbm < 0).then_some(NoiseDbm::new(i16::from(dbm)))
        }),
        counters: counters(record),
    })
}

fn counters(record: &str) -> Result<ChannelSurveyCounters, UnavailableReason> {
    let time_us = |key| {
        field(record, key, "ms", |value| {
            value.parse::<u64>().ok()?.checked_mul(1_000)
        })
    };
    Ok(ChannelSurveyCounters {
        total_time_us: time_us("channel active time:")?,
        busy_time_us: time_us("channel busy time:")?,
        rx_time_us: None,
        tx_time_us: None,
    })
}

fn field<T>(
    record: &str,
    key: &str,
    unit: &str,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<T, UnavailableReason> {
    let mut fields = record
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(key));
    let raw = fields.next().ok_or(UnavailableReason::NotMeasured)?;
    let mut parts = raw.split_ascii_whitespace();
    match (parts.next(), parts.next(), parts.next(), fields.next()) {
        (Some(value), Some(found_unit), None, None) if found_unit == unit => {
            parse(value).ok_or(UnavailableReason::InvalidMeasurement)
        }
        _ => Err(UnavailableReason::InvalidMeasurement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodaris_core::models::measurement::MeasurementSessionId;

    const FIRST: &str = include_str!("fixtures/survey-first.txt");
    const SECOND: &str = include_str!("fixtures/survey-second.txt");

    #[test]
    fn normalizes_samples_and_calculates_deltas_for_the_in_use_channel() {
        let interface = InterfaceId("wlan0".into());
        let link = IwLink {
            bssid: "02:00:00:00:00:01".into(),
            frequency_mhz: 5260,
            rssi_dbm: Some(-67),
        };
        let sample = |output, time| {
            let survey = parse(output, &interface.0, link.frequency_mhz).unwrap();
            assert_eq!(survey.noise, Ok(NoiseDbm::new(-95)));
            snapshot(
                &interface,
                &link,
                survey.counters.unwrap(),
                ObservationTime {
                    unix_time_ms: None,
                    monotonic_time_ns: time,
                    session_id: MeasurementSessionId("survey-test".into()),
                },
            )
        };

        let AirtimeCalculation::Available(result) =
            calculate_airtime(&sample(FIRST, 1_000), &sample(SECOND, 1_000_001_000))
        else {
            panic!("Expected airtime from the paired survey records");
        };
        assert_eq!(result.busy_delta_us, 250_000);
        assert_eq!(result.total_delta_us, 1_000_000);
        assert_eq!(result.interval_ns, 1_000_000_000);
        assert_eq!(result.utilization.percent(), 25.0);
    }

    #[test]
    fn ignores_empty_output_and_records_for_another_channel_or_interface() {
        for (output, name, channel) in [
            ("", "wlan0", 5260),
            (FIRST, "wlan0", 5180),
            (FIRST, "wlan2", 5260),
        ] {
            assert!(matches!(
                parse(output, name, channel),
                Err(UnavailableReason::NotMeasured)
            ));
        }
    }

    #[test]
    fn reports_noise_and_counter_failures_independently() {
        let noise_only =
            FIRST.replace("channel busy time:\t3000 ms", "channel busy time:\t3000 us");
        let survey = parse(&noise_only, "wlan0", 5260).unwrap();
        assert_eq!(survey.noise, Ok(NoiseDbm::new(-95)));
        assert_eq!(survey.counters, Err(UnavailableReason::InvalidMeasurement));

        let counters_only = FIRST.replace("noise:\t-95 dBm", "noise:\t0 dBm");
        let survey = parse(&counters_only, "wlan0", 5260).unwrap();
        assert_eq!(survey.noise, Err(UnavailableReason::InvalidMeasurement));
        assert!(survey.counters.is_ok());
    }

    #[test]
    fn rejects_duplicate_active_records() {
        assert!(matches!(
            parse(&format!("{FIRST}{FIRST}"), "wlan0", 5260),
            Err(UnavailableReason::InvalidMeasurement)
        ));
    }
}
