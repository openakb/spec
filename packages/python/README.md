# openakb-validate

`openakb-validate` is the pure Python reference validator library for the OpenAKB
descriptor format, spec major v1. It is a library only: no CLI, no network access,
no background services, and no storage, search, freshness, authentication, or
transport mechanism.

The package validates caller-provided descriptor data and reports stable findings.
It does not fetch referenced resources unless the caller supplies an explicit local
resolver.

## Install

Install from the package index into Python 3.12 or newer:

```bash
python -m pip install openakb-validate
```

For repository development:

```bash
cd packages/python
uv sync
```

## Validate a descriptor

Use `validate` for descriptor-only validation. The input is already-parsed JSON
data; callers own file reading and output formatting.

```python
import json
from pathlib import Path

from openakb_validate import ValidationResult, validate

descriptor = json.loads(Path("akb.json").read_text(encoding="utf-8"))
result: ValidationResult = validate(descriptor)

if not result.ok:
    for finding in result.findings:
        print(f"{finding.code}: {finding.message}")
```

Each `Finding` has a stable code, message, severity, and location. Advisory
diagnostics use `Advisory` so callers can distinguish portability advice from
validation failures.

The stable validation codes for spec major v1 are `AKB001` through `AKB012`.
Those code meanings are part of the public contract for v1 and only change through
the spec process.

## Strict mode

Strict mode includes portability and policy advisories in addition to required
validation findings:

```python
from openakb_validate import Advisory, validate

result = validate(descriptor, strict=True)
warnings: list[Advisory] = result.warnings
```

Strict mode remains local and deterministic. It does not make network requests or
freshness decisions.

## Check content

Use `LocalFileResolver(base_dir=...)` with `check_content` to verify descriptor
references against files under a caller-controlled directory.

```python
from pathlib import Path

from openakb_validate import ContentReport, LocalFileResolver, check_content

resolver = LocalFileResolver(base_dir=Path("example-akb"))
report: ContentReport = check_content(descriptor, resolver=resolver)

for finding in report.findings:
    print(f"{finding.code}: {finding.message}")
```

Resolvers define how descriptor references map to resources. Implement the
`Resolver` protocol when content lives somewhere other than a local directory.
Unresolvable resources are reported as unverifiable; the library does not fetch
them.

## Validate everything in one call

Use `validate_with_content` when a caller wants descriptor validation, strict-mode
advisories, provenance checks, and local content checks through one API.

```python
from pathlib import Path

from openakb_validate import FullReport, LocalFileResolver, validate_with_content

report: FullReport = validate_with_content(
    descriptor,
    resolver=LocalFileResolver(base_dir=Path("example-akb")),
    strict=True,
)

for finding in report.validation.findings:
    print(f"{finding.code}: {finding.message}")

for finding in report.content.findings:
    print(f"{finding.code}: {finding.message}")
```

This is still a pure library call. Callers choose where data comes from, how
findings are displayed, and whether any finding blocks their workflow.

## Conformance

The package participates in the shared OpenAKB conformance suite for spec major
v1. Conformance fixtures are the cross-language contract for validation behavior,
including the stable `AKB001` through `AKB012` code mapping.

Package development emits and checks a conformance report with:

```bash
cd packages/python
uv run python tests/conformance_report.py > conformance-report.json
node ../../scripts/ci/check-conformance-report.mjs conformance-report.json
```

Those commands land with the conformance harness. Until then, schema packaging and
the package smoke tests are the running-code gate for this scaffold.

## Development and contributing

Use `uv` for package work:

```bash
cd packages/python
uv sync
uv run pytest
uv run ruff format --check .
uv run ruff check .
uv run mypy
```

The package vendors byte-identical copies of the published schemas from
`schema/v1/` so they are available through `importlib.resources`. Check drift from
the repository root:

```bash
bash scripts/ci/check-schema-sync.sh
```

Follow the package conventions in `AGENTS.md` and the repository conventions in
the root `AGENTS.md`. Public artifacts must remain vendor-neutral.

## License

Apache-2.0.
