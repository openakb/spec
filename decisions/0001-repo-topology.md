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
cadence diverges.

The validator packages are **pure libraries**. They expose a validation API and ship no
command-line entrypoint in any language: no console script on the Python distribution, no
binary target on the Rust crate, no `bin` field on the npm package.

The bare `openakb` name is designated for the project's command-line tool, planned as
`openakb/openakb`: a thin front end over the reference validator libraries and the **only**
command-line surface, scoped to validation only — descriptors and provenance sidecars against
the schema and the structural rules, the strict lint, and the opt-in content-verification
checks. A general fetch/push/pull workflow CLI is **out of scope** — serving, sync, and
transport are infra/provider tooling per the razor.

## Consequences

Enhancement proposals land atomically across all artifacts in one PR. The conformance suite
stays with the standard even if libraries are later extracted. The repo is polyglot, so CI
becomes path-aware once the validators and packages land in a later phase. Command-line
concerns — flags, output formats, exit codes — live in one product rather than three
libraries, and the libraries stay embeddable with no CLI surface to keep in sync. The CLI repo
tracks published library releases on its own cadence without weakening the same-PR rule here.
