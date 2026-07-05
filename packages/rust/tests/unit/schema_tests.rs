use serde_json::{Value, json};

use openakb_validate::{Code, Mode, validate};

fn descriptor() -> Value {
    json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "valid-single",
        "title": "Valid single section",
        "description": "One root content section with a source; the floor of validity.",
        "tags": ["fixtures", "single_section"],
        "sources": [
            {
                "id": "s1",
                "type": "url",
                "uri": "https://docs.example.com/a/"
            }
        ],
        "sections": [
            {
                "id": "root",
                "title": "Root",
                "description": "The only section.",
                "content_uri": "root.md",
                "source_ids": ["s1"]
            }
        ]
    })
}

fn codes(descriptor: &Value) -> Vec<Code> {
    validate(descriptor, Mode::Lenient)
        .codes()
        .into_iter()
        .collect()
}

#[test]
fn test_minimal_descriptor_passes() {
    assert_eq!(codes(&descriptor()), Vec::new());
}

#[test]
fn test_trailing_newline_id_rejected() {
    let mut descriptor = descriptor();
    descriptor["sources"][0]["id"] = json!("s1\n");
    descriptor["sections"][0]["source_ids"][0] = json!("s1\n");

    assert!(codes(&descriptor).contains(&Code::Akb011));
}

#[test]
fn test_missing_required_field() {
    let mut descriptor = descriptor();
    descriptor.as_object_mut().unwrap().remove("title");

    assert_eq!(codes(&descriptor), vec![Code::Akb009]);
}

#[test]
fn test_oversized_title() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["title"] = json!("x".repeat(201));

    assert_eq!(codes(&descriptor), vec![Code::Akb005]);
}

#[test]
fn test_unknown_rel() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["links"] = json!([
        {
            "rel": "totally made up",
            "section_id": "root"
        }
    ]);

    assert!(codes(&descriptor).contains(&Code::Akb008));
}

#[test]
fn test_non_string_rel() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["links"] = json!([
        {
            "rel": 42,
            "section_id": "root"
        }
    ]);

    let codes = codes(&descriptor);
    assert!(codes.contains(&Code::Akb011));
    assert!(!codes.contains(&Code::Akb008));
}

#[test]
fn test_link_without_target() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["links"] = json!([
        {
            "rel": "see-also"
        }
    ]);

    assert!(codes(&descriptor).contains(&Code::Akb012));
}

#[test]
fn test_content_without_source() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]
        .as_object_mut()
        .unwrap()
        .remove("source_ids");

    assert!(codes(&descriptor).contains(&Code::Akb003));
}

#[test]
fn test_empty_source_ids() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["source_ids"] = json!([]);

    assert!(codes(&descriptor).contains(&Code::Akb003));
}

#[test]
fn test_malformed_timestamp() {
    let mut descriptor = descriptor();
    descriptor["sources"][0]["captured_at"] = json!("2026-06-28 00:00:00Z");

    assert!(codes(&descriptor).contains(&Code::Akb011));
}

#[test]
fn test_top_level_title_cap() {
    let mut descriptor = descriptor();
    descriptor["title"] = json!("x".repeat(201));

    assert_eq!(codes(&descriptor), vec![Code::Akb005]);
}

#[test]
fn test_empty_description() {
    let mut descriptor = descriptor();
    descriptor["description"] = json!("");

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_empty_sources() {
    let mut descriptor = descriptor();
    descriptor["sources"] = json!([]);

    assert_eq!(codes(&descriptor), vec![Code::Akb007, Code::Akb011]);
}

#[test]
fn test_negative_guide_length() {
    let mut descriptor = descriptor();
    descriptor["guide_length"] = json!(-1);

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_invalid_schema_uri() {
    let mut descriptor = descriptor();
    descriptor["$schema"] = json!("not a uri");

    assert!(codes(&descriptor).contains(&Code::Akb011));
}

#[test]
fn test_wrong_source_type() {
    let mut descriptor = descriptor();
    descriptor["sources"][0]["type"] = json!(42);

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_too_many_tags() {
    let mut descriptor = descriptor();
    descriptor["tags"] = json!(
        (0..33)
            .map(|index| format!("tag{index}"))
            .collect::<Vec<_>>()
    );

    assert_eq!(codes(&descriptor), vec![Code::Akb005]);
}

#[test]
fn test_duplicate_tags() {
    let mut descriptor = descriptor();
    descriptor["tags"] = json!(["same", "same"]);

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_duplicate_source_ids() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["source_ids"] = json!(["s1", "s1"]);

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_unknown_extension_key() {
    let mut descriptor = descriptor();
    descriptor["x"] = json!({
        "not valid": true
    });

    assert!(codes(&descriptor).contains(&Code::Akb011));
}

#[test]
fn test_extension_value_type() {
    let mut descriptor = descriptor();
    descriptor["x"] = json!({
        "docs.example": true
    });

    assert_eq!(codes(&descriptor), vec![Code::Akb011]);
}

#[test]
fn test_scalar_input_reports() {
    assert_eq!(codes(&json!(null)), vec![Code::Akb011]);
}

#[test]
fn test_array_input_reports() {
    assert_eq!(codes(&json!([])), vec![Code::Akb011]);
}
