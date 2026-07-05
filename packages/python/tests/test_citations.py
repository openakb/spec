"""The normative [cite:] extraction grammar (spec §4.4)."""

from dataclasses import FrozenInstanceError
from typing import Any, cast

import pytest

from openakb_validate import Citation, extract_citations

__all__ = ()


def _ids(markdown: str) -> list[list[str]]:
    return [list(citation.ids) for citation in extract_citations(markdown)]


def test_simple_marker() -> None:
    """A single well-formed marker yields one citation."""
    assert _ids("See [cite: alpha_1].") == [["alpha_1"]]


def test_comma_whitespace_variants() -> None:
    """Comma-separated ids allow horizontal whitespace around separators."""
    assert _ids("[cite:a,b]\n[cite: a ,b]\n[cite:\ta\t,\tb-2]") == [
        ["a", "b"],
        ["a", "b"],
        ["a", "b-2"],
    ]


def test_concatenated_stay_separate() -> None:
    """Adjacent markers are reported as separate entries."""
    assert _ids("[cite: a][cite: b]") == [["a"], ["b"]]


def test_document_order_blocks() -> None:
    """Markers preserve document order across prose blocks."""
    assert _ids("First [cite: a].\n\nSecond [cite: b].\n\n> Third [cite: c].") == [
        ["a"],
        ["b"],
        ["c"],
    ]


def test_duplicate_ids_preserved() -> None:
    """Repeated ids remain in written order."""
    assert _ids("[cite: a, b, a]") == [["a", "b", "a"]]


def test_fenced_code_ignored() -> None:
    """Backtick fenced code is not prose."""
    assert _ids("```text\n[cite: a]\n```\n\n[cite: b]") == [["b"]]


def test_tilde_fence_ignored() -> None:
    """Tilde fenced code is not prose."""
    assert _ids("~~~text\n[cite: a]\n~~~\n\n[cite: b]") == [["b"]]


def test_indented_code_ignored() -> None:
    """Indented code blocks are not prose."""
    assert _ids("    [cite: a]\n\n[cite: b]") == [["b"]]


def test_inline_code_ignored() -> None:
    """Inline code spans are masked out of the source."""
    assert _ids("Literal `[cite: a]` and prose [cite: b].") == [["b"]]


def test_unclosed_backtick_is_prose() -> None:
    """An unterminated backtick run is literal source, not a code span."""
    assert _ids("Text `x [cite: c] more") == [["c"]]


def test_backtick_run_length_matters() -> None:
    """A code span closes only on a backtick run of equal length."""
    assert _ids("A `x``y` [cite: c]") == [["c"]]


def test_double_backtick_span_masked() -> None:
    """A multi-backtick code span suppresses an enclosed marker."""
    assert _ids("``[cite: a]`` then [cite: b]") == [["b"]]


def test_html_block_ignored() -> None:
    """HTML blocks are not prose."""
    assert _ids("<section>\n[cite: a]\n</section>\n\n[cite: b]") == [["b"]]


def test_html_comment_ignored() -> None:
    """HTML comments as block constructs are not prose."""
    assert _ids("<!-- [cite: a] -->\n\n[cite: b]") == [["b"]]


def test_inline_comment_ignored() -> None:
    """Inline HTML comments are masked out of the source."""
    assert _ids("See <!-- [cite: a] --> [cite: b].") == [["b"]]


def test_unclosed_comment_is_prose() -> None:
    """An unterminated HTML comment is literal source, not a masked construct."""
    assert _ids("See <!-- open [cite: c]") == [["c"]]


def test_inline_comment_dashes_masked() -> None:
    """A `--` in the body is allowed (CommonMark 0.31.2): the comment closes at its
    first `-->`, so an enclosed marker is masked -- in a paragraph and in inline block
    containers (blockquote, list item) alike."""
    prose = "x <!-- todo -- note [cite: a] --> y"
    assert _ids(prose) == []
    assert _ids(f"> {prose}") == []
    assert _ids(f"- {prose}") == []


def test_inline_comment_wellformed_masked() -> None:
    """A valid inline comment (no `--` in its body) still masks an enclosed marker."""
    assert _ids("x <!-- [cite: a] --> y") == []


def test_degenerate_comments_masked() -> None:
    """The empty forms <!--> and <!---> are complete comments, so a following marker
    is prose and the trailing --> is never mis-paired to swallow it."""
    assert _ids("x <!-->[cite: a]--> y") == [["a"]]
    assert _ids("x <!--->[cite: a]--> y") == [["a"]]


