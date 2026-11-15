use serde::{Deserialize, Serialize};

/// Мощность принятого радиосигнала в dBm.
///
/// Это физическое значение, а не процент качества сигнала.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RssiDbm(i16);

impl RssiDbm {
    pub const fn new(value: i16) -> Self {
        Self(value)
    }

    pub const fn as_dbm(self) -> i16 {
        self.0
    }
    pub const fn value(self) -> i16 {
        self.0
    }
}

/// Уровень шума в dBm.
///
/// Не путать с уровнем интерференции:
/// это разные характеристики радиосреды.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NoiseDbm(i16);

impl NoiseDbm {
    pub const fn new(value: i16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i16 {
        self.0
    }
}

/// Отношение сигнал/шум в dB.
///
/// Значение вычисляется как RSSI - Noise Floor.
/// Может быть отрицательным.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SnrDb(i32);

impl SnrDb {
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i32 {
        self.0
    }
}

/// Процентная оценка качества сигнала.
///
/// Например, поле SIGNAL из nmcli.
/// Не является RSSI и не измеряется в dBm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct SignalQualityPercent(u8);

impl SignalQualityPercent {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 100;

    pub const fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SignalQualityPercent {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > Self::MAX {
            return Err("Signal quality must be between 0 and 100");
        }

        Ok(Self(value))
    }
}

impl From<SignalQualityPercent> for u8 {
    fn from(value: SignalQualityPercent) -> Self {
        value.0
    }
}

/// Вычисляет SNR по RSSI и Noise Floor.
///
/// ВАЖНО:
/// вызывающий код должен предварительно убедиться,
/// что оба значения относятся к одному интерфейсу,
/// одному радиоканалу и сопоставимому моменту времени.
pub fn calculate_snr(rssi: RssiDbm, noise: NoiseDbm) -> SnrDb {
    SnrDb::new(i32::from(rssi.value()) - i32::from(noise.value()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_snr() {
        let rssi = RssiDbm::new(-53);
        let noise = NoiseDbm::new(-93);

        let snr = calculate_snr(rssi, noise);

        assert_eq!(snr.value(), 40);
    }

    #[test]
    fn calculates_negative_snr() {
        let rssi = RssiDbm::new(-95);
        let noise = NoiseDbm::new(-90);

        let snr = calculate_snr(rssi, noise);

        assert_eq!(snr.value(), -5);
    }

    #[test]
    fn accepts_valid_signal_quality() {
        let quality = SignalQualityPercent::try_from(75).unwrap();

        assert_eq!(quality.value(), 75);
    }

    #[test]
    fn rejects_invalid_signal_quality() {
        assert!(SignalQualityPercent::try_from(101).is_err());
    }
}
