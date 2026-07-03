# AKEP-0001: Genesis — establishes OpenAKB v1.0

| Field | Value |
| ----- | ----- |
| AKEP | 0001 |
| Title | Genesis — establishes OpenAKB v1.0 |
| Author | Jian Fang <fj@openakb.org> |
| Status | Draft |
| Type | Standards |
| Created | 2026-07-02 |
| Requires | None |
| Supersedes | None |

## Abstract

This AKEP establishes the initial OpenAKB v1.0 standard. It records the format rationale for a portable, declarative AKB descriptor and points to the normative v1 specification and companion artifacts that define the first interoperable baseline.

## Motivation

Agentic knowledge bases sit in a recurring loop: distill knowledge from sources, share the result in a reusable form, and consume that shared knowledge in later work. OpenAKB focuses on the sharing and interchange joint in that loop. It standardizes a portable description of an AKB so authors, tools, validators, and readers can agree on structure without standardizing storage, serving, search, freshness, authentication, authorization, or transport.

A [prior-art markdown+frontmatter knowledge format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) was evaluated as a base and rejected for OpenAKB v1.0 because it is too permissive on the trust layer OpenAKB needs to standardize. It has no per-section provenance binding, uses untyped links, tolerates broken links, and does not provide structural verification strong enough for a portable interchange contract.

## Specification

The normative specification for this AKEP is [OpenAKB v1 Specification](../specs/v1/spec.md). This AKEP does not restate its normative rules.

The v1.0 baseline is defined by the specification plus these accompanying artifacts:

- descriptor and provenance JSON Schemas under [`schema/v1/`](../schema/v1/);
- worked examples under [`examples/`](../examples/), including the served-form example at [`examples/widget-platform-served/`](../examples/widget-platform-served/);
- conformance fixtures under [`conformance/`](../conformance/).

## Backwards compatibility

There is no earlier OpenAKB version to migrate from; v1.0 is the baseline. Compatibility within v1 follows the lenient-schema contract in [the v1 specification](../specs/v1/spec.md#6-extensions-and-versioning).

## Reference implementation

AKEP-0001 remains **Draft** during the current implementation phase. The running code at this point is the schema plus the AJV-based schema gate used by CI.

The proposal advances through the standard lifecycle — **Draft → Review → Accepted → Final** — as the validator lands: it moves to **Review** when the Rust reference validator is open for feedback alongside the v1 artifacts, to **Accepted** when the approach is approved in principle, and to **Final** only when that validator passes conformance execution and is published with the v1 artifacts.

## Security considerations

This AKEP introduces no security model beyond the v1 specification. See [spec §8](../specs/v1/spec.md#8-security-considerations-non-normative) and the repository [Security Policy](../SECURITY.md).

## Rejected alternatives

- **Use the prior-art markdown+frontmatter knowledge format as the base** — rejected because it is too permissive on the trust layer: no per-section provenance binding, untyped links, tolerance of broken links, and no structural verification. OpenAKB keeps one-way convertibility possible, but does not build the v1 contract on that format.
- **Registry addressing** — rejected in favor of direct URI references. Direct URIs keep resolution infrastructure-agnostic and avoid making a registry primitive part of v1. The `namespace/id` pair remains a self-declared label, not a globally resolved identity.
- **AKB-as-provenance-source** — rejected because provenance terminates in raw sources. Cross-AKB relationships are represented as links, not as citations that compose provenance across AKBs.
- **Reader-anticipating or hand-authored claim lists** — rejected because maintainers cannot reliably predict every future reader claim. OpenAKB instead models provenance as an authoring-time byproduct that binds sections and claims to sources.
- **Embedded content** — rejected because the descriptor is a bounded manifest. Section content remains behind `content_uri`, with optional hashes and lengths in served form.
- **`visibility`, `stakes`, and `suggested_prompts` in v1** — deferred because they are policy or advisory metadata that need more operational experience before standardization.
- **Validators in a separate repository now** — rejected for the pre-1.0 path because spec text, schema, validators, examples, and conformance should land atomically with AKEP changes. Extracting validators can be reconsidered after v1.0.
- **Rich/batteries-included or lean/structural-only scope** — rejected in favor of a core, provenance-first descriptor with optional modules under namespaced extension points.
- **Top-level freshness object or source-keyed parallel map** — rejected because it creates a second structure that must stay synchronized with sources. Freshness hints live on the Source object instead.
- **MIT throughout** — rejected for a standard because the project needs explicit patent protection for code contributions, a separate prose license for specification text, and a contribution sign-off trail. OpenAKB uses Apache-2.0 for code and schema, CC-BY-4.0 for specification prose, and DCO sign-off for contributions.
- **Strict-only schema** — rejected because it would make forward-compatible v1 minor additions fail against older pinned schemas. OpenAKB uses a single lenient schema plus validator strict-lint mode for typo-catching.
- **Two schema files** — rejected because strictness is a validator mode, not a separate conformance contract. A single published schema keeps editor validation, CI validation, and forward compatibility aligned.

## Future considerations (non-normative)

Deferred v1.1 candidates recorded here so they are not mistaken for omissions:

- **Multi-AKB composition.** The bounded-manifest caps are deliberate; corpora beyond them split into multiple AKBs joined by `part-of` links from a small index AKB. Deterministic composition semantics for such families are a v1.1 candidate.
- **A controlled `translation-of` rel.** Parallel translations currently use a custom reverse-DNS rel.
- **Deprecation/tombstone markers.** v1 has no way to distinguish intentional removal of a section from truncation; a declarative tombstone marker is a candidate, keeping the vocabulary/mechanism split.

## Copyright

This document is licensed under [CC-BY-4.0](../LICENSE-DOCS).
