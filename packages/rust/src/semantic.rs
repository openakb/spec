//! Cross-document semantic validation for descriptor-local references.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    Advisory, Code, Finding, PARENT_DEPTH_MAX, Segment, json_pointer,
    shape::{
        EntityIndex, EntityKind, Object, casefolded_duplicate_indices, id_kind, indexed_objects,
        is_typed_id, normalize_id, reference_code, typed_id_value,
    },
};

const DEPTH_LIMIT: usize = PARENT_DEPTH_MAX + 1;

#[derive(Debug, Clone)]
struct Graph<'descriptor> {
    sources: Vec<(usize, &'descriptor Object)>,
    sections: Vec<(usize, &'descriptor Object)>,
    entities: EntityIndex,
}

impl<'descriptor> Graph<'descriptor> {
    fn from_descriptor(descriptor: &'descriptor Value) -> Self {
        let Some(object) = descriptor.as_object() else {
            return Self {
                sources: Vec::new(),
                sections: Vec::new(),
                entities: EntityIndex::default(),
            };
        };

        let sources: Vec<_> = indexed_objects(object.get("sources")).collect();
        let sections: Vec<_> = indexed_objects(object.get("sections")).collect();
        let source_ids = ids(&sources);
        let section_ids = ids(&sections);

        Self {
            sources,
            sections,
            entities: EntityIndex::new(source_ids, section_ids),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct IdEntry<'descriptor> {
    kind: EntityKind,
    index: usize,
    value: &'descriptor str,
}

pub(crate) fn semantic_findings(descriptor: &Value) -> Vec<Finding> {
    let graph = Graph::from_descriptor(descriptor);
    let parent_by_id = parent_by_id(&graph);
    let mut findings = Vec::new();

    findings.extend(duplicate_ids(&graph));
    findings.extend(empty_sections(&graph));
    findings.extend(reference_findings(&graph));
    findings.extend(parent_cycle_findings(&graph, &parent_by_id));
    findings.extend(depth_findings(&graph, &parent_by_id));
    findings.sort();
    findings
}

pub(crate) fn semantic_warnings(descriptor: &Value) -> Vec<Advisory> {
    let graph = Graph::from_descriptor(descriptor);
    let mut warnings = source_cycle_warnings(&graph);
    warnings.sort();
    warnings.dedup();
    warnings
}

fn duplicate_ids(graph: &Graph<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut first_seen: BTreeMap<String, String> = BTreeMap::new();
    let entries = id_entries(graph);

    for entry in entries {
        let path = id_path(entry.kind, entry.index);
        let key = normalize_id(entry.value);
        if let Some(first_path) = first_seen.get(&key) {
            findings.push(finding(
                Code::Akb001,
                path,
                format!(
                    "duplicate id \"{}\" first declared at {}",
                    entry.value, first_path
                ),
            ));
        } else {
            first_seen.insert(key, json_pointer(path));
        }
    }

    findings
}

fn empty_sections(graph: &Graph<'_>) -> Vec<Finding> {
    let child_parent_ids: BTreeSet<_> = graph
        .sections
        .iter()
        .filter_map(|(_, section)| typed_id_value(section.get("parent_id")).map(normalize_id))
        .collect();

    graph
        .sections
        .iter()
        .filter_map(|(index, section)| {
            let id = typed_id_value(section.get("id"))?;
            if section.contains_key("content_uri") || child_parent_ids.contains(&normalize_id(id)) {
                return None;
            }
            Some(finding(
                Code::Akb002,
                [Segment::Key("sections"), Segment::Index(*index)],
                format!("section \"{id}\" has neither content_uri nor a child section"),
            ))
        })
        .collect()
}

fn reference_findings(graph: &Graph<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (index, source) in &graph.sources {
        append_ref(
            &mut findings,
            graph,
            EntityKind::Source,
            source.get("discovered_via_id"),
            [
                Segment::Key("sources"),
                Segment::Index(*index),
                Segment::Key("discovered_via_id"),
            ],
        );
    }

    for (section_index, section) in &graph.sections {
        append_ref(
            &mut findings,
            graph,
            EntityKind::Section,
            section.get("parent_id"),
            [
                Segment::Key("sections"),
                Segment::Index(*section_index),
                Segment::Key("parent_id"),
            ],
        );
        append_source_ids(
            &mut findings,
            graph,
            section.get("source_ids"),
            vec![
                Segment::Key("sections"),
                Segment::Index(*section_index),
                Segment::Key("source_ids"),
            ],
        );
        append_link_findings(&mut findings, graph, section, *section_index);
        append_claim_findings(&mut findings, graph, section, *section_index);
    }

    findings
}

fn append_ref<'path>(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    expected: EntityKind,
    value: Option<&Value>,
    path: impl IntoIterator<Item = Segment<'path>>,
) {
    if let Some(code) = reference_code(value, expected, &graph.entities) {
        findings.push(finding(
            code,
            path,
            reference_message(code, value, expected),
        ));
    }
}

fn reference_message(code: Code, value: Option<&Value>, expected: EntityKind) -> String {
    let token = value.and_then(Value::as_str).unwrap_or("<non-string>");
    if code == Code::Akb010 {
        format!(
            "reference \"{token}\" is a {} id where a {} is required",
            id_kind(token).map_or("", EntityKind::noun),
            expected.noun()
        )
    } else {
        format!(
            "unresolved reference \"{token}\"; no declared {} has this id",
            expected.noun()
        )
    }
}

fn append_source_ids(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    value: Option<&Value>,
    path_prefix: Vec<Segment<'_>>,
) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };

    for (index, item) in items.iter().enumerate() {
        let mut path = path_prefix.clone();
        path.push(Segment::Index(index));
        append_ref(findings, graph, EntityKind::Source, Some(item), path);
    }

    for index in casefolded_duplicate_indices(items) {
        let mut path = path_prefix.clone();
        path.push(Segment::Index(index));
        findings.push(finding(
            Code::Akb011,
            path,
            format!(
                "duplicate source id \"{}\" (case-insensitive) within this array",
                items[index].as_str().unwrap_or_default()
            ),
        ));
    }
}

