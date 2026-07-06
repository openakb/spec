//! Public validation entry points.

use serde::Serialize;
use serde_json::Value;

use crate::{
    Code, ContentReport, Finding, Resolver, STRUCTURAL_DEPTH_MAX, ValidationResult, check_content,
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
/// None. Validation is total on any [`Value`]. A descriptor nested deeper than
/// [`STRUCTURAL_DEPTH_MAX`] is diagnosed as an `AKB011` structural finding rather
/// than descending into the recursive `jsonschema` walk, so even a
/// programmatically-built or recursion-limit-disabled value that would otherwise
/// overflow the stack is turned into a diagnostic instead of an uncatchable
/// abort. [`serde_json::from_str`] caps nesting at 128, far below the cap, so no
/// parsed descriptor is ever affected by the guard.
#[must_use]
pub fn validate(descriptor: &Value, mode: Mode) -> ValidationResult {
    if exceeds_structural_depth(descriptor) {
        return ValidationResult {
            findings: vec![structural_depth_finding()],
            warnings: Vec::new(),
        };
    }

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

/// Returns true when any value nests deeper than [`STRUCTURAL_DEPTH_MAX`].
///
/// The traversal is iterative (an explicit worklist stack, never recursion) so
/// the depth check itself cannot overflow the call stack on the very inputs it
/// guards against. The root sits at depth 1 and every nested array or object
/// element is one level deeper; the walk stops as soon as the cap is exceeded.
fn exceeds_structural_depth(descriptor: &Value) -> bool {
    let mut worklist = vec![(descriptor, 1usize)];

    while let Some((value, depth)) = worklist.pop() {
        if depth > STRUCTURAL_DEPTH_MAX {
            return true;
        }
        match value {
            Value::Array(items) => {
                worklist.extend(items.iter().map(|item| (item, depth + 1)));
            }
            Value::Object(members) => {
                worklist.extend(members.values().map(|member| (member, depth + 1)));
            }
            _ => {}
        }
    }

    false
}

/// Builds the structural-depth guard finding.
///
/// An absurdly deep descriptor is structurally invalid at the root, so this
/// reuses `AKB011` — the same code the schema layer emits when the descriptor is
/// not a well-formed object of the expected shape.
fn structural_depth_finding() -> Finding {
    Finding {
        code: Code::Akb011,
        path: String::new(),
        message: format!(
            "descriptor nesting exceeds the structural depth limit of {STRUCTURAL_DEPTH_MAX}; \
             it is not a well-formed OpenAKB descriptor"
        ),
    }
}

/// Runs structural validation and opt-in content checks.
///
/// # Preconditions
///
/// None. Structural validation shares [`validate`]'s depth guard: a descriptor
/// nested deeper than [`STRUCTURAL_DEPTH_MAX`] is diagnosed as `AKB011` rather
/// than descending into the recursive schema walk.
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
