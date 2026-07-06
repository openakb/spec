//! Public validation entry points.

use serde::Serialize;
use serde_json::Value;

use crate::{
    ContentReport, Resolver, ValidationResult, check_content,
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

/// Combined structural and opt-in content validation result.
#[derive(Debug, Clone, Serialize)]
pub struct FullReport {
    /// Schema, semantic, and optional strict-profile validation result.
    pub validation: ValidationResult,
    /// Opt-in content validation result.
    pub content: ContentReport,
}

impl FullReport {
    /// Returns true when both structural validation and content checks pass.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.validation.ok() && self.content.ok()
    }
}

/// Validates a parsed OpenAKB descriptor against the embedded schema and
/// descriptor-local semantic rules.
///
/// [`Mode::Strict`] adds `AKB006` findings for unknown core members outside
/// extension payloads. Content behind URIs is never fetched here; use
/// [`validate_with_content`] for opt-in content verification.
///
/// # Preconditions
///
/// The `descriptor` must come from a recursion-limited deserializer.
/// [`serde_json::from_str`] caps nesting at 128 by default, which keeps the
/// schema walk within a safe depth; the untrusted-text path therefore cannot
/// reach it. Callers must not hand this function a `Value` built by a
/// deserializer whose recursion limit was disabled (or an equivalently deep
/// programmatically-constructed `Value`): the underlying `jsonschema` walk is
/// recursive, so an unbounded-depth value can overflow the stack — an
/// uncatchable abort this crate cannot turn into a diagnostic.
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

/// Runs structural validation and opt-in content checks.
///
/// # Preconditions
///
/// Shares [`validate`]'s precondition: `descriptor` must come from a
/// recursion-limited deserializer, since structural validation runs the same
/// recursive schema walk.
pub async fn validate_with_content(
    descriptor: &Value,
    resolver: &dyn Resolver,
    mode: Mode,
) -> FullReport {
    FullReport {
        validation: validate(descriptor, mode),
        content: check_content(descriptor, resolver).await,
    }
}