fn append_link_findings(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    section: &Object,
    section_index: usize,
) {
    for (link_index, link) in indexed_objects(section.get("links")) {
        let path = [
            Segment::Key("sections"),
            Segment::Index(section_index),
            Segment::Key("links"),
            Segment::Index(link_index),
            Segment::Key("section_id"),
        ];
        if link.contains_key("akb_uri") {
            append_cross_akb_section_kind(findings, link.get("section_id"), path);
            continue;
        }
        append_ref(
            findings,
            graph,
            EntityKind::Section,
            link.get("section_id"),
            path,
        );
    }
}

/// Enforces the section-kind prefix on a cross-AKB link's `section_id`.
///
/// Existence is not checked here: cross-AKB resolution is best-effort, so no
/// `Akb007` fires for a cross-AKB target. Only the prefix -- checkable offline
/// -- is enforced; a non-typed token is left to the schema layer's `Akb011`.
fn append_cross_akb_section_kind<'path>(
    findings: &mut Vec<Finding>,
    value: Option<&Value>,
    path: impl IntoIterator<Item = Segment<'path>>,
) {
    let Some(token) = value.and_then(Value::as_str) else {
        return;
    };
    if id_kind(token) == Some(EntityKind::Source) {
        findings.push(finding(
            Code::Akb010,
            path,
            reference_message(Code::Akb010, value, EntityKind::Section),
        ));
    }
}

fn append_claim_findings(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    section: &Object,
    section_index: usize,
) {
    for (claim_index, claim) in indexed_objects(section.get("provenance")) {
        append_source_ids(
            findings,
            graph,
            claim.get("source_ids"),
            vec![
                Segment::Key("sections"),
                Segment::Index(section_index),
                Segment::Key("provenance"),
                Segment::Index(claim_index),
                Segment::Key("source_ids"),
            ],
        );
    }
}

fn original_spellings<'a>(
    items: impl Iterator<Item = &'a (usize, &'a Object)>,
) -> BTreeMap<String, String> {
    // Cycle messages echo the author's casing, like the duplicate-id message;
    // graph keys stay normalized.
    let mut originals: BTreeMap<String, String> = BTreeMap::new();
    for (_, item) in items {
        if let Some(id) = item
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| is_typed_id(id))
        {
            originals
                .entry(normalize_id(id))
                .or_insert_with(|| id.to_string());
        }
    }
    originals
}

