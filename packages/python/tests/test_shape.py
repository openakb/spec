"""Typed id grammar helpers shared by the semantic and content layers (spec §7)."""

from __future__ import annotations

from openakb_validate._shape import id_kind, is_typed_id, normalize_id, reference_code

__all__ = ()


def test_is_typed_id_accepts_both_prefixes() -> None:
    """Both prefixes with exactly six base36 characters are typed ids."""
    assert is_typed_id("SEC-000001")
    assert is_typed_id("SRC-000001")
    assert is_typed_id("sec-abc123")
    assert is_typed_id("SRC-ZZZZZZ")


def test_is_typed_id_rejects_malformed() -> None:
    """Non-strings, slugs, and shape failures are not typed ids."""
    values: tuple[object, ...] = (
        "s1",
        "root",
        "ghost",
        "SEC-",
        "SEC-0000",
        "SEC-0000000",
        "SEC-0000_1",
        42,
        None,
        [],
    )
    for value in values:
        assert not is_typed_id(value)


def test_unicode_case_folds_rejected() -> None:
    """Unicode foldables (Kelvin sign, long s) are not ASCII base36 and not typed ids."""
    kelvin_sign = "SEC-00000" + chr(0x212A)  # U+212A KELVIN SIGN
    long_s = "SEC-00000" + chr(0x017F)  # U+017F LONG S
    assert not is_typed_id(kelvin_sign)
    assert not is_typed_id(long_s)
    assert id_kind(kelvin_sign) is None
    assert id_kind(long_s) is None


def test_id_kind_from_prefix_case_insensitive() -> None:
    """The prefix alone determines kind, case-insensitively."""
    assert id_kind("SEC-000001") == "section"
    assert id_kind("src-abc123") == "source"
    assert id_kind("nope") is None


def test_reference_code_wrong_prefix_is_akb010() -> None:
    """A section-id token where a source is expected is AKB010."""
    assert (
        reference_code("SEC-000001", "source", frozenset(), frozenset({"sec-000001"})) == "AKB010"
    )


def test_reference_code_unresolved_right_prefix_is_akb007() -> None:
    """A well-formed token that resolves nowhere is AKB007."""
    assert (
        reference_code("SRC-999999", "source", frozenset({"src-000001"}), frozenset()) == "AKB007"
    )


def test_reference_code_resolves_case_insensitively() -> None:
    """Registered ids match tokens regardless of ASCII case."""
    assert reference_code("SRC-00000A", "source", frozenset({"src-00000a"}), frozenset()) is None


def test_reference_code_skips_non_typed_tokens() -> None:
    """Malformed tokens are the schema layer's AKB011, never a second finding."""
    assert reference_code("ghost", "source", frozenset({"ghost"}), frozenset()) is None
    assert reference_code(42, "source", frozenset(), frozenset()) is None


def test_normalize_id_ascii_lower() -> None:
    """Comparison keys are ASCII-lowered, so case-insensitive lookups work."""
    assert normalize_id("SRC-00000A") == "src-00000a"
    assert normalize_id("SEC-ZZZZZZ") == "sec-zzzzzz"
