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
Start with manual prefixed tags and graduate to automated release PRs later.

## Consequences

Streams never cross-fire (a bare `v*` tag never matches a prefixed one). `$schema` URLs are
stable because the backend is swappable behind an owned domain. No accidental release on merge.
Provisioning the domain and cloud account is a prerequisite (tracked in
[`GOVERNANCE.md`](../GOVERNANCE.md)).
