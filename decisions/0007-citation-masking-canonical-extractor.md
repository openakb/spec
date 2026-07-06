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

Correcting root cause (2) is the harder part: `markdown-it-py` does not expose source spans for
inline link/image destinations and titles (their inline child tokens carry no source map), so
the hand-rolled masker cannot exclude a shape embedded in them by consulting the tokenizer. The
fix therefore keeps the raw-source-offset model and teaches the inline scan a **bounded**
destination/title sub-grammar (angle and balanced-paren destinations; single-, double-, and
paren-quoted titles; autolinks; raw inline HTML spans), rather than adopting a full CommonMark
inline-link delimiter stack.

## Decision

The **real-CommonMark-parser behavior is canonical**: the `[cite:…]` extraction contract of
spec §4.4 is whatever a conformant CommonMark parser masks, as realized by the Rust
validator's `pulldown-cmark`-driven extractor. The Python reference validator's hand-rolled
inline masker was the side that was **wrong** on the cases above, and this PR corrects it to
match. The correction keeps the raw-source-offset model and makes the inline scan:

- **escape-aware** — a backslash-escaped `` ` `` opens no code span and a backslash-escaped
  `<` opens no comment, closing root cause (1); and
- **link- and HTML-aware** — a link, image, or autolink destination or title, and a raw inline
  HTML span (open/close tag, processing instruction, declaration), are skipped whole so a
  code-span or comment shape inside them is left as ordinary source, closing root cause (2).

The escape awareness lives only in this masking layer; the marker grammar itself still has no
escapes (§4.4), so `\[cite: a]`, `[[cite: a]]`, and `[cite: _a_]` remain markers and
`&#91;cite: a]` remains literal text.

Conformance fixtures now pin the reconciled families through the public content surface —
`conformance/content/link-title-comment`, `link-destination-comment`, and
`escaped-backtick-live` — and the shared cross-validator gate
(`scripts/ci/check-conformance-report.mjs`) confirms per-fixture agreement between the Python
and Rust reports.

## Consequences

Both validators now agree on inline citation masking for the construct families above: the
Python reference validator no longer over-masks markers embedded in link/image destinations or
titles, in autolinks, or in raw inline HTML, nor after backslash-escaped openers. A
differential fuzz sweep (hundreds of thousands of assembled inputs) drives the two extractors
to identical output on inline content. The correction, the conformance fixtures, and the
regenerated cross-validator reports land together in one PR, per the same-PR "spec + running
code" rule.

The bounded destination/title sub-grammar is a deliberate non-goal-reduction: it recognizes the
destination and title shapes CommonMark actually admits without implementing the full inline
link-resolution delimiter stack, and it matches the canonical extractor on the reconciled
families. One pulldown-specific detail is mirrored on purpose: inline `<![CDATA[…]]>` is **not**
treated as raw HTML by the canonical extractor, so the Python masker does not treat it as raw
HTML either.

One difference remains, and it is **out of scope** for this decision: `markdown-it-py` and
`pulldown-cmark` draw a few **block**-level boundaries differently (for example, whether a
bare `<!` declaration or a tag line after a link-reference definition opens an HTML block).
That divergence lives in the block tokenizer, predates this correction, and is untouched by
it; it is a separate concern from the inline citation-masking contract this ADR governs.
