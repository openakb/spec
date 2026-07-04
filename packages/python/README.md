# openakb-validate

`openakb-validate` is the pure Python reference validator library for the OpenAKB
descriptor format, spec major v1. It is a library only: no CLI, no network access,
no background services, and no storage, search, freshness, authentication, or
transport mechanism.

The package validates caller-provided descriptor data and reports stable findings.
It does not fetch referenced resources unless the caller supplies an explicit
resolver. Source, issues, and the normative specification live in the
[openakb/spec repository](https://github.com/openakb/spec).

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

descriptor = json.loads(Path("openakb.json").read_text(encoding="utf-8"))
result: ValidationResult = validate(descriptor)

if not result.ok:
    for finding in result.findings:
        print(f"{finding.code}: {finding.message}")
```

Each `Finding` has a stable `code`, RFC 6901 JSON Pointer `path`, and `message`.
Its `.name` property derives the public finding name from the code catalog and,
for a code outside the catalog, echoes the code back rather than raising. Advisory
diagnostics use `Advisory` and appear in `warnings`; they never affect the
validation verdict. The re-exported `json_pointer` helper builds the same canonical
pointers the validator emits, for callers that construct or match against
`finding.path`.

The stable validation codes for spec major v1 are `AKB001` through `AKB012`, defined
in [spec §7, Validation and error codes][spec-errors]. Those code meanings are part
of the public contract for v1 and only change through the spec process.

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
import json
from pathlib import Path

from openakb_validate import validate

descriptor = json.loads(Path("openakb.json").read_text(encoding="utf-8"))
result = validate(descriptor, strict=True)
unknown_core_findings = [
    finding for finding in result.findings if finding.code == "AKB006"
]
```

Strict mode remains local and deterministic. It does not make network requests or
freshness decisions. Advisory `warnings` describe MAY-warn surfaces and never
affect `result.ok`.

## Check content

Content checks are opt-in: `check_content` only fetches a descriptor's referenced
resources through a resolver you supply, then verifies hashes, provenance sidecars,
citation markers, and claim quotes. `LocalFileResolver(base_dir=...)` reads files
under a caller-controlled directory.

```python
import json
from pathlib import Path

from openakb_validate import ContentReport, LocalFileResolver, check_content

descriptor = json.loads(Path("openakb.json").read_text(encoding="utf-8"))
resolver = LocalFileResolver(base_dir="example-akb")
report: ContentReport = check_content(descriptor, resolver=resolver)

if report.failed:
    for check in report.failed:
        print(f"FAILED {check.kind} {check.path}: {check.detail}")
elif not report.verified:
    print("nothing verified: no content was fetched or there was nothing to check")
else:
    print(f"verified {len(report.verified)} content reference(s)")

for finding in report.findings:
    print(f"{finding.code}: {finding.message}")
```

### Three outcomes, and why `ok` can be silent

Every content check reports one of three outcomes, exposed as the module constants
`VERIFIED`, `FAILED`, and `UNVERIFIABLE`:

- **verified** — the resource was fetched and matched: a hash matched, a quote was
  found in its capture, or citation markers resolved.
- **failed** — the resource was fetched and did *not* match: a hash mismatch, an
  absent quote, or malformed fetched content. Tampering surfaces here.
- **unverifiable** — the resource could not be fetched, or there was nothing to
  check it against. Nothing was proven either way; this is never an error.

`report.ok` is `True` when no check failed and no structural finding was raised. It
is deliberately **not** "everything verified": a report in which every check is
`unverifiable` — because nothing was fetched — is still `ok`. To tell "all good"
from "nothing checked", read the outcome accessors, which mirror `report.failed`:

- `report.verified`, `report.failed`, `report.unverifiable` collect the checks with
  each outcome.
- `report.findings` carries only the *structural* findings raised inside content
  (for example, a citation marker that points at a missing source). A hash or quote
  mismatch is not a structural finding — it appears in `report.failed`. Consuming
  code that watches only `report.findings` will therefore miss a corrupted capture;
  key on `report.failed` (and `report.ok`) to catch tampering.

Filter checks by their `kind` using the exported constants rather than hard-coding
strings: `KIND_GUIDE_HASH`, `KIND_CONTENT_HASH`, `KIND_CITATIONS`, `KIND_SIDECAR`,
`KIND_CAPTURE`, and `KIND_QUOTE`.

```python
import json
from pathlib import Path

from openakb_validate import KIND_CAPTURE, LocalFileResolver, check_content

descriptor = json.loads(Path("openakb.json").read_text(encoding="utf-8"))
report = check_content(descriptor, resolver=LocalFileResolver(base_dir="example-akb"))

capture_checks = [check for check in report.checks if check.kind == KIND_CAPTURE]
```

### Resolvers

`LocalFileResolver` accepts a `str` or `os.PathLike[str]` `base_dir` and confines
reads to files beneath it, rejecting absolute paths, `..` traversal, queries, and
path parameters before any read. That path policing is specific to
`LocalFileResolver`.

Implement the `Resolver` protocol when content lives somewhere other than a local
directory:

```python
from openakb_validate import Resolver, Unfetchable


class MappingResolver(Resolver):
    """Resolve references from an in-memory mapping."""

    def __init__(self, files: dict[str, bytes]) -> None:
        self._files = files

    def fetch(self, uri: str) -> bytes:
        try:
            return self._files[uri]
        except KeyError as error:
            raise Unfetchable(uri) from error
```

`check_content` calls `fetch(uri)` with the *effective reference*: the descriptor
reference (a `content_uri`, `capture_uri`, `guide_uri`, or `provenance_uri`) already
joined onto the descriptor's `base_uri` when one is present, with any `#fragment`
stripped. A resolver returns the resource `bytes` or raises `Unfetchable`; anything
it cannot fetch becomes an `unverifiable` outcome, never an error. A custom resolver
receives none of `LocalFileResolver`'s path guards and owns its own safety.

## Validate everything in one call

Use `validate_with_content` when a caller wants descriptor validation, strict-mode
advisories, provenance checks, and content checks through one API.

```python
import json
from pathlib import Path

from openakb_validate import FullReport, LocalFileResolver, validate_with_content

descriptor = json.loads(Path("openakb.json").read_text(encoding="utf-8"))
report: FullReport = validate_with_content(
    descriptor,
    resolver=LocalFileResolver(base_dir="example-akb"),
    strict=True,
)

if not report.ok:
    for finding in report.validation.findings:
        print(f"{finding.code}: {finding.message}")
    for check in report.content.failed:
        print(f"FAILED {check.kind} {check.path}: {check.detail}")
```

`report.ok` combines structural validity and content checks
(`validation.ok and content.ok`), so it carries the same caveat: it does not require
that anything was verified. This is still a pure library call — callers choose where
data comes from, how findings are displayed, and whether any finding blocks their
workflow.

## Conformance

The package participates in the shared
[OpenAKB conformance suite](https://github.com/openakb/spec/tree/main/conformance)
for spec major v1. Conformance fixtures are the cross-language contract for
validation behavior, including the stable `AKB001` through `AKB012` code mapping.

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

Follow the package conventions in `AGENTS.md` and the repository conventions in the
root `AGENTS.md`, and see
[CONTRIBUTING.md](https://github.com/openakb/spec/blob/main/CONTRIBUTING.md) for the
contribution process. Public artifacts must remain vendor-neutral.

## License

Apache-2.0.

[spec-errors]: https://github.com/openakb/spec/blob/main/specs/v1/spec.md#7-validation-and-error-codes
