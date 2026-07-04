"""Package-level smoke tests."""

import json
from importlib.resources import files

import openakb_validate


def test_version_is_exposed() -> None:
    assert openakb_validate.__version__ == "0.1.0"


def test_bundled_schemas_are_readable_json() -> None:
    for name in ("openakb.schema.json", "provenance.schema.json"):
        text = files("openakb_validate.schemas").joinpath(name).read_text("utf-8")
        assert isinstance(json.loads(text), dict)
