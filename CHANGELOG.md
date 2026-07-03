# Changelog

All notable changes to the OpenAKB specification, schema, and validators are documented
here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) starting
at v1.0.0.

## [Unreleased]

### Added

- Repository scaffold: Apache-2.0 / CC-BY-4.0 licensing, governance and neutrality
  commitments, contribution process (DCO), code of conduct, security policy, contributor
  and agent conventions, GitHub issue/PR templates and Dependabot, the AKEP process seed,
  the decision-record set, and a CI skeleton (markdown, link, and workflow lint) gated by a
  single `ci-ok` check plus a DCO check.
- Normative v1 specification (`specs/v1/spec.md`) with RFC 2119 language, the field catalog,
  and the `AKB001`–`AKB012` error-code catalog; the descriptor and self-describing provenance
  JSON Schemas (`schema/v1/`), lenient on unknown members for forward compatibility;
  vendor-neutral worked examples in both authoring and served form (minimal, widget-platform,
  cross-link, widget-platform-served, sidecar-provenance); the conformance suite (valid /
  invalid / forward-compat / content) with a manifest lint and traceability matrix; AKEP-0001
  (Genesis, Draft); and `ajv`-based schema + conformance CI gates plus a neutrality check.
- Source discovery edge `discovered_via_id`: an optional Source field recording the listing
  or feed source via which a source was discovered (spec §4.2), with schema, example, and
  conformance-fixture coverage (`unresolved-discovered-via`, `discovered-via-wrong-kind`).
- Source capture anchoring: optional Source fields `content_hash` (SRI-style integrity of the
  captured snapshot bytes taken at `captured_at`) and `capture_uri` (where the capture is
  re-servable), closing the chain from snapshot to verifiable `locator.quote` spans
  (spec §2, §4.2, §4.4, §5, §7), with schema coverage and a raw capture file in the
  widget-platform example stamped with its real hash in both authoring and served form.
- Normative "unverifiable is not invalid" rule for `--check-content` (spec §7): content
  checks yield verified / failed / unverifiable; an unresolvable or unfetchable URI is
  reported as unverifiable, never as a structural failure, and a conformance verdict never
  changes because of access. §8 notes that URIs skipped by a scheme allowlist are likewise
  unverifiable, not failures.
- Guide integrity: optional top-level `guide_hash` and `guide_length`, stamped from the
  served bytes at publish time like section `content_hash` / `content_length`, so every
  hosted payload a descriptor references is hash-verifiable (spec §4.1, §4.7, §5, §7), with
  schema, served-example, and conformance coverage.
- Provider-side source redaction: a RECOMMENDED `type: "redacted"` source form whose `uri`
  points at a provider-hosted stub as the accountable origin, a narrowly scoped exception to
  the §5 never-rewritten-on-serve rule, and unverifiable-by-construction content checks
  against redacted sources (spec §2, §4.2, §5, §7, §8), with a valid conformance fixture.
- Revision-fetch convention: pinned revisions are RECOMMENDED to be served at the URI derived
  from the descriptor's resolved fetch URI by inserting `revisions/<revision>/` before its
  final path segment, so a consumer resolves a pinned cross-AKB link zero-knowledge from
  `akb_uri` plus the link's `revision`; offering revision fetch and retaining revisions stay
  provider policy (spec §4.5; ADR-0006).
- §5.1 detach procedure: a RECOMMENDED inverse of the authoring→served transformation for
  pull/mirror/fork tools — resolve-then-relativize hosted references, drop `base_uri` and
  `revision`, verify-then-keep stamped hashes, keep external source URIs verbatim — so
  independently written tools produce interoperable working copies.

### Changed

- Bounded-manifest caps raised to a generous tier, frozen for the v1 major: every `title`
  is now one 200-char tier, AKB and section `description` and section `purpose` allow
  2000 chars, link `description` allows 500, and `sources` allows 100,000 entries
  (`sections` stays at 10,000). §7 clarifies that caps are properties of the interchange
  artifact, not provider capacity declarations.
- The bare `openakb` name is designated for the project's command-line tool, planned as
  `openakb/openakb` and scoped to validation only (schema + structural rules, `--strict`,
  `--check-content`) as a thin front end over the `packages/` reference libraries;
  fetch/push/pull workflow tooling remains out of scope per the razor (ADR-0001).
