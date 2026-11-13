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
}
