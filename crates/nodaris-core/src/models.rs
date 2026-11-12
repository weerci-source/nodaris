use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub bssid: String,
    pub signal: i32,    // RSSI в dBm
    pub frequency: u32, // MHz
}
