use async_trait::async_trait;

use crate::{
    error::CoreError,
    models::radio::RadioObservation,
};

use super::info::ProviderInfo;

#[async_trait]
pub trait RadioMetricsProvider: Send + Sync {
    fn provider_info(&self) -> &ProviderInfo;

    async fn collect(
        &self,
        interface_id: &str,
    ) -> Result<RadioObservation, CoreError>;
}