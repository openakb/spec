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
# nor bridge one; masking preserves length, keeping token line maps aligned. It shares
# _NULL's codepoint deliberately: normalization already replaced every source NUL with
# U+FFFD, so the sentinel can never collide with surviving input.
_MASK = "\x00"
_BACKTICK = "`"
_COMMENT_OPEN = "<!--"
_COMMENT_CLOSE = "-->"
# CommonMark's two degenerate comments; the general form closes on the first `-->`.
_COMMENT_EMPTY = "<!-->"
_COMMENT_DASH = "<!--->"

# Link, image, and autolink syntax is not one of the five suppressing constructs, so a
# code-span or comment *shape* inside a destination, title, or autolink is link syntax,
# not a construct, and must not be masked. Backslash also has no marker-level meaning
# (spec §4.4), but at the CommonMark masking layer an escaped `` ` `` opens no code span
# and an escaped `<` opens no comment, so the inline scan skips a backslash-escaped byte.
_BACKSLASH = "\\"
_OPEN_BRACKET = "["
_CLOSE_BRACKET = "]"
_OPEN_PAREN = "("
_CLOSE_PAREN = ")"
_ANGLE_OPEN = "<"
_ANGLE_CLOSE = ">"
_TITLE_QUOTES = frozenset("\"'")
# Inter-token space inside a link tail; a single inline run never spans a blank line.
_LINK_SPACE = frozenset(" \t\n")
# Largest ASCII control codepoint plus the DEL codepoint bound a bare destination, which
# admits neither space nor control characters (CommonMark link-destination grammar).
_LOW_CONTROL_MAX = 0x1F
_DEL = 0x7F
# CommonMark autolinks: an absolute URI (scheme, then no space/`<`/`>`/control) or an
# email address, wrapped in angle brackets. Their content is destination syntax, so a
# marker or construct shape inside is never masked.
_AUTOLINK_URI = r"[A-Za-z][A-Za-z0-9+.\-]{1,31}:[^\s<>\x00-\x1f\x7f]*"
_AUTOLINK_EMAIL = (
    r"[A-Za-z0-9.!#$%&'*+/=?^_`{|}~\-]+@"
    r"[A-Za-z0-9](?:[A-Za-z0-9\-]{0,61}[A-Za-z0-9])?"
    r"(?:\.[A-Za-z0-9](?:[A-Za-z0-9\-]{0,61}[A-Za-z0-9])?)*"
)
_AUTOLINK_RE = re.compile(rf"<(?:{_AUTOLINK_URI}|{_AUTOLINK_EMAIL})>")

# Inline raw HTML (canonical parser's tag grammar): an open/close tag, processing
# instruction, or declaration is opaque like an autolink -- a backtick inside is not a
# code span -- but a whole `<!--...-->` comment inside its span is still masked, mirroring
# how the canonical parser reports raw inline HTML and then removes any comment it covers.
# A CDATA section is deliberately absent: the canonical extractor does not treat inline
# `<![CDATA[...]]>` as raw HTML, so its bytes stay ordinary source here too.
_HTML_SPACE = r"[ \t\n]"
_HTML_TAG_NAME = r"[A-Za-z][A-Za-z0-9-]*"
_HTML_ATTR_NAME = r"[A-Za-z_:][A-Za-z0-9_.:-]*"
_HTML_ATTR_VALUE = r"(?:[^ \t\n\"'=<>`]+|'[^']*'|\"[^\"]*\")"
_HTML_ATTR_SPEC = rf"(?:{_HTML_SPACE}*={_HTML_SPACE}*{_HTML_ATTR_VALUE})"
_HTML_ATTR = rf"(?:{_HTML_SPACE}+{_HTML_ATTR_NAME}{_HTML_ATTR_SPEC}?)"
_HTML_OPEN = rf"<{_HTML_TAG_NAME}{_HTML_ATTR}*{_HTML_SPACE}*/?>"
_HTML_CLOSE = rf"</{_HTML_TAG_NAME}{_HTML_SPACE}*>"
_HTML_PI = r"<\?[\s\S]*?\?>"
_HTML_DECL = r"<![A-Za-z][^>]*>"
_HTML_TAG_RE = re.compile(rf"(?:{_HTML_OPEN}|{_HTML_CLOSE}|{_HTML_PI}|{_HTML_DECL})")


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
    """Mask inline code spans and HTML comments within one inline run's source region.

    The scan is escape- and link-aware so it masks exactly what a CommonMark parser would:
    a backslash-escaped `` ` `` or `<` opens no construct, and a link, image, or autolink
    destination/title is skipped whole -- its bytes are link syntax, so a code-span or
    comment shape inside them is never a suppressing construct (spec §4.4).
    """
    openers: list[int] = []
    index = start
    while index < end:
        index = _mask_inline_step(chars, source, index, end, openers)


