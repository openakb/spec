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

### Changed

- Bounded-manifest caps raised to a generous tier, frozen for the v1 major: every `title`
  is now one 200-char tier, AKB and section `description` and section `purpose` allow
  2000 chars, link `description` allows 500, and `sources` allows 100,000 entries
  (`sections` stays at 10,000). §7 clarifies that caps are properties of the interchange
  artifact, not provider capacity declarations.
