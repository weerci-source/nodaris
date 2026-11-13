use async_trait::async_trait;

use crate::error::CoreError;

use super::info::ProviderInfo;

#[derive(Debug, Clone)]
pub enum ProbeMethod {
    Icmp,
    Tcp,
    Https,
}

#[derive(Debug, Clone)]
pub struct ProbeRequest {
    pub target: String,
    pub method: ProbeMethod,
    pub count: u32,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub target: String,
    pub sent: u32,
    pub received: u32,
    pub min_rtt_ms: Option<f64>,
    pub avg_rtt_ms: Option<f64>,
    pub max_rtt_ms: Option<f64>,
    pub jitter_ms: Option<f64>,
    pub packet_loss_percent: f64,
    pub timestamp_ms: i64,
}

#[async_trait]
pub trait NetworkProbeProvider: Send + Sync {
    fn provider_info(&self) -> &ProviderInfo;

    async fn probe(&self, request: ProbeRequest) -> Result<ProbeResult, CoreError>;
}
