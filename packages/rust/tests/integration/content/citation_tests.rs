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
        "A [cite:SRC-000001, SRC-000009, SEC-000002] and again [cite:SRC-000001, SRC-000001].\n",
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }],
        "sections": [
            {
                "id": "SEC-000001",
                "content_uri": "section.md",
                "content_type": "text/Markdown; charset=utf-8"
            },
            { "id": "SEC-000002" }
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
        "duplicate citation id in marker: SRC-000001"
    );
}

#[tokio::test]
async fn test_duplicate_ids_sorted_deduped() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("section.md"),
        "A [cite:SRC-000002, SRC-000001, SRC-000002, SRC-000001, SRC-000003, SRC-000001].\n",
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }, { "id": "SRC-000002" }, { "id": "SRC-000003" }],
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.md",
            "content_type": "text/markdown"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert_eq!(report.checks.len(), 1);
    let check = &report.checks[0];
    assert_eq!(check.kind, CheckKind::Citations);
    assert_eq!(check.outcome, Outcome::Verified);
    assert_eq!(check.warnings.len(), 1);
    // Reported sorted and deduped: `SRC-000001` (x3) and `SRC-000002` (x2) are
    // duplicates; `SRC-000003` (x1) is not, and each duplicate id appears once.
    assert_eq!(
        check.warnings[0].message,
        "duplicate citation id in marker: SRC-000001, SRC-000002"
    );
}

#[tokio::test]
async fn test_duplicate_ids_large_marker() {
    let dir = TempDir::new().unwrap();
    let ids_list = std::iter::repeat_n("SRC-000001", 40_000)
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        dir.path().join("section.md"),
        format!("A [cite:{ids_list}].\n"),
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }],
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.md",
            "content_type": "text/markdown"
        }]
    });

    let report = report(descriptor, &dir).await;

    // A single-pass count keeps a 40k-id marker linear; the nested per-id rescan it
    // replaced would not finish within the suite.
    assert_eq!(report.checks.len(), 1);
    let check = &report.checks[0];
    assert_eq!(check.kind, CheckKind::Citations);
    assert_eq!(check.warnings.len(), 1);
    assert_eq!(
        check.warnings[0].message,
        "duplicate citation id in marker: SRC-000001"
    );
}

#[tokio::test]
async fn test_markdown_parameters() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), "A [cite:SRC-000001].\n").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }],
        "sections": [{
            "id": "SEC-000001",
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
            "id": "SEC-000001",
            "content_uri": "section.txt",
            "content_type": "text/plain"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert!(report.checks.is_empty());
}

#[tokio::test]
async fn test_non_markdown_hash() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.txt"), "A [cite:missing].\n").unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.txt",
            "content_type": "text/plain",
            "content_hash": sri(b"A [cite:missing].\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::ContentHash);
    assert_eq!(report.checks[0].path, "/sections/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_non_markdown_hash_warning() {
    let resolver = MapResolver::new([]);
    let descriptor = json!({
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "missing.txt",
            "content_type": "text/plain",
            "content_hash": "sha256-@@@"
        }]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::ContentHash);
    assert_eq!(report.checks[0].path, "/sections/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].warnings.len(), 1);
    assert_eq!(
        report.checks[0].warnings[0].path,
        "/sections/0/content_hash"
    );
    assert!(resolver.fetched().is_empty());
}

#[tokio::test]
async fn test_invalid_utf8() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), [0xff, 0xfe]).unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "SEC-000001",
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
    fs::write(dir.path().join("section.md"), "A [cite:SRC-000001].\n").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }],
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.md",
            "content_hash": sri(b"A [cite:SRC-000001].\n")
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
async fn test_markdown_malformed_hash() {
    // A Markdown section carries both a content_hash and citation checks. A malformed
    // hash SRI is an advisory, so the section is still fetched and its citations run.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("section.md"), "A [cite:SRC-000001].\n").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001" }],
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.md",
            "content_hash": "sha256-@@@"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::ContentHash);
    assert_eq!(report.checks[0].path, "/sections/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].warnings.len(), 1);
    assert_eq!(report.checks[1].kind, CheckKind::Citations);
    assert_eq!(report.checks[1].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_relative_base_reference_join() {
    // A relative base_uri resolves references RFC-3986-style without an absolute
    // authority: `..` pops a segment, a bare query keeps the base path, and a
    // trailing-slash reference stays a directory.
    let content = b"A [cite:SRC-000001].\n".to_vec();
    let resolver = MapResolver::new([
        ("a/c/section.md".to_owned(), content.clone()),
        ("a/b/index.akb.json?content".to_owned(), content.clone()),
        ("a/b/sub/".to_owned(), content.clone()),
    ]);
    let descriptor = json!({
        "base_uri": "a/b/index.akb.json",
        "sources": [{ "id": "SRC-000001" }],
        "sections": [
            { "id": "SEC-000001", "content_uri": "../c/section.md" },
            { "id": "SEC-000002", "content_uri": "?content" },
            { "id": "SEC-000003", "content_uri": "sub/" }
        ]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(
        resolver.fetched(),
        vec!["a/c/section.md", "a/b/index.akb.json?content", "a/b/sub/"]
    );
    assert!(
        report
            .checks
            .iter()
            .all(|check| check.kind == CheckKind::Citations && check.outcome == Outcome::Verified)
    );
}

#[tokio::test]
async fn test_local_prescreen() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "SEC-000001",
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
        b"A [cite:SRC-000001].\n".to_vec(),
    )]);
    let descriptor = json!({
        "base_uri": "https://docs.example.com/akb/index.akb.json?old",
        "sources": [{ "id": "SRC-000001" }],
        "sections": [{
            "id": "SEC-000001",
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

#[tokio::test]
async fn test_malformed_id_excluded() {
    // U+212A KELVIN SIGN renders identically to ASCII 'K' but fails the typed id
    // grammar. The content registry must not admit it, so a citation using the
    // real ASCII id resolves to nothing rather than to the malformed source --
    // pinning parity with the reference validator.
    let resolver =
        MapResolver::new([("section.md".to_owned(), b"A [cite:SRC-0000K1].\n".to_vec())]);
    let descriptor = json!({
        "sources": [{ "id": "SRC-0000\u{212a}1" }],
        "sections": [{
            "id": "SEC-000001",
            "content_uri": "section.md"
        }]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Citations);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert_eq!(report.checks[0].findings.len(), 1);
    assert_eq!(report.checks[0].findings[0].code, Code::Akb007);
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
