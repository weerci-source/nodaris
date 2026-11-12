use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorCode {
    NetworkError,
    PermissionDenied,
    HardwareError,
    InternalError,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
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
    pub fn new(key: &str, code: ErrorCode, severity: ErrorSeverity) -> Self {
        Self {
            key: key.to_string(),
            code,
            severity,
            args: Vec::new(),
            technical_message: None,
        }
    }

    pub fn with_tech(mut self, msg: impl Into<String>) -> Self {
        self.technical_message = Some(msg.into());
        self
    }
}
