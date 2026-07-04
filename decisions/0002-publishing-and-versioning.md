# ADR-0002: Publishing & versioning

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

The schema and each validator have different audiences and release cadences, and a published
`$schema` URL must never break.

## Decision

Adopt SemVer from v1.0.0. The descriptor's `$schema` is keyed by MAJOR (`.../v1/...`, tracking
the latest backward-compatible 1.x) with an immutable `.../v1.0.0/...` pin for reproducibility.
Run **independent version streams** driven by prefixed git tags: `v*` publishes the schema to a
domain the project owns (`schema.openakb.org`, served from object storage behind a CDN);
`rust-v*` / `js-v*` / `py-v*` publish the respective validator to its registry. CI (path-filtered
tests) is separate from publishing (tag-gated); each validator publish is conformance-gated.
Start with manual prefixed tags and graduate to automated release PRs later. The first bare
`v*` tag — spec v1.0.0 and the go-live of `schema.openakb.org` — is cut only after at least
two independent validator implementations pass the shared conformance suite in agreement
(identical verdicts and error codes per fixture). Until then the schema URI is a reserved
identifier that validators pattern-match but never dereference: every validator bundles the
published schemas and validates fully offline.

## Consequences

Streams never cross-fire (a bare `v*` tag never matches a prefixed one). `$schema` URLs are
stable because the backend is swappable behind an owned domain. No accidental release on merge.
Provisioning the domain and cloud account is a prerequisite for the `v*` stream only —
validator packages publish to their registries before it exists (tracked in
[`GOVERNANCE.md`](../GOVERNANCE.md)).
