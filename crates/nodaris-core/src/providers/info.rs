use serde::{Deserialize, Serialize};

use super::capabilities::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
}
