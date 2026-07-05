use std::{collections::BTreeMap, fs, sync::Mutex};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use openakb_validate::{
    CheckKind, Code, ContentReport, LocalFileResolver, Outcome, Resolver, Unfetchable,
    check_content,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn sri(payload: &[u8]) -> String {
    let digest = Sha256::digest(payload);
    format!("sha256-{}", STANDARD.encode(digest))
}

async fn report(descriptor: Value, dir: &TempDir) -> ContentReport {
    let resolver = LocalFileResolver::new(dir.path());
    check_content(&descriptor, &resolver).await
}

#[tokio::test]
async fn test_citation_findings() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("section.md"),
        "A [cite:s1, missing, sec_other] and again [cite:s1, s1].\n",
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [
            {
                "id": "sec",
                "content_uri": "section.md",
                "content_type": "text/Markdown; charset=utf-8"
            },
            { "id": "sec_other" }
        ]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    let check = &report.checks[0];
    assert_eq!(check.kind, CheckKind::Citations);
    assert_eq!(check.path, "/sections/0/content_uri");
    assert_eq!(check.outcome, Outcome::Failed);
    assert_eq!(check.detail, "citation markers checked");
    assert_eq!(check.findings.len(), 2);
    assert_eq!(check.findings[0].code, Code::Akb007);
    assert_eq!(
        check.findings[0].path,
        "/sections/0/content_uri/citations/0/1"
    );
    assert_eq!(check.findings[1].code, Code::Akb010);
    assert_eq!(
        check.findings[1].path,
        "/sections/0/content_uri/citations/0/2"
    );
    assert_eq!(check.warnings.len(), 1);
    assert_eq!(check.warnings[0].path, "/sections/0/content_uri");
    assert_eq!(
        check.warnings[0].message,
        "duplicate citation id in marker: s1"
    );
}

#[tokio::test]
async fn test_markdown_parameters() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), "A [cite:s1].\n").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [{
            "id": "sec",
            "content_uri": "section.md",
            "content_type": "text/Markdown; charset=utf-8"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Citations);
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_non_markdown_skip() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.txt"), "A [cite:missing].\n").unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "sec",
            "content_uri": "section.txt",
            "content_type": "text/plain"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert!(report.checks.is_empty());
}

#[tokio::test]
async fn test_invalid_utf8() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), [0xff, 0xfe]).unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "sec",
            "content_uri": "section.md"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Citations);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert!(report.checks[0].detail.contains("invalid utf-8"));
}

#[tokio::test]
async fn test_content_hash_order() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), "A [cite:s1].\n").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [{
            "id": "sec",
            "content_uri": "section.md",
            "content_hash": sri(b"A [cite:s1].\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::ContentHash);
    assert_eq!(report.checks[0].path, "/sections/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].kind, CheckKind::Citations);
    assert_eq!(report.checks[1].path, "/sections/0/content_uri");
    assert_eq!(report.checks[1].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_local_prescreen() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "sec",
            "content_uri": "section.md?cache=1"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Citations);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(
        report.checks[0].detail,
        "unfetchable: outside local base: section.md?cache=1"
    );
}

#[tokio::test]
async fn test_custom_query() {
    let resolver = MapResolver::new([(
        "https://docs.example.com/akb/index.akb.json?content".to_owned(),
        b"A [cite:s1].\n".to_vec(),
    )]);
    let descriptor = json!({
        "base_uri": "https://docs.example.com/akb/index.akb.json?old",
        "sources": [{ "id": "s1" }],
        "sections": [{
            "id": "sec",
            "content_uri": "?content"
        }]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Citations);
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(
        resolver.fetched(),
        vec!["https://docs.example.com/akb/index.akb.json?content"]
    );
}

struct MapResolver {
    payloads: BTreeMap<String, Vec<u8>>,
    fetched: Mutex<Vec<String>>,
}

impl MapResolver {
    fn new(payloads: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self {
            payloads: payloads.into_iter().collect(),
            fetched: Mutex::new(Vec::new()),
        }
    }

    fn fetched(&self) -> Vec<String> {
        self.fetched.lock().unwrap().clone()
    }
}

#[async_trait]
impl Resolver for MapResolver {
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable> {
        self.fetched.lock().unwrap().push(uri.to_owned());
        self.payloads.get(uri).cloned().ok_or_else(|| Unfetchable {
            reason: format!("missing {uri}"),
        })
    }
}
