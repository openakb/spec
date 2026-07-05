use std::fs;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use openakb_validate::{CheckKind, ContentReport, LocalFileResolver, Outcome, check_content};
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
