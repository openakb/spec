# Contributing to OpenAKB

Thanks for helping build an open, vendor-neutral standard. This guide covers how to propose
changes and the checks your contribution must pass.

## How changes flow

1. **Open an issue or a [discussion](https://github.com/openakb/spec/discussions)** describing
   the problem or idea.
2. **Discuss** the approach before writing a large change.
3. **Open a pull request.** Larger, normative changes go through the
   [AKEP process](proposals/README.md).

## Developer Certificate of Origin (DCO)

All commits must be signed off under the
[Developer Certificate of Origin](https://developercertificate.org/). Sign off certifies you
wrote the change or have the right to submit it under the project's licenses. Add the trailer
automatically with:

```bash
git commit -s -m "your message"
```

This appends a `Signed-off-by: Your Name <you@example.com>` line. Amend an existing commit with
`git commit --amend -s`, or sign off a series with `git rebase --signoff <base>`. A CI check
enforces this; there is **no CLA**.

## Licensing of contributions

By contributing you agree your changes are licensed under the project's licenses:
[Apache-2.0](LICENSE) for code, schema, and validators; [CC-BY-4.0](LICENSE-DOCS) for
normative prose.

## Vendor neutrality

Public artifacts must name **no** real product or company. Use neutral wording and, for any
illustrative material, fictional subjects on the RFC 2606 reserved `example.com` / `example.org`
domains. See [AGENTS.md](AGENTS.md) for the full rule.

## Running the checks locally

The same checks CI runs:

```bash
npx --yes markdownlint-cli2 "**/*.md"          # markdown style
lychee --offline --config lychee.toml .        # relative-link integrity
actionlint                                     # GitHub Actions workflow lint
```

Later phases add per-package tests and the conformance harness; this document is updated as
those land.

## Commit messages

Use short, imperative summaries. Conventional-Commits prefixes (`feat:`, `fix:`, `docs:`,
`build:`, `ci:`) are encouraged but not required.

## Please don't commit

The `docs/superpowers/` tree is internal working state and is gitignored. Never add it to a
commit or link to it from published files.
