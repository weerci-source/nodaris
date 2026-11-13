use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub bssid: String,
    pub signal: i32,
    pub frequency: u32,
    pub noise: Option<i32>,
    pub snr: Option<i32>,
    pub is_active: bool, 
}