def _mask_inline_step(
    chars: list[str], source: str, index: int, end: int, openers: list[int]
) -> int:
    """Advance one inline token from index, masking or skipping it; return the next index."""
    char = source[index]
    if char == _BACKSLASH:
        return index + 2  # an escaped byte opens no code span or comment (masking layer)
    if char == _BACKTICK:
        return _mask_code_span(chars, source, index, end)
    if source.startswith(_COMMENT_OPEN, index, end):
        return _mask_comment(chars, source, index, end)
    if char == _ANGLE_OPEN:
        return _skip_angle_construct(chars, source, index, end)
    if char == _OPEN_BRACKET:
        openers.append(index)
        return index + 1
    if char == _CLOSE_BRACKET:
        return _close_bracket(source, index, end, openers)
    return index + 1


def _close_bracket(source: str, index: int, end: int, openers: list[int]) -> int:
    """Resolve a `]`: skip a link/image tail when one opened, else advance one char."""
    if not openers:
        return index + 1
    openers.pop()
    tail_end = _link_tail_end(source, index + 1, end)
    return tail_end if tail_end is not None else index + 1


def _link_tail_end(source: str, pos: int, end: int) -> int | None:
    """End offset of an inline `](dest title)` tail opening at pos, else None.

    Reference, collapsed, and shortcut tails carry no destination or title to skip -- any
    label or definition sits elsewhere in the run and is scanned as ordinary source -- so
    only the inline form needs a skip here.
    """
    if pos < end and source[pos] == _OPEN_PAREN:
        return _inline_link_end(source, pos, end)
    return None


def _inline_link_end(source: str, index: int, end: int) -> int | None:
    """End offset just past the `)` of an inline link tail at `(`, else None."""
    index = _skip_link_space(source, index + 1, end)
    dest_end = _skip_destination(source, index, end)
    if dest_end is None:
        return None
    index = _skip_link_space(source, dest_end, end)
    title_end = _skip_title(source, index, end)
    if title_end is None:
        return None
    index = _skip_link_space(source, title_end, end)
    if index < end and source[index] == _CLOSE_PAREN:
        return index + 1
    return None


def _skip_destination(source: str, index: int, end: int) -> int | None:
    """End offset of a link destination (angle or bare) at index, else None."""
    if index < end and source[index] == _ANGLE_OPEN:
        return _skip_angle_dest(source, index, end)
    return _skip_bare_dest(source, index, end)


def _skip_angle_dest(source: str, index: int, end: int) -> int | None:
    """End offset just past the `>` of a `<...>` destination, else None."""
    index += 1
    while index < end:
        char = source[index]
        if char == _BACKSLASH and index + 1 < end:
            index += 2
            continue
        if char in ("\n", _ANGLE_OPEN):
            return None
        if char == _ANGLE_CLOSE:
            return index + 1
        index += 1
    return None


def _skip_bare_dest(source: str, index: int, end: int) -> int | None:
    """End offset of a bare destination: non-space, non-control run, balanced parens.

    An unmatched `(` (parens still open where the run ends) is not a valid destination,
    so the tail is rejected and its bytes stay ordinary source (CommonMark link grammar).
    """
    depth = 0
    while index < end:
        char = source[index]
        if char == _BACKSLASH and index + 1 < end:
            index += 2
            continue
        if char == " " or ord(char) <= _LOW_CONTROL_MAX or ord(char) == _DEL:
            break
        if char == _OPEN_PAREN:
            depth += 1
        elif char == _CLOSE_PAREN:
            if depth == 0:
                break
            depth -= 1
        index += 1
    return None if depth else index