def test_comment_block_versus_inline() -> None:
    """An unterminated <!-- opens an HTML block at line start (marker suppressed by the
    HTML-block rule) yet stays literal text inline (marker kept): §4.4's two comment
    suppressors resolve one source two ways by position, not by a `--` in the body."""
    unterminated = "<!-- open [cite: a]"
    assert _ids(unterminated) == []
    assert _ids(f"x {unterminated}") == [["a"]]


def test_after_links_prose() -> None:
    """Citations after links, images, and autolinks remain normal prose."""
    assert _ids("[foo](https://example.org) [cite: a]") == [["a"]]
    assert _ids("![alt](https://example.org/image.png) [cite: b]") == [["b"]]
    assert _ids("<https://example.org> [cite: c]") == [["c"]]


def test_escaped_bracket_is_marker() -> None:
    """There is no escape syntax in v1: a leading backslash is literal source."""
    assert _ids(r"\[cite: a]") == [["a"]]


def test_stray_close_bracket_marker() -> None:
    """A trailing stray `]` is adjacent literal text, not a suppressor."""
    assert _ids("see [cite: a]] done") == [["a"]]


def test_double_bracket_marker() -> None:
    """Enclosing brackets are adjacent literal text; the inner marker is recognized."""
    assert _ids("[[cite: a]]") == [["a"]]


def test_inner_marker_in_malformed() -> None:
    """A well-formed marker nested in malformed brackets is still extracted."""
    assert _ids("[cite: bad [cite: a]]") == [["a"]]


def test_underscore_id_marker() -> None:
    """An underscore is a legal id character, never emphasis."""
    assert _ids("[cite: _a_]") == [["_a_"]]


def test_asterisk_id_char_rejected() -> None:
    """An asterisk is not a legal id character, so no marker forms."""
    assert _ids("[cite: *a*]") == []


def test_marker_inside_emphasis() -> None:
    """Emphasis markup around a marker is literal source; the marker is recognized."""
    assert _ids("*see [cite: a]*") == [["a"]]


def test_malformed_markers_literal() -> None:
    """Bracketed text that never matches the grammar is literal, not a citation."""
    assert (
        _ids(
            "\n".join(
                [
                    "[cite:]",
                    "[cite: ]",
                    "[cite: A]",
                    "[cite: a,]",
                    "[cite: a,,b]",
                    "[cite: a b]",
                    "[cite:\na]",
                ]
            )
        )
        == []
    )


def test_entity_bracket_not_marker() -> None:
    """Character references are literal source and never synthesize marker delimiters."""
    assert _ids("&#91;cite: a]\n[cite&colon; a]\n[cite: a&#93;") == []
    assert _ids("&lbrack;cite: a]\n[cite: a&rbrack;") == []
    assert _ids("[&#99;ite: a]\n[cit&#101;: a]") == []


def test_unknown_entity_keeps_marker() -> None:
    """An unknown HTML entity is literal text and does not drop a later marker (B2)."""
    assert _ids("&notanentity; [cite: ghost]") == [["ghost"]]


def test_headings_lists_prose() -> None:
    """Inline prose in headings and list items is scanned."""
    assert _ids("# Heading [cite: h]\n\n- Item [cite: i]\n- Next [cite: j]") == [
        ["h"],
        ["i"],
        ["j"],
    ]


def test_carriage_returns_normalized() -> None:
    """CRLF line endings normalize so block line maps still align."""
    assert _ids("```text\r\n[cite: a]\r\n```\r\n\r\n[cite: b]") == [["b"]]


def test_null_character_handled() -> None:
    """A NUL byte is normalized like CommonMark and does not drop a marker."""
    assert _ids("a\x00b [cite: c]") == [["c"]]


def test_id_length_cap() -> None:
    """Ids with 64 characters are accepted; 65-character ids are ignored."""
    valid = "a" * 64
    invalid = "a" * 65
    assert _ids(f"[cite: {valid}]\n[cite: {invalid}]") == [[valid]]


def test_citation_value_object() -> None:
    """Citation is a frozen value object keyed by ids."""
    citation = Citation(ids=("a", "b"))
    assert citation == Citation(ids=("a", "b"))
    with pytest.raises(FrozenInstanceError):
        cast(Any, citation).ids = ("b",)
