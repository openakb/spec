"""Opt-in content checks that require fetching descriptor-related resources."""

from __future__ import annotations

import base64
import binascii
import hashlib
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Protocol
from urllib.parse import urljoin, urlparse

from ._shape import indexed_dicts, reference_code
from .citations import extract_citations
from .result import Advisory, Finding, json_pointer

__all__ = [
    "FAILED",
    "UNVERIFIABLE",
    "VERIFIED",
    "ContentCheck",
    "ContentReport",
    "LocalFileResolver",
    "Resolver",
    "Unfetchable",
    "check_content",
]

VERIFIED = "verified"
FAILED = "failed"
UNVERIFIABLE = "unverifiable"
_MARKDOWN_TYPE = "text/markdown"
_SHA256 = "sha256"
_SHA256_LENGTH = 32


class Unfetchable(Exception):
    """Raised by resolvers when a resource cannot be fetched."""


class Resolver(Protocol):
    """Injected content resolver; implementations decide what URI schemes exist."""

    def fetch(self, uri: str) -> bytes:
        """Return bytes for uri or raise Unfetchable."""


@dataclass(frozen=True)
class LocalFileResolver:
    """Resolve scheme-less relative paths under one local base directory.

    URI fragments are client-side identifiers and do not affect the fetched bytes.
    Queries and path parameters are rejected because they would otherwise alias to
    the same local path.
    """

    base_dir: Path

    def fetch(self, uri: str) -> bytes:
        path = self._local_path(uri)
        try:
            return path.read_bytes()
        except OSError as error:
            raise Unfetchable(str(error)) from error

    def _local_path(self, uri: str) -> Path:
        parsed = urlparse(uri)
        if (
            parsed.scheme
            or parsed.netloc
            or parsed.params
            or parsed.query
            or ";" in parsed.path
            or ".." in parsed.path.split("/")
            or Path(parsed.path).is_absolute()
        ):
            raise Unfetchable(f"outside local base: {uri}")
        base = self.base_dir.resolve()
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
    def failed(self) -> tuple[ContentCheck, ...]:
        return tuple(check for check in self.checks if check.outcome == FAILED)

    @property
    def ok(self) -> bool:
        return not self.failed and not self.findings


def check_content(descriptor: object, resolver: Resolver) -> ContentReport:
    """Run opt-in content checks against fetched descriptor resources."""
    if not isinstance(descriptor, dict):
        return ContentReport(checks=())
    graph = _Graph.from_descriptor(descriptor)
    base_uri = descriptor.get("base_uri") if isinstance(descriptor.get("base_uri"), str) else None
    checks: list[ContentCheck] = []
    checks.extend(_guide_check(descriptor, base_uri, resolver))
    checks.extend(_section_checks(graph, base_uri, resolver))
    _capture_checks(descriptor, resolver)
    _sidecar_checks(descriptor, resolver)
    _quote_checks(descriptor, resolver)
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


def _section_checks(graph: _Graph, base_uri: str | None, resolver: Resolver) -> list[ContentCheck]:
    checks: list[ContentCheck] = []
    for index, section in graph.sections:
        reference = section.get("content_uri")
        if not isinstance(reference, str):
            continue
        resolved = _fetch_section(index, section, reference, base_uri, resolver)
        if isinstance(section.get("content_hash"), str):
            checks.append(_check_sri("content-hash", ["sections", index, "content_hash"], resolved))
        if _is_markdown(section.get("content_type")):
            checks.append(_citation_check(graph, resolved))
    return checks


def _guide_check(
    descriptor: dict[str, Any], base_uri: str | None, resolver: Resolver
) -> list[ContentCheck]:
    if not isinstance(descriptor.get("guide_uri"), str) or not isinstance(
        descriptor.get("guide_hash"), str
    ):
        return []
    uri = _effective_reference(descriptor["guide_uri"], base_uri)
    path: list[str | int] = ["guide_hash"]
    try:
        payload = resolver.fetch(uri)
    except Unfetchable as error:
        return [_check(UNVERIFIABLE, "guide-hash", path, str(error))]
    return [_check_sri_bytes("guide-hash", path, payload, descriptor["guide_hash"])]


def _check_sri(
    kind: str, path: list[str | int], resolved: _ResolvedContent | _UnfetchedContent
) -> ContentCheck:
    if isinstance(resolved, _UnfetchedContent):
        return _check(UNVERIFIABLE, kind, path, str(resolved.error))
    content_hash = resolved.section.get("content_hash")
    if not isinstance(content_hash, str):
        return _check(UNVERIFIABLE, kind, path, "content_hash is not a string")
    return _check_sri_bytes(kind, path, resolved.payload, content_hash)


def _citation_check(graph: _Graph, resolved: _ResolvedContent | _UnfetchedContent) -> ContentCheck:
    if isinstance(resolved, _UnfetchedContent):
        return _check(
            UNVERIFIABLE,
            "citations",
            _content_path(resolved.index),
            str(resolved.error),
        )
    try:
        markdown = resolved.payload.decode("utf-8")
    except UnicodeDecodeError as error:
        return _check(UNVERIFIABLE, "citations", _content_path(resolved.index), str(error))
    findings, warnings = _citation_results(graph, resolved.index, markdown)
    return ContentCheck(
        kind="citations",
        path=json_pointer(_content_path(resolved.index)),
        outcome=FAILED if findings else VERIFIED,
        detail="citation markers checked",
        findings=tuple(findings),
        warnings=tuple(warnings),
    )


def _effective_reference(reference: str, base_uri: str | None) -> str:
    if base_uri is None:
        return reference
    return urljoin(base_uri, reference)


def _capture_checks(_descriptor: dict[str, Any], _resolver: Resolver) -> dict[str, Any]:
    return {}


def _sidecar_checks(_descriptor: dict[str, Any], _resolver: Resolver) -> None:
    return None


def _quote_checks(_descriptor: dict[str, Any], _resolver: Resolver) -> None:
    return None


def _fetch_section(
    index: int,
    section: dict[str, Any],
    reference: str,
    base_uri: str | None,
    resolver: Resolver,
) -> _ResolvedContent | _UnfetchedContent:
    uri = _effective_reference(reference, base_uri)
    try:
        return _ResolvedContent(index=index, section=section, uri=uri, payload=resolver.fetch(uri))
    except Unfetchable as error:
        return _UnfetchedContent(index=index, error=error)


def _check_sri_bytes(kind: str, path: list[str | int], payload: bytes, sri: str) -> ContentCheck:
    expected = _parse_sri(kind, path, sri)
    if isinstance(expected, ContentCheck):
        return expected
    actual = base64.b64encode(hashlib.sha256(payload).digest()).decode("ascii")
    if actual != base64.b64encode(expected).decode("ascii"):
        return _check(FAILED, kind, path, "sha256 digest mismatch")
    return _check(VERIFIED, kind, path, "sha256 digest matches")


def _parse_sri(kind: str, path: list[str | int], sri: str) -> bytes | ContentCheck:
    algorithm, separator, encoded = sri.partition("-")
    if not separator or algorithm != _SHA256:
        detail = f"unsupported hash algorithm: {algorithm}"
        return _warning_check(kind, path, detail)
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


def _is_markdown(content_type: object) -> bool:
    return not isinstance(content_type, str) or content_type == _MARKDOWN_TYPE


def _ids(items: Iterable[tuple[int, dict[str, Any]]]) -> list[str]:
    return [item["id"] for _, item in items if isinstance(item.get("id"), str)]
