use serde_json::{Value, json};

use openakb_validate::{Code, Mode, validate};

fn descriptor() -> Value {
    json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "strict-fixture",
        "title": "Strict",
        "description": "Base for strict-rule tests.",
        "sources": [{"id":"s1","type":"url","uri":"https://docs.example.com/a/"}],
        "sections": [{"id":"root","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["s1"]}]
    })
}

fn akb006_paths(descriptor: &Value) -> Vec<String> {
    let mut paths: Vec<_> = validate(descriptor, Mode::Strict)
        .findings
        .into_iter()
        .filter(|finding| finding.code == Code::Akb006)
        .map(|finding| finding.path)
        .collect();
    paths.sort();
    paths
}

#[test]
fn test_unknown_top_key() {
    let mut descriptor = descriptor();
    descriptor["future_field"] = json!(true);

    assert!(validate(&descriptor, Mode::Lenient).ok());
    assert_eq!(akb006_paths(&descriptor), vec!["/future_field"]);
}

#[test]
fn test_extension_keys_exempt() {
    let mut descriptor = descriptor();
    descriptor["x"] = json!({"com.example": {"future_field": true}});
    descriptor["sections"][0]["x"] = json!({"org.example": {"nested": {"unknown": true}}});

    assert!(validate(&descriptor, Mode::Strict).ok());
}

#[test]
fn test_nested_unknown_key() {
    let mut descriptor = descriptor();
    descriptor["sources"][0]["wild"] = json!(1);
    descriptor["sections"][0]["links"] = json!([{"rel":"see-also","section_id":"root","why":"?"}]);

    assert_eq!(
        akb006_paths(&descriptor),
        vec!["/sections/0/links/0/why", "/sources/0/wild"]
    );
}

#[test]
fn test_known_keys_clean() {
    assert!(validate(&descriptor(), Mode::Strict).ok());
}

#[test]
fn test_claim_locator_unknown() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["provenance"] = json!([
        {"text":"Inline claim.","source_ids":["s1"],"confidence":0.8,"locator":{"quote":"Inline","extra":true}}
    ]);

    assert_eq!(
        akb006_paths(&descriptor),
        vec![
            "/sections/0/provenance/0/confidence",
            "/sections/0/provenance/0/locator/extra"
        ]
    );
}

#[test]
fn test_non_object_ignored() {
    let descriptor = json!(null);

    assert!(akb006_paths(&descriptor).is_empty());
}
