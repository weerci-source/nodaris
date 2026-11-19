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
    pub const PROBE_BUSY: &str = "diagnostics_probe_busy";
}
