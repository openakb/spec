//! Opt-in content checks that fetch descriptor-related resources.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Component, Path, PathBuf},
    string::FromUtf8Error,
};

use async_trait::async_trait;
use base64::{
    Engine as _, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig, general_purpose::STANDARD},
};
use fluent_uri::{Uri, UriRef};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::fs;

use crate::{
    Advisory, Finding, Segment, json_pointer,
    schema::provenance_schema_findings,
    shape::{EntityIndex, EntityKind, Object, indexed_objects, local_id_value, reference_code_id},
};

const SHA256_ALGORITHM: &str = "sha256";
const SHA256_LENGTH: usize = 32;
const STRICT_STANDARD_BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::RequireCanonical)
        .with_decode_allow_trailing_bits(false),
);

/// Fetches descriptor-related content bytes.
///
/// `check_content` calls [`Resolver::fetch`] with effective references:
/// descriptor URI references are resolved against `base_uri` when present and
/// fragments are stripped before fetch.
#[async_trait]
pub trait Resolver: Send + Sync {
    /// Returns bytes for `uri`, or an [`Unfetchable`] error when the resource
    /// cannot be fetched.
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable>;

    /// Returns true when `check_content` should pre-screen local path references.
    ///
    /// Resolvers that serve scheme-less local filesystem paths should opt into
    /// this. The screening rejects raw descriptor references and `base_uri`
    /// values containing queries, semicolon path parameters, backslashes, or
    /// `..` path traversal before any fetch happens. Custom URI resolvers should
    /// keep the default `false` and enforce their own safety policy in `fetch`.
    fn screens_local_paths(&self) -> bool {
        false
    }
}

/// A resolver-level fetch failure.
#[derive(Debug, Error)]
#[error("unfetchable: {reason}")]
pub struct Unfetchable {
    /// Human-readable reason the resource could not be fetched.
    pub reason: String,
}

impl Unfetchable {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    fn outside_local_base(uri: &str) -> Self {
        Self::new(format!("outside local base: {uri}"))
    }
}

/// Resolves scheme-less relative paths under a local base directory.
///
/// Fragments are ignored. Schemes, authorities, queries, semicolon path
/// parameters, backslashes, absolute paths, `..` traversal, and symlink escapes
/// are rejected as outside the base.
pub struct LocalFileResolver {
    base_dir: PathBuf,
}

impl LocalFileResolver {
    /// Creates a resolver rooted at `base_dir`.
    #[must_use]
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    async fn local_path(&self, uri: &str) -> Result<PathBuf, Unfetchable> {
        let raw_reference = strip_fragment(uri);
        let raw_path = raw_reference
            .split_once('?')
            .map_or(raw_reference, |(path, _)| path);
        if rejects_local_reference(raw_reference, raw_path) {
            return Err(Unfetchable::outside_local_base(uri));
        }

        let base = fs::canonicalize(&self.base_dir)
            .await
            .map_err(|error| Unfetchable::new(error.to_string()))?;
        let path = fs::canonicalize(base.join(raw_path))
            .await
            .map_err(|error| Unfetchable::new(error.to_string()))?;
        if path != base && !path.starts_with(&base) {
            return Err(Unfetchable::outside_local_base(uri));
        }
        Ok(path)
    }
}

#[async_trait]
impl Resolver for LocalFileResolver {
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable> {
        let path = self.local_path(uri).await?;
        fs::read(path)
            .await
            .map_err(|error| Unfetchable::new(error.to_string()))
    }

    fn screens_local_paths(&self) -> bool {
        true
    }
}

/// Three-state outcome for a content check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The check proved the content did not match its descriptor claim.
    Failed,
    /// The check could not prove or disprove the descriptor claim.
    Unverifiable,
    /// The check proved the descriptor claim.
    Verified,
}

/// Content check category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckKind {
    /// Source capture bytes and `content_hash`.
    Capture,
    /// Citation markers in fetched Markdown content.
    Citations,
    /// Section content bytes and `content_hash`.
    ContentHash,
    /// Descriptor guide bytes and `guide_hash`.
    GuideHash,
    /// Quoted claim text in source captures.
    Quote,
    /// Provenance sidecar bytes and binding.
    Sidecar,
}

/// One content check and its three-state outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContentCheck {
    /// Kind of content check performed.
    pub kind: CheckKind,
    /// JSON Pointer path to the descriptor field or object the check covers.
    pub path: String,
    /// Three-state check result.
    pub outcome: Outcome,
    /// Human-readable explanation of the result.
    pub detail: String,
    /// Fatal validation findings raised by the check.
    pub findings: Vec<Finding>,
    /// Non-fatal advisories raised by the check.
    pub warnings: Vec<Advisory>,
}

