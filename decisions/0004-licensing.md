# ADR-0004: Licensing & contribution terms

- **Status:** Accepted
- **Date:** 2026-07-02

## Context

A standard needs a patent grant and a clean contribution trail — neither of which MIT provides.
The interim repository license was MIT.

## Decision

License code, schema, and validators under **Apache-2.0** (for its explicit patent grant) and
normative prose (specification and AKEPs) under **CC-BY-4.0**. Accept contributions under the
**Developer Certificate of Origin** (sign-off), enforced by a CI check; no CLA. This must be in
place before the repo accepts external PRs, because relicensing after contributions arrive needs
every contributor's consent.

## Consequences

Implementers are protected from later patent claims by contributors. Prose is freely reusable
with attribution. Contribution is lightweight (a sign-off trailer, no paperwork). The interim MIT
license is replaced at scaffold.
