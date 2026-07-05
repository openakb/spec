//! Public validation entry points.

use serde_json::Value;

use crate::{ValidationResult, schema::schema_findings};

/// Validation mode.
///
/// Reserved for later semantic and strict validation phases. Task 3 validation
/// is schema-only, so both modes currently emit the same schema findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Schema-only validation with future lenient semantic behavior reserved.
    #[default]
    Lenient,
    /// Schema-only validation with future strict-profile behavior reserved.
    Strict,
}

/// Validates a parsed OpenAKB descriptor against the embedded schema.
///
/// The `mode` parameter is accepted for API stability, but this stage does not
/// yet emit semantic or strict-profile findings.
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