/// Aggregate result for opt-in content checks.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ContentReport {
    /// Individual content checks in execution order.
    pub checks: Vec<ContentCheck>,
}

impl ContentReport {
    /// Returns every fatal finding raised by the report's checks.
    pub fn findings(&self) -> impl Iterator<Item = &Finding> {
        self.checks.iter().flat_map(|check| check.findings.iter())
    }

    /// Returns every non-fatal advisory raised by the report's checks.
    pub fn warnings(&self) -> impl Iterator<Item = &Advisory> {
        self.checks.iter().flat_map(|check| check.warnings.iter())
    }

    /// Returns checks whose outcome equals `outcome`.
    pub fn with_outcome(&self, outcome: Outcome) -> impl Iterator<Item = &ContentCheck> {
        self.checks
            .iter()
            .filter(move |check| check.outcome == outcome)
    }

    /// Returns true when no check failed and no fatal findings were raised.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.with_outcome(Outcome::Failed).next().is_none() && self.findings().next().is_none()
    }
}

/// Runs opt-in content checks against fetched descriptor resources.
pub async fn check_content(descriptor: &Value, resolver: &dyn Resolver) -> ContentReport {
    let Some(object) = descriptor.as_object() else {
        return ContentReport::default();
    };

    let graph = Graph::from_descriptor(object);
    let mut checks = Vec::new();
    checks.extend(guide_checks(object, resolver).await);
    checks.extend(section_checks(&graph, object, resolver).await);
    let sidecars = sidecar_checks(&graph, object, resolver).await;
    checks.extend(sidecars.checks);
    let mut quote_claims = inline_quote_claims(&graph);
    quote_claims.extend(sidecars.claims);
    let captures = capture_checks(&graph, object, resolver).await;
    let quote_checks = quote_checks(&graph, &captures, &quote_claims);
    checks.extend(captures.checks);
    checks.extend(quote_checks);
    ContentReport { checks }
}

