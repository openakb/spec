use std::{
    fs,
    path::{Path, PathBuf},
};

use openakb_validate::{CheckKind, LocalFileResolver, Mode, Outcome, check_content, validate};
use serde_json::Value;

const AUTHORING: &[&str] = &[
    "minimal",
    "widget-platform",
    "cross-link",
    "sidecar-provenance",
];

fn examples_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn load_example(root: &Path, example: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(root.join(example).join("openakb.json")).unwrap())
        .unwrap()
}

fn examples_or_skip() -> Option<PathBuf> {
    let root = examples_root();
    if !root.exists() {
        eprintln!("skipping examples tests: {} absent", root.display());
        return None;
    }
    Some(root)
}

#[test]
fn test_examples_validate() {
    let Some(root) = examples_or_skip() else {
        return;
    };

    for example in AUTHORING.iter().copied().chain(["widget-platform-served"]) {
        let descriptor = load_example(&root, example);

        assert!(
            validate(&descriptor, Mode::Strict).ok(),
            "{example} should be strict-valid"
        );
    }
}

#[tokio::test]
async fn test_authoring_content() {
    let Some(root) = examples_or_skip() else {
        return;
    };

    for example in AUTHORING {
        let descriptor = load_example(&root, example);
        let resolver = LocalFileResolver::new(root.join(example));
        let report = check_content(&descriptor, &resolver).await;

        assert!(report.ok(), "{example} content report should be ok");
        assert!(!report.checks.is_empty(), "{example} should emit checks");
        for check in report.checks {
            if check.kind == CheckKind::Quote {
                assert!(
                    matches!(check.outcome, Outcome::Verified | Outcome::Unverifiable),
                    "{example} quote check should be verified or unverifiable: {check:?}"
                );
                if check.outcome == Outcome::Unverifiable {
                    assert_eq!(check.detail, "no cited source capture fetched", "{check:?}");
                }
            } else {
                assert_eq!(check.outcome, Outcome::Verified, "{example}: {check:?}");
            }
        }
    }
}

#[tokio::test]
async fn test_widget_artifacts() {
    let Some(root) = examples_or_skip() else {
        return;
    };
    let descriptor = load_example(&root, "widget-platform");
    let resolver = LocalFileResolver::new(root.join("widget-platform"));
    let report = check_content(&descriptor, &resolver).await;

    let verified: Vec<_> = report
        .checks
        .iter()
        .filter(|check| check.outcome == Outcome::Verified)
        .map(|check| check.kind)
        .collect();

    assert!(verified.contains(&CheckKind::Capture));
    assert!(verified.contains(&CheckKind::Sidecar));
    assert!(verified.contains(&CheckKind::Citations));
    assert!(report.with_outcome(Outcome::Failed).next().is_none());
}

#[tokio::test]
async fn test_served_unverifiable() {
    let Some(root) = examples_or_skip() else {
        return;
    };
    let descriptor = load_example(&root, "widget-platform-served");
    let resolver = LocalFileResolver::new(root.join("widget-platform-served"));
    let report = check_content(&descriptor, &resolver).await;

    assert!(report.ok());
    assert!(!report.checks.is_empty());
    assert!(
        report
            .checks
            .iter()
            .all(|check| check.outcome == Outcome::Unverifiable),
        "{:?}",
        report.checks
    );
}
