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

### Changed

- Bounded-manifest caps raised to a generous tier, frozen for the v1 major: every `title`
  is now one 200-char tier, AKB and section `description` and section `purpose` allow
  2000 chars, link `description` allows 500, and `sources` allows 100,000 entries
  (`sections` stays at 10,000). §7 clarifies that caps are properties of the interchange
  artifact, not provider capacity declarations.
