# OpenAKB Conformance Fixtures

This directory holds portable fixtures for descriptor validators and content citation parsers.
Fixtures are intentionally small and vendor-neutral; examples use fictional subjects and
reserved `example.com` / `example.org` domains.

## Directory layout

- `valid/` contains descriptors that MUST pass the v1 descriptor schema and structural
  validation.
- `invalid/` contains descriptors that declare one or more expected validation codes in
  `expected.json`.
- `forward-compat/` contains descriptors that are valid in lenient mode and invalid in strict
  mode for forward-compatibility checks.
- `content/` contains Markdown citation grammar fixtures for content validation. Content cases
  are extraction-only: they carry no descriptor, and their ids need not resolve to declared
  sources.

## Expected formats

Invalid descriptor fixtures use:

```json
{ "codes": ["AKB004"] }
```

A validator passes an invalid fixture if and only if it emits every code in `codes`
(spec §7). Extra codes are permitted only for distinct additional violations; duplicate
emissions of a code are ignored. An optional `"schema": false` member marks a fixture whose
code is normally schema-catchable but whose specific violation is not JSON-Schema-expressible
(for example, the `parent_id` depth cap under `AKB005`); the harness then treats it as a
semantic fixture.

Forward-compatibility fixtures use:

```json
{ "lenient": "valid", "strict": ["AKB006"] }
```

Content citation fixtures use:

```json
{ "citations": [{ "ids": ["a"] }] }
```

Citation entries follow the normative extraction output contract (spec §4.4): one entry per
recognized marker, in document order, each carrying the marker's id list in written order.
Concatenated markers stay separate entries — the combined-list equivalence is provenance
semantics, not a normalization license. Markers inside ignored CommonMark constructs (fenced
and indented code blocks, inline code spans, HTML blocks and comments) are literal text, and
bracketed text that does not match the grammar is literal text, never an error.

## Implementation reports (cross-validator harness)

Each validator implementation MAY emit a generated conformance report for the shared
cross-validator harness. Reports are dumb observations of validator behavior: they MUST NOT
read fixture `expected.json` files or pre-normalize results to match expectations. The shared
checker owns pass/fail semantics and cross-validator agreement.

Reports use this shape:

```json
{
  "implementation": "implementation-id",
  "version": "0.1.0",
  "spec_major": 1,
  "fixtures": {
    "valid/name": { "lenient": [], "strict": [] },
    "invalid/name": { "lenient": ["AKB007"], "strict": ["AKB007"] },
    "forward-compat/name": { "lenient": [], "strict": ["AKB006"] },
    "content/name": { "citations": [["a"], ["b", "c"]] }
  }
}
```

Descriptor code arrays are sorted and duplicate code emissions are collapsed in the report,
because fixture matching ignores duplicate emissions. For descriptor fixtures, `lenient`
records normal validation and `strict` records strict-mode validation. For content fixtures,
`citations` records one id list per recognized marker in document order.

The shared checker grades a report against the full fixture suite: valid fixtures require empty
lenient and strict arrays; invalid fixtures must include every declared expected code in
lenient mode; forward-compatibility fixtures require lenient success and every declared strict
code; content citations must exactly equal expected citation id lists. When two or more
reports are supplied, the checker also requires exact JSON agreement for every fixture key
present in any report.

## Phase coverage

The `ajv`-based manifest lint (`scripts/ci/check-conformance.mjs`) validates fixture
shape, rule coverage, and the schema-catchable codes, including the keyword→code
mapping from spec §7. The Python reference validator (`packages/python`) executes the
full suite — both modes for `valid/` and `forward-compat/`, code-level assertions for
`invalid/`, and the `content/` extraction contract — via its conformance runner
(`uv run pytest tests/test_conformance.py`) and emits an implementation report (see
§Implementation reports). Every further validator passes the identical suite by
shipping a report emitter for the shared checker; `scripts/ci/check-conformance-report.mjs`
applies the match semantics and asserts cross-validator agreement per fixture on verdicts and
error codes.

## Rule traceability

| Code | Rule | Fixture |
| --- | --- | --- |
| `AKB001` | IDs are unique across the shared source and section ID space. | `invalid/duplicate-id`, `invalid/duplicate-source-id` |
| `AKB002` | Every section has `content_uri` or at least one child. | `invalid/empty-section` |
| `AKB003` | Every section with `content_uri` cites at least one source. | `invalid/content-without-source` |
| `AKB004` | The `parent_id` graph is acyclic. | `invalid/parent-cycle` |
| `AKB005` | Length and cardinality caps are respected. | `invalid/oversized-title` |
| `AKB006` | Strict mode rejects unknown core properties outside `x`. | `forward-compat/unknown-core-field` |
| `AKB007` | References resolve to declared IDs. | `invalid/unresolved-parent`, `invalid/unresolved-source-ref`, `invalid/unresolved-discovered-via`, `invalid/link-target-unresolved`, `invalid/unresolved-claim-source` |
| `AKB008` | Link `rel` values are controlled or reverse-DNS namespaced. | `invalid/unknown-rel` |
| `AKB009` | Required top-level, source, and section fields are present. | `invalid/missing-required-field` |
| `AKB010` | References resolve to the expected entity kind. | `invalid/wrong-reference-kind`, `invalid/parent-is-source`, `invalid/discovered-via-wrong-kind`, `invalid/link-target-wrong-kind`, `invalid/claim-source-wrong-kind` |
| `AKB011` | Charset, format, and type constraints hold. | `invalid/malformed-id`, `invalid/malformed-timestamp`, `invalid/malformed-guide-hash`, `invalid/trailing-newline-id` |
| `AKB012` | Every link carries `section_id`, `akb_uri`, or both. | `invalid/link-without-target` |
