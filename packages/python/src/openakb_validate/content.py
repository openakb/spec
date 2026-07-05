"""Opt-in content checks that require fetching descriptor-related resources."""

from __future__ import annotations

import base64
import binascii
import hashlib
import json
import os
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Protocol, cast
from urllib.parse import urldefrag, urljoin, urlparse

from ._shape import indexed_dicts, reference_code
from .citations import extract_citations
from .result import Advisory, Finding, json_pointer
from .schema import provenance_validator, schema_findings

__all__ = [
    "FAILED",
    "KIND_CAPTURE",
    "KIND_CITATIONS",
    "KIND_CONTENT_HASH",
    "KIND_GUIDE_HASH",
    "KIND_QUOTE",
    "KIND_SIDECAR",
    "UNVERIFIABLE",
    "VERIFIED",
    "ContentCheck",
    "ContentReport",
    "LocalFileResolver",
    "Resolver",
    "Unfetchable",
    "check_content",
]

# The three outcomes a content check can report (spec §7).
VERIFIED = "verified"
FAILED = "failed"
UNVERIFIABLE = "unverifiable"

# The `ContentCheck.kind` values, named so callers can filter without hard-coding the
# strings. Each names what a check proves: a hash over the authoring guide, a section's
# content, or a source capture; the citation markers inside fetched Markdown; a
# provenance sidecar; or a claim quote located in a fetched capture.
KIND_GUIDE_HASH = "guide-hash"
KIND_CONTENT_HASH = "content-hash"
KIND_CITATIONS = "citations"
KIND_SIDECAR = "sidecar"
KIND_CAPTURE = "capture"
KIND_QUOTE = "quote"

_MARKDOWN_TYPE = "text/markdown"
_SHA256 = "sha256"
_SHA256_LENGTH = 32

# Unified detail for both `_quote_outcome` branches where a cited source's capture was
# fetched but proved wrong by its content_hash -- distinct from the "not fetched at
# all" fetch-gap details below, which describe a different situation.
_QUOTE_HASH_FAILED_DETAIL = "a cited source's capture failed its content_hash"


class Unfetchable(Exception):
    """Raised by resolvers when a resource cannot be fetched."""


class Resolver(Protocol):
    """Injected content resolver; implementations decide what URI schemes exist.

    `check_content` calls `fetch` with the *effective reference*: the descriptor
    reference (a `content_uri`, `capture_uri`, `guide_uri`, or `provenance_uri`)
    already joined onto the descriptor's `base_uri` when one is present, with any
    `#fragment` stripped -- fragments are client-side identifiers that never change
    the fetched bytes. The `uri` is therefore ready to dereference as-is.

    An implementation MUST return the resource `bytes` or raise `Unfetchable`; any
    other exception propagates to the caller. Anything a resolver cannot fetch is an
    `Unfetchable`, which `check_content` records as an `unverifiable` outcome -- never
    an error and never a `failed` verdict.

    The pre-fetch path policing that `LocalFileResolver` applies (rejecting absolute
    paths, `..` traversal, queries, and path parameters) is specific to that concrete
    class; a custom resolver receives no such guard and owns its own safety.
    """

    def fetch(self, uri: str) -> bytes:
        """Return bytes for uri or raise Unfetchable."""


