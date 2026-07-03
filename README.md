# OpenAKB

[![CI](https://github.com/openakb/spec/actions/workflows/ci.yml/badge.svg)](https://github.com/openakb/spec/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Docs: CC BY 4.0](https://img.shields.io/badge/Docs-CC_BY_4.0-lightgrey.svg)](LICENSE-DOCS)
[![Status: draft](https://img.shields.io/badge/spec-v1_draft-orange.svg)](CHANGELOG.md)

**OpenAKB** is an open, infrastructure-agnostic JSON standard for describing an
**agentic knowledge base (AKB)** in a single portable file. One descriptor captures an
AKB's identity, its raw sources, its section tree, where each section's content lives
(by URI), how sections are grounded in sources (provenance), and how they link to one
another and to other AKBs.

OpenAKB standardizes the **description** of an AKB. It defines a portable vocabulary and
defines **no mechanism** for storage, serving, search, freshness, auth, or transport —
those belong to whatever platform hosts the AKB. This keeps knowledge portable instead of
trapped in one vendor's storage or serving model.

## Status

Pre-1.0 and under active design. The normative specification, JSON Schema, worked
examples, and conformance suite have landed; reference validators are forthcoming.
Today's tooling checks JSON-Schema conformance only; the structural rules (`AKB001`
and up) are enforced by the Phase 3 reference validator.
See [CHANGELOG.md](CHANGELOG.md) for what has landed.

## Specification

- **Normative spec** — [specs/v1/spec.md](specs/v1/spec.md)
- **JSON Schema** — [schema/v1/openakb.schema.json](schema/v1/openakb.schema.json),
  [provenance.schema.json](schema/v1/provenance.schema.json)
- **Worked examples** — [examples/](examples/): [minimal](examples/minimal/),
  [widget-platform](examples/widget-platform/) (authoring) and
  [widget-platform-served](examples/widget-platform-served/) (served form),
  [cross-link](examples/cross-link/),
  [sidecar-provenance](examples/sidecar-provenance/)
- **Conformance** — [conformance/](conformance/README.md)
- **Genesis proposal** — [AKEP-0001](proposals/akep-0001-genesis.md)

## Get involved

- **Propose a change** — read [CONTRIBUTING.md](CONTRIBUTING.md) and open an issue or a
  discussion.
- **Enhancement proposals** — the AKEP process lives in [proposals/](proposals/README.md).
- **How decisions were made** — see the [decision records](decisions/README.md).
- **Governance & neutrality** — see [GOVERNANCE.md](GOVERNANCE.md).
- **Learn more** — <https://openakb.org>.

## License

Code, schema, and validators are licensed under [Apache-2.0](LICENSE). Normative prose
(the specification and AKEPs) is licensed under [CC-BY-4.0](LICENSE-DOCS), © 2026 OpenAKB.org. Contributions
are accepted under the [Developer Certificate of Origin](https://developercertificate.org/)
(sign off commits with `git commit -s`).
