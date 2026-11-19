use super::airtime::AirtimeCalculation;
use super::measurement::UnavailableReason;
use super::radio_observation::{RadioObservation, SnrResult};
use super::retries::RetryCalculation;
use super::sticky_client::StickyClientReport;

/// Состояние секции в рамках одного запуска диагностики.
#[derive(Debug, Clone, PartialEq)]
pub enum ReportSection<T> {
    /// Сбор этой секции не запускался.
    NotCollected,

    /// Секцию пытались собрать, но данные получить не удалось.
    Unavailable(UnavailableReason),

    /// Секция собрана. Отдельные метрики внутри неё могут быть недоступны.
    Available(T),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadioReport {
    pub observation: RadioObservation,
    pub snr: SnrResult,
}

impl RadioReport {
    pub fn from_observation(
        observation: RadioObservation,
        max_snr_time_difference_ns: u64,
    ) -> Self {
        let snr = observation.calculate_snr(max_snr_time_difference_ns);

        Self { observation, snr }
    }
}

/// Результат одного запуска диагностики.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticReport {
    pub radio: ReportSection<RadioReport>,
    pub sticky_client: ReportSection<StickyClientReport>,
    pub airtime: ReportSection<AirtimeCalculation>,
    pub retries: ReportSection<RetryCalculation>,
}

impl DiagnosticReport {
    /// Собирает отчёт по радионаблюдению. Остальные секции не запускались.
    pub fn from_radio(
        radio: ReportSection<RadioObservation>,
        max_snr_time_difference_ns: u64,
    ) -> Self {
        let radio = match radio {
            ReportSection::NotCollected => ReportSection::NotCollected,
            ReportSection::Unavailable(reason) => ReportSection::Unavailable(reason),
            ReportSection::Available(observation) => ReportSection::Available(
                RadioReport::from_observation(observation, max_snr_time_difference_ns),
            ),
        };

        Self {
            radio,
            sticky_client: ReportSection::NotCollected,
            airtime: ReportSection::NotCollected,
            retries: ReportSection::NotCollected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::radio_observation::{FrequencyMhz, InterfaceId, SnrUnavailableReason};

    #[test]
    fn keeps_radio_section_when_snr_is_unavailable() {
        let observation = RadioObservation {
            interface: InterfaceId("wlan0".into()),
            bssid: None,
            frequency: Some(FrequencyMhz(5260)),
            rssi: None,
            noise: None,
            signal_quality: None,
        };

        let report = DiagnosticReport::from_radio(ReportSection::Available(observation), 1_000_000);

        let ReportSection::Available(radio) = report.radio else {
            panic!("Expected an available radio section");
        };

        assert_eq!(
            radio.snr,
            SnrResult::Unavailable(SnrUnavailableReason::RssiUnavailable),
        );
        assert_eq!(radio.observation.frequency, Some(FrequencyMhz(5260)));
        assert_eq!(report.sticky_client, ReportSection::NotCollected);
        assert_eq!(report.airtime, ReportSection::NotCollected);
        assert_eq!(report.retries, ReportSection::NotCollected);
    }

    #[test]
    fn preserves_radio_collection_failure() {
        let report = DiagnosticReport::from_radio(
            ReportSection::Unavailable(UnavailableReason::PermissionDenied),
            1_000_000,
        );

        assert_eq!(
            report.radio,
            ReportSection::Unavailable(UnavailableReason::PermissionDenied),
        );
    }

    #[test]
    fn keeps_sticky_client_report_when_observations_are_insufficient() {
        use crate::models::sticky_client::{
            StickyClientAssessment, StickyClientConfig, StickyClientUnavailableReason,
            assess_sticky_client_report,
        };

        let report = DiagnosticReport {
            radio: ReportSection::NotCollected,
            sticky_client: ReportSection::Available(assess_sticky_client_report(
                &[],
                &StickyClientConfig::default(),
            )),
            airtime: ReportSection::NotCollected,
            retries: ReportSection::NotCollected,
        };

        let ReportSection::Available(sticky_client) = report.sticky_client else {
            panic!("Expected a completed Sticky Client assessment");
        };

        assert_eq!(
            sticky_client.assessment,
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::InsufficientObservations,
            ),
        );
        assert_eq!(sticky_client.observation_count, 0);
    }

    #[test]
    fn keeps_other_sections_when_airtime_cannot_be_calculated() {
        use crate::models::airtime::AirtimeUnavailableReason;
        use crate::models::radio_observation::{FrequencyMhz, InterfaceId};
        use crate::models::sticky_client::{StickyClientConfig, assess_sticky_client_report};

        let radio = RadioReport::from_observation(
            RadioObservation {
                interface: InterfaceId("wlan0".into()),
                bssid: None,
                frequency: Some(FrequencyMhz(5260)),
                rssi: None,
                noise: None,
                signal_quality: None,
            },
            1_000_000,
        );
        let sticky_client = assess_sticky_client_report(&[], &StickyClientConfig::default());

        let report = DiagnosticReport {
            radio: ReportSection::Available(radio.clone()),
            sticky_client: ReportSection::Available(sticky_client.clone()),
            airtime: ReportSection::Available(AirtimeCalculation::Unavailable(
                AirtimeUnavailableReason::CounterReset,
            )),
            retries: ReportSection::NotCollected,
        };

        assert_eq!(report.radio, ReportSection::Available(radio));
        assert_eq!(
            report.sticky_client,
            ReportSection::Available(sticky_client),
        );
        assert_eq!(
            report.airtime,
            ReportSection::Available(AirtimeCalculation::Unavailable(
                AirtimeUnavailableReason::CounterReset,
            )),
        );
    }

    #[test]
    fn keeps_retry_result_when_airtime_is_unavailable() {
        use crate::models::retries::{RetryCounterSemantics, RetryResult};

        let retries = RetryCalculation::Available(RetryResult {
            retries_delta: 20,
            attempts_delta: 100,
            failed_delta: None,
            retry_rate_percent: Some(20),
            interval_ns: 1_000_000_000,
            semantics: RetryCounterSemantics::RetryAttempts,
        });

        let report = DiagnosticReport {
            radio: ReportSection::NotCollected,
            sticky_client: ReportSection::NotCollected,
            airtime: ReportSection::Unavailable(UnavailableReason::NotSupported),
            retries: ReportSection::Available(retries.clone()),
        };

        assert_eq!(report.retries, ReportSection::Available(retries));
        assert_eq!(
            report.airtime,
            ReportSection::Unavailable(UnavailableReason::NotSupported),
        );
    }
}
