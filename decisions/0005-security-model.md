# ADR-0005: Security model

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

Descriptors are consumed by agents and their URIs are resolved by tooling, which makes them a
security-relevant surface. The specification's binding rules are structural; the trust posture is
advisory.

## Decision

Adopt an "all content untrusted" posture — first-party and linked AKB content are equally a
prompt-injection surface. Advise resolvers to apply a URI-scheme allowlist (guarding against SSRF
and `file:`/`data:` abuse) and size/time/redirect fetch bounds. Treat content and provenance
hashes as attesting serving *integrity*, not *authenticity*; ship no descriptor-signing primitive
before 1.0 and reserve an attestation layer for a post-1.0 proposal. Publish a `SECURITY.md` with
the threat model and a coordinated-disclosure process.

## Consequences

Implementers get clear guidance without over-constraining the format. Signing is deliberately
deferred. The advisory nature is explicit so it is not mistaken for conformance requirements.
