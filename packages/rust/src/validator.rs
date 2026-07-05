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
