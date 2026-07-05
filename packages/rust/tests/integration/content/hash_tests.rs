use std::{fs, sync::Mutex};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use openakb_validate::{
    CheckKind, ContentReport, LocalFileResolver, Outcome, Resolver, Unfetchable, check_content,
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
async fn test_guide_verified() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("guide.md"), "guide\n").unwrap();
    let descriptor = json!({
        "guide_uri": "guide.md",
        "guide_hash": sri(b"guide\n")
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::GuideHash);
    assert_eq!(report.checks[0].path, "/guide_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[0].detail, "sha256 digest matches");
}

#[tokio::test]
async fn test_guide_mismatch() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("guide.md"), "guide\n").unwrap();
    let descriptor = json!({
        "guide_uri": "guide.md",
        "guide_hash": sri(b"other\n")
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert_eq!(report.checks[0].detail, "sha256 digest mismatch");
    assert_eq!(report.with_outcome(Outcome::Failed).count(), 1);
    assert_eq!(report.findings().count(), 0);
    assert_eq!(report.warnings().count(), 0);
}

#[tokio::test]
async fn test_malformed_sri() {
    let cases = [
        "no-separator-missing",
        "md5-abcd",
        "sha256-@@@",
        "sha256-YWJj",
        "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB",
    ];

    for guide_hash in cases {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("guide.md"), "guide\n").unwrap();
        let descriptor = json!({
            "guide_uri": "guide.md",
            "guide_hash": guide_hash
        });

        let report = report(descriptor, &dir).await;

        assert!(report.ok(), "{guide_hash}");
        assert_eq!(report.checks.len(), 1, "{guide_hash}");
        assert_eq!(report.checks[0].kind, CheckKind::GuideHash);
        assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
        assert_eq!(report.checks[0].warnings.len(), 1);
        assert_eq!(report.checks[0].warnings[0].path, "/guide_hash");
    }
}

#[tokio::test]
async fn test_missing_guide_unverifiable() {
    let dir = TempDir::new().unwrap();
    let missing_uri = json!({ "guide_hash": sri(b"guide\n") });
    let missing_resource = json!({
        "guide_uri": "missing.md",
        "guide_hash": sri(b"guide\n")
    });

    for descriptor in [missing_uri, missing_resource] {
        let report = report(descriptor, &dir).await;

        assert!(report.ok());
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].kind, CheckKind::GuideHash);
        assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    }
}

