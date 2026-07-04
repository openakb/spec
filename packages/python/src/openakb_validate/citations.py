"""Inline [cite:] extraction per the normative grammar (spec §4.4).

Markdown structure is interpreted per CommonMark via markdown-it-py. Markers inside
fenced code blocks, indented code blocks, inline code spans, HTML blocks, and HTML
comments are literal text; bracketed text that does not match the grammar exactly is
literal text, never an error. Extraction reports one entry per recognized marker, in
document order, ids in written order (duplicates preserved as written).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from functools import cache
from html import unescape

from markdown_it import MarkdownIt

__all__ = ["Citation", "extract_citations"]

_MARKER_RE = re.compile(r"\[cite:[ \t]*([a-z0-9_-]{1,64}(?:[ \t]*,[ \t]*[a-z0-9_-]{1,64})*)\]")
_SEPARATOR_RE = re.compile(r"[ \t]*,[ \t]*")
_CHARACTER_REFERENCE_RE = re.compile(r"&(?:#[0-9]+|#[xX][0-9a-fA-F]+|[A-Za-z][A-Za-z0-9]+);")
_ESCAPABLE = frozenset(r'!"#$%&\'()*+,-./:;<=>?@[\]^_`{|}~')


@dataclass(frozen=True)
class Citation:
    """One recognized marker: its source ids in written order."""

    ids: tuple[str, ...]


def extract_citations(markdown: str) -> list[Citation]:
    """Extract one Citation per recognized marker, in document order (spec §4.4)."""
    return [
        Citation(ids=tuple(_SEPARATOR_RE.split(match.group(1))))
        for segment, literal in _prose_segments(markdown)
        for match in _MARKER_RE.finditer(segment)
        if _has_literal_source(match, literal) and _has_marker_boundaries(match, segment)
    ]


@cache
def _parser() -> MarkdownIt:
    return MarkdownIt("commonmark")


def _prose_segments(markdown: str) -> list[tuple[str, tuple[bool, ...]]]:
    """Contiguous plain-text runs of prose, in document order.

    Only `inline` tokens hold prose (code blocks and HTML blocks never produce them);
    within one, any non-`text` child -- a code span, inline HTML or comment, emphasis
    delimiter, or line break -- terminates the current run, so a marker can never be
    assembled across an ignored or structural construct.
    """
    segments: list[tuple[str, tuple[bool, ...]]] = []
    for token in _parser().parse(markdown):
        if token.type != "inline":
            continue
        run: list[str] = []
        literal_run: list[bool] = []
        cursor = 0
        for child in token.children or []:
            if child.type == "text":
                cursor, text, literal = _consume_text(token.content, cursor, child.content)
                run.append(text)
                literal_run.extend(literal)
            elif run:
                segments.append(("".join(run), tuple(literal_run)))
                run = []
                literal_run = []
        if run:
            segments.append(("".join(run), tuple(literal_run)))
    return segments


def _has_literal_source(match: re.Match[str], literal: tuple[bool, ...]) -> bool:
    return all(literal[match.start() : match.end()])


def _has_marker_boundaries(match: re.Match[str], segment: str) -> bool:
    start = match.start()
    close = match.end() - 1
    return (start == 0 or segment[start - 1] != "[") and (
        close + 1 == len(segment) or segment[close + 1] != "]"
    )


def _consume_text(source: str, cursor: int, expected: str) -> tuple[int, str, tuple[bool, ...]]:
    text: list[str] = []
    literal: list[bool] = []
    cursor = _find_decoded(source, cursor, expected)
    while len(text) < len(expected) and cursor < len(source):
        character_reference = _CHARACTER_REFERENCE_RE.match(source, cursor)
        if character_reference is not None:
            decoded = unescape(character_reference.group(0))
            if decoded != character_reference.group(0):
                text.extend(decoded)
                literal.extend([False] * len(decoded))
                cursor = character_reference.end()
                continue
        if source[cursor] == "\\" and cursor + 1 < len(source) and source[cursor + 1] in _ESCAPABLE:
            text.append(source[cursor + 1])
            literal.append(False)
            cursor += 2
            continue
        text.append(source[cursor])
        literal.append(True)
        cursor += 1
    decoded = "".join(text)
    if decoded != expected:
        return cursor, expected, (False,) * len(expected)
    return cursor, decoded, tuple(literal)


def _find_decoded(source: str, cursor: int, expected: str) -> int:
    while cursor < len(source):
        _, decoded, _ = _decode_text_prefix(source, cursor, len(expected))
        if decoded == expected:
            return cursor
        cursor += 1
    return cursor


def _decode_text_prefix(source: str, cursor: int, length: int) -> tuple[int, str, tuple[bool, ...]]:
    text: list[str] = []
    literal: list[bool] = []
    while len(text) < length and cursor < len(source):
        character_reference = _CHARACTER_REFERENCE_RE.match(source, cursor)
        if character_reference is not None:
            decoded = unescape(character_reference.group(0))
            if decoded != character_reference.group(0):
                text.extend(decoded)
                literal.extend([False] * len(decoded))
                cursor = character_reference.end()
                continue
        if source[cursor] == "\\" and cursor + 1 < len(source) and source[cursor + 1] in _ESCAPABLE:
            text.append(source[cursor + 1])
            literal.append(False)
            cursor += 2
            continue
        text.append(source[cursor])
        literal.append(True)
        cursor += 1
    return cursor, "".join(text), tuple(literal)
