# AKB Enhancement Proposals (AKEPs)

An **AKEP** is the unit of significant change to OpenAKB — a new field or module, a breaking
change, or a policy that needs a written rationale. The name (over PEP/AEP/AIP/KEP/OEP) avoids
collision with existing proposal series.

## When you need one

- Small, non-normative changes (typos, tooling, docs) → a plain pull request.
- Anything that changes the descriptor model, the schema, or normative behavior → an AKEP.

See [GOVERNANCE.md](../GOVERNANCE.md) for how the process is staged before and after 1.0.

## Lifecycle

`Draft → Review → Accepted → Final → (Superseded)`

- **Draft** — being authored; not yet ready for wide review.
- **Review** — open for community and editor feedback.
- **Accepted** — approved in principle; the artifacts are being implemented.
- **Final** — merged and in force (see the artifact bar below).
- **Superseded** — replaced by a later AKEP, retained for the historical record.

An AKEP reaches **Final** only when its specification text, JSON Schema, reference validator,
worked example, and conformance fixtures all land together ("rough consensus and running
code"). Additive changes are a MINOR release; breaking changes are batched into a MAJOR.

## How to propose

1. Copy [`template.md`](template.md) to `akep-XXXX-short-title.md` (the editors assign the
   number when the draft is opened).
2. Fill in every section.
3. Open a pull request and link the discussion.

## Index

| AKEP | Title | Status |
| ---- | ----- | ------ |
| 0001 | Genesis — establishes v1.0 | Planned |

*(Status `Planned` = reserved but not yet drafted — a pre-lifecycle state, distinct from the
`Draft → … → Final` states above. AKEP-0001 is authored in a later phase alongside the normative
spec, schema, examples, and conformance suite.)*
