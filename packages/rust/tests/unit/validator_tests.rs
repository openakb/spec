use serde_json::{Value, json};

use openakb_validate::{Advisory, Code, Finding, Mode, ValidationResult, validate};

fn descriptor() -> Value {
    json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "facade-fixture",
        "title": "Facade",
        "description": "Base for facade tests.",
        "sources": [{"id":"s1","type":"url","uri":"https://docs.example.com/a/"}],
        "sections": [{"id":"root","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["s1"]}]
    })
}

fn codes(result: &ValidationResult) -> Vec<Code> {
    result.codes().into_iter().collect()
}

#[test]
fn test_valid_descriptor_ok() {
    let descriptor = descriptor();

    let lenient = validate(&descriptor, Mode::Lenient);
    let strict = validate(&descriptor, Mode::Strict);

    assert!(lenient.ok());
    assert!(lenient.findings.is_empty());
    assert!(lenient.warnings.is_empty());
    assert!(strict.ok());
    assert!(strict.findings.is_empty());
    assert!(strict.warnings.is_empty());
}

#[test]
fn test_lenient_vs_strict() {
    let mut descriptor = descriptor();
    descriptor["future_field"] = json!(true);

    let lenient = validate(&descriptor, Mode::Lenient);
    let strict = validate(&descriptor, Mode::Strict);

    assert!(lenient.ok());
    assert_eq!(codes(&strict), vec![Code::Akb006]);
}

#[test]
fn test_findings_sorted_deduped() {
    let mut descriptor = descriptor();
    descriptor.as_object_mut().unwrap().remove("title");
    descriptor["sections"][0]["source_ids"] = json!(["ghost"]);

    let result = validate(&descriptor, Mode::Strict);
    let mut sorted_deduped = result.findings.clone();
    sorted_deduped.sort();
    sorted_deduped.dedup();

    assert!(!result.findings.is_empty());
    assert_eq!(result.findings, sorted_deduped);
}

#[test]
fn test_total_on_any_json() {
    for descriptor in [
        Value::Null,
        json!(42),
        json!("not a descriptor"),
        json!([]),
        json!({}),
        json!({"sources":[null, 7], "sections":{"root": true}}),
    ] {
        let _ = validate(&descriptor, Mode::Lenient);
        let _ = validate(&descriptor, Mode::Strict);
    }
}

#[test]
fn test_warnings_never_block() {
    let mut descriptor = descriptor();
    descriptor["sources"] = json!([
        {"id":"s1","type":"url","uri":"https://docs.example.com/a/","discovered_via_id":"s2"},
        {"id":"s2","type":"url","uri":"https://docs.example.com/b/","discovered_via_id":"s1"}
    ]);

    let result = validate(&descriptor, Mode::Lenient);

    assert!(result.ok());
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(codes(&result), Vec::<Code>::new());
}

#[test]
fn test_mode_default_lenient() {
    assert_eq!(Mode::default(), Mode::Lenient);
}

#[test]
fn test_public_exports_compile() {
    let _finding = Finding {
        code: Code::Akb001,
        path: String::new(),
        message: String::new(),
    };
    let _advisory = Advisory {
        path: String::new(),
        message: String::new(),
    };
    let _result = ValidationResult::default();
}
