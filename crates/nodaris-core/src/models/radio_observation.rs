use serde::{Deserialize, Serialize};

use super::measurement::{Measurement, MeasurementState};
use super::radio::{NoiseDbm, RssiDbm, SignalQualityPercent, SnrDb};

/// Идентификатор сетевого интерфейса.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterfaceId(pub String);

/// BSSID точки доступа.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bssid(pub String);

/// Частота радиоканала в MHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrequencyMhz(pub u32);

/// Причина, по которой невозможно вычислить SNR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnrUnavailableReason {
    RssiUnavailable,
    NoiseUnavailable,
    DifferentSessions,
    TimeMismatch,
    MissingFrequency,
}

/// Результат вычисления SNR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnrResult {
    Available(SnrDb),
    Unavailable(SnrUnavailableReason),
}

/// Радионаблюдение за конкретным интерфейсом.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadioObservation {
    /// Интерфейс, для которого получены данные.
    pub interface: InterfaceId,

    /// BSSID подключённой точки доступа.
    /// None означает, что BSSID неизвестен или подключения нет.
    pub bssid: Option<Bssid>,

    /// Частота радиоканала.
    pub frequency: Option<FrequencyMhz>,

    /// Фактическая мощность принятого сигнала.
    pub rssi: Option<Measurement<RssiDbm>>,

    /// Уровень шума.
    pub noise: Option<Measurement<NoiseDbm>>,

    /// Процентная оценка качества сигнала.
    pub signal_quality: Option<Measurement<SignalQualityPercent>>,
}

impl RadioObservation {
    /// Вычисляет SNR, если RSSI и Noise сопоставимы.
    ///
    /// max_time_difference_ns — максимально допустимая
    /// разница времени между измерениями.
    pub fn calculate_snr(&self, max_time_difference_ns: u64) -> SnrResult {
        let Some(frequency) = self.frequency else {
            return SnrResult::Unavailable(SnrUnavailableReason::MissingFrequency);
        };

        // Частота должна быть известна для обоих измерений.
        let _ = frequency;

        let Some(rssi) = &self.rssi else {
            return SnrResult::Unavailable(SnrUnavailableReason::RssiUnavailable);
        };

        let Some(noise) = &self.noise else {
            return SnrResult::Unavailable(SnrUnavailableReason::NoiseUnavailable);
        };

        let rssi_value = match &rssi.state {
            MeasurementState::Available(value) => value,
            MeasurementState::Unavailable(_) => {
                return SnrResult::Unavailable(SnrUnavailableReason::RssiUnavailable);
            }
        };

        let noise_value = match &noise.state {
            MeasurementState::Available(value) => value,
            MeasurementState::Unavailable(_) => {
                return SnrResult::Unavailable(SnrUnavailableReason::NoiseUnavailable);
            }
        };

        // Монотонное время можно сравнивать только
        // внутри одной сессии.
        if rssi.observed_at.session_id != noise.observed_at.session_id {
            return SnrResult::Unavailable(SnrUnavailableReason::DifferentSessions);
        }

        let rssi_time = rssi.observed_at.monotonic_time_ns;
        let noise_time = noise.observed_at.monotonic_time_ns;

        let time_difference = rssi_time.abs_diff(noise_time);

        if time_difference > max_time_difference_ns {
            return SnrResult::Unavailable(SnrUnavailableReason::TimeMismatch);
        }

        SnrResult::Available(SnrDb::new(
            i32::from(rssi_value.value()) - i32::from(noise_value.value()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::measurement::{
        MeasurementSessionId, ObservationTime, SourceId, UnavailableReason,
    };
    use super::*;

    fn observation_time(monotonic_time_ns: u64) -> ObservationTime {
        ObservationTime {
            unix_time_ms: Some(1_800_000_000_000),
            monotonic_time_ns,
            session_id: MeasurementSessionId("test-session".into()),
        }
    }

    fn make_observation(
        rssi: Measurement<RssiDbm>,
        noise: Measurement<NoiseDbm>,
        frequency: Option<FrequencyMhz>,
    ) -> RadioObservation {
        RadioObservation {
            interface: InterfaceId("wlan0".into()),
            bssid: Some(Bssid("AA:BB:CC:DD:EE:FF".into())),
            frequency,
            rssi: Some(rssi),
            noise: Some(noise),
            signal_quality: None,
        }
    }

    fn available_rssi(value: i16, time: u64) -> Measurement<RssiDbm> {
        Measurement::available(
            RssiDbm::new(value),
            SourceId("test.provider".into()),
            observation_time(time),
        )
    }

    fn available_noise(value: i16, time: u64) -> Measurement<NoiseDbm> {
        Measurement::available(
            NoiseDbm::new(value),
            SourceId("test.provider".into()),
            observation_time(time),
        )
    }

    #[test]
    fn calculates_snr_for_compatible_measurements() {
        let observation = make_observation(
            available_rssi(-53, 1_000_000),
            available_noise(-93, 1_500_000),
            Some(FrequencyMhz(5180)),
        );

        let result = observation.calculate_snr(1_000_000);

        assert_eq!(result, SnrResult::Available(SnrDb::new(40)));
    }

    #[test]
    fn rejects_unavailable_rssi() {
        let rssi = Measurement::unavailable(
            UnavailableReason::PermissionDenied,
            SourceId("test.provider".into()),
            observation_time(1_000_000),
        );

        let observation = make_observation(
            rssi,
            available_noise(-93, 1_000_000),
            Some(FrequencyMhz(5180)),
        );

        assert_eq!(
            observation.calculate_snr(1_000_000),
            SnrResult::Unavailable(SnrUnavailableReason::RssiUnavailable)
        );
    }

    #[test]
    fn rejects_unavailable_noise() {
        let noise = Measurement::unavailable(
            UnavailableReason::NotSupported,
            SourceId("test.provider".into()),
            observation_time(1_000_000),
        );

        let observation = make_observation(
            available_rssi(-53, 1_000_000),
            noise,
            Some(FrequencyMhz(5180)),
        );

        assert_eq!(
            observation.calculate_snr(1_000_000),
            SnrResult::Unavailable(SnrUnavailableReason::NoiseUnavailable)
        );
    }

    #[test]
    fn rejects_different_sessions() {
        let mut noise = available_noise(-93, 1_000_000);
        noise.observed_at.session_id = MeasurementSessionId("another-session".into());

        let observation = make_observation(
            available_rssi(-53, 1_000_000),
            noise,
            Some(FrequencyMhz(5180)),
        );

        assert_eq!(
            observation.calculate_snr(1_000_000),
            SnrResult::Unavailable(SnrUnavailableReason::DifferentSessions)
        );
    }

    #[test]
    fn rejects_time_mismatch() {
        let observation = make_observation(
            available_rssi(-53, 1_000_000),
            available_noise(-93, 5_000_000),
            Some(FrequencyMhz(5180)),
        );

        assert_eq!(
            observation.calculate_snr(1_000_000),
            SnrResult::Unavailable(SnrUnavailableReason::TimeMismatch)
        );
    }

    #[test]
    fn rejects_missing_frequency() {
        let observation = make_observation(
            available_rssi(-53, 1_000_000),
            available_noise(-93, 1_000_000),
            None,
        );

        assert_eq!(
            observation.calculate_snr(1_000_000),
            SnrResult::Unavailable(SnrUnavailableReason::MissingFrequency)
        );
    }
}
