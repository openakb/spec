//! Content-layer citation verification through the public `check_content` contract.

use std::collections::BTreeMap;

use async_trait::async_trait;
use openakb_validate::{
    CheckKind, Code, ContentCheck, ContentReport, Outcome, Resolver, Unfetchable, check_content,
};
use serde_json::{Value, json};

struct MapResolver {
    payloads: BTreeMap<String, Vec<u8>>,
}

#[async_trait]
impl Resolver for MapResolver {
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable> {
        self.payloads.get(uri).cloned().ok_or_else(|| Unfetchable {
            reason: format!("missing {uri}"),
        })
    }
}

fn descriptor() -> Value {
    json!({
        "sources": [{"id":"SRC-000001","type":"url","uri":"https://docs.example.com/a/"}],
        "sections": [{"id":"SEC-000001","title":"Root","description":"The only section.","content_uri":"root.md","source_ids":["SRC-000001"]}]
    })
}

fn citation_check(report: &ContentReport) -> &ContentCheck {
    report
        .checks
        .iter()
        .find(|check| check.kind == CheckKind::Citations)
        .unwrap()
}

fn finding_codes(report: &ContentReport) -> Vec<Code> {
    citation_check(report)
        .findings
        .iter()
        .map(|finding| finding.code)
        .collect()
}

#[tokio::test]
async fn test_non_typed_citation_akb011() {
    // Fetched Markdown is invisible to the descriptor schema, so a non-typed
    // token was not pre-reported there as AKB011 -- the content layer reports
    // it itself instead of silently verifying.
    let resolver = MapResolver {
        payloads: BTreeMap::from([("root.md".to_owned(), b"See [cite: s1].".to_vec())]),
    };

    let report = check_content(&descriptor(), &resolver).await;

    let check = citation_check(&report);
    assert_eq!(check.outcome, Outcome::Failed);
    assert_eq!(finding_codes(&report), vec![Code::Akb011]);
    assert!(
        check.findings[0]
            .message
            .contains("is not a typed source id")
    );
}

#[tokio::test]
async fn test_typed_undeclared_citation_akb007() {
    // A well-formed typed token that resolves nowhere is still AKB007.
    let resolver = MapResolver {
        payloads: BTreeMap::from([("root.md".to_owned(), b"See [cite: SRC-999999].".to_vec())]),
    };

    let report = check_content(&descriptor(), &resolver).await;

    assert_eq!(citation_check(&report).outcome, Outcome::Failed);
    assert_eq!(finding_codes(&report), vec![Code::Akb007]);
}

#[tokio::test]
async fn test_case_insensitive_citation_resolves() {
    // The source registry is casefolded, so a marker spelling the id in a
    // different ASCII case resolves.
    let resolver = MapResolver {
        payloads: BTreeMap::from([("root.md".to_owned(), b"See [cite: src-000001].".to_vec())]),
    };

    let report = check_content(&descriptor(), &resolver).await;

    let check = citation_check(&report);
    assert_eq!(check.outcome, Outcome::Verified);
    assert!(check.findings.is_empty());
}
