use serde::Serialize;

use crate::keys;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorCode {
    NetworkError,
    PermissionDenied,
    HardwareError,
    ValidationError,
    ParseError,
    InternalError,
    IOError,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Serialize, Debug, Clone)]
pub struct CoreError {
    pub key: String,
    pub code: ErrorCode,
    pub severity: ErrorSeverity,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    #[serde(skip)]
    pub technical_message: Option<String>,
}

impl CoreError {
    fn new(key: &str, code: ErrorCode, severity: ErrorSeverity) -> Self {
        Self {
            key: key.to_owned(),
            code,
            severity,
            args: Vec::new(),
            technical_message: None,
        }
    }

    pub fn validation(key: &str) -> Self {
        Self::new(key, ErrorCode::ValidationError, ErrorSeverity::Error)
    }

    pub fn network_err(key: &str) -> Self {
        Self::new(key, ErrorCode::NetworkError, ErrorSeverity::Error)
    }

    pub fn hardware_err(key: &str) -> Self {
        Self::new(key, ErrorCode::HardwareError, ErrorSeverity::Error)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(
            keys::common::INTERNAL_ERROR,
            ErrorCode::InternalError,
            ErrorSeverity::Critical,
        )
        .with_tech(msg)
    }

    pub fn internal_warning(msg: impl Into<String>) -> Self {
        Self::new(
            keys::common::INTERNAL_WARNING,
            ErrorCode::InternalError,
            ErrorSeverity::Warning,
        )
        .with_tech(msg)
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_tech(mut self, msg: impl Into<String>) -> Self {
        self.technical_message = Some(msg.into());
        self
    }

    pub fn permission_err(key: &str) -> Self {
        Self::new(key, ErrorCode::PermissionDenied, ErrorSeverity::Error)
    }
}

/// The public error contract shared by HTTP and IPC.
#[derive(Serialize, Debug)]
pub struct ApiError {
    pub key: String,
    pub code: ErrorCode,
    pub severity: ErrorSeverity,
    pub args: Vec<String>,
    pub request_id: String,
}

impl CoreError {
    pub fn into_api(self) -> ApiError {
        use std::{
            sync::atomic::{AtomicU64, Ordering},
            time::{SystemTime, UNIX_EPOCH},
        };

        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |value| value.as_millis());
        let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let request_id = format!("{}-{time}-{sequence}", std::process::id());
        if let Some(technical) = &self.technical_message {
            let context: String = technical.chars().take(512).collect();
            eprintln!(
                "request_id={request_id} code={:?} key={} error={context:?}",
                self.code, self.key
            );
        } else {
            eprintln!(
                "request_id={request_id} code={:?} key={}",
                self.code, self.key
            );
        }
        ApiError {
            key: self.key,
            code: self.code,
            severity: self.severity,
            args: self.args,
            request_id,
        }
    }
}

#[cfg(test)]
mod api_tests {
    use super::*;

    #[test]
    fn public_error_keeps_arguments_and_request_id_without_technical_details() {
        let error = CoreError::validation(keys::diagnostics::INVALID_INTERFACE)
            .with_args(["wlan0"])
            .with_tech("internal failure detail")
            .into_api();
        let body = serde_json::to_value(error).unwrap();
        assert_eq!(body["key"], keys::diagnostics::INVALID_INTERFACE);
        assert_eq!(body["code"], "ValidationError");
        assert_eq!(body["severity"], "error");
        assert_eq!(body["args"], serde_json::json!(["wlan0"]));
        assert!(body["request_id"].as_str().is_some_and(|id| !id.is_empty()));
        assert!(body.get("technical_message").is_none());
    }
}
