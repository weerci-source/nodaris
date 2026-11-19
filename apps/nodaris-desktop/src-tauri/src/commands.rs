use nodaris_core::{
    error::{ApiError, CoreError},
    models::{
        access_point::AccessPoint,
        ethernet::{EthernetLink, EthernetReport},
        session::DiagnosticSession,
    },
};
use serde::Serialize;

#[tauri::command]
pub(super) async fn scan_wifi() -> Result<Vec<AccessPoint>, ApiError> {
    #[cfg(target_os = "linux")]
    {
        run_blocking(nodaris_linux::scan_wifi).await
    }
    #[cfg(not(target_os = "linux"))]
    Err(unsupported())
}

#[tauri::command]
pub(super) async fn wifi_interfaces() -> Result<Vec<WifiInterface>, ApiError> {
    #[cfg(target_os = "linux")]
    {
        run_blocking(|| {
            nodaris_linux::list_wifi_interfaces().map(|interfaces| {
                interfaces
                    .into_iter()
                    .map(|item| WifiInterface {
                        name: item.name,
                        connected: item.connected,
                    })
                    .collect()
            })
        })
        .await
    }
    #[cfg(not(target_os = "linux"))]
    Err(unsupported())
}

#[derive(Serialize)]
pub(super) struct WifiInterface {
    name: String,
    connected: bool,
}

#[tauri::command]
pub(super) async fn diagnostic_session(
    interface: String,
    target: Option<String>,
) -> Result<DiagnosticSession, ApiError> {
    #[cfg(target_os = "linux")]
    {
        nodaris_linux::collect_session(interface, target)
            .await
            .map_err(CoreError::into_api)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (interface, target);
        Err(unsupported())
    }
}

#[tauri::command]
pub(super) async fn ethernet_interfaces() -> Result<Vec<EthernetLink>, ApiError> {
    #[cfg(target_os = "linux")]
    {
        run_blocking(nodaris_linux::list_ethernet_links).await
    }
    #[cfg(not(target_os = "linux"))]
    Err(ethernet_unsupported())
}

#[tauri::command]
pub(super) async fn ethernet_diagnostics(
    interface: String,
    target: String,
) -> Result<EthernetReport, ApiError> {
    #[cfg(target_os = "linux")]
    {
        nodaris_linux::collect_ethernet_report(interface, target)
            .await
            .map_err(CoreError::into_api)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (interface, target);
        Err(ethernet_unsupported())
    }
}

#[cfg(not(target_os = "linux"))]
fn ethernet_unsupported() -> ApiError {
    CoreError::hardware_err(nodaris_core::keys::diagnostics::ETHERNET_UNSUPPORTED).into_api()
}

#[cfg(not(target_os = "linux"))]
fn unsupported() -> ApiError {
    CoreError::hardware_err(nodaris_core::keys::diagnostics::SESSION_UNSUPPORTED).into_api()
}

#[cfg(target_os = "linux")]
async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, CoreError> + Send + 'static,
) -> Result<T, ApiError> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| CoreError::internal(format!("IPC worker: {error}")).into_api())?
        .map_err(CoreError::into_api)
}