#[tokio::test]
async fn test_base_uri_joined() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs").join("guide.md"), "guide\n").unwrap();
    fs::write(dir.path().join("docs").join("capture.txt"), "capture\n").unwrap();
    let descriptor = json!({
        "base_uri": "docs/index.akb.json#descriptor",
        "guide_uri": "guide.md#top",
        "guide_hash": sri(b"guide\n"),
        "sources": [{
            "id": "s1",
            "capture_uri": "capture.txt#quote",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_base_uri_directory() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs").join("guide.md"), "guide\n").unwrap();
    let descriptor = json!({
        "base_uri": "docs/",
        "guide_uri": "guide.md",
        "guide_hash": sri(b"guide\n")
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_base_uri_dot_segments() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::create_dir(dir.path().join("snapshots")).unwrap();
    fs::write(
        dir.path().join("snapshots").join("capture.txt"),
        "capture\n",
    )
    .unwrap();
    let descriptor = json!({
        "base_uri": "docs/index.akb.json",
        "sources": [{
            "id": "s1",
            "capture_uri": "../snapshots/capture.txt",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_base_uri_fragment_only() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs").join("capture.txt"), "capture\n").unwrap();
    let descriptor = json!({
        "base_uri": "docs/capture.txt",
        "sources": [{
            "id": "s1",
            "capture_uri": "#quote",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_relative_path_drops_base_query() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs").join("guide.md"), "guide\n").unwrap();
    let descriptor = json!({
        "base_uri": "docs/index.akb.json?old",
        "guide_uri": "guide.md",
        "guide_hash": sri(b"guide\n")
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_query_reference() {
    let resolver = RecordingResolver::new(b"guide\n");
    let descriptor = json!({
        "base_uri": "https://docs.example.com/akb/index.akb.json?old",
        "guide_uri": "?new",
        "guide_hash": sri(b"guide\n")
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(
        resolver.fetched(),
        vec!["https://docs.example.com/akb/index.akb.json?new"]
    );
}

#[tokio::test]
async fn test_rfc_absolute_base_resolution() {
    let resolver = RecordingResolver::new(b"capture\n");
    let descriptor = json!({
        "base_uri": "https://docs.example.com/a/b/index.akb.json",
        "sources": [{
            "id": "s1",
            "capture_uri": "../capture.txt#quote",
            "content_hash": sri(b"capture\n")
        }, {
            "id": "s2",
            "capture_uri": "/root.txt",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(
        resolver.fetched(),
        vec![
            "https://docs.example.com/a/capture.txt",
            "https://docs.example.com/root.txt"
        ]
    );
}

#[tokio::test]
async fn test_resolved_uri_not_normalized() {
    let resolver = RecordingResolver::new(b"capture\n");
    let descriptor = json!({
        "base_uri": "HTTP://Docs.Example.COM:80/a/b/index.akb.json",
        "sources": [{
            "id": "s1",
            "capture_uri": "%7e/capture.txt#quote",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert_eq!(
        resolver.fetched(),
        vec!["HTTP://Docs.Example.COM:80/a/b/%7e/capture.txt"]
    );
}

#[tokio::test]
async fn test_absolute_capture_uri() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "capture\n").unwrap();
    let descriptor = json!({
        "base_uri": "docs/index.akb.json",
        "sources": [{
            "id": "s1",
            "capture_uri": "capture.txt",
            "content_hash": sri(b"capture\n")
        }, {
            "id": "s2",
            "capture_uri": "https://example.com/capture.txt",
            "content_hash": sri(b"capture\n")
        }, {
            "id": "s3",
            "capture_uri": "/capture.txt",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 3);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[2].outcome, Outcome::Unverifiable);
}

#[tokio::test]
async fn test_capture_missing_uri() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sources": [{
            "id": "s1",
            "content_hash": sri(b"capture\n")
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].path, "/sources/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].detail, "missing capture_uri");
}

#[tokio::test]
async fn test_capture_fetch_failures() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sources": [{
            "id": "s1",
            "capture_uri": "missing.txt",
            "content_hash": sri(b"capture\n")
        }, {
            "id": "s2",
            "capture_uri": "also-missing.txt"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].path, "/sources/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].path, "/sources/1/capture_uri");
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
}

#[tokio::test]
async fn test_capture_malformed_sri() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "capture\n").unwrap();
    let descriptor = json!({
        "sources": [{
            "id": "s1",
            "capture_uri": "capture.txt",
            "content_hash": "sha256-@@@"
        }, {
            "id": "s2",
            "capture_uri": "missing.txt",
            "content_hash": "sha256-@@@"
        }, {
            "id": "s3",
            "content_hash": "sha256-@@@"
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 4);
    assert_eq!(report.checks[0].path, "/sources/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].path, "/sources/1/content_hash");
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[2].path, "/sources/1/capture_uri");
    assert_eq!(report.checks[2].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[3].path, "/sources/2/content_hash");
    assert_eq!(report.checks[3].outcome, Outcome::Unverifiable);
}

#[tokio::test]
async fn test_capture_verified_failed() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "capture\n").unwrap();
    let descriptor = json!({
        "sources": [
            {
                "id": "s1",
                "capture_uri": "capture.txt",
                "content_hash": sri(b"capture\n")
            },
            {
                "id": "s2",
                "capture_uri": "capture.txt",
                "content_hash": sri(b"other\n")
            }
        ]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].path, "/sources/0/content_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].kind, CheckKind::Capture);
    assert_eq!(report.checks[1].path, "/sources/1/content_hash");
    assert_eq!(report.checks[1].outcome, Outcome::Failed);
}

#[tokio::test]
async fn test_redacted_capture_fields() {
    let dir = TempDir::new().unwrap();
    let with_capture_fields = json!({
        "sources": [{
            "id": "s1",
            "type": "redacted",
            "capture_uri": "capture.txt",
            "content_hash": sri(b"capture\n")
        }]
    });
    let without_capture_fields = json!({
        "sources": [{
            "id": "s1",
            "type": "redacted"
        }]
    });

    let with_report = report(with_capture_fields, &dir).await;

    assert!(with_report.ok());
    assert_eq!(with_report.checks.len(), 1);
    assert_eq!(with_report.checks[0].kind, CheckKind::Capture);
    assert_eq!(with_report.checks[0].path, "/sources/0");
    assert_eq!(with_report.checks[0].outcome, Outcome::Unverifiable);

    let without_report = report(without_capture_fields, &dir).await;

    assert!(without_report.ok());
    assert!(without_report.checks.is_empty());
}

#[tokio::test]
async fn test_garbage_descriptors() {
    let dir = TempDir::new().unwrap();
    let descriptors = [
        json!(null),
        json!([]),
        json!({
            "guide_hash": false,
            "sources": [
                null,
                {
                    "capture_uri": 3,
                    "content_hash": false
                }
            ]
        }),
    ];

    for descriptor in descriptors {
        let report = report(descriptor, &dir).await;

        assert!(report.ok());
        assert!(report.checks.is_empty());
    }
}

struct RecordingResolver {
    payload: Vec<u8>,
    fetched: Mutex<Vec<String>>,
}

impl RecordingResolver {
    fn new(payload: &[u8]) -> Self {
        Self {
            payload: payload.to_vec(),
            fetched: Mutex::new(Vec::new()),
        }
    }

    fn fetched(&self) -> Vec<String> {
        self.fetched.lock().unwrap().clone()
    }
}

#[async_trait]
impl Resolver for RecordingResolver {
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable> {
        self.fetched.lock().unwrap().push(uri.to_owned());
        Ok(self.payload.clone())
    }
}
