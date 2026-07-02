# ADR-0003: Governance & neutrality

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

Single-vendor capture is the primary adoption risk for a vendor-incubated standard. Good
intentions do not convince adopters; structure does.

## Decision

Stage governance: lightweight (issue → discussion → PR, steward has final say) before 1.0, and
numbered AKEPs with a multi-organization steering group at/after 1.0. Commit anti-capture
structure in `GOVERNANCE.md`: a named non-profit entity (OpenAKB.org) holds the name, domains,
and registries; a stated intent to donate to a vendor-neutral foundation; a falsifiable
transition trigger (≥2 independent production implementers); and bus-factored credentials
(≥2 admins). Bind the process to artifacts — an AKEP is Final only with spec, schema, validator,
example, and conformance together.

## Consequences

Neutrality is verifiable rather than aspirational. The model mirrors CloudEvents/CNCF. Forming
the entity and filing the trademark are prerequisites tracked as provisioning items.
