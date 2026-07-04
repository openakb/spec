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

from markdown_it import MarkdownIt

__all__ = ["Citation", "extract_citations"]

_MARKER_RE = re.compile(r"\[cite:[ \t]*([a-z0-9_-]{1,64}(?:[ \t]*,[ \t]*[a-z0-9_-]{1,64})*)\]")
_SEPARATOR_RE = re.compile(r"[ \t]*,[ \t]*")


@dataclass(frozen=True)
class Citation:
    """One recognized marker: its source ids in written order."""

    ids: tuple[str, ...]


def extract_citations(markdown: str) -> list[Citation]:
    """Extract one Citation per recognized marker, in document order (spec §4.4)."""
    return [
        Citation(ids=tuple(_SEPARATOR_RE.split(match.group(1))))
        for segment in _prose_segments(markdown)
        for match in _MARKER_RE.finditer(segment)
    ]


@cache
def _parser() -> MarkdownIt:
    return MarkdownIt("commonmark")


def _prose_segments(markdown: str) -> list[str]:
    """Contiguous plain-text runs of prose, in document order.

    Only `inline` tokens hold prose (code blocks and HTML blocks never produce them);
    within one, any non-`text` child -- a code span, inline HTML or comment, emphasis
    delimiter, or line break -- terminates the current run, so a marker can never be
    assembled across an ignored or structural construct.
    """
    segments: list[str] = []
    for token in _parser().parse(markdown):
        if token.type != "inline":
            continue
        run: list[str] = []
        for child in token.children or []:
            if child.type == "text":
                run.append(child.content)
            elif run:
                segments.append("".join(run))
                run = []
        if run:
            segments.append("".join(run))
    return segments