#[derive(Debug, Clone)]
struct Graph<'descriptor> {
    sources: Vec<(usize, &'descriptor Object)>,
    sections: Vec<(usize, &'descriptor Object)>,
    entities: EntityIndex,
}

impl<'descriptor> Graph<'descriptor> {
    fn from_descriptor(descriptor: &'descriptor Object) -> Self {
        let sources: Vec<_> = indexed_objects(descriptor.get("sources")).collect();
        let sections: Vec<_> = indexed_objects(descriptor.get("sections")).collect();
        let source_ids = ids(&sources);
        let section_ids = ids(&sections);
        Self {
            sources,
            sections,
            entities: EntityIndex::new(source_ids, section_ids),
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedContent<'descriptor> {
    index: usize,
    payload: Vec<u8>,
    _section: &'descriptor Object,
}

#[derive(Debug, Clone)]
struct UnfetchedContent {
    index: usize,
    error: String,
}

#[derive(Debug, Clone)]
enum SectionFetch<'descriptor> {
    Resolved(ResolvedContent<'descriptor>),
    Unfetched(UnfetchedContent),
}

#[derive(Debug, Clone)]
struct ResolvedCapture {
    payload: Vec<u8>,
}

#[derive(Debug, Clone)]
enum CaptureFetch {
    Resolved(ResolvedCapture),
    Unfetched(UnfetchedContent),
}

#[derive(Debug, Clone, Default)]
struct CaptureResult {
    payloads: HashMap<String, Vec<u8>>,
    checks: Vec<ContentCheck>,
    hash_failed: HashSet<String>,
}

#[derive(Debug, Clone)]
struct QuoteClaim {
    path: Vec<PathSegment>,
    quote: String,
    source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Key(&'static str),
    Index(usize),
}

#[derive(Debug, Clone, Default)]
struct SidecarResult {
    claims: Vec<QuoteClaim>,
    checks: Vec<ContentCheck>,
}

async fn guide_checks(object: &Map<String, Value>, resolver: &dyn Resolver) -> Vec<ContentCheck> {
    let Some(guide_hash) = object.get("guide_hash").and_then(Value::as_str) else {
        return Vec::new();
    };
    let path = guide_hash_path();
    let expected = match parse_sri(CheckKind::GuideHash, path.clone(), guide_hash) {
        Ok(expected) => expected,
        Err(check) => return vec![check],
    };
    let Some(guide_uri) = object.get("guide_uri").and_then(Value::as_str) else {
        return vec![check(
            Outcome::Unverifiable,
            CheckKind::GuideHash,
            path,
            "missing guide_uri",
        )];
    };

    let uri = match effective_reference(object, guide_uri, resolver.screens_local_paths()) {
        Ok(uri) => uri,
        Err(error) => {
            return vec![check(
                Outcome::Unverifiable,
                CheckKind::GuideHash,
                path,
                error.to_string(),
            )];
        }
    };
    let payload = match resolver.fetch(&uri).await {
        Ok(payload) => payload,
        Err(error) => {
            return vec![check(
                Outcome::Unverifiable,
                CheckKind::GuideHash,
                path,
                error.to_string(),
            )];
        }
    };
    vec![compare_sri(CheckKind::GuideHash, path, &payload, &expected)]
}

async fn section_checks(
    graph: &Graph<'_>,
    descriptor: &Map<String, Value>,
    resolver: &dyn Resolver,
) -> Vec<ContentCheck> {
    let mut checks = Vec::new();

    for (index, section) in &graph.sections {
        let Some(reference) = section.get("content_uri").and_then(Value::as_str) else {
            continue;
        };
        let content_hash = section.get("content_hash").and_then(Value::as_str);
        let checks_citations = !section.contains_key("content_type")
            || section.get("content_type").is_some_and(is_markdown);
        if content_hash.is_none() && !checks_citations {
            continue;
        }
        let hash_check = content_hash
            .map(|hash| parse_sri(CheckKind::ContentHash, section_hash_path(*index), hash));
        if !checks_citations && matches!(hash_check, Some(Err(_))) {
            if let Some(Err(check)) = hash_check {
                checks.push(check);
            }
            continue;
        }

        let resolved = fetch_section(*index, section, descriptor, reference, resolver).await;
        if let Some(hash_check) = hash_check {
            match hash_check {
                Ok(expected) => checks.push(check_sri(
                    CheckKind::ContentHash,
                    section_hash_path(*index),
                    &resolved,
                    &expected,
                )),
                Err(check) => checks.push(check),
            }
        }
        if checks_citations {
            checks.push(citation_check(graph, &resolved));
        }
    }

    checks
}

async fn fetch_section<'descriptor>(
    index: usize,
    section: &'descriptor Object,
    descriptor: &Map<String, Value>,
    reference: &str,
    resolver: &dyn Resolver,
) -> SectionFetch<'descriptor> {
    let uri = match effective_reference(descriptor, reference, resolver.screens_local_paths()) {
        Ok(uri) => uri,
        Err(error) => {
            return SectionFetch::Unfetched(UnfetchedContent {
                index,
                error: error.to_string(),
            });
        }
    };
    match resolver.fetch(&uri).await {
        Ok(payload) => SectionFetch::Resolved(ResolvedContent {
            index,
            payload,
            _section: section,
        }),
        Err(error) => SectionFetch::Unfetched(UnfetchedContent {
            index,
            error: error.to_string(),
        }),
    }
}

fn citation_check(graph: &Graph<'_>, resolved: &SectionFetch<'_>) -> ContentCheck {
    let resolved = match resolved {
        SectionFetch::Resolved(resolved) => resolved,
        SectionFetch::Unfetched(unfetched) => {
            return check(
                Outcome::Unverifiable,
                CheckKind::Citations,
                section_content_uri_path(unfetched.index),
                unfetched.error.clone(),
            );
        }
    };
    let markdown = match String::from_utf8(resolved.payload.clone()) {
        Ok(markdown) => markdown,
        Err(error) => {
            return check(
                Outcome::Failed,
                CheckKind::Citations,
                section_content_uri_path(resolved.index),
                utf8_error_detail(error),
            );
        }
    };
    let (findings, warnings) = citation_results(graph, resolved.index, &markdown);
    ContentCheck {
        kind: CheckKind::Citations,
        path: section_content_uri_path(resolved.index),
        outcome: if findings.is_empty() {
            Outcome::Verified
        } else {
            Outcome::Failed
        },
        detail: "citation markers checked".to_owned(),
        findings,
        warnings,
    }
}

fn citation_results(
    graph: &Graph<'_>,
    section_index: usize,
    markdown: &str,
) -> (Vec<Finding>, Vec<Advisory>) {
    let mut findings = Vec::new();
    let mut warnings = Vec::new();
    let base_path = vec![
        PathSegment::Key("sections"),
        PathSegment::Index(section_index),
        PathSegment::Key("content_uri"),
    ];
    for (marker_index, citation) in crate::extract_citations(markdown).iter().enumerate() {
        append_duplicate_warnings(&mut warnings, &base_path, &citation.ids);
        for (id_index, source_id) in citation.ids.iter().enumerate() {
            if let Some(code) = reference_code_id(source_id, EntityKind::Source, &graph.entities) {
                let mut path = base_path.clone();
                path.extend([
                    PathSegment::Key("citations"),
                    PathSegment::Index(marker_index),
                    PathSegment::Index(id_index),
                ]);
                findings.push(Finding {
                    code,
                    path: pointer(path),
                    message: format!(
                        "citation source id {source_id:?} does not resolve to a source"
                    ),
                });
            }
        }
    }
    (findings, warnings)
}

fn append_duplicate_warnings(warnings: &mut Vec<Advisory>, path: &[PathSegment], ids: &[String]) {
    let duplicates: Vec<_> = ids
        .iter()
        .filter(|id| ids.iter().filter(|candidate| *candidate == *id).count() > 1)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect();
    if duplicates.is_empty() {
        return;
    }
    warnings.push(Advisory {
        path: pointer(path.iter().cloned()),
        message: format!("duplicate citation id in marker: {}", duplicates.join(", ")),
    });
}

async fn capture_checks(
    graph: &Graph<'_>,
    descriptor: &Map<String, Value>,
    resolver: &dyn Resolver,
) -> CaptureResult {
    let mut result = CaptureResult::default();

    for (index, source) in &graph.sources {
        append_capture_checks(&mut result, descriptor, *index, source, resolver).await;
    }

    result
}

async fn append_capture_checks(
    result: &mut CaptureResult,
    descriptor: &Map<String, Value>,
    index: usize,
    source: &Map<String, Value>,
    resolver: &dyn Resolver,
) {
    let content_hash_path = source_content_hash_path(index);
    let capture_uri = source.get("capture_uri").and_then(Value::as_str);
    let content_hash = source.get("content_hash").and_then(Value::as_str);

    if source.get("type").and_then(Value::as_str) == Some("redacted") {
        if capture_uri.is_some() || content_hash.is_some() {
            result.checks.push(check(
                Outcome::Unverifiable,
                CheckKind::Capture,
                source_path(index),
                "redacted source",
            ));
        }
        return;
    }

    let hash_check =
        content_hash.map(|hash| parse_sri(CheckKind::Capture, content_hash_path.clone(), hash));
    let Some(capture_uri) = capture_uri else {
        if let Some(hash_check) = hash_check {
            match hash_check {
                Ok(_) => result.checks.push(check(
                    Outcome::Unverifiable,
                    CheckKind::Capture,
                    content_hash_path,
                    "missing capture_uri",
                )),
                Err(check) => result.checks.push(check),
            }
        }
        return;
    };

    let resolved = fetch_capture(index, descriptor, capture_uri, resolver).await;
    if let CaptureFetch::Resolved(resolved_capture) = &resolved
        && let Some(source_id) = source.get("id").and_then(Value::as_str)
    {
        result
            .payloads
            .insert(source_id.to_owned(), resolved_capture.payload.clone());
    }

    if let Some(hash_check) = hash_check {
        match hash_check {
            Ok(expected) => match &resolved {
                CaptureFetch::Resolved(resolved_capture) => {
                    let comparison = compare_sri(
                        CheckKind::Capture,
                        content_hash_path,
                        &resolved_capture.payload,
                        &expected,
                    );
                    if comparison.outcome == Outcome::Failed
                        && let Some(source_id) = source.get("id").and_then(Value::as_str)
                    {
                        result.hash_failed.insert(source_id.to_owned());
                    }
                    result.checks.push(comparison);
                }
                CaptureFetch::Unfetched(error) => result.checks.push(check(
                    Outcome::Unverifiable,
                    CheckKind::Capture,
                    content_hash_path,
                    error.error.clone(),
                )),
            },
            Err(warning) => {
                result.checks.push(warning);
                if let CaptureFetch::Unfetched(error) = resolved {
                    result.checks.push(check(
                        Outcome::Unverifiable,
                        CheckKind::Capture,
                        source_capture_uri_path(index),
                        error.error,
                    ));
                }
            }
        }
    } else if let CaptureFetch::Unfetched(error) = resolved {
        result.checks.push(check(
            Outcome::Unverifiable,
            CheckKind::Capture,
            source_capture_uri_path(index),
            error.error,
        ));
    }
}

async fn fetch_capture(
    index: usize,
    descriptor: &Map<String, Value>,
    reference: &str,
    resolver: &dyn Resolver,
) -> CaptureFetch {
    let uri = match effective_reference(descriptor, reference, resolver.screens_local_paths()) {
        Ok(uri) => uri,
        Err(error) => {
            return CaptureFetch::Unfetched(UnfetchedContent {
                index,
                error: error.to_string(),
            });
        }
    };
    match resolver.fetch(&uri).await {
        Ok(payload) => CaptureFetch::Resolved(ResolvedCapture { payload }),
        Err(error) => CaptureFetch::Unfetched(UnfetchedContent {
            index,
            error: error.to_string(),
        }),
    }
}

async fn sidecar_checks(
    graph: &Graph<'_>,
    descriptor: &Map<String, Value>,
    resolver: &dyn Resolver,
) -> SidecarResult {
    let mut result = SidecarResult::default();
    for (index, section) in &graph.sections {
        let Some(reference) = section.get("provenance_uri").and_then(Value::as_str) else {
            continue;
        };
        let resolved = fetch_section(*index, section, descriptor, reference, resolver).await;
        if let Some(hash) = section.get("provenance_hash").and_then(Value::as_str) {
            let path = section_provenance_hash_path(*index);
            match parse_sri(CheckKind::Sidecar, path.clone(), hash) {
                Ok(expected) => {
                    result
                        .checks
                        .push(check_sri(CheckKind::Sidecar, path, &resolved, &expected));
                }
                Err(check) => result.checks.push(check),
            }
        }
        let SectionFetch::Resolved(resolved) = resolved else {
            if let SectionFetch::Unfetched(unfetched) = resolved {
                result.checks.push(check(
                    Outcome::Unverifiable,
                    CheckKind::Sidecar,
                    section_provenance_uri_path(unfetched.index),
                    unfetched.error,
                ));
            }
            continue;
        };
        let sidecar = match parse_sidecar(&resolved.payload) {
            Ok(sidecar) => sidecar,
            Err(error) => {
                result.checks.push(check(
                    Outcome::Failed,
                    CheckKind::Sidecar,
                    section_provenance_uri_path(*index),
                    error,
                ));
                continue;
            }
        };
        let (findings, binding_mismatch) = sidecar_findings(graph, *index, section, &sidecar);
        // Match the reference validator: a failed provenance_hash remains a
        // failed Sidecar check, but the fetched sidecar still contributes
        // quote claims so callers can see every observable content result.
        result.claims.extend(sidecar_quote_claims(*index, &sidecar));
        let detail = sidecar_detail(section, &sidecar, binding_mismatch);
        result.checks.push(ContentCheck {
            kind: CheckKind::Sidecar,
            path: section_provenance_uri_path(*index),
            outcome: if findings.is_empty() && !binding_mismatch {
                Outcome::Verified
            } else {
                Outcome::Failed
            },
            detail,
            findings,
            warnings: Vec::new(),
        });
    }
    result
}

fn parse_sidecar(payload: &[u8]) -> Result<Value, String> {
    let text = String::from_utf8(payload.to_vec()).map_err(utf8_error_detail)?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn sidecar_findings(
    graph: &Graph<'_>,
    section_index: usize,
    section: &Object,
    sidecar: &Value,
) -> (Vec<Finding>, bool) {
    let mut findings: Vec<_> = provenance_schema_findings(sidecar)
        .into_iter()
        .map(|finding| Finding {
            code: finding.code,
            path: format!(
                "{}{}",
                section_provenance_uri_path(section_index),
                finding.path
            ),
            message: finding.message,
        })
        .collect();

    let Some(sidecar_object) = sidecar.as_object() else {
        findings.sort();
        return (findings, false);
    };

    let section_id = sidecar_object.get("section_id").and_then(Value::as_str);
    let mut binding_mismatch = false;
    if let Some(section_id) = section_id {
        if let Some(code) = reference_code_id(section_id, EntityKind::Section, &graph.entities) {
            findings.push(Finding {
                code,
                path: pointer([
                    PathSegment::Key("sections"),
                    PathSegment::Index(section_index),
                    PathSegment::Key("provenance_uri"),
                    PathSegment::Key("section_id"),
                ]),
                message: format!("sidecar section_id {section_id:?} does not resolve to a section"),
            });
        } else if Some(section_id) != section.get("id").and_then(Value::as_str) {
            binding_mismatch = true;
        }
    }

    for (claim_index, claim) in indexed_objects(sidecar_object.get("claims")) {
        append_sidecar_source_findings(&mut findings, graph, section_index, claim_index, claim);
    }
    findings.sort();
    (findings, binding_mismatch)
}

fn append_sidecar_source_findings(
    findings: &mut Vec<Finding>,
    graph: &Graph<'_>,
    section_index: usize,
    claim_index: usize,
    claim: &Object,
) {
    let Some(source_ids) = claim.get("source_ids").and_then(Value::as_array) else {
        return;
    };
    for (source_index, source_id) in source_ids.iter().enumerate() {
        let Some(source_id) = source_id.as_str() else {
            continue;
        };
        if let Some(code) = reference_code_id(source_id, EntityKind::Source, &graph.entities) {
            findings.push(Finding {
                code,
                path: pointer([
                    PathSegment::Key("sections"),
                    PathSegment::Index(section_index),
                    PathSegment::Key("provenance_uri"),
                    PathSegment::Key("claims"),
                    PathSegment::Index(claim_index),
                    PathSegment::Key("source_ids"),
                    PathSegment::Index(source_index),
                ]),
                message: format!(
                    "sidecar claim source id {source_id:?} does not resolve to a source"
                ),
            });
        }
    }
}

fn sidecar_detail(section: &Object, sidecar: &Value, binding_mismatch: bool) -> String {
    if !binding_mismatch {
        return "provenance sidecar checked".to_owned();
    }
    let section_id = sidecar
        .as_object()
        .and_then(|object| object.get("section_id"))
        .and_then(Value::as_str);
    let expected = section.get("id").and_then(Value::as_str);
    format!("sidecar section_id {section_id:?} names a different section; expected {expected:?}")
}

fn inline_quote_claims(graph: &Graph<'_>) -> Vec<QuoteClaim> {
    let mut claims = Vec::new();
    for (section_index, section) in &graph.sections {
        for (claim_index, claim) in indexed_objects(section.get("provenance")) {
            if let Some(claim) = quote_claim(
                vec![
                    PathSegment::Key("sections"),
                    PathSegment::Index(*section_index),
                    PathSegment::Key("provenance"),
                    PathSegment::Index(claim_index),
                ],
                claim,
            ) {
                claims.push(claim);
            }
        }
    }
    claims
}

fn sidecar_quote_claims(section_index: usize, sidecar: &Value) -> Vec<QuoteClaim> {
    let Some(sidecar_object) = sidecar.as_object() else {
        return Vec::new();
    };
    let mut claims = Vec::new();
    for (claim_index, claim) in indexed_objects(sidecar_object.get("claims")) {
        if let Some(claim) = quote_claim(
            vec![
                PathSegment::Key("sections"),
                PathSegment::Index(section_index),
                PathSegment::Key("provenance_uri"),
                PathSegment::Key("claims"),
                PathSegment::Index(claim_index),
            ],
            claim,
        ) {
            claims.push(claim);
        }
    }
    claims
}

fn quote_claim(path: Vec<PathSegment>, claim: &Object) -> Option<QuoteClaim> {
    let quote = claim
        .get("locator")
        .and_then(Value::as_object)
        .and_then(|locator| locator.get("quote"))
        .and_then(Value::as_str)?;
    if quote.is_empty() {
        return None;
    }
    let source_ids = claim
        .get("source_ids")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if source_ids.is_empty() {
        return None;
    }
    Some(QuoteClaim {
        path,
        quote: quote.to_owned(),
        source_ids,
    })
}

fn quote_checks(
    graph: &Graph<'_>,
    captures: &CaptureResult,
    quote_claims: &[QuoteClaim],
) -> Vec<ContentCheck> {
    quote_claims
        .iter()
        .map(|claim| {
            let (outcome, detail) = quote_outcome(claim, captures);
            let mut path = claim.path.clone();
            path.extend([PathSegment::Key("locator"), PathSegment::Key("quote")]);
            let path = pointer(path);
            ContentCheck {
                kind: CheckKind::Quote,
                path: path.clone(),
                outcome,
                detail,
                findings: Vec::new(),
                warnings: redacted_warnings(graph, &path, &claim.source_ids),
            }
        })
        .collect()
}

fn quote_outcome(claim: &QuoteClaim, captures: &CaptureResult) -> (Outcome, String) {
    let usable: Vec<_> = claim
        .source_ids
        .iter()
        .filter(|source_id| !captures.hash_failed.contains(*source_id))
        .filter_map(|source_id| captures.payloads.get(source_id))
        .collect();
    let needle = claim.quote.as_bytes();
    if usable
        .iter()
        .any(|payload| payload.windows(needle.len()).any(|window| window == needle))
    {
        return (Outcome::Verified, "quote found in capture".to_owned());
    }
    if claim.source_ids.iter().all(|source_id| {
        captures.payloads.contains_key(source_id) && !captures.hash_failed.contains(source_id)
    }) {
        return (
            Outcome::Failed,
            "quote absent from fetched captures".to_owned(),
        );
    }
    if usable.is_empty() {
        if claim
            .source_ids
            .iter()
            .any(|source_id| captures.hash_failed.contains(source_id))
        {
            return (
                Outcome::Unverifiable,
                "a cited source's capture failed its content_hash".to_owned(),
            );
        }
        return (
            Outcome::Unverifiable,
            "no cited source capture fetched".to_owned(),
        );
    }
    if claim
        .source_ids
        .iter()
        .any(|source_id| captures.hash_failed.contains(source_id))
    {
        return (
            Outcome::Unverifiable,
            "a cited source's capture failed its content_hash".to_owned(),
        );
    }
    (
        Outcome::Unverifiable,
        "some cited source captures were not fetched".to_owned(),
    )
}

fn redacted_warnings(graph: &Graph<'_>, path: &str, source_ids: &[String]) -> Vec<Advisory> {
    let redacted: Vec<_> = source_ids
        .iter()
        .filter(|source_id| {
            graph.sources.iter().any(|(_, source)| {
                source.get("id").and_then(Value::as_str) == Some(source_id.as_str())
                    && source.get("type").and_then(Value::as_str) == Some("redacted")
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect();
    if redacted.is_empty() {
        return Vec::new();
    }
    vec![Advisory {
        path: path.to_owned(),
        message: format!("quote cites redacted source(s): {}", redacted.join(", ")),
    }]
}

fn check_sri(
    kind: CheckKind,
    path: String,
    resolved: &SectionFetch<'_>,
    expected: &[u8],
) -> ContentCheck {
    match resolved {
        SectionFetch::Resolved(resolved) => compare_sri(kind, path, &resolved.payload, expected),
        SectionFetch::Unfetched(unfetched) => {
            check(Outcome::Unverifiable, kind, path, unfetched.error.clone())
        }
    }
}

fn parse_sri(kind: CheckKind, path: String, sri: &str) -> Result<Vec<u8>, ContentCheck> {
    let Some((algorithm, encoded)) = sri.split_once('-') else {
        return Err(warning_check(
            kind,
            path,
            "malformed SRI: expected '<algorithm>-<base64>'",
        ));
    };
    if algorithm != SHA256_ALGORITHM {
        return Err(warning_check(
            kind,
            path,
            format!("unsupported hash algorithm: {algorithm}"),
        ));
    }
    let Ok(decoded) = STRICT_STANDARD_BASE64.decode(encoded.as_bytes()) else {
        return Err(warning_check(kind, path, "malformed sha256 digest"));
    };
    if decoded.len() != SHA256_LENGTH {
        return Err(warning_check(kind, path, "sha256 digest has wrong length"));
    }
    if STANDARD.encode(&decoded) != encoded {
        return Err(warning_check(kind, path, "non-canonical sha256 digest"));
    }
    Ok(decoded)
}

fn compare_sri(kind: CheckKind, path: String, payload: &[u8], expected: &[u8]) -> ContentCheck {
    let actual = Sha256::digest(payload);
    if actual.as_slice() == expected {
        check(Outcome::Verified, kind, path, "sha256 digest matches")
    } else {
        check(Outcome::Failed, kind, path, "sha256 digest mismatch")
    }
}

fn check(
    outcome: Outcome,
    kind: CheckKind,
    path: String,
    detail: impl Into<String>,
) -> ContentCheck {
    ContentCheck {
        kind,
        path,
        outcome,
        detail: detail.into(),
        findings: Vec::new(),
        warnings: Vec::new(),
    }
}

fn warning_check(kind: CheckKind, path: String, detail: impl Into<String>) -> ContentCheck {
    let detail = detail.into();
    ContentCheck {
        kind,
        path: path.clone(),
        outcome: Outcome::Unverifiable,
        detail: detail.clone(),
        findings: Vec::new(),
        warnings: vec![Advisory {
            path,
            message: detail,
        }],
    }
}

fn effective_reference(
    descriptor: &Map<String, Value>,
    reference: &str,
    screen_local_paths: bool,
) -> Result<String, Unfetchable> {
    if screen_local_paths {
        local_raw_reference_error(reference)?;
        if let Some(base_uri) = descriptor.get("base_uri").and_then(Value::as_str) {
            local_raw_reference_error(base_uri)?;
        }
    }
    let joined = descriptor
        .get("base_uri")
        .and_then(Value::as_str)
        .map_or_else(
            || reference.to_owned(),
            |base_uri| join_reference(base_uri, reference),
        );
    if screen_local_paths {
        local_raw_reference_error(&joined)?;
    }
    Ok(strip_fragment(&joined).to_owned())
}

fn local_raw_reference_error(reference: &str) -> Result<(), Unfetchable> {
    let raw_reference = strip_fragment(reference);
    let raw_path = raw_reference
        .split_once('?')
        .map_or(raw_reference, |(path, _)| path);
    if raw_reference.contains('?')
        || raw_path.contains(';')
        || raw_path.contains('\\')
        || raw_path.split('/').any(|segment| segment == "..")
    {
        return Err(Unfetchable::outside_local_base(reference));
    }
    Ok(())
}

fn join_reference(base_uri: &str, reference: &str) -> String {
    let Ok(reference) = UriRef::parse(reference) else {
        return reference.to_owned();
    };
    let base_without_fragment = strip_fragment(base_uri);
    if let Ok(base) = Uri::parse(base_without_fragment) {
        return reference
            .resolve_against(&base)
            .map(|uri| uri.to_string())
            .unwrap_or_else(|_| reference.to_string());
    }
    join_relative_reference(base_without_fragment, &reference)
}

fn join_relative_reference(base_uri: &str, reference: &UriRef<&str>) -> String {
    if reference.scheme().is_some() || reference.authority().is_some() {
        return reference.to_string();
    }
    let (base_path, base_query) = split_query(base_uri);
    let reference_path = reference.path().as_str();
    let path = if reference_path.is_empty() {
        base_path.to_owned()
    } else if reference_path.starts_with('/') {
        remove_dot_segments(reference_path)
    } else {
        let directory = base_path
            .rfind('/')
            .map_or("", |index| &base_path[..=index]);
        remove_dot_segments(&format!("{directory}{reference_path}"))
    };
    let query = if reference_path.is_empty() {
        reference
            .query()
            .map_or(base_query, |query| Some(query.as_str()))
    } else {
        reference.query().map(|query| query.as_str())
    };
    match query {
        Some(query) => format!("{path}?{query}"),
        None => path,
    }
}

fn split_query(reference: &str) -> (&str, Option<&str>) {
    reference
        .split_once('?')
        .map_or((reference, None), |(path, query)| (path, Some(query)))
}

fn remove_dot_segments(path: &str) -> String {
    let mut output = Vec::new();
    let absolute = path.starts_with('/');
    let trailing_slash = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..");
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                output.pop();
            }
            value => output.push(value),
        }
    }
    let mut normalized = String::new();
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&output.join("/"));
    if trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    if normalized.is_empty() && absolute {
        normalized.push('/');
    }
    normalized
}

fn rejects_local_reference(raw_reference: &str, raw_path: &str) -> bool {
    raw_reference.contains('?')
        || raw_path.contains(';')
        || raw_path.contains('\\')
        || raw_path.split('/').any(|segment| segment == "..")
        || is_absolute_path(raw_path)
        || raw_reference.starts_with("//")
        || has_uri_scheme(raw_reference)
}

fn is_absolute_path(raw_path: &str) -> bool {
    raw_path.starts_with('/')
        || Path::new(raw_path)
            .components()
            .next()
            .is_some_and(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
}

fn has_uri_scheme(reference: &str) -> bool {
    let Some((scheme, _)) = reference.split_once(':') else {
        return false;
    };
    if scheme.contains('/') || scheme.contains('?') {
        return false;
    }
    let mut bytes = scheme.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn strip_fragment(uri: &str) -> &str {
    uri.split_once('#')
        .map_or(uri, |(without_fragment, _)| without_fragment)
}

fn guide_hash_path() -> String {
    json_pointer([Segment::Key("guide_hash")])
}

fn source_path(index: usize) -> String {
    json_pointer([Segment::Key("sources"), Segment::Index(index)])
}

fn source_capture_uri_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sources"),
        Segment::Index(index),
        Segment::Key("capture_uri"),
    ])
}

fn source_content_hash_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sources"),
        Segment::Index(index),
        Segment::Key("content_hash"),
    ])
}

