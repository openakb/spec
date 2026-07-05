//! Defensive JSON shape helpers for semantic validation.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{Code, LOCAL_ID_MAX_LENGTH};

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

#[derive(Debug, Clone, Default)]
pub(crate) struct EntityIndex {
    source_ids: BTreeSet<String>,
    section_ids: BTreeSet<String>,
}

impl EntityIndex {
    pub(crate) fn new(source_ids: BTreeSet<String>, section_ids: BTreeSet<String>) -> Self {
        Self {
            source_ids,
            section_ids,
        }
    }

    pub(crate) fn contains_kind(&self, kind: EntityKind, id: &str) -> bool {
        match kind {
            EntityKind::Source => self.source_ids.contains(id),
            EntityKind::Section => self.section_ids.contains(id),
        }
    }

    fn contains_other_kind(&self, kind: EntityKind, id: &str) -> bool {
        match kind {
            EntityKind::Source => self.section_ids.contains(id),
            EntityKind::Section => self.source_ids.contains(id),
        }
    }
}

pub(crate) fn is_local_id(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.len() <= LOCAL_ID_MAX_LENGTH
        && candidate.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

pub(crate) fn indexed_objects(value: Option<&Value>) -> impl Iterator<Item = (usize, &Object)> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter().enumerate())
        .filter_map(|(index, item)| item.as_object().map(|object| (index, object)))
}

pub(crate) fn local_id_value(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|id| is_local_id(id))
}

pub(crate) fn reference_code(
    value: Option<&Value>,
    expected: EntityKind,
    index: &EntityIndex,
) -> Option<Code> {
    let id = local_id_value(value)?;
    if index.contains_kind(expected, id) {
        None
    } else if index.contains_other_kind(expected, id) {
        Some(Code::Akb010)
    } else {
        Some(Code::Akb007)
    }
}