fn parent_cycle_findings(
    graph: &Graph<'_>,
    parent_by_id: &BTreeMap<String, String>,
) -> Vec<Finding> {
    let index_by_id = index_by_id(&graph.sections);
    let originals = original_spellings(graph.sections.iter());

    cycles(parent_by_id)
        .into_iter()
        .filter_map(|cycle| {
            let index = index_by_id.get(cycle.first()?)?;
            let spelled: Vec<String> = cycle
                .iter()
                .map(|id| originals.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect();
            Some(finding(
                Code::Akb004,
                [
                    Segment::Key("sections"),
                    Segment::Index(*index),
                    Segment::Key("parent_id"),
                ],
                format!("parent_id cycle: {}", render_cycle(&spelled)),
            ))
        })
        .collect()
}

fn depth_findings(graph: &Graph<'_>, parent_by_id: &BTreeMap<String, String>) -> Vec<Finding> {
    index_by_id(&graph.sections)
        .into_iter()
        .filter_map(|(section_id, index)| {
            let depth = section_depth(&section_id, parent_by_id);
            if depth <= PARENT_DEPTH_MAX {
                return None;
            }
            Some(finding(
                Code::Akb005,
                [
                    Segment::Key("sections"),
                    Segment::Index(index),
                    Segment::Key("parent_id"),
                ],
                format!("parent_id chain depth {depth} exceeds the maximum of {PARENT_DEPTH_MAX}"),
            ))
        })
        .collect()
}

fn section_depth(section_id: &str, parent_by_id: &BTreeMap<String, String>) -> usize {
    let mut depth = 1;
    let mut seen = BTreeSet::from([section_id.to_owned()]);
    let mut current = section_id;

    while let Some(parent_id) = parent_by_id.get(current) {
        if seen.contains(parent_id) {
            return 1;
        }
        seen.insert(parent_id.clone());
        depth += 1;
        current = parent_id;
        if depth > DEPTH_LIMIT {
            return depth;
        }
    }

    depth
}

fn source_cycle_warnings(graph: &Graph<'_>) -> Vec<Advisory> {
    let mut next_by_id = BTreeMap::new();
    for (_, source) in &graph.sources {
        let Some(source_id) = typed_id_value(source.get("id")) else {
            continue;
        };
        let Some(discovered_via_id) = typed_id_value(source.get("discovered_via_id")) else {
            continue;
        };
        next_by_id.insert(normalize_id(source_id), normalize_id(discovered_via_id));
    }

    let index_by_id = index_by_id(&graph.sources);
    let originals = original_spellings(graph.sources.iter());
    cycles(&next_by_id)
        .into_iter()
        .filter_map(|cycle| {
            let index = index_by_id.get(cycle.first()?)?;
            let spelled: Vec<String> = cycle
                .iter()
                .map(|id| originals.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect();
            Some(Advisory {
                path: json_pointer([
                    Segment::Key("sources"),
                    Segment::Index(*index),
                    Segment::Key("discovered_via_id"),
                ]),
                message: format!("discovered_via_id cycle: {}", render_cycle(&spelled)),
            })
        })
        .collect()
}

fn cycles(next_by_id: &BTreeMap<String, String>) -> BTreeSet<Vec<String>> {
    let mut cycles = BTreeSet::new();
    let mut done = BTreeSet::new();

    for start in next_by_id.keys() {
        if done.contains(start) {
            continue;
        }

        let mut position = BTreeMap::new();
        let mut path = Vec::new();
        let mut current = start;

        while next_by_id.contains_key(current) && !done.contains(current) {
            if let Some(cycle_start) = position.get(current) {
                cycles.insert(canonical_cycle(&path[*cycle_start..]));
                break;
            }

            position.insert(current.clone(), path.len());
            path.push(current.clone());

            let Some(next) = next_by_id.get(current) else {
                break;
            };
            current = next;
        }

        done.extend(path);
    }

    cycles
}

fn canonical_cycle(cycle: &[String]) -> Vec<String> {
    let Some((pivot, _)) = cycle
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| left.cmp(right))
    else {
        return Vec::new();
    };

    cycle[pivot..]
        .iter()
        .chain(cycle[..pivot].iter())
        .cloned()
        .collect()
}

fn render_cycle(cycle: &[String]) -> String {
    cycle
        .iter()
        .chain(cycle.first())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn parent_by_id(graph: &Graph<'_>) -> BTreeMap<String, String> {
    graph
        .sections
        .iter()
        .filter_map(|(_, section)| {
            let id = typed_id_value(section.get("id"))?;
            let parent_id = typed_id_value(section.get("parent_id"))?;
            if graph.entities.contains_kind(EntityKind::Section, parent_id) {
                Some((normalize_id(id), normalize_id(parent_id)))
            } else {
                None
            }
        })
        .collect()
}

fn index_by_id(items: &[(usize, &Object)]) -> BTreeMap<String, usize> {
    items
        .iter()
        .filter_map(|(index, item)| Some((normalize_id(typed_id_value(item.get("id"))?), *index)))
        .collect()
}

fn id_entries<'descriptor>(graph: &Graph<'descriptor>) -> Vec<IdEntry<'descriptor>> {
    let source_entries = graph.sources.iter().filter_map(|(index, item)| {
        Some(IdEntry {
            kind: EntityKind::Source,
            index: *index,
            value: typed_id_value(item.get("id"))?,
        })
    });
    let section_entries = graph.sections.iter().filter_map(|(index, item)| {
        Some(IdEntry {
            kind: EntityKind::Section,
            index: *index,
            value: typed_id_value(item.get("id"))?,
        })
    });

    source_entries.chain(section_entries).collect()
}

fn ids(items: &[(usize, &Object)]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|(_, item)| Some(normalize_id(typed_id_value(item.get("id"))?)))
        .collect()
}

fn id_path(kind: EntityKind, index: usize) -> [Segment<'static>; 3] {
    [
        Segment::Key(match kind {
            EntityKind::Source => "sources",
            EntityKind::Section => "sections",
        }),
        Segment::Index(index),
        Segment::Key("id"),
    ]
}

fn finding<'path>(
    code: Code,
    path: impl IntoIterator<Item = Segment<'path>>,
    message: String,
) -> Finding {
    Finding {
        code,
        path: json_pointer(path),
        message,
    }
}
