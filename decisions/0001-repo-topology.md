# ADR-0001: Repository topology

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

The standard, its schema, its examples, its conformance suite, and its validator libraries
evolve together. Splitting them across repos makes atomic "spec + running code" changes hard.

## Decision

Use a single monorepo, `openakb/spec`, holding the specification, schema, AKEP proposals,
examples, conformance suite, and the validator libraries under `packages/{rust,js,python}`.
Reserve `openakb/validator` for a possible post-1.0 extraction of the libraries if their
cadence diverges. A general fetch/push/pull CLI is **out of scope** — that is infra/provider
tooling per the razor; the shipped deliverable is the validator library, `openakb-validate`.

## Consequences

Enhancement proposals land atomically across all artifacts in one PR. The conformance suite
stays with the standard even if libraries are later extracted. The repo is polyglot, so CI
becomes path-aware once the validators and packages land in a later phase. The bare `openakb`
name is reserved but unused.
