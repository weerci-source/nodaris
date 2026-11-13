use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{error::CoreError, models::access_point::AccessPoint};

use super::info::ProviderInfo;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanRequest {
    pub interface_id: Option<String>,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub timestamp_ms: i64,
    pub interface_id: String,
    pub access_points: Vec<AccessPoint>,
}

#[async_trait]
pub trait WifiScanner: Send + Sync {
    fn provider_info(&self) -> &ProviderInfo;

    async fn scan(&self, request: ScanRequest) -> Result<ScanSnapshot, CoreError>;
}
