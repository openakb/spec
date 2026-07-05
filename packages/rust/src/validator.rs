//! Public validation entry points.

use serde_json::Value;

use crate::{
    ValidationResult,
    schema::schema_findings,
    semantic::{semantic_findings, semantic_warnings},
};

/// Validation mode.
///
/// Reserved for later strict validation phases. This stage emits schema and
/// semantic findings for both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Schema and semantic validation without future strict-profile findings.
    #[default]
    Lenient,
    /// Schema and semantic validation with future strict-profile behavior reserved.
    Strict,
}

/// Validates a parsed OpenAKB descriptor against the embedded schema and
/// descriptor-local semantic rules.
///
/// The `mode` parameter is accepted for API stability, but this stage does not
/// yet emit strict-profile findings.
#[must_use]
pub fn validate(descriptor: &Value, mode: Mode) -> ValidationResult {
    let _ = mode;

    let mut findings = schema_findings(descriptor);
    findings.extend(semantic_findings(descriptor));
    findings.sort();
    findings.dedup();

    ValidationResult {
        findings,
        warnings: semantic_warnings(descriptor),
    }
}