def _skip_title(source: str, index: int, end: int) -> int | None:
    """End offset of an optional link title; index unchanged when absent, None if malformed."""
    if index >= end:
        return index
    char = source[index]
    if char in _TITLE_QUOTES:
        return _skip_delimited(source, index, end, char)
    if char == _OPEN_PAREN:
        return _skip_paren_title(source, index, end)
    return index


def _skip_delimited(source: str, index: int, end: int, quote: str) -> int | None:
    """End offset just past the closing quote of a `"..."` or `'...'` title, else None."""
    index += 1
    while index < end:
        char = source[index]
        if char == _BACKSLASH and index + 1 < end:
            index += 2
            continue
        if char == quote:
            return index + 1
        index += 1
    return None


def _skip_paren_title(source: str, index: int, end: int) -> int | None:
    """End offset just past the `)` of a `(...)` title, else None; nesting is disallowed."""
    index += 1
    while index < end:
        char = source[index]
        if char == _BACKSLASH and index + 1 < end:
            index += 2
            continue
        if char == _OPEN_PAREN:
            return None
        if char == _CLOSE_PAREN:
            return index + 1
        index += 1
    return None


def _skip_link_space(source: str, index: int, end: int) -> int:
    """Advance past inter-token link whitespace starting at index."""
    while index < end and source[index] in _LINK_SPACE:
        index += 1
    return index


def _skip_angle_construct(chars: list[str], source: str, index: int, end: int) -> int:
    """Skip a `<...>` autolink or raw inline HTML span, else advance past the `<`.

    An autolink cannot enclose a comment. Raw inline HTML can (a `<!--...-->` may fall
    inside a declaration or a quoted attribute), so its span is scanned for comments and
    any it fully encloses is masked, matching the canonical parser; its other bytes are
    link/HTML syntax, so an enclosed marker or backtick is left as ordinary source.
    """
    autolink = _AUTOLINK_RE.match(source, index, end)
    if autolink is not None:
        return autolink.end()
    html = _HTML_TAG_RE.match(source, index, end)
    if html is None:
        return index + 1
    _mask_comments_in_range(chars, source, index, html.end())
    return html.end()


def _mask_comments_in_range(chars: list[str], source: str, start: int, end: int) -> None:
    """Mask every complete `<!--...-->` comment fully inside [start, end)."""
    index = start
    while index < end:
        if not source.startswith(_COMMENT_OPEN, index, end):
            index += 1
            continue
        comment_end = _comment_end(source, index, end)
        if comment_end is None:
            return
        _mask_range(chars, index, comment_end)
        index = comment_end


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
    """Mask a CommonMark HTML comment; leave a non-comment `<!--` run literal.

    A comment is `<!-->`, `<!--->`, or `<!--` followed by any text and closed at the
    first `-->` (CommonMark 0.31.2 -- a `--` in the body is allowed, unlike the older
    0.29 rule). An enclosed `[cite:]` is therefore suppressed, matching how the block
    path masks such comments. When no `-->` closes the run it is not a comment (an
    enclosed marker stays live); advance past the opening `<` only.
    """
    comment_end = _comment_end(source, open_start, end)
    if comment_end is None:
        return open_start + 1
    _mask_range(chars, open_start, comment_end)
    return comment_end


def _comment_end(source: str, open_start: int, end: int) -> int | None:
    """End offset of the CommonMark comment opening at open_start, else None.

    The two degenerate forms `<!-->` / `<!--->` are complete; otherwise the comment
    closes at the first `-->` (CommonMark 0.31.2), so a `--` in the body is allowed.
    """
    for degenerate in (_COMMENT_EMPTY, _COMMENT_DASH):
        if source.startswith(degenerate, open_start, end):
            return open_start + len(degenerate)
    body = open_start + len(_COMMENT_OPEN)
    close = source.find(_COMMENT_CLOSE, body, end)
    if close == -1:
        return None
    return close + len(_COMMENT_CLOSE)


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
