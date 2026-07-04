"""Inline [cite:] extraction per the normative grammar (spec §4.4).

The marker grammar is matched against the raw Markdown source. Five CommonMark
constructs are masked out first, so a marker they contain is never recognized: fenced
code blocks, indented code blocks, inline code spans (any backtick run), HTML blocks,
and HTML comments. Nothing else suppresses a marker -- there is no backslash escape, no
character-entity decoding, and no bracket-adjacency rule. Extraction reports one entry
per recognized marker, in document order, with the marker's ids in written order
(duplicates preserved as written).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from functools import cache

from markdown_it import MarkdownIt

from .catalog import LOCAL_ID_CHARSET, LOCAL_ID_MAX_LENGTH

__all__ = ["Citation", "extract_citations"]

_ID = rf"[{LOCAL_ID_CHARSET}]{{1,{LOCAL_ID_MAX_LENGTH}}}"
_MARKER_RE = re.compile(rf"\[cite:[ \t]*({_ID}(?:[ \t]*,[ \t]*{_ID})*)\]")
_SEPARATOR_RE = re.compile(r"[ \t]*,[ \t]*")

# CommonMark normalizes the source before assigning block/inline line numbers; mirror it
# so token line maps line up with the string the marker regex scans (spec §4.4).
_NEWLINES_RE = re.compile(r"\r\n?|\n")
_NULL = "\x00"
_REPLACEMENT = "\ufffd"  # CommonMark maps NUL to U+FFFD before line numbering

# Block tokens whose whole source line span is masked out of the prose.
_MASKED_BLOCKS = frozenset({"fence", "code_block", "html_block"})
# A sentinel in no part of the marker grammar, so masked text can neither form a marker
# nor bridge one; masking preserves length, keeping token line maps aligned.
_MASK = "\x00"
_BACKTICK = "`"
_COMMENT_OPEN = "<!--"
_COMMENT_CLOSE = "-->"


@dataclass(frozen=True)
class Citation:
    """One recognized marker: its source ids in written order."""

    ids: tuple[str, ...]


def extract_citations(markdown: str) -> list[Citation]:
    """Extract one Citation per recognized marker, in document order (spec §4.4)."""
    masked = _mask_non_prose(markdown)
    return [
        Citation(ids=tuple(_SEPARATOR_RE.split(match.group(1))))
        for match in _MARKER_RE.finditer(masked)
    ]


@cache
def _parser() -> MarkdownIt:
    return MarkdownIt("commonmark")


def _mask_non_prose(markdown: str) -> str:
    """Return the source with every marker-suppressing construct blanked to sentinels.

    Block code and HTML are masked by their token line ranges; inline code spans and
    HTML comments are masked within each inline run's own source region, so a run can
    never pair backticks -- or open a comment -- across a block boundary.
    """
    source = _NEWLINES_RE.sub("\n", markdown).replace(_NULL, _REPLACEMENT)
    offsets = _line_offsets(source)
    chars = list(source)
    for token in _parser().parse(source):
        if token.map is None:
            continue
        start, end = offsets[token.map[0]], offsets[token.map[1]]
        if token.type in _MASKED_BLOCKS:
            _mask_range(chars, start, end)
        elif token.type == "inline":
            _mask_inline(chars, source, start, end)
    return "".join(chars)


def _line_offsets(source: str) -> list[int]:
    """Character offset of each line start, plus a final offset at end of source."""
    offsets = [0]
    offsets.extend(index + 1 for index, char in enumerate(source) if char == "\n")
    offsets.append(len(source))
    return offsets


def _mask_range(chars: list[str], start: int, end: int) -> None:
    """Blank chars[start:end] to the mask sentinel, preserving length."""
    for index in range(start, end):
        chars[index] = _MASK


def _mask_inline(chars: list[str], source: str, start: int, end: int) -> None:
    """Mask inline code spans and HTML comments within one inline run's source region."""
    index = start
    while index < end:
        if source[index] == _BACKTICK:
            index = _mask_code_span(chars, source, index, end)
        elif source.startswith(_COMMENT_OPEN, index, end):
            index = _mask_comment(chars, source, index, end)
        else:
            index += 1


def _mask_code_span(chars: list[str], source: str, open_start: int, end: int) -> int:
    """Mask a backtick code span; when unterminated, skip only its opening run."""
    run = _backtick_run(source, open_start, end)
    close_start = _find_backtick_run(source, open_start + run, end, run)
    if close_start is None:
        return open_start + run
    close_end = close_start + run
    _mask_range(chars, open_start, close_end)
    return close_end


def _mask_comment(chars: list[str], source: str, open_start: int, end: int) -> int:
    """Mask an HTML comment; when unterminated, skip only its opening `<`."""
    close = source.find(_COMMENT_CLOSE, open_start + len(_COMMENT_OPEN), end)
    if close == -1:
        return open_start + 1
    comment_end = close + len(_COMMENT_CLOSE)
    _mask_range(chars, open_start, comment_end)
    return comment_end


def _backtick_run(source: str, index: int, end: int) -> int:
    """Length of the maximal backtick run starting at index, bounded by end."""
    run = 0
    while index + run < end and source[index + run] == _BACKTICK:
        run += 1
    return run


def _find_backtick_run(source: str, index: int, end: int, run: int) -> int | None:
    """Start of the next maximal backtick run of exactly `run` backticks, or None."""
    while index < end:
        if source[index] != _BACKTICK:
            index += 1
            continue
        length = _backtick_run(source, index, end)
        if length == run:
            return index
        index += length
    return None
