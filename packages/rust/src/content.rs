//! Opt-in content checks that fetch descriptor-related resources.

use std::path::{Component, Path, PathBuf};

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

use crate::{Advisory, Finding, Segment, json_pointer, shape::indexed_objects};

const SHA256_ALGORITHM: &str = "sha256";
const SHA256_LENGTH: usize = 32;
const STRICT_STANDARD_BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::RequireCanonical)
        .with_decode_allow_trailing_bits(false),
);

/// Fetches descriptor-related content bytes.
#[async_trait]
pub trait Resolver: Send + Sync {
    /// Returns bytes for `uri`, or an [`Unfetchable`] error when the resource
    /// cannot be fetched.
    async fn fetch(&self, uri: &str) -> Result<Vec<u8>, Unfetchable>;
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

    let mut checks = Vec::new();
    checks.extend(guide_checks(object, resolver).await);
    checks.extend(capture_checks(object, resolver).await);
    ContentReport { checks }
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

    let uri = effective_reference(object, guide_uri);
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

async fn capture_checks(object: &Map<String, Value>, resolver: &dyn Resolver) -> Vec<ContentCheck> {
    let mut checks = Vec::new();

    for (index, source) in indexed_objects(object.get("sources")) {
        append_capture_checks(&mut checks, object, index, source, resolver).await;
    }

    checks
}

async fn append_capture_checks(
    checks: &mut Vec<ContentCheck>,
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
            checks.push(check(
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
                Ok(_) => checks.push(check(
                    Outcome::Unverifiable,
                    CheckKind::Capture,
                    content_hash_path,
                    "missing capture_uri",
                )),
                Err(check) => checks.push(check),
            }
        }
        return;
    };

    let uri = effective_reference(descriptor, capture_uri);
    let resolved = resolver.fetch(&uri).await;

    if let Some(hash_check) = hash_check {
        match hash_check {
            Ok(expected) => match resolved {
                Ok(payload) => {
                    checks.push(compare_sri(
                        CheckKind::Capture,
                        content_hash_path,
                        &payload,
                        &expected,
                    ));
                }
                Err(error) => checks.push(check(
                    Outcome::Unverifiable,
                    CheckKind::Capture,
                    content_hash_path,
                    error.to_string(),
                )),
            },
            Err(warning) => {
                checks.push(warning);
                if let Err(error) = resolved {
                    checks.push(check(
                        Outcome::Unverifiable,
                        CheckKind::Capture,
                        source_capture_uri_path(index),
                        error.to_string(),
                    ));
                }
            }
        }
    } else if let Err(error) = resolved {
        checks.push(check(
            Outcome::Unverifiable,
            CheckKind::Capture,
            source_capture_uri_path(index),
            error.to_string(),
        ));
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

fn effective_reference(descriptor: &Map<String, Value>, reference: &str) -> String {
    let joined = descriptor
        .get("base_uri")
        .and_then(Value::as_str)
        .map_or_else(
            || reference.to_owned(),
            |base_uri| join_reference(base_uri, reference),
        );
    strip_fragment(&joined).to_owned()
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
