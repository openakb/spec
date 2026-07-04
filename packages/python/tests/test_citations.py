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
    """Inline code spans terminate prose runs."""
    assert _ids("Literal `[cite: a]` and prose [cite: b].") == [["b"]]


def test_html_block_ignored() -> None:
    """HTML blocks are not prose."""
    assert _ids("<section>\n[cite: a]\n</section>\n\n[cite: b]") == [["b"]]


def test_html_comment_ignored() -> None:
    """HTML comments as block constructs are not prose."""
    assert _ids("<!-- [cite: a] -->\n\n[cite: b]") == [["b"]]


def test_inline_comment_ignored() -> None:
    """Inline HTML comments terminate prose runs."""
    assert _ids("See <!-- [cite: a] --> [cite: b].") == [["b"]]


def test_malformed_markers_literal() -> None:
    """Malformed bracketed text is literal text, not a citation or error."""
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


def test_encoded_delimiters_literal() -> None:
    """Character references cannot synthesize literal citation syntax."""
    assert _ids("&#91;cite: a]\n[cite&colon; a]\n[cite: a&#93;") == []
    assert _ids("&lbrack;cite: a]\n[cite&colon; a]\n[cite: a&rbrack;") == []
    assert _ids("[&#99;ite: a]\n[c&#105;te: a]\n[ci&#116;e: a]\n[cit&#101;: a]") == []


def test_no_marker_across_emphasis() -> None:
    """Structural inline children prevent assembling a marker across emphasis."""
    assert _ids("[cite: *a*]") == []


def test_marker_inside_emphasis() -> None:
    """A complete marker inside one emphasized text run is prose."""
    assert _ids("*see [cite: a]*") == [["a"]]


def test_headings_lists_prose() -> None:
    """Inline prose in headings and list items is scanned."""
    assert _ids("# Heading [cite: h]\n\n- Item [cite: i]\n- Next [cite: j]") == [
        ["h"],
        ["i"],
        ["j"],
    ]


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
