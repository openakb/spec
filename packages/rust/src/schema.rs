//! Embedded JSON Schemas and schema-error code mapping.
//!
//! The Rust validator keeps vendored schema copies in `schemas/` and maps
//! jsonschema errors to the same stable AKB codes used by the Python validator
//! and repository conformance gate.

use std::sync::LazyLock;

use jsonschema::{
    Draft, PatternOptions, Validator,
    error::{ValidationError, ValidationErrorKind},
};
use serde_json::Value;

use crate::{Code, Finding};

static DESCRIPTOR_SCHEMA: LazyLock<Value> = LazyLock::new(parse_descriptor_schema);
static PROVENANCE_SCHEMA: LazyLock<Value> = LazyLock::new(parse_provenance_schema);
static DESCRIPTOR_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| build_validator(descriptor_schema()));
static PROVENANCE_VALIDATOR: LazyLock<Validator> =
    LazyLock::new(|| build_validator(provenance_schema()));

/// Returns the embedded OpenAKB descriptor JSON Schema.
#[must_use]
pub(crate) fn descriptor_schema() -> &'static Value {
    &DESCRIPTOR_SCHEMA
}

/// Returns schema findings for an OpenAKB descriptor value.
#[must_use]
pub(crate) fn schema_findings(descriptor: &Value) -> Vec<Finding> {
    findings_for(&DESCRIPTOR_VALIDATOR, descriptor)
}

/// Returns schema findings for a provenance sidecar value.
#[allow(dead_code)] // Required sidecar schema seam; later validation tasks call it.
#[must_use]
pub(crate) fn provenance_schema_findings(sidecar: &Value) -> Vec<Finding> {
    findings_for(&PROVENANCE_VALIDATOR, sidecar)
}

fn provenance_schema() -> &'static Value {
    &PROVENANCE_SCHEMA
}

#[allow(clippy::expect_used)] // PANIC: embedded schema is CI-verified byte-identical to schema/v1.
fn parse_descriptor_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/openakb.schema.json"))
        .expect("embedded OpenAKB descriptor schema must parse")
}

#[allow(clippy::expect_used)] // PANIC: embedded schema is CI-verified byte-identical to schema/v1.
fn parse_provenance_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/provenance.schema.json"))
        .expect("embedded OpenAKB provenance schema must parse")
}

#[allow(clippy::expect_used)] // PANIC: embedded schema is CI-verified byte-identical to schema/v1.
fn build_validator(schema: &Value) -> Validator {
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .should_validate_formats(true)
        .with_pattern_options(PatternOptions::regex())
        .build(schema)
        .expect("embedded OpenAKB schema must compile")
}

fn findings_for(validator: &Validator, instance: &Value) -> Vec<Finding> {
    let mut findings: Vec<_> = validator
        .iter_errors(instance)
        .filter_map(|error| {
            code_for(&error).map(|code| Finding {
                code,
                path: error.instance_path().to_string(),
                message: error.to_string(),
            })
        })
        .collect();
    findings.sort();
    findings
}

fn code_for(error: &ValidationError<'_>) -> Option<Code> {
    let instance_path = error.instance_path().to_string();
    let schema_path = error.schema_path().to_string();

    if is_link_level_any_of(&instance_path, &schema_path) {
        return Some(Code::Akb012);
    }

    if is_then_schema_path(&schema_path) {
        return Some(Code::Akb003);
    }

    if instance_path.ends_with("/rel") {
        return Some(match keyword_of(error) {
            "type" => Code::Akb011,
            _ => Code::Akb008,
        });
    }

    match keyword_of(error) {
        "maxItems" | "maxLength" => Some(Code::Akb005),
        "required" => Some(Code::Akb009),
        "anyOf" | "enum" | "format" | "minimum" | "minItems" | "minLength" | "pattern"
        | "propertyNames" | "type" | "uniqueItems" => Some(Code::Akb011),
        _ => None,
    }
}

fn keyword_of<'error>(error: &'error ValidationError<'_>) -> &'error str {
    match error.kind() {
        ValidationErrorKind::AnyOf { .. } => "anyOf",
        ValidationErrorKind::Enum { .. } => "enum",
        ValidationErrorKind::Format { .. } => "format",
        ValidationErrorKind::MaxItems { .. } => "maxItems",
        ValidationErrorKind::MaxLength { .. } => "maxLength",
        ValidationErrorKind::Minimum { .. } => "minimum",
        ValidationErrorKind::MinItems { .. } => "minItems",
        ValidationErrorKind::MinLength { .. } => "minLength",
        ValidationErrorKind::Pattern { .. }
        | ValidationErrorKind::BacktrackLimitExceeded { .. }
        | ValidationErrorKind::RegexEngineFailure { .. } => "pattern",
        ValidationErrorKind::PropertyNames { .. } => "propertyNames",
        ValidationErrorKind::Required { .. } => "required",
        ValidationErrorKind::Type { .. } => "type",
        ValidationErrorKind::UniqueItems => "uniqueItems",
        _ => error.kind().keyword(),
    }
}

fn is_link_level_any_of(instance_path: &str, schema_path: &str) -> bool {
    schema_path_has_segment(schema_path, "anyOf") && is_link_instance_path(instance_path)
}

fn is_link_instance_path(instance_path: &str) -> bool {
    let mut segments = instance_path.rsplit('/');
    let last = segments.next().unwrap_or_default();
    let Some(previous) = segments.next() else {
        return false;
    };

    last.parse::<usize>().is_ok() && previous == "links"
}

fn is_then_schema_path(schema_path: &str) -> bool {
    schema_path_has_segment(schema_path, "then")
}

fn schema_path_has_segment(schema_path: &str, expected: &str) -> bool {
    schema_path.split('/').any(|segment| segment == expected)
}
