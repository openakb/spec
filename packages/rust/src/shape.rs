//! Defensive JSON shape helpers for semantic validation.

use std::{collections::BTreeSet, sync::LazyLock};

use regex::Regex;
use serde_json::Value;

use crate::Code;

pub(crate) type Object = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityKind {
    Source,
    Section,
}

impl EntityKind {
    pub(crate) const fn noun(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Section => "section",
        }
    }
}

// Explicit ASCII character classes instead of an `(?i)` flag: the regex crate's
// Unicode-aware simple case folding would otherwise admit foldable non-ASCII
// characters (e.g. U+212A KELVIN SIGN, U+017F LONG S) that the schema's
// ASCII-only `[0-9A-Za-z]{6}` pattern rejects as AKB011.
static SECTION_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    // PANIC: the typed section-id regex is fixed ASCII syntax.
    #[expect(
        clippy::expect_used,
        reason = "compile-time section id regex must be valid"
    )]
    Regex::new(r"^[Ss][Ee][Cc]-[0-9A-Za-z]{6}$").expect("section id regex")
});
static SOURCE_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    // PANIC: the typed source-id regex is fixed ASCII syntax.
    #[expect(
        clippy::expect_used,
        reason = "compile-time source id regex must be valid"
    )]
    Regex::new(r"^[Ss][Rr][Cc]-[0-9A-Za-z]{6}$").expect("source id regex")
});

pub(crate) fn is_typed_id(candidate: &str) -> bool {
    SECTION_ID_RE.is_match(candidate) || SOURCE_ID_RE.is_match(candidate)
}

pub(crate) fn id_kind(candidate: &str) -> Option<EntityKind> {
    if SECTION_ID_RE.is_match(candidate) {
        Some(EntityKind::Section)
    } else if SOURCE_ID_RE.is_match(candidate) {
        Some(EntityKind::Source)
    } else {
        None
    }
}

/// Casefolded comparison key for ids (ASCII lower); typed ids are ASCII.
pub(crate) fn normalize_id(id: &str) -> String {
    id.to_ascii_lowercase()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct EntityIndex {
    source_ids: BTreeSet<String>,
    section_ids: BTreeSet<String>,
}

impl EntityIndex {
    /// Creates an index from already-normalized (lowercased) id sets.
    pub(crate) fn new(source_ids: BTreeSet<String>, section_ids: BTreeSet<String>) -> Self {
        Self {
            source_ids,
            section_ids,
        }
    }

    pub(crate) fn contains_kind(&self, kind: EntityKind, id: &str) -> bool {
        let key = normalize_id(id);
        match kind {
            EntityKind::Source => self.source_ids.contains(&key),
            EntityKind::Section => self.section_ids.contains(&key),
        }
    }
}

pub(crate) fn indexed_objects(value: Option<&Value>) -> impl Iterator<Item = (usize, &Object)> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().enumerate())
        .filter_map(|(index, item)| item.as_object().map(|object| (index, object)))
}

pub(crate) fn typed_id_value(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|id| is_typed_id(id))
}

pub(crate) fn reference_code(
    value: Option<&Value>,
    expected: EntityKind,
    index: &EntityIndex,
) -> Option<Code> {
    let id = value.and_then(Value::as_str)?;
    reference_code_id(id, expected, index)
}

pub(crate) fn reference_code_id(
    id: &str,
    expected: EntityKind,
    index: &EntityIndex,
) -> Option<Code> {
    match id_kind(id) {
        None => None, // schema already reported AKB011
        Some(kind) if kind != expected => Some(Code::Akb010),
        Some(_) => {
            if index.contains_kind(expected, id) {
                None
            } else {
                Some(Code::Akb007)
            }
        }
    }
}
