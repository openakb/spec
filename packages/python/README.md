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

Each `Finding` has a stable `code`, RFC 6901 JSON Pointer `path`, and `message`.
Its `.name` property derives the public finding name from the code catalog.
Advisory diagnostics use `Advisory` and appear in `warnings`; they never affect
the validation verdict.

The stable validation codes for spec major v1 are `AKB001` through `AKB012`.
Those code meanings are part of the public contract for v1 and only change through
the spec process.

## Extract citations

Use `extract_citations` to read normative `[cite:]` markers from Markdown prose.
The parser follows CommonMark structure, so markers in code and HTML constructs
remain literal text. Each returned `Citation` contains source ids in written order,
including duplicates.

```python
from openakb_validate import Citation, extract_citations

citations: list[Citation] = extract_citations("See [cite: source_a, source_b].")
ids = [citation.ids for citation in citations]
```

## Strict mode

Strict mode adds `AKB006 unknown-core-property` findings for unknown core
members outside extension payloads:

```python
from openakb_validate import validate

result = validate(descriptor, strict=True)
unknown_core_findings = [
    finding for finding in result.findings if finding.code == "AKB006"
]
```

Strict mode remains local and deterministic. It does not make network requests or
freshness decisions. Advisory `warnings` describe MAY-warn surfaces and never
affect `result.ok`.

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

These commands are the package conformance/report gate used by CI and release
checks.

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
