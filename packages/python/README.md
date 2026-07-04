# openakb-validate

`openakb-validate` is the pure Python reference validator library for the OpenAKB descriptor
format, spec major v1. It ships the v1 descriptor and provenance schemas as package resources,
runs locally, does not use the network, and intentionally provides no command-line interface.

## Install

Install the package into a Python 3.12 or newer environment:

```bash
python -m pip install openakb-validate
```

For local development from this repository:

```bash
cd packages/python
uv sync
```

## Validate a descriptor

Use `validate_descriptor` to validate a parsed descriptor document against the bundled spec
major v1 schema:

```python
import json
from pathlib import Path

from openakb_validate import validate_descriptor

descriptor = json.loads(Path("akb.json").read_text(encoding="utf-8"))
result = validate_descriptor(descriptor)

if not result.valid:
    for diagnostic in result.diagnostics:
        print(diagnostic.message)
```

The validator accepts already-loaded data. Reading files, choosing storage, and reporting
diagnostics are caller responsibilities.

## Strict mode

Strict mode asks the library to report optional portability diagnostics in addition to schema
errors:

```python
from openakb_validate import validate_descriptor

result = validate_descriptor(descriptor, strict=True)
```

Strict diagnostics are still local checks. The library does not fetch remote resources or make
freshness decisions.

## Check content

Use `LocalFileResolver` with `check_content` to verify local content referenced by a descriptor:

```python
from pathlib import Path

from openakb_validate import LocalFileResolver, check_content

resolver = LocalFileResolver(root=Path("example-akb"))
result = check_content(descriptor, resolver=resolver)
```

Resolvers define how descriptor references map to caller-controlled resources. The default
library behavior remains offline and side-effect free.

## Validate everything in one call

Use `validate_with_content` when a caller wants descriptor validation and local content checks
in one library call:

```python
from pathlib import Path

from openakb_validate import LocalFileResolver, validate_with_content

result = validate_with_content(
    descriptor,
    resolver=LocalFileResolver(root=Path("example-akb")),
    strict=True,
)
```

This combines schema, strict-mode, provenance, and content diagnostics as those APIs land.

## Conformance

The package is intended to run the shared OpenAKB conformance fixtures for spec major v1.
Conformance execution is library-driven so downstream tools can choose their own interfaces,
output formats, and integration points.

## Development and contributing

Use `uv` for package development:

```bash
cd packages/python
uv sync
uv run pytest
uv run ruff format .
uv run ruff check .
uv run mypy
```

The package vendors byte-identical copies of the published schemas from `schema/v1/`. Check
schema drift from the repository root:

```bash
bash scripts/ci/check-schema-sync.sh
```

Repository-wide contribution rules still apply, including strict vendor neutrality in public
artifacts and keeping spec, schema, examples, validators, and conformance fixtures aligned.

## License

Apache-2.0.
