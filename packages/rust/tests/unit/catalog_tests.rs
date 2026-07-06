use openakb_validate::{Code, LOCAL_ID_CHARSET, LOCAL_ID_MAX_LENGTH, PARENT_DEPTH_MAX};

#[test]
fn test_code_round_trip() {
    assert_eq!(Code::ALL.len(), 12);

    for (index, code) in Code::ALL.iter().copied().enumerate() {
        let expected = format!("AKB{:03}", index + 1);

        assert_eq!(code.as_str(), expected);
        assert_eq!(code.to_string(), expected);
    }
}

#[test]
fn test_code_names() {
    let expected = [
        (Code::Akb001, "id-not-unique"),
        (Code::Akb002, "empty-section"),
        (Code::Akb003, "missing-source-cite"),
        (Code::Akb004, "parent-cycle"),
        (Code::Akb005, "cap-exceeded"),
        (Code::Akb006, "unknown-core-property"),
        (Code::Akb007, "unresolved-reference"),
        (Code::Akb008, "unknown-rel"),
        (Code::Akb009, "missing-required-field"),
        (Code::Akb010, "invalid-reference-kind"),
        (Code::Akb011, "malformed-value"),
        (Code::Akb012, "link-missing-target"),
    ];

    assert_eq!(Code::ALL.len(), expected.len());
    for (code, name) in expected {
        assert_eq!(code.name(), name);
    }
}

#[test]
fn test_spec_caps() {
    assert_eq!(PARENT_DEPTH_MAX, 64);
    assert_eq!(LOCAL_ID_MAX_LENGTH, 64);
    assert_eq!(LOCAL_ID_CHARSET, "abcdefghijklmnopqrstuvwxyz0123456789_-");
}
