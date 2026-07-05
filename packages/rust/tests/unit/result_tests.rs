use std::collections::BTreeSet;

use serde_json::json;

use openakb_validate::{Advisory, Code, Finding, Segment, ValidationResult, json_pointer};

fn finding(code: Code, path: &str) -> Finding {
    Finding {
        code,
        path: path.to_owned(),
        message: "m".to_owned(),
    }
}

#[test]
fn test_pointer_escaping() {
    assert_eq!(
        json_pointer([Segment::Key("a/b"), Segment::Key("m~n"), Segment::Index(3)]),
        "/a~1b/m~0n/3",
    );
}

#[test]
fn test_pointer_root_empty() {
    assert_eq!(json_pointer([]), "");
}

#[test]
fn test_result_ok_and_codes() {
    let mut result = ValidationResult::default();
    assert!(result.ok());
    assert_eq!(result.codes(), BTreeSet::new());

    result.findings = vec![
        finding(Code::Akb003, "/b"),
        finding(Code::Akb001, "/a"),
        finding(Code::Akb003, "/c"),
    ];
    result.warnings = vec![Advisory {
        path: "/warnings/0".to_owned(),
        message: "w".to_owned(),
    }];

    assert!(!result.ok());
    assert_eq!(result.codes(), BTreeSet::from([Code::Akb001, Code::Akb003]));
}

#[test]
fn test_finding_name() {
    assert_eq!(
        finding(Code::Akb012, "/sections/0/links/0").name(),
        "link-missing-target"
    );
}

#[test]
fn test_finding_sort_order() {
    let mut findings = vec![
        Finding {
            code: Code::Akb002,
            path: "/a".to_owned(),
            message: "a".to_owned(),
        },
        Finding {
            code: Code::Akb001,
            path: "/b".to_owned(),
            message: "a".to_owned(),
        },
        Finding {
            code: Code::Akb001,
            path: "/a".to_owned(),
            message: "b".to_owned(),
        },
        Finding {
            code: Code::Akb001,
            path: "/a".to_owned(),
            message: "a".to_owned(),
        },
    ];

    findings.sort();

    assert_eq!(
        findings,
        vec![
            Finding {
                code: Code::Akb001,
                path: "/a".to_owned(),
                message: "a".to_owned(),
            },
            Finding {
                code: Code::Akb001,
                path: "/a".to_owned(),
                message: "b".to_owned(),
            },
            Finding {
                code: Code::Akb001,
                path: "/b".to_owned(),
                message: "a".to_owned(),
            },
            Finding {
                code: Code::Akb002,
                path: "/a".to_owned(),
                message: "a".to_owned(),
            },
        ],
    );
}

#[test]
fn test_finding_serializes() {
    assert_eq!(
        serde_json::to_value(finding(Code::Akb001, "/sources/0/id")).unwrap(),
        json!({
            "code": "AKB001",
            "path": "/sources/0/id",
            "message": "m",
        }),
    );
}

#[test]
fn test_result_serializes() {
    let result = ValidationResult {
        findings: vec![finding(Code::Akb001, "/sources/0/id")],
        warnings: vec![Advisory {
            path: "/links/0/target".to_owned(),
            message: "w".to_owned(),
        }],
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({
            "findings": [
                {
                    "code": "AKB001",
                    "path": "/sources/0/id",
                    "message": "m",
                },
            ],
            "warnings": [
                {
                    "path": "/links/0/target",
                    "message": "w",
                },
            ],
        }),
    );
}
