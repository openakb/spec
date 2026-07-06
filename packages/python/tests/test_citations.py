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


def test_link_title_comment_live() -> None:
    """A comment shape in a link title is link syntax, not a suppressor: marker stays live."""
    assert _ids('[t](https://example.org/ "<!-- [cite: a] -->")') == [["a"]]


def test_link_dest_comment_live() -> None:
    """A comment shape in an angle-bracket destination is link syntax: marker stays live."""
    assert _ids("[t](<!--%20[cite:b]%20-->)") == [["b"]]


def test_image_title_comment_live() -> None:
    """An image title is destination syntax like a link title: an enclosed marker is live."""
    assert _ids('![alt](https://example.org/i.png "<!-- [cite: b] -->")') == [["b"]]


def test_link_title_code_live() -> None:
    """A backtick run inside a link title is title text, not a code span: marker stays live."""
    assert _ids('[t](https://example.org/ "`code [cite: c]`")') == [["c"]]


def test_escaped_backtick_no_span() -> None:
    """Backslash-escaped backticks open no code span, so an enclosed marker is live (§4.4)."""
    assert _ids(r"a \`not code [cite: x]\` b") == [["x"]]


def test_escaped_backtick_then_real_span() -> None:
    """An escaped backtick is skipped, so a later real code span is still paired and masked."""
    assert _ids(r"a \`still [cite: x] ` real ` y") == [["x"]]


def test_escaped_angle_no_comment() -> None:
    """A backslash-escaped `<` opens no HTML comment, so an enclosed marker is live (§4.4)."""
    assert _ids(r"a \<!-- [cite: e] --> b") == [["e"]]


def test_control_real_span_masked() -> None:
    """Control: a genuine inline code span still suppresses its marker (no over-correction)."""
    assert _ids("a `[cite: z]` b") == []


def test_control_real_comment_masked() -> None:
    """Control: a genuine HTML comment still suppresses its marker."""
    assert _ids("<!-- [cite: w] -->") == []


def test_control_plain_marker_recognized() -> None:
    """Control: an ordinary marker outside any construct is recognized."""
    assert _ids("see [cite: m]") == [["m"]]


def test_angle_destination_marker_live() -> None:
    """A marker inside an angle-bracket destination is destination syntax: it stays live."""
    assert _ids("[t](<https://example.org/[cite: e]>) x") == [["e"]]


def test_paren_title_comment_live() -> None:
    """A parenthesized title is destination syntax: an enclosed comment shape stays live."""
    assert _ids("[t](https://example.org/ (<!-- [cite: a] -->)) x") == [["a"]]


def test_single_quote_title_comment_live() -> None:
    """A single-quoted title is destination syntax: an enclosed comment shape stays live."""
    assert _ids("[t](https://example.org/ '<!-- [cite: a] -->') x") == [["a"]]


def test_balanced_paren_destination_marker() -> None:
    """A bare destination with balanced parentheses is skipped whole; a later marker is live."""
    assert _ids("[t](https://example.org/p(a)th) [cite: a]") == [["a"]]


def test_backtick_bare_destination_live() -> None:
    """A backtick in a bare destination is destination text, not a code span opener."""
    assert _ids("[t](https://example.org/pa`th) [cite: a]") == [["a"]]


def test_escaped_angle_destination() -> None:
    """A backslash-escaped `>` does not close an angle destination; the tail still parses."""
    assert _ids(r"[t](<a\>b>) [cite: a]") == [["a"]]


def test_escaped_space_bare_destination() -> None:
    """A backslash before a space is a literal backslash (only ASCII punctuation is
    escapable), so the space ends the bare destination: the tail is not a link and the
    following code span masks the enclosed marker, matching the canonical extractor."""
    assert _ids("[t](a\\ `[cite:s]`)") == []
    assert _ids("See [t](a\\ `[cite:s]`) and [cite: t].") == [["t"]]


def test_image_escaped_space_bare_dest() -> None:
    """An image tail behaves like a link: a backslash before a space is literal, so the
    space ends the bare destination and the trailing code span masks the enclosed
    marker (matches the canonical extractor)."""
    assert _ids("![img](a\\ `[cite:s]`)") == []


def test_escaped_punct_keeps_bare_dest() -> None:
    """The punctuation control to the space case: a backslash escapes ASCII punctuation,
    so `\\!` keeps the bare destination running; the enclosed code-span shape is then
    destination syntax, not a real span, so the marker stays live (matches Rust)."""
    assert _ids("[t](a\\!`[cite:s]`)") == [["s"]]


def test_escaped_newline_angle_dest() -> None:
    """A backslash before a line ending is literal, so the line ending invalidates the
    angle destination: the tail is not a link and the trailing code span masks the
    enclosed marker (matches the canonical extractor)."""
    assert _ids("[t](<a\\\n`[cite:s]`>)") == []


def test_escaped_paren_title() -> None:
    """A backslash-escaped `)` does not close a parenthesized title; the tail still parses."""
    assert _ids(r"[t](https://example.org/ (a\)b)) [cite: a]") == [["a"]]


