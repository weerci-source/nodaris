pub mod transport {
    pub const AIRTIME_HTTP_FAILURE: &str = "airtime_http_failure";
    pub const AIRTIME_IPC_FAILURE: &str = "airtime_ipc_failure";
    pub const AIRTIME_INVALID_RESPONSE: &str = "airtime_invalid_response";
    pub const HISTORY_HTTP_FAILURE: &str = "history_http_failure";
    pub const HISTORY_IPC_FAILURE: &str = "history_ipc_failure";
    pub const HISTORY_INVALID_RESPONSE: &str = "history_invalid_response";
    pub const SESSION_IPC_FAILURE: &str = "session_ipc_failure";
    pub const SESSION_HTTP_FAILURE: &str = "session_http_failure";
    pub const SESSION_INVALID_RESPONSE: &str = "session_invalid_response";
    pub const ETHERNET_IPC_FAILURE: &str = "ethernet_ipc_failure";
    pub const ETHERNET_HTTP_FAILURE: &str = "ethernet_http_failure";
    pub const ETHERNET_INVALID_RESPONSE: &str = "ethernet_invalid_response";
}

pub mod settings {
    pub const SAVE_FAILED: &str = "settings_save_failed";
}

pub mod history {
    pub const EMPTY: &str = "history_empty";
    pub const READ_FAILED: &str = "history_read_failed";
    pub const SAVE_FAILED: &str = "history_save_failed";
    pub const DELETE_FAILED: &str = "history_delete_failed";
    pub const INVALID_ID: &str = "history_invalid_id";
    pub const FORBIDDEN: &str = "history_forbidden";
    pub const UNSUPPORTED: &str = "history_unsupported";
    pub const REPEAT_UNAVAILABLE: &str = "history_repeat_unavailable";
}
pub mod common {
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const INTERNAL_WARNING: &str = "internal_warning";
}

pub mod scan {
    pub const WIFI_PERMISSION_DENIED: &str = "scan_wifi_permission_denied";
    pub const INTERFACE_DOWN: &str = "scan_interface_down";
    pub const UNKNOWN_FAILURE: &str = "scan_unknown_failure";
}

pub mod diagnostics {
    pub const AIRTIME_FORBIDDEN: &str = "diagnostics_airtime_forbidden";
    pub const AIRTIME_INVALID_REQUEST: &str = "diagnostics_airtime_invalid_request";
    pub const AIRTIME_UNSUPPORTED: &str = "diagnostics_airtime_unsupported";
    pub const INVALID_INTERFACE: &str = "diagnostics_invalid_interface";
    pub const RADIO_BUSY: &str = "diagnostics_radio_busy";
    pub const RADIO_TIMEOUT: &str = "diagnostics_radio_timeout";
    pub const PROBE_INVALID_REQUEST: &str = "diagnostics_probe_invalid_request";
    pub const PROBE_FORBIDDEN: &str = "diagnostics_probe_forbidden";
    pub const CONNECTION_FORBIDDEN: &str = "diagnostics_connection_forbidden";
    pub const CONNECTION_BUSY: &str = "diagnostics_connection_busy";
    pub const SESSION_BUSY: &str = "diagnostics_session_busy";
    pub const SESSION_FORBIDDEN: &str = "diagnostics_session_forbidden";
    pub const SESSION_INVALID_TARGET: &str = "diagnostics_session_invalid_target";
    pub const SESSION_INVALID_REQUEST: &str = "diagnostics_session_invalid_request";
    pub const SESSION_UNSUPPORTED: &str = "diagnostics_session_unsupported";
    pub const PROBE_BUSY: &str = "diagnostics_probe_busy";
    pub const ETHERNET_FORBIDDEN: &str = "diagnostics_ethernet_forbidden";
    pub const ETHERNET_INVALID_REQUEST: &str = "diagnostics_ethernet_invalid_request";
    pub const ETHERNET_INVALID_TARGET: &str = "diagnostics_ethernet_invalid_target";
    pub const ETHERNET_TARGET_UNRESOLVED: &str = "diagnostics_ethernet_target_unresolved";
    pub const ETHERNET_INTERFACE_UNAVAILABLE: &str = "diagnostics_ethernet_interface_unavailable";
    pub const ETHERNET_READ_FAILED: &str = "diagnostics_ethernet_read_failed";
    pub const ETHERNET_UNSUPPORTED: &str = "diagnostics_ethernet_unsupported";
}
