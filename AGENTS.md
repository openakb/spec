# Repository conventions (for contributors — human and coding agents)

This file is the working guide for contributing to the **OpenAKB spec repository**. It is
distinct from the `AKB.md` guide that a *knowledge base described by the spec* would carry.
(`CLAUDE.md` is a symlink to this file.)

## What this repo is

The `openakb/spec` monorepo holds the OpenAKB standard and its supporting artifacts. Current
top-level map:

- `specs/` — the normative specification
- `schema/` — JSON Schema for the descriptor and provenance sidecar
- `proposals/` — the AKEP process and enhancement proposals
- `examples/` — worked, vendor-neutral example AKBs
- `conformance/` — cross-validator conformance fixtures
- `packages/` — validator libraries in Rust / TypeScript / Python *(later phases)*
- `decisions/` — architecture decision records
- `.github/`, `scripts/` — CI and automation

## The razor

OpenAKB standardizes the **declarative description** of an AKB. It defines a portable
vocabulary; it defines **no mechanism** for storage, serving, search, freshness, auth, or
transport. When a proposal adds a *mechanism*, it is probably out of scope — infra's job, not
the spec's.

## Naming conventions (for descriptor fields)

- `snake_case`, lowercase keys.
- Timestamps end in `_at` (RFC 3339 UTC). Related-resource URIs end in `_uri`. ID references
  end in `_id` / `_ids`; an object's own identity is `id`. Booleans use `is_` / `has_`; enums
  are lowercase strings.

## Spec + running code

Any change under `schema/` or `specs/` MUST land **in the same pull request** as its matching
validator, conformance, and example updates. A proposal is not "done" until spec text, schema,
validator, example, and conformance move together.

Before the reference validator lands (implementation Phase 3), the committed JSON Schema and
its `ajv` gate are the "running code" that must accompany `schema/`/`specs/` changes; the
full validator + conformance-execution bar attaches with `packages/` in Phase 3, which also
carries AKEP-0001 through its lifecycle (Draft → Review → Accepted → Final).

## Strict vendor neutrality

**No real product or company name** appears in any public artifact — not in examples, spec
prose, conformance fixtures, the README, or the schema. Use neutral wording. Illustrative
material uses **fictional subjects** on the RFC 2606 reserved `example.com` / `example.org`
domains. Three narrow carve-outs: `GOVERNANCE.md` may name the first implementer for the
anti-capture story; development tooling and infrastructure referenced in contributor/meta
files — the hosting platform, package manager, lint/validation tooling, dependency
automation, coding-agent configuration files, and CI/coverage **status badges** (build,
coverage, release) — may appear in the README and other meta files, since they identify
tooling, not spec subjects or endorsements; and prior-art and external technical references
MAY appear as hyperlinks to the cited source (a technical citation, not an endorsement).
This is enforced in review; `scripts/ci/check-neutrality.sh` is only a regression backstop
whose deny-list is non-exhaustive.

## Running the checks locally

```bash
npm ci                                          # install ajv (first run only)
npm run check:schema                            # examples + sidecars validate against the schema
npm run check:conformance                       # conformance manifest is coherent
npx --yes markdownlint-cli2 "**/*.md"          # markdown style
lychee --offline --config lychee.toml .        # relative-link integrity
actionlint                                     # GitHub Actions workflow lint
bash scripts/ci/check-neutrality.sh            # public artifact neutrality
```

Per-package tests and the conformance harness are documented here as they land.

## Proposing changes

Small changes: issue → discussion → PR. Larger/normative changes: the
[AKEP process](proposals/README.md). Sign off every commit for the DCO (`git commit -s`); see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Do not commit

The `docs/superpowers/` tree is internal working state (gitignored). Never commit it or link
to it from published files.