def test_unbalanced_destination_not_link() -> None:
    """An unmatched `(` is not a valid destination, so the `(...)` is ordinary source and its
    comment shape is masked as a real inline comment."""
    assert _ids('x []((  "<!-- [cite: a] -->") y') == []


def test_unclosed_title_not_link() -> None:
    """An unterminated title is not a valid tail, so its bytes stay ordinary source and an
    enclosed marker is recognized."""
    assert _ids('[t](https://example.org/ "unclosed [cite: a]') == [["a"]]


def test_missing_close_paren_not_link() -> None:
    """A tail with trailing junk before any `)` is not a link; its marker stays live source."""
    assert _ids("[t](url x [cite: a]") == [["a"]]


def test_nested_paren_title_not_link() -> None:
    """A parenthesized title may not nest unescaped parens, so this tail is not a link."""
    assert _ids("[t](https://example.org/ (a(b)) x [cite: a]") == [["a"]]


def test_unclosed_angle_destination_not_link() -> None:
    """An angle destination spanning a line break is invalid, so the tail is not a link."""
    assert _ids("one [t](<a\nb>) two [cite: a]") == [["a"]]


def test_bad_angle_destination_not_link() -> None:
    """An angle destination with no closing `>` is invalid; the following marker stays live."""
    assert _ids("[t](<bad dest [cite: a]") == [["a"]]


def test_inline_html_tag_opaque() -> None:
    """A raw inline HTML tag is opaque: a backtick in an attribute opens no code span, so a
    marker inside the tag is live and a marker after it is recognized."""
    assert _ids('x <span title="`[cite: a]`"> [cite: b]') == [["a"], ["b"]]


def test_inline_html_declaration_opaque() -> None:
    """A raw inline HTML declaration (`<!` + letter) is opaque like a tag: a backtick inside
    opens no code span, so the enclosed marker is live."""
    assert _ids("x <!x `[cite: a]`> [cite: b]") == [["a"], ["b"]]


def test_inline_html_pi_opaque() -> None:
    """A processing instruction is opaque: a backtick inside opens no code span."""
    assert _ids("x <?pi `[cite: a]`?> [cite: b]") == [["a"], ["b"]]


def test_comment_inside_tag_masked() -> None:
    """A complete `<!--...-->` comment enclosed by a raw HTML span is still masked."""
    assert _ids('x <a t="<!-- [cite: a] -->"> [cite: b]') == [["b"]]


def test_unclosed_comment_in_tag_live() -> None:
    """A `<!--` with no closing `-->` inside a raw HTML span masks nothing; the marker stays."""
    assert _ids('x <a t="<!-- [cite: a]"> [cite: b]') == [["a"], ["b"]]


def test_cdata_not_opaque() -> None:
    """Inline `<![CDATA[...]]>` is not raw HTML here, so a backtick inside opens a real code
    span and its marker is masked."""
    assert _ids("x <![CDATA[`[cite: a]`]]> [cite: b]") == [["b"]]


def test_reference_link_marker_after() -> None:
    """A reference link carries no destination to skip; a marker after it is recognized."""
    assert _ids("[text][label] [cite: a]") == [["a"]]


def test_shortcut_link_marker_after() -> None:
    """A shortcut link carries no destination to skip; a marker after it is recognized."""
    assert _ids("[text] [cite: a]") == [["a"]]


def test_image_reference_marker_after() -> None:
    """An image reference carries no destination to skip; a marker after it is recognized."""
    assert _ids("look ![img][ref] [cite: a]") == [["a"]]


def test_bare_destination_runs_to_end() -> None:
    """A bare destination with no closing `)` runs to the run's end and forms no link, so a
    marker before the unfinished tail is still recognized."""
    assert _ids("[cite: a] [t](https://example.org") == [["a"]]


def test_escaped_quote_in_title() -> None:
    """A backslash-escaped quote does not close a quoted title; the tail still parses."""
    assert _ids(r'[t](url "a\"b") [cite: a]') == [["a"]]


def test_unclosed_paren_title_not_link() -> None:
    """A parenthesized title with no closing `)` is invalid, so the tail is not a link and its
    enclosed marker stays live source."""
    assert _ids("[t](https://example.org/ (unclosed [cite: a]") == [["a"]]


def test_marker_escape_layer_isolated() -> None:
    """The masking layer's escape awareness never reaches the marker grammar, which has no
    escapes (§4.4): a leading backslash and literal brackets are ignored, an underscore is
    an id character, and a character reference synthesizes no bracket."""
    assert _ids(r"\[cite: a]") == [["a"]]
    assert _ids("[[cite: a]]") == [["a"]]
    assert _ids("[cite: _a_]") == [["_a_"]]
    assert _ids("&#91;cite: a]") == []


def test_citation_value_object() -> None:
    """Citation is a frozen value object keyed by ids."""
    citation = Citation(ids=("a", "b"))
    assert citation == Citation(ids=("a", "b"))
    with pytest.raises(FrozenInstanceError):
        cast(Any, citation).ids = ("b",)
