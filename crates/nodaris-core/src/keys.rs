pub mod transport {
    pub const SESSION_IPC_FAILURE: &str = "session_ipc_failure";
    pub const SESSION_HTTP_FAILURE: &str = "session_http_failure";
    pub const SESSION_INVALID_RESPONSE: &str = "session_invalid_response";
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
}
