use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    WifiScan,
    RadioSignal,
    NoiseFloor,
    ChannelUtilization,
    Retransmissions,
    ConnectionMonitoring,
    RoamingEvents,
    IcmpProbe,
    TcpProbe,
    HttpsProbe,
}
