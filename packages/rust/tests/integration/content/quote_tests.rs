use std::fs;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use openakb_validate::{
    CheckKind, ContentReport, FullReport, LocalFileResolver, Mode, Outcome, check_content,
    validate_with_content,
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
async fn test_quote_verified() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "prefix quoted text suffix").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(
        report.checks[0].path,
        "/sections/0/provenance/0/locator/quote"
    );
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[0].detail, "quote found in capture");
}

#[tokio::test]
async fn test_quote_missing() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "different text").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert_eq!(
        report.checks[0].detail,
        "quote absent from fetched captures"
    );
}

#[tokio::test]
async fn test_empty_quote_no_check() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "some capture text").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    // The schema caps `quote` at `minLength: 1`, so an empty quote is not a
    // checkable claim: it is dropped before any substring search, so no Quote check
    // is produced and the linear searcher never receives an empty needle.
    assert!(
        report
            .checks
            .iter()
            .all(|check| check.kind != CheckKind::Quote)
    );
}

#[tokio::test]
async fn test_hash_failed_distrust() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "quoted text").unwrap();
    let descriptor = json!({
        "sources": [{
            "id": "SRC-000001",
            "capture_uri": "capture.txt",
            "content_hash": sri(b"other")
        }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(
        report.checks[1].detail,
        "a cited source's capture failed its content_hash"
    );
}

#[tokio::test]
async fn test_quote_unfetched() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "missing.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].detail, "no cited source capture fetched");
}

#[tokio::test]
async fn test_confusable_claim_id_unverifiable() {
    // U+212A KELVIN SIGN renders identically to ASCII 'K' but ASCII-only
    // `normalize_id` does not fold it: a claim citing the confusable variant of a
    // real source id must not resolve against that source's capture.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "hay stack").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-0000K1", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-0000\u{212a}1"],
                "locator": { "quote": "needle" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].detail, "no cited source capture fetched");
}

#[tokio::test]
async fn test_wrong_kind_source_id_unverifiable() {
    // A source's own id of the wrong kind (SEC- prefix) must not seed the capture
    // cache: the gate requires `id_kind == Some(EntityKind::Source)`, not merely a
    // well-formed typed id, so a claim repeating the same invalid id stays
    // unverifiable instead of resolving against bytes cached under an invalid key.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "hay needle stack").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SEC-000002", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SEC-000002"],
                "locator": { "quote": "needle" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].detail, "no cited source capture fetched");
}

#[tokio::test]
async fn test_malformed_source_id_unverifiable() {
    // A source's own id that fails the ASCII-only source-id pattern (Unicode
    // confusable) must not seed the capture cache either.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "hay needle stack").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-0000\u{212a}1", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-0000\u{212a}1"],
                "locator": { "quote": "needle" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[0].detail, "no cited source capture fetched");
}

#[tokio::test]
async fn test_valid_source_id_still_verifies() {
    // Regression guard: a well-formed source id still seeds the cache and VERIFIES.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "hay needle stack").unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "capture.txt" }],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "needle" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_quote_partial_gap() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "different text").unwrap();
    let descriptor = json!({
        "sources": [
            { "id": "SRC-000001", "capture_uri": "capture.txt" },
            { "id": "SRC-000002", "capture_uri": "missing.txt" }
        ],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001", "SRC-000002"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(
        report.checks[1].detail,
        "some cited source captures were not fetched"
    );
}

#[tokio::test]
async fn test_redacted_warning() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "different text").unwrap();
    let descriptor = json!({
        "sources": [
            { "id": "SRC-000001", "capture_uri": "capture.txt" },
            { "id": "SRC-000002", "type": "redacted", "capture_uri": "redacted.txt" }
        ],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001", "SRC-000002"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(report.checks[1].warnings.len(), 1);
    assert_eq!(
        report.checks[1].warnings[0].message,
        "quote cites redacted source(s): SRC-000002"
    );
}

#[tokio::test]
async fn test_quote_non_string_source_ids() {
    // A claim whose source_ids hold no strings has no usable citation, so it yields
    // no quote check rather than a claim with an empty source list.
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": [123],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert!(
        report
            .checks
            .iter()
            .all(|check| check.kind != CheckKind::Quote)
    );
}

#[tokio::test]
async fn test_quote_usable_with_hash_failure() {
    // One cited capture fetches cleanly without the quote while another fails its
    // content_hash: the usable capture is not empty, yet a distrusted sibling keeps
    // the quote unverifiable rather than failed.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("clean.txt"), "unrelated prose").unwrap();
    fs::write(dir.path().join("tampered.txt"), "tampered bytes").unwrap();
    let descriptor = json!({
        "sources": [
            { "id": "SRC-000001", "capture_uri": "clean.txt" },
            { "id": "SRC-000002", "capture_uri": "tampered.txt", "content_hash": sri(b"expected") }
        ],
        "sections": [{
            "id": "SEC-000001",
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001", "SRC-000002"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Capture);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(report.checks[1].outcome, Outcome::Unverifiable);
    assert_eq!(
        report.checks[1].detail,
        "a cited source's capture failed its content_hash"
    );
}

#[tokio::test]
async fn test_sidecar_quote() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "source quote").unwrap();
    fs::write(
        dir.path().join("sec.prov.json"),
        br#"{"section_id":"SEC-000001","claims":[{"text":"Claim.","source_ids":["SRC-000001"],"locator":{"quote":"source quote"}}]}"#,
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "SRC-000001", "capture_uri": "capture.txt" }],
        "sections": [{ "id": "SEC-000001", "provenance_uri": "sec.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].kind, CheckKind::Quote);
    assert_eq!(
        report.checks[1].path,
        "/sections/0/provenance_uri/claims/0/locator/quote"
    );
    assert_eq!(report.checks[1].outcome, Outcome::Verified);
}

#[tokio::test]
async fn test_validate_with_content() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("capture.txt"), "quoted text").unwrap();
    let resolver = LocalFileResolver::new(dir.path());
    let descriptor = json!({
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "kb",
        "title": "Example KB",
        "description": "A vendor-neutral example descriptor.",
        "sources": [{
            "id": "SRC-000001",
            "type": "document",
            "uri": "https://docs.example.com/source",
            "capture_uri": "capture.txt"
        }],
        "sections": [{
            "id": "SEC-000001",
            "title": "Section",
            "description": "A section.",
            "content_uri": "section.bin",
            "content_type": "application/octet-stream",
            "source_ids": ["SRC-000001"],
            "provenance": [{
                "text": "Claim.",
                "source_ids": ["SRC-000001"],
                "locator": { "quote": "quoted text" }
            }]
        }]
    });

    let report: FullReport = validate_with_content(&descriptor, &resolver, Mode::Lenient).await;

    assert!(report.ok());
    assert!(report.validation.ok());
    assert!(report.content.ok());
    assert_eq!(report.content.checks.len(), 1);
    assert_eq!(report.content.checks[0].kind, CheckKind::Quote);
    assert_eq!(report.content.checks[0].outcome, Outcome::Verified);
}
