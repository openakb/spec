# ADR-0007: Canonical citation extractor for masking edge cases

- **Status:** Accepted
- **Date:** 2026-07-06

## Context

Spec §4.4 defines `[cite:…]` extraction against the raw Markdown source with exactly five
CommonMark constructs masked first: fenced code, indented code, inline code spans, HTML
blocks, and HTML comments. A marker inside any other construct is live prose. The two v1
validators implement the masking differently, and the implementations disagree on a family of
edge cases:

- The Rust validator (`packages/rust`) masks by walking a real CommonMark parser
  (`pulldown-cmark`): it masks the source ranges the parser reports as code spans, code
  blocks, HTML blocks, and HTML comments.
- The Python reference validator (`packages/python`) masks by hand-scanning each inline run's
  source region for backtick runs and `<!--` openers, using its CommonMark tokenizer
  (`markdown-it-py`) only for block ranges and inline-run boundaries.

The hand-rolled inline scan does not model two CommonMark rules that the real parser applies,
so it **over-masks** — it blanks source that is actually live prose and drops an enclosed
marker. Observed divergences (neutral, `example.org`), Rust versus Python:

| Source | Per CommonMark / Rust | Python (over-masks) |
| --- | --- | --- |
| `[t](http://example.org "<!-- [cite: a] -->")` | `a` is live (comment shape sits in a link title) | dropped |
| `![alt](http://example.org/i.png "<!-- [cite: b] -->")` | `b` is live (image title) | dropped |
| `` [t](http://example.org "`code [cite: c]`") `` | `c` is live (backtick run in a link title) | dropped |
| `` a \`not code [cite: d]\` b `` | `d` is live (backslash-escaped backticks open no span) | dropped |
| `a \<!-- [cite: e] --> b` | `e` is live (backslash-escaped `<` opens no comment) | dropped |

The two root causes are (1) no backslash-escape handling, so an escaped `` ` `` or `<` is
mis-read as a construct opener, and (2) no link/image awareness, so a code span or comment
*shape* embedded in a link or image destination or title is masked even though CommonMark has
already consumed those bytes as link syntax. In content-verification mode this flips whether
`AKB007` fires for the enclosed id, so "the Rust validator is behaviorally equivalent to the
reference" holds only on the constructs the shared conformance suite currently exercises.

Correcting root cause (2) is the hard part: `markdown-it-py` does not expose source spans for
inline link/image destinations and titles (their inline child tokens carry no source map), so
the hand-rolled masker cannot exclude a shape embedded in them without reimplementing
CommonMark link, image, and autolink parsing — including its precedence over code spans. That
is a real CommonMark inline parser, which is a larger change than this decision resolves.

## Decision

The **real-CommonMark-parser behavior is canonical**: the `[cite:…]` extraction contract of
spec §4.4 is whatever a conformant CommonMark parser masks, as realized by the Rust
validator's `pulldown-cmark`-driven extractor. The Python reference validator's hand-rolled
inline masker is the side that is **wrong** on the cases above and is to be corrected to
match; until it is, its over-masking is a known deviation, not the contract.

No conformance fixture is added for these constructs yet. A fixture pins both validators to
one expected extraction, and the shared cross-validator gate
(`scripts/ci/check-conformance-report.mjs`) requires per-fixture agreement; adding one now
would red the gate until the Python masker is fixed. The suite therefore continues to cover
only constructs on which both validators already agree.

## Consequences

The follow-up is to replace the Python reference validator's hand-rolled inline masker with a
real CommonMark inline walk, so it masks exactly the code spans, code blocks, HTML blocks, and
HTML comments a conformant parser reports — the same basis the Rust validator already uses —
rather than re-deriving them by hand-scanning. Because `markdown-it-py` does not surface inline
source spans, that walk must recover them (or the masker must adopt a parser that does), which
is why the fix is deferred to a dedicated task rather than bolted onto the current scanner: a
piecemeal patch (for example, adding only backslash-escape handling) would close root cause
(1) while leaving the link/image family of root cause (2) divergent, giving a false impression
of parity. When that task lands, it lands with the Python correction, the conformance fixtures
that pin this construct family, and regenerated cross-validator reports in one PR, per the
same-PR "spec + running code" rule. Until then, a downstream tool that must agree with the
standard on markers embedded in link/image titles or destinations, or after backslash-escaped
openers, should follow the Rust extractor.
