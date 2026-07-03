# Governance

This document defines who stewards OpenAKB, how decisions are made, and the structural
commitments that keep the standard vendor-neutral.

## Steward

OpenAKB is stewarded by **OpenAKB.org**, a named non-profit entity that holds the
"OpenAKB" name and mark, the `openakb.org` and `schema.openakb.org` domains, the
GitHub organization, and any registry namespaces. These assets are held by the entity —
not by any individual founder's personal account and not by any single company.

## Decision-making (staged)

The process is intentionally lightweight before 1.0 and formalizes as adoption grows.

### Phase 1 — pre-1.0 (now)

- Changes flow **GitHub issue → discussion → pull request**.
- Design decisions are recorded as ADRs in [`decisions/`](decisions/README.md); versioned
  changes follow SemVer and are logged in [`CHANGELOG.md`](CHANGELOG.md).
- The steward has final say to keep momentum. The bar is "rough consensus and running
  code": a normative change lands only with its schema, validator, example, and
  conformance updates together.
- The genesis proposal (AKEP-0001) is authored pre-1.0 under steward sign-off, ahead of
  the Phase 2 AKEP machinery.

### Phase 2 — at/after 1.0, once there are ≥2 independent implementers

- Enhancement proposals become numbered **AKEPs** (see [`proposals/`](proposals/README.md))
  with the lifecycle Draft → Review → Accepted → Final → (Superseded).
- A small multi-organization editors/steering group operates under OpenAKB.org.

## Neutrality & anti-capture

Single-vendor capture is the primary adoption risk for any vendor-incubated standard.
These commitments are structural, not aspirational:

- **Named asset holder.** All project assets are held by OpenAKB.org, the non-profit
  entity described above — distinct from any implementer.
- **Neutral-home intent.** The stated intent is to donate the standard to a
  vendor-neutral foundation (of the CNCF / Linux Foundation / OpenJS class) once it clears
  the adoption bar below.
- **Transition trigger (falsifiable).** When **≥2 independent production implementers**
  ship — where "independent" means not under common ownership or control with the steward
  — governance MUST move to a multi-organization steering group, and the neutral-home
  donation is put to that group.
- **Bus factor.** Domain, cloud account, and package-registry ownership are held by the
  entity with **≥2 human administrators** and documented recovery, so the project survives
  any one person's departure.

Nurok is the first implementer to build infrastructure and tooling on OpenAKB. It is a
downstream implementer, not the owner of the standard; the spec is designed to be
vendor-neutral and Nurok adapts to the spec, not the reverse.

## Amending this document

Changes to governance follow the same issue → discussion → PR flow and are approved by the
steward (Phase 1) or the steering group (Phase 2).
