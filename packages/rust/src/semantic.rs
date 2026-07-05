//! Cross-document semantic validation for descriptor-local references.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    Advisory, Code, Finding, PARENT_DEPTH_MAX, Segment, json_pointer,
    shape::{EntityIndex, EntityKind, Object, indexed_objects, local_id_value, reference_code},
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
    let mut first_seen: BTreeMap<&str, String> = BTreeMap::new();
    let entries = id_entries(graph);

    for entry in entries {
        let path = id_path(entry.kind, entry.index);
        if let Some(first_path) = first_seen.get(entry.value) {
            findings.push(finding(
                Code::Akb001,
                path,
                format!(
                    "duplicate id \"{}\" first declared at {}",
                    entry.value, first_path
                ),
            ));
        } else {
            first_seen.insert(entry.value, json_pointer(path));
        }
    }

    findings
}

fn empty_sections(graph: &Graph<'_>) -> Vec<Finding> {
    let child_parent_ids: BTreeSet<_> = graph
        .sections
        .iter()
        .filter_map(|(_, section)| local_id_value(section.get("parent_id")))
        .collect();

    graph
        .sections
        .iter()
        .filter_map(|(index, section)| {
            let id = local_id_value(section.get("id"))?;
            if section.contains_key("content_uri") || child_parent_ids.contains(id) {
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
            "reference \"{token}\" resolves to the wrong kind; expected a {}",
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
}

fn append_link_findings(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    section: &Object,
    section_index: usize,
) {
    for (link_index, link) in indexed_objects(section.get("links")) {
        if link.contains_key("akb_uri") {
            continue;
        }
        append_ref(
            findings,
            graph,
            EntityKind::Section,
            link.get("section_id"),
            [
                Segment::Key("sections"),
                Segment::Index(section_index),
                Segment::Key("links"),
                Segment::Index(link_index),
                Segment::Key("section_id"),
            ],
        );
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

fn parent_cycle_findings(
    graph: &Graph<'_>,
    parent_by_id: &BTreeMap<String, String>,
) -> Vec<Finding> {
    let index_by_id = index_by_id(&graph.sections);

    cycles(parent_by_id)
        .into_iter()
        .filter_map(|cycle| {
            let index = index_by_id.get(cycle.first()?)?;
            Some(finding(
                Code::Akb004,
                [
                    Segment::Key("sections"),
                    Segment::Index(*index),
                    Segment::Key("parent_id"),
                ],
                format!("parent_id cycle: {}", render_cycle(&cycle)),
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
        let Some(source_id) = local_id_value(source.get("id")) else {
            continue;
        };
        let Some(discovered_via_id) = local_id_value(source.get("discovered_via_id")) else {
            continue;
        };
        next_by_id.insert(source_id.to_owned(), discovered_via_id.to_owned());
    }

    let index_by_id = index_by_id(&graph.sources);
    cycles(&next_by_id)
        .into_iter()
        .filter_map(|cycle| {
            let index = index_by_id.get(cycle.first()?)?;
            Some(Advisory {
                path: json_pointer([
                    Segment::Key("sources"),
                    Segment::Index(*index),
                    Segment::Key("discovered_via_id"),
                ]),
                message: format!("discovered_via_id cycle: {}", render_cycle(&cycle)),
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
            let id = local_id_value(section.get("id"))?;
            let parent_id = local_id_value(section.get("parent_id"))?;
            if graph
                .sections
                .iter()
                .any(|(_, section)| local_id_value(section.get("id")) == Some(parent_id))
            {
                Some((id.to_owned(), parent_id.to_owned()))
            } else {
                None
            }
        })
        .collect()
}

fn index_by_id(items: &[(usize, &Object)]) -> BTreeMap<String, usize> {
    items
        .iter()
        .filter_map(|(index, item)| Some((local_id_value(item.get("id"))?.to_owned(), *index)))
        .collect()
}

fn id_entries<'descriptor>(graph: &Graph<'descriptor>) -> Vec<IdEntry<'descriptor>> {
    let source_entries = graph.sources.iter().filter_map(|(index, item)| {
        Some(IdEntry {
            kind: EntityKind::Source,
            index: *index,
            value: local_id_value(item.get("id"))?,
        })
    });
    let section_entries = graph.sections.iter().filter_map(|(index, item)| {
        Some(IdEntry {
            kind: EntityKind::Section,
            index: *index,
            value: local_id_value(item.get("id"))?,
        })
    });

    source_entries.chain(section_entries).collect()
}

fn ids(items: &[(usize, &Object)]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|(_, item)| Some(local_id_value(item.get("id"))?.to_owned()))
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
