"""Package-level smoke tests."""

import json
from importlib.resources import files

import openakb_validate

__all__ = ()


def test_version_is_exposed() -> None:
    """__version__ equals the package's current release version."""
    assert openakb_validate.__version__ == "0.1.0"


def test_public_names_are_importable() -> None:
    """Every __all__ entry resolves as a top-level attribute (public == re-exports)."""
    for name in openakb_validate.__all__:
        assert hasattr(openakb_validate, name), name


def test_json_pointer_is_re_exported() -> None:
    """json_pointer is reachable from the package, keeping result.__all__ consistent."""
    from openakb_validate.result import json_pointer as source

    assert openakb_validate.json_pointer is source


def test_kind_constants_are_re_exported() -> None:
    """The ContentCheck.kind constants are exported for filtering without source-diving."""
    from openakb_validate import content

    exported = {name for name in openakb_validate.__all__ if name.startswith("KIND_")}
    assert exported == {name for name in content.__all__ if name.startswith("KIND_")}
    for name in exported:
        assert getattr(openakb_validate, name) is getattr(content, name)


def test_bundled_schemas_are_readable_json() -> None:
    """Both bundled schema files parse as JSON objects via importlib.resources."""
    for name in ("openakb.schema.json", "provenance.schema.json"):
        text = files("openakb_validate.schemas").joinpath(name).read_text("utf-8")
        assert isinstance(json.loads(text), dict)