fn section_content_uri_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sections"),
        Segment::Index(index),
        Segment::Key("content_uri"),
    ])
}

fn section_hash_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sections"),
        Segment::Index(index),
        Segment::Key("content_hash"),
    ])
}

fn section_provenance_uri_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sections"),
        Segment::Index(index),
        Segment::Key("provenance_uri"),
    ])
}

fn section_provenance_hash_path(index: usize) -> String {
    json_pointer([
        Segment::Key("sections"),
        Segment::Index(index),
        Segment::Key("provenance_hash"),
    ])
}

fn pointer(segments: impl IntoIterator<Item = PathSegment>) -> String {
    json_pointer(segments.into_iter().map(|segment| match segment {
        PathSegment::Key(key) => Segment::Key(key),
        PathSegment::Index(index) => Segment::Index(index),
    }))
}

fn is_markdown(content_type: &Value) -> bool {
    content_type.as_str().is_some_and(|value| {
        value
            .split_once(';')
            .map_or(value, |(essence, _)| essence)
            .trim()
            .eq_ignore_ascii_case("text/markdown")
    })
}

fn utf8_error_detail(error: FromUtf8Error) -> String {
    error.utf8_error().to_string()
}

fn ids(items: &[(usize, &Object)]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|(_, item)| Some(local_id_value(item.get("id"))?.to_owned()))
        .collect()
}
