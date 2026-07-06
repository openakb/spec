//! Diagnostic model.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::Code;

/// One segment in a JSON Pointer path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment<'a> {
    /// Object member key.
    Key(&'a str),
    /// Array index.
    Index(usize),
}

/// Builds an RFC 6901 JSON Pointer from path segments.
#[must_use]
pub fn json_pointer<'a>(segments: impl IntoIterator<Item = Segment<'a>>) -> String {
    let mut pointer = String::new();

    for segment in segments {
        pointer.push('/');
        match segment {
            Segment::Key(key) => push_escaped_key(&mut pointer, key),
            Segment::Index(index) => pointer.push_str(&index.to_string()),
        }
    }

    pointer
}

fn push_escaped_key(pointer: &mut String, key: &str) {
    for character in key.chars() {
        match character {
            '~' => pointer.push_str("~0"),
            '/' => pointer.push_str("~1"),
            _ => pointer.push(character),
        }
    }
}

/// A validation finding with a stable diagnostic code.
///
/// The derived ordering is intentional: findings sort by `(code, path, message)`
/// for deterministic diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Finding {
    /// Stable diagnostic code.
    pub code: Code,
    /// JSON Pointer path to the affected value, or an empty string for the root.
    pub path: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl Finding {
    /// Returns the stable name of this finding's diagnostic code.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.code.name()
    }
}

/// A non-fatal validation advisory.
///
/// The derived ordering is intentional: advisories sort by `(path, message)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Advisory {
    /// JSON Pointer path to the affected value, or an empty string for the root.
    pub path: String,
    /// Human-readable advisory message.
    pub message: String,
}

/// Complete validation outcome.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationResult {
    /// Fatal validation findings.
    pub findings: Vec<Finding>,
    /// Non-fatal validation warnings.
    pub warnings: Vec<Advisory>,
}

impl ValidationResult {
    /// Returns true when validation produced no fatal findings.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns the deduplicated set of fatal diagnostic codes in this result.
    #[must_use]
    pub fn codes(&self) -> BTreeSet<Code> {
        self.findings.iter().map(|finding| finding.code).collect()
    }
}
