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
- `content/` contains Markdown citation grammar fixtures for later content validation.

## Expected formats

Invalid descriptor fixtures use:

```json
{ "codes": ["AKB004"] }
```

Forward-compatibility fixtures use:

```json
{ "lenient": "valid", "strict": ["AKB006"] }
```

Content citation fixtures use:

```json
{ "citations": [{ "ids": ["a"] }] }
```

Citation entries are listed in document order. Code spans and fenced code blocks are ignored by
the content fixtures.

## Phase coverage

Phase 2 executes positive descriptor validation for `valid/` and `forward-compat/`, plus the
manifest lint that checks fixture shape and rule coverage. Execution for `invalid/` structural
checks and `content/` citation parsing lands in Phase 3; until then, those fixtures are guarded
by manifest lint.

## Rule traceability

| Code | Rule | Fixture |
| --- | --- | --- |
| `AKB001` | IDs are unique across the shared source and section ID space. | `invalid/duplicate-id` |
| `AKB002` | Every section has `content_uri` or at least one child. | `invalid/empty-section` |
| `AKB003` | Every section with `content_uri` cites at least one source. | `invalid/content-without-source` |
| `AKB004` | The `parent_id` graph is acyclic. | `invalid/parent-cycle` |
| `AKB005` | Length and cardinality caps are respected. | `invalid/oversized-title` |
| `AKB006` | Strict mode rejects unknown core properties outside `x`. | `forward-compat/unknown-core-field` |
| `AKB007` | References resolve to declared IDs. | `invalid/unresolved-parent` |
| `AKB008` | Link `rel` values are controlled or reverse-DNS namespaced. | `invalid/unknown-rel` |
| `AKB009` | Required top-level, source, and section fields are present. | `invalid/missing-required-field` |
| `AKB010` | References resolve to the expected entity kind. | `invalid/wrong-reference-kind` |
| `AKB011` | Charset, format, and type constraints hold. | `invalid/malformed-id` |
