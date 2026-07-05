//! Public validation entry points.

use serde_json::Value;

use crate::{
    ValidationResult,
    schema::schema_findings,
    semantic::{semantic_findings, semantic_warnings},
    strict::strict_findings,
};

/// Validation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Schema and semantic validation without future strict-profile findings.
    #[default]
    Lenient,
    /// Schema and semantic validation plus AKB006 unknown-core-property lint.
    Strict,
}

/// Validates a parsed OpenAKB descriptor against the embedded schema and
/// descriptor-local semantic rules.
///
/// [`Mode::Strict`] adds `AKB006` findings for unknown core members outside
/// extension payloads. Content checks are reserved for later phases.
#[must_use]
pub fn validate(descriptor: &Value, mode: Mode) -> ValidationResult {
    let mut findings = schema_findings(descriptor);
    findings.extend(semantic_findings(descriptor));
    if mode == Mode::Strict {
        findings.extend(strict_findings(descriptor));
    }
    findings.sort();
    findings.dedup();

    ValidationResult {
        findings,
        warnings: semantic_warnings(descriptor),
    }
}
