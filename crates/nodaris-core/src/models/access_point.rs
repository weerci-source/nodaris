use serde::{Deserialize, Serialize};

use super::measurement::Measurement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPoint {
    pub ssid: String,
    pub bssid: String,

    /// RSSI в dBm, если источник его предоставляет.
    pub rssi_dbm: Option<Measurement<i32>>,

    /// Процентная оценка качества сигнала.
    pub signal_quality_percent: Option<u8>,

    /// Частота в MHz.
    pub frequency_mhz: u32,

    /// Номер канала, если известен.
    pub channel: Option<u16>,

    /// Ширина канала в MHz.
    pub channel_width_mhz: Option<u16>,

    /// Используется ли эта точка доступа текущим соединением.
    pub is_connected: bool,
}
