//! Strict-profile lint for unknown core members.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

use serde_json::Value;

use crate::{
    Code, Finding, Segment, json_pointer,
    schema::descriptor_schema,
    shape::{Object, indexed_objects},
};

static KNOWN_KEYS: LazyLock<BTreeMap<&'static str, BTreeSet<String>>> = LazyLock::new(known_keys);

#[must_use]
pub(crate) fn strict_findings(descriptor: &Value) -> Vec<Finding> {
    let Some(object) = descriptor.as_object() else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    lint(object, "top", Vec::new(), &mut findings);
    lint_sources(object, &mut findings);
    lint_sections(object, &mut findings);
    findings.sort();
    findings
}

fn lint_sources(descriptor: &Object, findings: &mut Vec<Finding>) {
    for (index, source) in indexed_objects(descriptor.get("sources")) {
        lint(
            source,
            "source",
            vec![Segment::Key("sources"), Segment::Index(index)],
            findings,
        );
    }
}

fn lint_sections(descriptor: &Object, findings: &mut Vec<Finding>) {
    for (index, section) in indexed_objects(descriptor.get("sections")) {
        let parts = vec![Segment::Key("sections"), Segment::Index(index)];
        lint(section, "section", parts.clone(), findings);
        lint_claims(section, &parts, findings);
        lint_links(section, &parts, findings);
    }
}

fn lint_claims(section: &Object, parts: &[Segment<'_>], findings: &mut Vec<Finding>) {
    for (index, claim) in indexed_objects(section.get("provenance")) {
        let claim_parts = appended(parts, [Segment::Key("provenance"), Segment::Index(index)]);
        lint(claim, "claim", claim_parts.clone(), findings);
        if let Some(locator) = claim.get("locator").and_then(Value::as_object) {
            lint(
                locator,
                "locator",
                appended(&claim_parts, [Segment::Key("locator")]),
                findings,
            );
        }
    }
}

fn lint_links(section: &Object, parts: &[Segment<'_>], findings: &mut Vec<Finding>) {
    for (index, link) in indexed_objects(section.get("links")) {
        lint(
            link,
            "link",
            appended(parts, [Segment::Key("links"), Segment::Index(index)]),
            findings,
        );
    }
}

fn lint(object: &Object, kind: &'static str, parts: Vec<Segment<'_>>, findings: &mut Vec<Finding>) {
    let Some(known) = KNOWN_KEYS.get(kind) else {
        return;
    };

    for key in object.keys() {
        if known.contains(key) {
            continue;
        }
        findings.push(Finding {
            code: Code::Akb006,
            path: json_pointer(appended(&parts, [Segment::Key(key)])),
            message: format!(
                "unknown core member {key:?}: forward-minor data is tolerated by the lenient default; vendor data belongs under 'x'"
            ),
        });
    }
}

fn known_keys() -> BTreeMap<&'static str, BTreeSet<String>> {
    let schema = descriptor_schema();
    BTreeMap::from([
        ("top", property_names(schema.get("properties"))),
        ("source", def_property_names(schema, "source")),
        ("section", def_property_names(schema, "section")),
        ("claim", def_property_names(schema, "claim")),
        ("locator", locator_property_names(schema)),
        ("link", def_property_names(schema, "link")),
    ])
}

fn def_property_names(schema: &Value, def: &str) -> BTreeSet<String> {
    property_names(
        schema
            .get("$defs")
            .and_then(Value::as_object)
            .and_then(|defs| defs.get(def))
            .and_then(|item| item.get("properties")),
    )
}

fn locator_property_names(schema: &Value) -> BTreeSet<String> {
    property_names(
        schema
            .get("$defs")
            .and_then(Value::as_object)
            .and_then(|defs| defs.get("claim"))
            .and_then(|claim| claim.get("properties"))
            .and_then(|properties| properties.get("locator"))
            .and_then(|locator| locator.get("properties")),
    )
}

fn property_names(properties: Option<&Value>) -> BTreeSet<String> {
    properties
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn appended<'path>(
    prefix: &[Segment<'path>],
    suffix: impl IntoIterator<Item = Segment<'path>>,
) -> Vec<Segment<'path>> {
    prefix.iter().copied().chain(suffix).collect()
}
