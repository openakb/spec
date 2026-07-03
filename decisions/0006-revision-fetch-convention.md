# ADR-0006: Revision-fetch convention

- **Status:** Accepted
- **Date:** 2026-07-03

## Context

Spec §4.5 makes `revision` the pinning primitive — a link is pinned if and only if it carries
`revision` — but on its own that gives a consumer holding a pinned link no way to resolve it:
the consumer can drift-*detect* (fetch the head, notice the mismatch) but never
drift-*resolve*. Without even a RECOMMENDED convention, each provider would mint an
incompatible scheme. Three candidate shapes were evaluated (issue #7): (a) a revision-scoped
URL convention derived from the descriptor's fetch URI, (b) an optional top-level
`revisions_uri` URI-template field, and (c) a well-known discovery document. Constraints:
offering revision fetch stays OPTIONAL, retention stays provider policy (an unretained
revision is best-effort/unverifiable), the convention must compose with `base_uri` and §5
relative-reference resolution, and the razor — the spec declares, infrastructure mechanizes.

## Decision

Candidate (a): pinned revisions are RECOMMENDED to be served at the URI formed by inserting
the path segments `revisions/<revision>/` immediately before the final segment of the
descriptor's resolved fetch URI's path, with the revision value percent-encoded as a single
path segment. The deciding property is zero-knowledge resolution: a consumer constructs the
revision URI directly from the resolved `akb_uri` and the link's `revision` — one fetch, no
head read — and the descriptor surface gains no new field. Because the derivation is
relative to whatever URI the consumer holds, each mirror serves its own revision tree; a
mirror that does not is simply the not-retained case. (b) was rejected because it requires
fetching the target head just to learn the template and adds a top-level field for what is
purely serving behavior. (c) was rejected as the most mechanism-shaped option: a second
fetchable discovery artifact with its own lifecycle.

## Consequences

Pinned cross-AKB links become resolvable across providers that opt in, with no descriptor or
schema change; the convention is spec prose only, so there is nothing for a validator to
check offline. The accepted trade-off: the convention standardizes a slice of URL structure —
descriptor fetch locations only; content paths stay opaque per §5 — and a provider whose
descriptor URLs are not path-shaped (for example, query-driven endpoints) cannot offer it,
in which case pins against it degrade to unverifiable (§7) exactly as when revisions are
pruned. The convention composes with §5 because the derivation applies to the resolved fetch
URI, after any relative `akb_uri` has been resolved.
