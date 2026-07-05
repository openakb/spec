use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use openakb_validate::{Mode, extract_citations, validate};
use serde_json::Value;

fn conformance_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn case_dirs(kind: &str) -> Option<Vec<PathBuf>> {
    let root = conformance_root();
    if !root.exists() {
        eprintln!("skipping conformance tests: {} absent", root.display());
        return None;
    }

    let mut paths: Vec<_> = fs::read_dir(root.join(kind))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    paths.sort();
    Some(paths)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn expected_strings<'a>(expected: &'a Value, key: &str) -> BTreeSet<&'a str> {
    expected
        .get(key)
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|code| code.as_str().unwrap())
        .collect()
}

fn result_code_strings(result: &openakb_validate::ValidationResult) -> BTreeSet<&str> {
    result
        .codes()
        .into_iter()
        .map(|code| code.as_str())
        .collect()
}

#[test]
fn test_valid_fixtures() {
    let Some(case_dirs) = case_dirs("valid") else {
        return;
    };

    for case_dir in case_dirs {
        let descriptor = read_json(&case_dir.join("openakb.json"));

        assert!(
            validate(&descriptor, Mode::Lenient).ok(),
            "{} should be lenient-valid",
            case_dir.display()
        );
        assert!(
            validate(&descriptor, Mode::Strict).ok(),
            "{} should be strict-valid",
            case_dir.display()
        );
    }
}

#[test]
fn test_invalid_fixtures() {
    let Some(case_dirs) = case_dirs("invalid") else {
        return;
    };

    for case_dir in case_dirs {
        let descriptor = read_json(&case_dir.join("openakb.json"));
        let expected = read_json(&case_dir.join("expected.json"));
        let result = validate(&descriptor, Mode::Lenient);
        let actual_codes = result_code_strings(&result);

        assert!(!result.ok(), "{} should be invalid", case_dir.display());
        for expected_code in expected_strings(&expected, "codes") {
            assert!(
                actual_codes.contains(expected_code),
                "{} should include expected code {expected_code}; actual: {actual_codes:?}",
                case_dir.display()
            );
        }
    }
}

#[test]
fn test_forward_fixtures() {
    let Some(case_dirs) = case_dirs("forward-compat") else {
        return;
    };

    for case_dir in case_dirs {
        let descriptor = read_json(&case_dir.join("openakb.json"));
        let expected = read_json(&case_dir.join("expected.json"));

        assert!(
            validate(&descriptor, Mode::Lenient).ok(),
            "{} should be lenient-valid",
            case_dir.display()
        );
        let strict = validate(&descriptor, Mode::Strict);
        let actual_codes = result_code_strings(&strict);
        for expected_code in expected_strings(&expected, "strict") {
            assert!(
                actual_codes.contains(expected_code),
                "{} should include strict code {expected_code}; actual: {actual_codes:?}",
                case_dir.display()
            );
        }
    }
}

#[test]
fn test_content_fixtures() {
    let Some(case_dirs) = case_dirs("content") else {
        return;
    };

    for case_dir in case_dirs {
        let markdown = fs::read_to_string(case_dir.join("content.md")).unwrap();
        let expected = read_json(&case_dir.join("expected.json"));
        let expected_ids: Vec<Vec<String>> = expected
            .get("citations")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|citation| {
                citation
                    .get("ids")
                    .and_then(Value::as_array)
                    .unwrap()
                    .iter()
                    .map(|id| id.as_str().unwrap().to_owned())
                    .collect()
            })
            .collect();
        let actual_ids: Vec<Vec<String>> = extract_citations(&markdown)
            .into_iter()
            .map(|citation| citation.ids)
            .collect();

        assert_eq!(
            actual_ids,
            expected_ids,
            "{} citation ids should match",
            case_dir.display()
        );
    }
}
