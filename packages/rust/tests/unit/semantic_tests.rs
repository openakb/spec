use serde_json::{Value, json};

use openakb_validate::{Code, Mode, validate};

fn descriptor() -> Value {
    json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "semantic-fixture",
        "title": "Semantic",
        "description": "Base for semantic-rule tests.",
        "sources": [{"id":"s1","type":"url","uri":"https://docs.example.com/a/"}],
        "sections": [{"id":"root","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["s1"]}]
    })
}

fn codes(descriptor: &Value) -> Vec<Code> {
    validate(descriptor, Mode::Lenient)
        .codes()
        .into_iter()
        .collect()
}

fn paths_for(descriptor: &Value, code: Code) -> Vec<String> {
    validate(descriptor, Mode::Lenient)
        .findings
        .into_iter()
        .filter(|finding| finding.code == code)
        .map(|finding| finding.path)
        .collect()
}

#[test]
fn test_duplicate_ids() {
    let mut descriptor = descriptor();
    descriptor["sources"] = json!([
        {"id":"s1","type":"url","uri":"https://docs.example.com/a/"},
        {"id":"s1","type":"url","uri":"https://docs.example.com/b/"}
    ]);

    let paths = paths_for(&descriptor, Code::Akb001);

    assert_eq!(paths, vec!["/sources/1/id"]);
}

#[test]
fn test_shared_namespace_duplicate() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["id"] = json!("s1");

    assert!(codes(&descriptor).contains(&Code::Akb001));
}

#[test]
fn test_empty_section() {
    let mut descriptor = descriptor();
    descriptor["sections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"hollow","title":"Hollow","description":"No content or child."}));

    let paths = paths_for(&descriptor, Code::Akb002);

    assert_eq!(paths, vec!["/sections/1"]);
}

#[test]
fn test_unresolved_references() {
    let mut descriptor = descriptor();
    descriptor["sources"][0]["discovered_via_id"] = json!("ghost-source");
    descriptor["sections"][0]["parent_id"] = json!("ghost-parent");
    descriptor["sections"][0]["source_ids"] = json!(["s1", "ghost-source"]);
    descriptor["sections"][0]["links"] = json!([
        {"rel":"see-also","section_id":"ghost-section"}
    ]);
    descriptor["sections"][0]["provenance"] = json!([
        {"text":"Inline claim.","source_ids":["ghost-claim-source"]}
    ]);

    let paths = paths_for(&descriptor, Code::Akb007);

    assert_eq!(
        paths,
        vec![
            "/sections/0/links/0/section_id",
            "/sections/0/parent_id",
            "/sections/0/provenance/0/source_ids/0",
            "/sections/0/source_ids/1",
            "/sources/0/discovered_via_id",
        ]
    );
}

#[test]
fn test_wrong_kind_references() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["parent_id"] = json!("s1");
    descriptor["sections"][0]["source_ids"] = json!(["s1", "root"]);

    let result = validate(&descriptor, Mode::Lenient);
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|finding| finding.code == Code::Akb010)
        .collect();

    assert_eq!(
        result.codes().into_iter().collect::<Vec<_>>(),
        vec![Code::Akb010]
    );
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_link_with_akb_uri_skipped() {
    let mut descriptor = descriptor();
    descriptor["sections"][0]["links"] = json!([
        {"rel":"see-also","akb_uri":"https://kb.example.org/other","section_id":"ghost-section"}
    ]);

    assert_eq!(codes(&descriptor), Vec::new());
}

#[test]
fn test_parent_cycle() {
    let mut descriptor = descriptor();
    descriptor["sections"] = json!([
        {"id":"a","title":"A","description":"A.","parent_id":"b","content_uri":"a.md","source_ids":["s1"]},
        {"id":"b","title":"B","description":"B.","parent_id":"a","content_uri":"b.md","source_ids":["s1"]}
    ]);

    let result = validate(&descriptor, Mode::Lenient);
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|finding| finding.code == Code::Akb004)
        .collect();

    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("a -> b -> a"));
}

#[test]
fn test_parent_depth_cap() {
    let mut valid = descriptor();
    valid["sections"] = chain_sections(64);
    assert!(!codes(&valid).contains(&Code::Akb005));

    let mut invalid = descriptor();
    invalid["sections"] = chain_sections(65);
    assert!(codes(&invalid).contains(&Code::Akb005));
}

#[test]
fn test_discovery_cycle_warns() {
    let mut descriptor = descriptor();
    descriptor["sources"] = json!([
        {"id":"s1","type":"url","uri":"https://docs.example.com/a/","discovered_via_id":"s2"},
        {"id":"s2","type":"url","uri":"https://docs.example.com/b/","discovered_via_id":"s1"}
    ]);

    let result = validate(&descriptor, Mode::Lenient);

    assert!(result.ok());
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].message.contains("s1 -> s2 -> s1"));
}

#[test]
fn test_garbage_shapes_ignored() {
    for descriptor in [
        json!({"sources":42,"sections":{"a":1}}),
        json!([]),
        json!("x"),
    ] {
        let codes = codes(&descriptor);

        assert!(!codes.contains(&Code::Akb001));
        assert!(!codes.contains(&Code::Akb002));
        assert!(!codes.contains(&Code::Akb004));
        assert!(!codes.contains(&Code::Akb007));
        assert!(!codes.contains(&Code::Akb010));
    }
}

fn chain_sections(depth: usize) -> Value {
    let sections: Vec<_> = (0..depth)
        .map(|index| {
            let id = format!("n{index}");
            let parent_id = index
                .checked_sub(1)
                .map(|parent_index| format!("n{parent_index}"));
            let mut section = json!({
                "id": id,
                "title": format!("Node {index}"),
                "description": "Depth fixture.",
                "content_uri": format!("n{index}.md"),
                "source_ids": ["s1"]
            });
            if let Some(parent_id) = parent_id {
                section["parent_id"] = json!(parent_id);
            }
            section
        })
        .collect();

    json!(sections)
}
