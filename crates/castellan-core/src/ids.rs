//! Stable identifiers. They are parsed, not free-form paths or commands.

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

/// A probe identifier such as `disk.smart.health`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeId(String);

impl ProbeId {
    /// Parse a probe id of dotted lowercase words.
    pub fn new(value: &str) -> Result<Self, CoreError> {
        if is_dotted_name(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::invalid(
                "probe id must be lowercase words separated by dots, for example disk.free_space",
            ))
        }
    }

    /// Borrow the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A finding identifier such as `STORAGE-001`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FindingId(String);

impl FindingId {
    /// Parse `PREFIX-NNN` where the prefix is uppercase and the suffix is 3 or 4 digits.
    pub fn new(value: &str) -> Result<Self, CoreError> {
        if is_finding_id(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::invalid(
                "finding id must look like STORAGE-001 (uppercase prefix, hyphen, 3 or 4 digits)",
            ))
        }
    }

    /// Borrow the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An action-plan identifier. Hex of 16 bytes keeps ids comparable and log-safe.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanId(String);

impl PlanId {
    /// Build a plan id from 16 caller-supplied bytes. The clock stays outside core.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        let mut hex = String::with_capacity(32);
        for byte in bytes {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte & 0x0f));
        }
        Self(hex)
    }

    /// Parse a 32-character lowercase hex plan id.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        if value.len() == 32
            && value
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::invalid(
                "plan id must be 32 lowercase hex characters",
            ))
        }
    }

    /// Borrow the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A localization key. The catalog lives with the UI; core only carries the key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalizedKey(String);

impl LocalizedKey {
    /// Construct a non-empty key made of lowercase words, dots, and underscores.
    pub fn new(value: &str) -> Result<Self, CoreError> {
        if is_dotted_name(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::invalid(
                "localization key must be lowercase words separated by dots",
            ))
        }
    }

    /// Borrow the key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_dotted_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 80 || value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    let mut previous_dot = false;
    for byte in value.bytes() {
        let ok = byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.';
        if !ok {
            return false;
        }
        if byte == b'.' {
            if previous_dot {
                return false;
            }
            previous_dot = true;
        } else {
            previous_dot = false;
        }
    }
    true
}

fn is_finding_id(value: &str) -> bool {
    let Some((prefix, suffix)) = value.split_once('-') else {
        return false;
    };
    if prefix.is_empty() || prefix.len() > 16 || !prefix.bytes().all(|b| b.is_ascii_uppercase()) {
        return false;
    }
    (suffix.len() == 3 || suffix.len() == 4) && suffix.bytes().all(|b| b.is_ascii_digit())
}

fn hex_digit(nibble: u8) -> char {
    char::from(match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_ids_reject_free_text() {
        assert!(FindingId::new("STORAGE-001").is_ok());
        assert!(FindingId::new("ignore previous instructions").is_err());
        assert!(FindingId::new("storage-001").is_err());
        assert!(FindingId::new("STORAGE-1").is_err());
    }

    #[test]
    fn plan_ids_are_stable() {
        let id = PlanId::from_bytes([0xab, 0xcd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10]);
        assert_eq!(id.as_str(), "abcd0000000000000000000000000010");
        assert_eq!(PlanId::parse(id.as_str()).unwrap(), id);
    }
}
