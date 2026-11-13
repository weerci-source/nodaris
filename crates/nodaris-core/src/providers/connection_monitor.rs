use async_trait::async_trait;

use crate::error::CoreError;

use super::info::ProviderInfo;

#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub interface_id: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub connected: bool,
    pub timestamp_ms: i64,
}

#[async_trait]
pub trait ConnectionMonitor: Send + Sync {
    fn provider_info(&self) -> &ProviderInfo;

    async fn current_state(
        &self,
        interface_id: &str,
    ) -> Result<ConnectionState, CoreError>;
}