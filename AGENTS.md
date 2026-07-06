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
- `packages/` — validator libraries; `packages/python` is the reference validator and
  `packages/rust` is the Rust validator. Each package carries its own `AGENTS.md` with
  package-level conventions.
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

The reference validator (`packages/python`) and the conformance suite are the "running
code": its conformance runner executes every fixture group, so a `schema/`/`specs/`
change must keep `uv run pytest` green in `packages/python` in the same PR, and the
vendored schema copies must stay byte-identical to `schema/v1/`
(`scripts/ci/check-schema-sync.sh`). AKEP-0001 advances through its lifecycle
(Draft → Review → Accepted → Final) with this validator.

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

Python validator checks (from `packages/python/`):

```bash
uv sync                    # install the dev environment (first run only)
uv run ruff format --check . && uv run ruff check .
uv run mypy
uv run pytest              # tests + conformance suite + coverage gate
uv run python tests/conformance_report.py > conformance-report.json
node ../../scripts/ci/check-conformance-report.mjs conformance-report.json
```

## Proposing changes

Small changes: issue → discussion → PR. Larger/normative changes: the
[AKEP process](proposals/README.md). Sign off every commit for the DCO (`git commit -s`); see
[CONTRIBUTING.md](CONTRIBUTING.md).

## Do not commit

The `docs/superpowers/` tree is internal working state (gitignored). Never commit it or link
to it from published files.
