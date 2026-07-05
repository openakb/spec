//! Public validation entry points.

use serde_json::Value;

use crate::{ValidationResult, schema::schema_findings};

/// Validation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Report schema and semantic findings while allowing extension-oriented
    /// descriptor features.
    #[default]
    Lenient,
    /// Also report strict-profile findings.
    Strict,
}

/// Validates a parsed OpenAKB descriptor.
#[must_use]
pub fn validate(descriptor: &Value, mode: Mode) -> ValidationResult {
    let _ = mode;

    let mut findings = schema_findings(descriptor);
    findings.sort();
    findings.dedup();

    ValidationResult {
        findings,
        warnings: Vec::new(),
    }
}