@dataclass(frozen=True)
class LocalFileResolver:
    """Resolve scheme-less relative paths under one local base directory.

    `base_dir` may be a `str` or any `os.PathLike[str]`; it is coerced to a `Path` on
    construction. URI fragments are client-side identifiers and do not affect the
    fetched bytes. Queries and path parameters are rejected because they would
    otherwise alias to the same local path. This pre-fetch policing is unique to this
    resolver -- the `Resolver` protocol imposes none of it on custom implementations.
    """

    base_dir: str | os.PathLike[str]
    # Path-typed cache of the coerced base_dir, computed once in __post_init__. Internal
    # code reads this instead of `self.base_dir` so mypy sees a `Path` directly, with no
    # repeated `Path(...)` re-wrap to reassure the checker about the field's wide
    # declared type.
    _resolved_base_dir: Path = field(init=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        # base_dir is a frozen field; coerce a str/PathLike to Path here so downstream
        # path handling never sees a raw str (which has no `.resolve()`).
        resolved = Path(self.base_dir)
        object.__setattr__(self, "base_dir", resolved)
        object.__setattr__(self, "_resolved_base_dir", resolved)

    def fetch(self, uri: str) -> bytes:
        try:
            path = self._local_path(uri)
            return path.read_bytes()
        except Unfetchable:
            raise
        except (OSError, RuntimeError, ValueError) as error:
            raise Unfetchable(str(error)) from error

    def _local_path(self, uri: str) -> Path:
        parsed = urlparse(uri)
        raw_reference = uri.split("#", 1)[0]
        raw_path = raw_reference.split("?", 1)[0]
        if (
            parsed.scheme
            or parsed.netloc
            or parsed.params
            or parsed.query
            or "?" in raw_reference
            or ";" in raw_path
            or "\\" in raw_path
            or ".." in parsed.path.split("/")
            or Path(parsed.path).is_absolute()
        ):
            raise Unfetchable(f"outside local base: {uri}")
        base = self._resolved_base_dir.resolve()
        path = (base / parsed.path).resolve()
        if path != base and base not in path.parents:
            raise Unfetchable(f"outside local base: {uri}")
        return path


@dataclass(frozen=True)
class ContentCheck:
    """One opt-in content check and its three-state outcome."""

    kind: str
    path: str
    outcome: str
    detail: str
    findings: tuple[Finding, ...] = ()
    warnings: tuple[Advisory, ...] = ()


@dataclass(frozen=True)
class ContentReport:
    """Aggregate result for all opt-in content checks."""

    checks: tuple[ContentCheck, ...]

    @property
    def findings(self) -> tuple[Finding, ...]:
        return tuple(finding for check in self.checks for finding in check.findings)

    @property
    def warnings(self) -> tuple[Advisory, ...]:
        return tuple(warning for check in self.checks for warning in check.warnings)

    @property
    def verified(self) -> tuple[ContentCheck, ...]:
        return tuple(check for check in self.checks if check.outcome == VERIFIED)

    @property
    def failed(self) -> tuple[ContentCheck, ...]:
        return tuple(check for check in self.checks if check.outcome == FAILED)

    @property
    def unverifiable(self) -> tuple[ContentCheck, ...]:
        return tuple(check for check in self.checks if check.outcome == UNVERIFIABLE)

    @property
    def ok(self) -> bool:
        # `ok` is true when nothing failed and no structural finding was raised -- which
        # includes the all-unverifiable case where nothing was actually verified. Callers
        # that must distinguish "all good" from "nothing checked" read `verified`.
        return not self.failed and not self.findings


def check_content(descriptor: object, resolver: Resolver) -> ContentReport:
    """Run opt-in content checks against fetched descriptor resources."""
    if not isinstance(descriptor, dict):
        return ContentReport(checks=())
    graph = _Graph.from_descriptor(descriptor)
    base_uri = descriptor.get("base_uri") if isinstance(descriptor.get("base_uri"), str) else None
    local = isinstance(resolver, LocalFileResolver)
    checks: list[ContentCheck] = []
    checks.extend(_guide_check(descriptor, base_uri, resolver, local))
    checks.extend(_section_checks(graph, base_uri, resolver, local))
    sidecars = _sidecar_checks(graph, base_uri, resolver, local)
    checks.extend(sidecars.checks)
    quote_claims = [*_inline_quote_claims(graph), *sidecars.claims]
    captures = _capture_checks(graph, base_uri, resolver, local, quote_claims)
    checks.extend(captures.checks)
    checks.extend(_quote_checks(graph, captures.payloads, captures.hash_failed, quote_claims))
    return ContentReport(checks=tuple(checks))


@dataclass(frozen=True)
class _ResolvedContent:
    index: int
    section: dict[str, Any]
    uri: str
    payload: bytes


@dataclass(frozen=True)
class _UnfetchedContent:
    index: int
    error: Unfetchable


@dataclass(frozen=True)
class _ResolvedCapture:
    source_index: int
    source: dict[str, Any]
    uri: str
    payload: bytes


@dataclass(frozen=True)
class _CaptureResult:
    payloads: dict[str, bytes]
    checks: list[ContentCheck]
    # Source ids whose fetched capture failed its content_hash: the bytes are proven
    # wrong, so a quote MUST NOT be checked against them (spec 7).
    hash_failed: frozenset[str]


@dataclass(frozen=True)
class _QuoteClaim:
    path: list[str | int]
    quote: str
    source_ids: tuple[str, ...]


@dataclass(frozen=True)
class _SidecarResult:
    claims: list[_QuoteClaim]
    checks: list[ContentCheck]


@dataclass(frozen=True)
class _Graph:
    sources: list[tuple[int, dict[str, Any]]]
    sections: list[tuple[int, dict[str, Any]]]
    source_ids: frozenset[str]
    section_ids: frozenset[str]

    @classmethod
    def from_descriptor(cls, descriptor: dict[str, Any]) -> _Graph:
        sources = indexed_dicts(descriptor.get("sources"))
        sections = indexed_dicts(descriptor.get("sections"))
        return cls(
            sources=sources,
            sections=sections,
            source_ids=frozenset(_ids(sources)),
            section_ids=frozenset(_ids(sections)),
        )


def _section_checks(
    graph: _Graph, base_uri: str | None, resolver: Resolver, local: bool
) -> list[ContentCheck]:
    checks: list[ContentCheck] = []
    for index, section in graph.sections:
        reference = section.get("content_uri")
        if not isinstance(reference, str):
            continue
        has_content_hash = isinstance(section.get("content_hash"), str)
        checks_citations = "content_type" not in section or _is_markdown(
            section.get("content_type")
        )
        if not has_content_hash and not checks_citations:
            continue
        hash_check = None
        if has_content_hash:
            hash_check = _parse_sri(
                KIND_CONTENT_HASH,
                ["sections", index, "content_hash"],
                section["content_hash"],
            )
            if isinstance(hash_check, ContentCheck) and not checks_citations:
                checks.append(hash_check)
                continue
        resolved = _fetch_section(index, section, reference, base_uri, resolver, local)
        if has_content_hash:
            if isinstance(hash_check, ContentCheck):
                checks.append(hash_check)
            else:
                checks.append(
                    _check_sri(
                        KIND_CONTENT_HASH,
                        ["sections", index, "content_hash"],
                        section["content_hash"],
                        resolved,
                    )
                )
        if checks_citations:
            checks.append(_citation_check(graph, resolved))
    return checks


def _guide_check(
    descriptor: dict[str, Any], base_uri: str | None, resolver: Resolver, local: bool
) -> list[ContentCheck]:
    if not isinstance(descriptor.get("guide_hash"), str):
        return []
    path: list[str | int] = ["guide_hash"]
    expected = _parse_sri(KIND_GUIDE_HASH, path, descriptor["guide_hash"])
    if isinstance(expected, ContentCheck):
        return [expected]
    # Mirror a source content_hash with no capture_uri: a guide_hash with nothing to
    # fetch is unverifiable, not silently dropped.
    if not isinstance(descriptor.get("guide_uri"), str):
        return [_check(UNVERIFIABLE, KIND_GUIDE_HASH, path, "missing guide_uri")]
    uri = _effective_reference(descriptor["guide_uri"], base_uri, local)
    if isinstance(uri, Unfetchable):
        return [_check(UNVERIFIABLE, KIND_GUIDE_HASH, path, str(uri))]
    try:
        payload = resolver.fetch(uri)
    except Unfetchable as error:
        return [_check(UNVERIFIABLE, KIND_GUIDE_HASH, path, str(error))]
    return [_compare_sri(KIND_GUIDE_HASH, path, payload, expected)]


def _check_sri(
    kind: str,
    path: list[str | int],
    sri: str,
    resolved: _ResolvedContent | _UnfetchedContent,
) -> ContentCheck:
    expected = _parse_sri(kind, path, sri)
    if isinstance(expected, ContentCheck):
        return expected
    if isinstance(resolved, _UnfetchedContent):
        return _check(UNVERIFIABLE, kind, path, str(resolved.error))
    return _compare_sri(kind, path, resolved.payload, expected)


def _citation_check(graph: _Graph, resolved: _ResolvedContent | _UnfetchedContent) -> ContentCheck:
    if isinstance(resolved, _UnfetchedContent):
        return _check(
            UNVERIFIABLE,
            KIND_CITATIONS,
            _content_path(resolved.index),
            str(resolved.error),
        )
    try:
        markdown = resolved.payload.decode("utf-8")
    except UnicodeDecodeError as error:
        # Fetched-but-malformed content is a failure, matching the sidecar JSON parse
        # precedent and the spec 7 "malformed ... fetched" rule -- not an access gap.
        return _check(FAILED, KIND_CITATIONS, _content_path(resolved.index), str(error))
    findings, warnings = _citation_results(graph, resolved.index, markdown)
    return ContentCheck(
        kind=KIND_CITATIONS,
        path=json_pointer(_content_path(resolved.index)),
        outcome=FAILED if findings else VERIFIED,
        detail="citation markers checked",
        findings=tuple(findings),
        warnings=tuple(warnings),
    )


def _effective_reference(reference: str, base_uri: str | None, local: bool) -> str | Unfetchable:
    if local:
        raw_error = _local_raw_reference_error(reference)
        if raw_error is not None:
            return raw_error
        if base_uri is not None:
            base_error = _local_raw_reference_error(base_uri)
            if base_error is not None:
                return base_error
    try:
        effective = reference if base_uri is None else urljoin(base_uri, reference)
        if local:
            # Defense in depth: the raw reference and base_uri are already screened
            # above and urljoin only resolves "../" segments away, so the joined
            # result should carry no new hostile pattern. Re-screening it costs
            # nothing and fails safe should a future join introduce one.
            effective_error = _local_raw_reference_error(effective)
            if effective_error is not None:
                # No hostile pattern survives the pre-join screens above to reach
                # here, so no input drives this branch in practice.
                return effective_error  # not coverable in CI: see comment above
        return urldefrag(effective).url
    except ValueError as error:
        return Unfetchable(str(error))


def _capture_checks(
    graph: _Graph,
    base_uri: str | None,
    resolver: Resolver,
    local: bool,
    quote_claims: list[_QuoteClaim],
) -> _CaptureResult:
    payloads: dict[str, bytes] = {}
    hash_failed: set[str] = set()
    checks: list[ContentCheck] = []
    for index, source in graph.sources:
        source_id = source.get("id")
        path: list[str | int] = ["sources", index, "content_hash"]
        if source.get("type") == "redacted":
            if isinstance(source.get("capture_uri"), str) or isinstance(
                source.get("content_hash"), str
            ):
                checks.append(
                    _check(UNVERIFIABLE, KIND_CAPTURE, ["sources", index], "redacted source")
                )
            continue
        reference = source.get("capture_uri")
        has_hash = isinstance(source.get("content_hash"), str)
        hash_check = _parse_sri(KIND_CAPTURE, path, source["content_hash"]) if has_hash else None
        if not isinstance(reference, str):
            if has_hash:
                if isinstance(hash_check, ContentCheck):
                    checks.append(hash_check)
                else:
                    checks.append(_check(UNVERIFIABLE, KIND_CAPTURE, path, "missing capture_uri"))
            continue
        resolved = _fetch_capture(index, source, reference, base_uri, resolver, local)
        if isinstance(resolved, _ResolvedCapture) and isinstance(source_id, str):
            payloads[source_id] = resolved.payload
        if isinstance(hash_check, ContentCheck):
            checks.append(hash_check)
        if isinstance(resolved, _UnfetchedContent) and (
            not has_hash or isinstance(hash_check, ContentCheck)
        ):
            checks.append(
                _check(
                    UNVERIFIABLE,
                    KIND_CAPTURE,
                    ["sources", index, "capture_uri"],
                    str(resolved.error),
                )
            )
        if has_hash:
            if isinstance(hash_check, ContentCheck):
                continue
            elif isinstance(resolved, _UnfetchedContent):
                checks.append(_check(UNVERIFIABLE, KIND_CAPTURE, path, str(resolved.error)))
            else:
                # hash_check is bytes here: the ContentCheck case continued above and
                # resolved is a _ResolvedCapture. cast narrows without a runtime assert.
                expected = cast("bytes", hash_check)
                comparison = _compare_sri(KIND_CAPTURE, path, resolved.payload, expected)
                checks.append(comparison)
                if comparison.outcome == FAILED and isinstance(source_id, str):
                    hash_failed.add(source_id)
    return _CaptureResult(payloads=payloads, checks=checks, hash_failed=frozenset(hash_failed))


def _sidecar_checks(
    graph: _Graph, base_uri: str | None, resolver: Resolver, local: bool
) -> _SidecarResult:
    claims: list[_QuoteClaim] = []
    checks: list[ContentCheck] = []
    for index, section in graph.sections:
        reference = section.get("provenance_uri")
        if not isinstance(reference, str):
            continue
        resolved = _fetch_sidecar(index, section, reference, base_uri, resolver, local)
        if isinstance(section.get("provenance_hash"), str):
            checks.append(_sidecar_hash_check(index, section["provenance_hash"], resolved))
        if isinstance(resolved, _UnfetchedContent):
            checks.append(
                _check(UNVERIFIABLE, KIND_SIDECAR, _sidecar_path(index), str(resolved.error))
            )
            continue
        sidecar, parse_error = _parse_sidecar(resolved.payload)
        if parse_error is not None:
            checks.append(_check(FAILED, KIND_SIDECAR, _sidecar_path(index), parse_error))
            continue
        findings, binding_mismatch = _sidecar_findings(graph, index, section, sidecar)
        claims.extend(_sidecar_quote_claims(index, sidecar))
        detail = _sidecar_detail(section, sidecar, binding_mismatch)
        checks.append(
            ContentCheck(
                kind=KIND_SIDECAR,
                path=json_pointer(_sidecar_path(index)),
                outcome=FAILED if findings or binding_mismatch else VERIFIED,
                detail=detail,
                findings=tuple(findings),
            )
        )
    return _SidecarResult(claims=claims, checks=checks)


def _quote_checks(
    graph: _Graph,
    captures: dict[str, bytes],
    hash_failed: frozenset[str],
    quote_claims: list[_QuoteClaim],
) -> list[ContentCheck]:
    checks: list[ContentCheck] = []
    for claim in quote_claims:
        outcome, detail = _quote_outcome(claim, captures, hash_failed)
        checks.append(
            ContentCheck(
                kind=KIND_QUOTE,
                path=json_pointer([*claim.path, "locator", "quote"]),
                outcome=outcome,
                detail=detail,
                warnings=tuple(_redacted_warnings(graph, claim.path, claim.source_ids)),
            )
        )
    return checks


def _quote_outcome(
    claim: _QuoteClaim, captures: dict[str, bytes], hash_failed: frozenset[str]
) -> tuple[str, str]:
    """Three-state quote outcome; a hash-failed capture's bytes are never trusted."""
    usable = [
        captures[source_id]
        for source_id in claim.source_ids
        if source_id in captures and source_id not in hash_failed
    ]
    needle = claim.quote.encode("utf-8")
    if any(needle in payload for payload in usable):
        return VERIFIED, "quote found in capture"
    if all(
        source_id in captures and source_id not in hash_failed for source_id in claim.source_ids
    ):
        return FAILED, "quote absent from fetched captures"
    if not usable:
        if any(source_id in hash_failed for source_id in claim.source_ids):
            return UNVERIFIABLE, _QUOTE_HASH_FAILED_DETAIL
        return UNVERIFIABLE, "no cited source capture fetched"
    if any(source_id in hash_failed for source_id in claim.source_ids):
        # A usable capture lacks the needle and a co-cited capture failed its hash:
        # the bytes were fetched but proven wrong, so this is a hash failure, not a
        # fetch gap.
        return UNVERIFIABLE, _QUOTE_HASH_FAILED_DETAIL
    return UNVERIFIABLE, "some cited source captures were not fetched"


def _local_raw_reference_error(reference: str) -> Unfetchable | None:
    raw_reference = reference.split("#", 1)[0]
    raw_path = raw_reference.split("?", 1)[0]
    if "?" in raw_reference or ";" in raw_path or "\\" in raw_path or ".." in raw_path.split("/"):
        return Unfetchable(f"outside local base: {reference}")
    return None


def _fetch_capture(
    index: int,
    source: dict[str, Any],
    reference: str,
    base_uri: str | None,
    resolver: Resolver,
    local: bool,
) -> _ResolvedCapture | _UnfetchedContent:
    uri = _effective_reference(reference, base_uri, local)
    if isinstance(uri, Unfetchable):
        return _UnfetchedContent(index=index, error=uri)
    try:
        return _ResolvedCapture(index, source, uri, resolver.fetch(uri))
    except Unfetchable as error:
        return _UnfetchedContent(index=index, error=error)


def _fetch_sidecar(
    index: int,
    section: dict[str, Any],
    reference: str,
    base_uri: str | None,
    resolver: Resolver,
    local: bool,
) -> _ResolvedContent | _UnfetchedContent:
    return _fetch_section(index, section, reference, base_uri, resolver, local)


def _sidecar_hash_check(
    index: int, sri: str, resolved: _ResolvedContent | _UnfetchedContent
) -> ContentCheck:
    return _check_sri(KIND_SIDECAR, ["sections", index, "provenance_hash"], sri, resolved)


def _parse_sidecar(payload: bytes) -> tuple[object, str | None]:
    try:
        return json.loads(payload.decode("utf-8")), None
    except UnicodeDecodeError as error:
        return None, str(error)
    except json.JSONDecodeError as error:
        return None, error.msg


def _sidecar_findings(
    graph: _Graph,
    section_index: int,
    section: dict[str, Any],
    sidecar: object,
) -> tuple[list[Finding], bool]:
    findings = _sidecar_schema_findings(section_index, sidecar)
    if not isinstance(sidecar, dict):
        return findings, False
    section_id = sidecar.get("section_id")
    binding_mismatch = False
    if code := reference_code(section_id, "section", graph.source_ids, graph.section_ids):
        findings.append(
            Finding(
                code=code,
                path=json_pointer([*_sidecar_path(section_index), "section_id"]),
                message=f"sidecar section_id {section_id!r} does not resolve to a section",
            )
        )
    elif isinstance(section_id, str) and section_id != section.get("id"):
        binding_mismatch = True
    for claim_index, claim in indexed_dicts(sidecar.get("claims")):
        _append_sidecar_source_findings(findings, graph, section_index, claim_index, claim)
    return sorted(findings), binding_mismatch


def _sidecar_schema_findings(section_index: int, sidecar: object) -> list[Finding]:
    base = json_pointer(_sidecar_path(section_index))
    return [
        Finding(
            code=finding.code,
            path=base + finding.path,
            message=finding.message,
        )
        for finding in schema_findings(sidecar, validator=provenance_validator())
    ]


def _sidecar_detail(section: dict[str, Any], sidecar: object, binding_mismatch: bool) -> str:
    if not binding_mismatch or not isinstance(sidecar, dict):
        return "provenance sidecar checked"
    return (
        "sidecar section_id "
        f"{sidecar.get('section_id')!r} names a different section; "
        f"expected {section.get('id')!r}"
    )


def _append_sidecar_source_findings(
    findings: list[Finding],
    graph: _Graph,
    section_index: int,
    claim_index: int,
    claim: dict[str, Any],
) -> None:
    source_ids = claim.get("source_ids")
    if not isinstance(source_ids, list):
        return
    for source_index, source_id in enumerate(source_ids):
        if code := reference_code(source_id, "source", graph.source_ids, graph.section_ids):
            findings.append(
                Finding(
                    code=code,
                    path=json_pointer(
                        [
                            *_sidecar_path(section_index),
                            "claims",
                            claim_index,
                            "source_ids",
                            source_index,
                        ]
                    ),
                    message=f"sidecar claim source id {source_id!r} does not resolve to a source",
                )
            )


def _inline_quote_claims(graph: _Graph) -> list[_QuoteClaim]:
    claims: list[_QuoteClaim] = []
    for section_index, section in graph.sections:
        for claim_index, claim in indexed_dicts(section.get("provenance")):
            quote = _claim_quote(claim)
            source_ids = _claim_source_ids(claim)
            if quote is not None and source_ids:
                claims.append(
                    _QuoteClaim(
                        path=["sections", section_index, "provenance", claim_index],
                        quote=quote,
                        source_ids=source_ids,
                    )
                )
    return claims


def _sidecar_quote_claims(section_index: int, sidecar: object) -> list[_QuoteClaim]:
    if not isinstance(sidecar, dict):
        return []
    claims: list[_QuoteClaim] = []
    for claim_index, claim in indexed_dicts(sidecar.get("claims")):
        quote = _claim_quote(claim)
        source_ids = _claim_source_ids(claim)
        if quote is not None and source_ids:
            claims.append(
                _QuoteClaim(
                    path=[*_sidecar_path(section_index), "claims", claim_index],
                    quote=quote,
                    source_ids=source_ids,
                )
            )
    return claims


def _claim_quote(claim: dict[str, Any]) -> str | None:
    locator = claim.get("locator")
    quote = locator.get("quote") if isinstance(locator, dict) else None
    # An empty quote is schema-owned (locator.quote has minLength 1 -> AKB011); the
    # content layer skips it rather than let an empty needle match every capture.
    if not isinstance(quote, str) or not quote:
        return None
    return quote


def _claim_source_ids(claim: dict[str, Any]) -> tuple[str, ...]:
    source_ids = claim.get("source_ids")
    if not isinstance(source_ids, list):
        return ()
    return tuple(source_id for source_id in source_ids if isinstance(source_id, str))


def _redacted_warnings(
    graph: _Graph, path: list[str | int], source_ids: tuple[str, ...]
) -> list[Advisory]:
    redacted = sorted(
        source_id
        for source_id in source_ids
        if any(
            source.get("id") == source_id and source.get("type") == "redacted"
            for _, source in graph.sources
        )
    )
    if not redacted:
        return []
    return [
        Advisory(
            path=json_pointer([*path, "locator", "quote"]),
            message=f"quote cites redacted source(s): {', '.join(redacted)}",
        )
    ]


def _fetch_section(
    index: int,
    section: dict[str, Any],
    reference: str,
    base_uri: str | None,
    resolver: Resolver,
    local: bool,
) -> _ResolvedContent | _UnfetchedContent:
    uri = _effective_reference(reference, base_uri, local)
    if isinstance(uri, Unfetchable):
        return _UnfetchedContent(index=index, error=uri)
    try:
        return _ResolvedContent(index=index, section=section, uri=uri, payload=resolver.fetch(uri))
    except Unfetchable as error:
        return _UnfetchedContent(index=index, error=error)


def _compare_sri(kind: str, path: list[str | int], payload: bytes, expected: bytes) -> ContentCheck:
    actual = base64.b64encode(hashlib.sha256(payload).digest()).decode("ascii")
    if actual != base64.b64encode(expected).decode("ascii"):
        return _check(FAILED, kind, path, "sha256 digest mismatch")
    return _check(VERIFIED, kind, path, "sha256 digest matches")


def _parse_sri(kind: str, path: list[str | int], sri: str) -> bytes | ContentCheck:
    algorithm, separator, encoded = sri.partition("-")
    if not separator:
        return _warning_check(kind, path, "malformed SRI: expected '<algorithm>-<base64>'")
    if algorithm != _SHA256:
        return _warning_check(kind, path, f"unsupported hash algorithm: {algorithm}")
    try:
        decoded = base64.b64decode(encoded.encode("ascii"), validate=True)
    except (UnicodeEncodeError, binascii.Error):
        return _warning_check(kind, path, "malformed sha256 digest")
    if len(decoded) != _SHA256_LENGTH:
        return _warning_check(kind, path, "sha256 digest has wrong length")
    if base64.b64encode(decoded).decode("ascii") != encoded:
        return _warning_check(kind, path, "non-canonical sha256 digest")
    return decoded


def _citation_results(
    graph: _Graph, section_index: int, markdown: str
) -> tuple[list[Finding], list[Advisory]]:
    findings: list[Finding] = []
    warnings: list[Advisory] = []
    for marker_index, citation in enumerate(extract_citations(markdown)):
        path: list[str | int] = ["sections", section_index, "content_uri"]
        _append_duplicate_warnings(warnings, path, citation.ids)
        for id_index, source_id in enumerate(citation.ids):
            code = reference_code(source_id, "source", graph.source_ids, graph.section_ids)
            if code is not None:
                # The "/citations/<marker>/<id>" tail is a synthetic locator into the
                # fetched Markdown, not an RFC 6901 pointer that dereferences within the
                # descriptor: content_uri is a string there and citations live in the
                # external content. It locates the offending marker for a reader.
                findings.append(
                    Finding(
                        code=code,
                        path=json_pointer([*path, "citations", marker_index, id_index]),
                        message=f"citation source id {source_id!r} does not resolve to a source",
                    )
                )
    return findings, warnings


def _append_duplicate_warnings(
    warnings: list[Advisory], path: list[str | int], ids: tuple[str, ...]
) -> None:
    duplicates = sorted({source_id for source_id in ids if ids.count(source_id) > 1})
    if duplicates:
        warnings.append(
            Advisory(
                path=json_pointer(path),
                message=f"duplicate citation id in marker: {', '.join(duplicates)}",
            )
        )


def _check(outcome: str, kind: str, path: list[str | int], detail: str) -> ContentCheck:
    return ContentCheck(kind=kind, path=json_pointer(path), outcome=outcome, detail=detail)


def _warning_check(kind: str, path: list[str | int], detail: str) -> ContentCheck:
    pointer = json_pointer(path)
    return ContentCheck(
        kind=kind,
        path=pointer,
        outcome=UNVERIFIABLE,
        detail=detail,
        warnings=(Advisory(path=pointer, message=detail),),
    )


def _content_path(index: int) -> list[str | int]:
    return ["sections", index, "content_uri"]


def _sidecar_path(index: int) -> list[str | int]:
    return ["sections", index, "provenance_uri"]


def _is_markdown(content_type: object) -> bool:
    if not isinstance(content_type, str):
        return False
    # A media type is `type/subtype` plus optional `; parameter=value` parts; the type
    # and subtype are case-insensitive. Strip parameters and casefold so
    # `text/Markdown` and `text/markdown; charset=utf-8` both count as Markdown.
    essence = content_type.split(";", 1)[0].strip().casefold()
    return essence == _MARKDOWN_TYPE


def _ids(items: Iterable[tuple[int, dict[str, Any]]]) -> list[str]:
    return [item["id"] for _, item in items if isinstance(item.get("id"), str)]
