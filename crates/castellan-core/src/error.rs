//! Errors raised by pure domain code.

use core::fmt;

/// Stable error code shown to users (`CST-E` + four digits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorCode(&'static str);

impl ErrorCode {
    /// Invalid value supplied to a domain constructor.
    pub const INVALID: Self = Self("CST-E0001");
    /// Configuration or policy failed validation.
    pub const CONFIG: Self = Self("CST-E0002");
    /// A more permissive lower layer tried to override a higher restriction.
    pub const POLICY: Self = Self("CST-E0003");

    /// The stable code string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A domain error with a stable code and a human explanation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CoreError {
    /// The value is not a valid domain identifier or quantity.
    #[error("{code}: {message}")]
    Invalid {
        /// Stable code.
        code: ErrorCode,
        /// What was wrong, and what a valid value looks like.
        message: String,
    },
    /// A policy or config document is not acceptable.
    #[error("{code}: {message}")]
    Config {
        /// Stable code.
        code: ErrorCode,
        /// What was wrong.
        message: String,
    },
}

impl CoreError {
    /// Build an invalid-value error.
    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            code: ErrorCode::INVALID,
            message: message.into(),
        }
    }

    /// Build a config/policy error.
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            code: ErrorCode::CONFIG,
            message: message.into(),
        }
    }

    /// The stable code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::Invalid { code, .. } | Self::Config { code, .. } => *code,
        }
    }
}
