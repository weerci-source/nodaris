use serde::Serialize;

use super::{icmp::IcmpReport, measurement::Measurement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EthernetLinkState {
    Up,
    Down,
    Disabled,
}

#[derive(Serialize)]
pub struct EthernetLink {
    pub interface: String,
    pub state: Measurement<EthernetLinkState>,
    pub speed_mbps: Measurement<u32>,
}

#[derive(Serialize)]
pub struct EthernetReport {
    pub link: EthernetLink,
    pub probes: IcmpReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}
