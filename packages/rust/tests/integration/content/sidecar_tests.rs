use std::fs;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use openakb_validate::{CheckKind, Code, ContentReport, LocalFileResolver, Outcome, check_content};
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
async fn test_sidecar_verified() {
    let dir = TempDir::new().unwrap();
    let sidecar = br#"{
        "section_id": "sec",
        "claims": [{
            "text": "Claim text.",
            "source_ids": ["s1"]
        }]
    }"#;
    fs::write(dir.path().join("sec.prov.json"), sidecar).unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [{
            "id": "sec",
            "provenance_uri": "sec.prov.json",
            "provenance_hash": sri(sidecar)
        }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].path, "/sections/0/provenance_hash");
    assert_eq!(report.checks[0].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[1].path, "/sections/0/provenance_uri");
    assert_eq!(report.checks[1].outcome, Outcome::Verified);
    assert_eq!(report.checks[1].detail, "provenance sidecar checked");
}

#[tokio::test]
async fn test_binding_mismatch() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("sec.prov.json"),
        br#"{"section_id":"other","claims":[{"text":"Claim.","source_ids":["s1"]}]}"#,
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [
            { "id": "sec", "provenance_uri": "sec.prov.json" },
            { "id": "other" }
        ]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert!(
        report.checks[0]
            .detail
            .contains("names a different section")
    );
    assert!(report.checks[0].findings.is_empty());
}

#[tokio::test]
async fn test_claim_source_unresolved() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("sec.prov.json"),
        br#"{"section_id":"sec","claims":[{"text":"Claim.","source_ids":["missing","other"]}]}"#,
    )
    .unwrap();
    let descriptor = json!({
        "sections": [
            { "id": "sec", "provenance_uri": "sec.prov.json" },
            { "id": "other" }
        ]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    let check = &report.checks[0];
    assert_eq!(check.kind, CheckKind::Sidecar);
    assert_eq!(check.outcome, Outcome::Failed);
    assert_eq!(check.findings.len(), 2);
    assert_eq!(check.findings[0].code, Code::Akb007);
    assert_eq!(
        check.findings[0].path,
        "/sections/0/provenance_uri/claims/0/source_ids/0"
    );
    assert_eq!(check.findings[1].code, Code::Akb010);
    assert_eq!(
        check.findings[1].path,
        "/sections/0/provenance_uri/claims/0/source_ids/1"
    );
}

#[tokio::test]
async fn test_section_id_unresolved() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("sec.prov.json"),
        br#"{"section_id":"s1","claims":[{"text":"Claim.","source_ids":["s1"]}]}"#,
    )
    .unwrap();
    let descriptor = json!({
        "sources": [{ "id": "s1" }],
        "sections": [{ "id": "sec", "provenance_uri": "sec.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].findings.len(), 1);
    assert_eq!(report.checks[0].findings[0].code, Code::Akb010);
    assert_eq!(
        report.checks[0].findings[0].path,
        "/sections/0/provenance_uri/section_id"
    );
}

#[tokio::test]
async fn test_malformed_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("sec.prov.json"), "{").unwrap();
    let descriptor = json!({
        "sections": [{ "id": "sec", "provenance_uri": "sec.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].path, "/sections/0/provenance_uri");
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
}

#[tokio::test]
async fn test_unfetchable() {
    let dir = TempDir::new().unwrap();
    let descriptor = json!({
        "sections": [{ "id": "sec", "provenance_uri": "missing.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].path, "/sections/0/provenance_uri");
    assert_eq!(report.checks[0].outcome, Outcome::Unverifiable);
}

#[tokio::test]
async fn test_malformed_provenance_hash() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("sec.prov.json"), br#"{"section_id":"sec"}"#).unwrap();
    let descriptor = json!({
        "sections": [{
            "id": "sec",
            "provenance_uri": "sec.prov.json",
            "provenance_hash": "sha256-@@@"
        }]
    });

    let report = report(descriptor, &dir).await;

    // A malformed provenance_hash SRI is an advisory, not a fatal finding: the
    // provenance_hash check is unverifiable while the sidecar still gets checked.
    let hash_check = report
        .checks
        .iter()
        .find(|check| check.path == "/sections/0/provenance_hash")
        .expect("provenance_hash check");
    assert_eq!(hash_check.kind, CheckKind::Sidecar);
    assert_eq!(hash_check.outcome, Outcome::Unverifiable);
    assert_eq!(hash_check.warnings.len(), 1);
}

#[tokio::test]
async fn test_non_object_sidecar() {
    let dir = TempDir::new().unwrap();
    // Valid JSON, but an array where the schema requires an object: schema findings
    // are raised, then the non-object short-circuits binding and claim extraction.
    fs::write(dir.path().join("sec.prov.json"), b"[]").unwrap();
    let descriptor = json!({
        "sections": [{ "id": "sec", "provenance_uri": "sec.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert!(!report.checks[0].findings.is_empty());
}

#[tokio::test]
async fn test_sidecar_schema_prefix() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("sec.prov.json"), br#"{"section_id":"sec"}"#).unwrap();
    let descriptor = json!({
        "sections": [{ "id": "sec", "provenance_uri": "sec.prov.json" }]
    });

    let report = report(descriptor, &dir).await;

    assert!(!report.ok());
    assert_eq!(report.checks.len(), 1);
    assert_eq!(report.checks[0].kind, CheckKind::Sidecar);
    assert_eq!(report.checks[0].outcome, Outcome::Failed);
    assert!(
        report.checks[0]
            .findings
            .iter()
            .any(|finding| finding.path.starts_with("/sections/0/provenance_uri"))
    );
}
