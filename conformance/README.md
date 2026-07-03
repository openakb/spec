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

## Phase coverage

Phase 2 executes positive descriptor validation for `valid/` and `forward-compat/`, negative
schema assertions for `invalid/` fixtures whose declared codes are schema-catchable (including
the keyword→code mapping check from spec §7), and the manifest lint that checks fixture shape
and rule coverage. Execution of the semantic structural checks (`AKB001`–`AKB004`, `AKB007`,
`AKB010`) and `content/` citation parsing lands with the Phase 3 reference validator; until
then, those fixtures are guarded by manifest lint.

## Rule traceability

| Code | Rule | Fixture |
| --- | --- | --- |
| `AKB001` | IDs are unique across the shared source and section ID space. | `invalid/duplicate-id` |
| `AKB002` | Every section has `content_uri` or at least one child. | `invalid/empty-section` |
| `AKB003` | Every section with `content_uri` cites at least one source. | `invalid/content-without-source` |
| `AKB004` | The `parent_id` graph is acyclic. | `invalid/parent-cycle` |
| `AKB005` | Length and cardinality caps are respected. | `invalid/oversized-title` |
| `AKB006` | Strict mode rejects unknown core properties outside `x`. | `forward-compat/unknown-core-field` |
| `AKB007` | References resolve to declared IDs. | `invalid/unresolved-parent`, `invalid/unresolved-source-ref` |
| `AKB008` | Link `rel` values are controlled or reverse-DNS namespaced. | `invalid/unknown-rel` |
| `AKB009` | Required top-level, source, and section fields are present. | `invalid/missing-required-field` |
| `AKB010` | References resolve to the expected entity kind. | `invalid/wrong-reference-kind`, `invalid/parent-is-source` |
| `AKB011` | Charset, format, and type constraints hold. | `invalid/malformed-id`, `invalid/malformed-timestamp` |
| `AKB012` | Every link carries `section_id`, `akb_uri`, or both. | `invalid/link-without-target` |
