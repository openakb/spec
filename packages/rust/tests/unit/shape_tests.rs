//! Typed-id grammar behavior pinned through the public `validate` contract.
//!
//! The shape helpers (`is_typed_id`, `id_kind`, `normalize_id`,
//! `reference_code_id`) are crate-internal, so these tests assert their
//! observable semantics through `validate`: both typed forms pass in any ASCII
//! case, the prefix alone decides kind (AKB010 on the wrong prefix), right-prefix
//! tokens that resolve nowhere are AKB007, and non-typed tokens are the schema
//! layer's AKB011, never a second finding.

use serde_json::{Value, json};

use openakb_validate::{Code, Mode, validate};

fn descriptor() -> Value {
    json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "shape-fixture",
        "title": "Shape",
        "description": "Base for typed-id shape tests.",
        "sources": [{"id":"SRC-000001","type":"url","uri":"https://docs.example.com/a/"}],
        "sections": [{"id":"SEC-000001","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["SRC-000001"]}]
    })
}

fn codes(descriptor: &Value) -> Vec<Code> {
    validate(descriptor, Mode::Lenient)
        .codes()
        .into_iter()
        .collect()
}

#[test]
fn test_typed_ids_clean() {
    // Both typed forms validate cleanly in any ASCII case.
    for (source_id, section_id) in [
        ("SRC-000001", "SEC-000001"),
        ("src-abc123", "sec-ABC123"),
        ("SRC-ZZZZZZ", "SEC-zzzzzz"),
    ] {
        let mut descriptor = descriptor();
        descriptor["sources"][0]["id"] = json!(source_id);
        descriptor["sections"][0]["id"] = json!(section_id);
        descriptor["sections"][0]["source_ids"] = json!([source_id]);

        assert_eq!(codes(&descriptor), Vec::new());
    }
}

#[test]
fn test_typed_reference_case_insensitive() {
    // A reference and the declared id differing only in ASCII case resolve.
    let mut descriptor = descriptor();
    descriptor["sections"] = json!([
        {"id":"SEC-000001","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["SRC-000001"]},
        {"id":"SEC-000002","title":"Child","description":"A child section.","content_uri":"child.md","source_ids":["SRC-000001"],"parent_id":"sec-000001"}
    ]);

    assert_eq!(codes(&descriptor), Vec::new());
}

#[test]
fn test_prefix_kind_wrong_reference() {
    // The prefix alone decides kind, case-insensitively: a section reference
    // carrying the source prefix is AKB010, not AKB007.
    for parent_id in ["SRC-000001", "src-000001"] {
        let mut descriptor = descriptor();
        descriptor["sections"][0]["parent_id"] = json!(parent_id);

        assert_eq!(codes(&descriptor), vec![Code::Akb010]);
    }
}

#[test]
fn test_right_prefix_unresolved() {
    // A well-formed token that resolves nowhere is AKB007.
    let mut descriptor = descriptor();
    descriptor["sections"][0]["parent_id"] = json!("SEC-999999");

    assert_eq!(codes(&descriptor), vec![Code::Akb007]);
}

#[test]
fn test_malformed_typed_token_schema_only() {
    // Malformed typed-looking tokens are the schema layer's AKB011; the semantic
    // layer never double-reports them as AKB007.
    for malformed in ["SEC-", "SEC-0000", "SEC-0000000", "SEC-0000_1", "SRC-00000"] {
        let mut descriptor = descriptor();
        descriptor["sections"][0]["parent_id"] = json!(malformed);

        assert_eq!(codes(&descriptor), vec![Code::Akb011]);
    }
}

#[test]
fn test_unicode_case_folds_rejected() {
    // U+212A KELVIN SIGN and U+017F LONG S fold to ASCII 'k'/'s' under Unicode
    // simple case folding but are not ASCII base36. The schema reports AKB011,
    // and the semantic layer must not add AKB007 by treating them as typed ids.
    for foldable in ["SEC-00000\u{212a}", "SEC-00000\u{017f}"] {
        let mut descriptor = descriptor();
        descriptor["sections"][0]["parent_id"] = json!(foldable);

        assert_eq!(codes(&descriptor), vec![Code::Akb011]);
    }
}

#[test]
fn test_non_typed_token_schema_only() {
    // A non-typed token is the schema layer's AKB011, never a second AKB007.
    let mut descriptor = descriptor();
    descriptor["sections"][0]["parent_id"] = json!("ghost");

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}
