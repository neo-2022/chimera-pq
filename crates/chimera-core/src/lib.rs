#![forbid(unsafe_code)]

use core::fmt;

pub type ChimeraResult<T> = Result<T, ChimeraError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChimeraError {
    InvalidConfig(String),
    InvalidFrame(String),
    ReplayDetected,
    PolicyDenied(String),
    Unsupported(String),
}

impl fmt::Display for ChimeraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid config: {message}"),
            Self::InvalidFrame(message) => write!(f, "invalid frame: {message}"),
            Self::ReplayDetected => write!(f, "replay detected"),
            Self::PolicyDenied(message) => write!(f, "policy denied: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
        }
    }
}

impl std::error::Error for ChimeraError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionLevel {
    SecretSensitive,
    LocalOnly,
    SafeAdmin,
    AggregateOnly,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticField {
    pub key: String,
    pub value: String,
    pub redaction: RedactionLevel,
}

impl DiagnosticField {
    pub fn redacted_value(&self) -> &str {
        match self.redaction {
            RedactionLevel::SecretSensitive | RedactionLevel::LocalOnly => "<redacted>",
            RedactionLevel::SafeAdmin | RedactionLevel::AggregateOnly | RedactionLevel::Public => {
                &self.value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticField, RedactionLevel};

    #[test]
    fn secret_sensitive_field_is_redacted() {
        let field = DiagnosticField {
            key: "token".to_string(),
            value: "secret".to_string(),
            redaction: RedactionLevel::SecretSensitive,
        };

        assert_eq!(field.redacted_value(), "<redacted>");
    }

    #[test]
    fn public_field_is_visible() {
        let field = DiagnosticField {
            key: "carrier".to_string(),
            value: "in-memory".to_string(),
            redaction: RedactionLevel::Public,
        };

        assert_eq!(field.redacted_value(), "in-memory");
    }
}
