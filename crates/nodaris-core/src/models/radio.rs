use serde::{Deserialize, Serialize};

use super::measurement::Measurement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioObservation {
    /// Уровень принимаемого сигнала в dBm.
    pub rssi_dbm: Option<Measurement<i32>>,

    /// Уровень фонового шума в dBm.
    pub noise_dbm: Option<Measurement<i32>>,

    /// Отношение сигнал/шум в dB.
    pub snr_db: Option<Measurement<i32>>,

    /// Загруженность радиоканала.
    pub channel_utilization: Option<Measurement<f64>>,

    /// Счётчик повторных передач.
    pub retries: Option<Measurement<u64>>,

    /// Счётчик переданных пакетов.
    pub tx_packets: Option<Measurement<u64>>,
}
