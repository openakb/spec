# OpenAKB v1 Specification

Status: Draft

## §1 Conformance language

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, NOT RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as described in BCP 14 [RFC 2119] [RFC 8174] when, and only when, they appear in all capitals, as shown here.

## §2 Scope

OpenAKB standardizes the declarative description of an agentic knowledge base (AKB). It defines a portable vocabulary for expressing an AKB's identity, sources, section tree, content locations, provenance, and links. It defines no mechanism for storage, serving, search, freshness, authentication, authorization, or transport.

| Concern | In the spec (declaration) | Infra's job (mechanism) |
| --- | --- | --- |
| Provenance | The binding: section to `source_ids`; source to `captured_at` and its captured-bytes pin (`content_hash`, `capture_uri`). | Capturing or snapshotting sources, generating the binding, re-serving raw bytes. |
| Freshness | Thin descriptive hints: `refresh_class`, `cadence`, and a last-edit `refreshed_at`. | The refresh loop, change detection, health polling, and honoring cadence. |
| Discovery | The `discovered_via_id` edge from a discovered source to the listing source that surfaced it. | Monitoring feeds, detecting new entries, minting the new source. |
| Redaction | The redacted source form: `type: "redacted"` with a provider-hosted stub `uri` as the accountable origin. | Audience projection and access control: deciding who sees which sources, serving per-audience projections. |
| Verification | Per-section `purpose` and the structural rules a validator asserts. | Running judges or refetch validation. |
| Addressing | The `namespace/id` grammar and URI references. | Resolving names, registry authority, and mirroring. |
| Revisions | The `revision` marker as declarative desired state. | Copy-on-write builds, diffing, changelogs, and live-head flips. |
| Auth / transport | Nothing. | Credentials, tokens, signed URLs, and fetching. |

## §3 Naming conventions

Descriptor keys use `snake_case` and lowercase. Timestamps end in `_at` and are RFC 3339 UTC values. The UTC profile is explicit: the date and time are separated by an uppercase `T`, and the offset is an uppercase `Z` or `+00:00`.

Related-resource URIs end in `_uri`, such as `content_uri`, `guide_uri`, `provenance_uri`, `base_uri`, and `akb_uri`. A bare `uri` is used only when the object itself is a pointer, as with a Source.

ID references end in `_id` or `_ids`, such as `parent_id`, `source_ids`, and `section_id`. An object's own identity is `id`. Arrays of embedded objects use the plural noun, such as `sources`, `sections`, and `links`.

Boolean fields use `is_` or `has_` prefixes. Enum values are lowercase strings.

## §4 Descriptor model

An OpenAKB descriptor is a JSON object, conventionally named `openakb.json`. The descriptor is the bounded manifest; section content and provenance sidecars are payloads behind URIs.

### §4.1 Top-level fields

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `$schema` | REQUIRED | URI | MUST be the major-keyed schema URI `https://schema.openakb.org/v1/openakb.schema.json` or an immutable SemVer pin of the form `https://schema.openakb.org/v1.X.Y/openakb.schema.json`. A v1 validator MUST reject any other value as `AKB011`. |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Human-readable AKB id. |
| `namespace` | optional | `[a-z0-9_-]`, ≤64 chars | Owner or grouping segment. |
| `title` | REQUIRED | string, 1-200 chars | Display name. |
| `description` | REQUIRED | string, 1-2000 chars | Bounded AKB abstract. |
| `subject_type` | optional | non-empty string | Open subject classification. |
| `tags` | optional | array, max 32 unique items; each `[a-z0-9_-]`, max 40 chars | Discovery and filtering labels. Tags share the local ID charset, so an id within the tag length cap can serve as a tag verbatim; tags are labels, not references. |
| `language` | optional | language pattern `[A-Za-z0-9]+(-[A-Za-z0-9]+)*` | Primary language of AKB content. Sections MAY override. |
| `guide_uri` | optional | URI reference | Maintainer guide, conventionally `AKB.md`. |
| `guide_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the decoded guide bytes at `guide_uri`. The §4.3 hash rules apply: canonical base64, and `sha256` MUST be supported. |
| `guide_length` | optional | integer, minimum 0 | Decoded guide byte count: the same bytes `guide_hash` covers. An advisory, untrusted size hint, like section `content_length`. |
| `revision` | optional | string | Infra-minted revision marker, normally present on served descriptors. |
| `base_uri` | optional | URI reference | Explicit base for resolving relative references. |
| `created_at` | optional | RFC 3339 UTC timestamp | Descriptor creation timestamp. |
| `updated_at` | optional | RFC 3339 UTC timestamp | Descriptor update timestamp. |
| `sources` | REQUIRED | array, 1-100000 Source objects | Raw-evidence registry. |
| `sections` | REQUIRED | array, 1-10000 Section objects | Section forest. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

`namespace/id` is a self-declared canonical name. It is not registry-enforced and is not resolvable on its own. Resolution is always by URI, such as the URL or file path from which a descriptor was fetched. Two unrelated AKBs MAY assert the same `namespace/id`. A consumer MUST treat the fetch URI as the identity of record and MUST NOT assume `namespace/id` is globally unique.

The `x` extension object MAY appear at the top level, on sources, on sections, on links, and on provenance claims and their locators. Extension keys MUST be reverse-DNS names, such as `com.example` or `org.example.tools`, and each extension value is an object.

### §4.2 Source fields

A Source is raw provenance material a section can be grounded in. Sources are not other AKBs; cross-AKB relationships are represented as links.

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Citation target; unique in the shared source and section id space. |
| `type` | REQUIRED | non-empty string | Source kind, such as `url` or `file`; extensible. |
| `uri` | REQUIRED | URI reference | Location of the evidence. External URLs remain absolute. |
| `title` | optional | string, max 200 chars | Display title. |
| `captured_at` | optional | RFC 3339 UTC timestamp | When the source snapshot was captured. |
| `content_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the captured evidence bytes: the snapshot taken at `captured_at` (the bytes at `capture_uri` when present), not a claim about the live resource at `uri`. The §4.3 hash rules apply: canonical base64, and `sha256` MUST be supported. |
| `capture_uri` | optional | URI reference | Where the captured snapshot is re-servable, when the provider offers one. The spec defines no capture, retention, or serving mechanism. |
| `content_length` | optional | integer, minimum 0 | Captured-evidence byte count: the same bytes the source `content_hash` covers (the bytes at `capture_uri` when present). An advisory, untrusted size hint, like section `content_length`. |
| `refresh_class` | optional | non-empty string | Freshness policy hint, such as `static`, `polled`, `event`, or `streaming`; descriptive only. |
| `cadence` | optional | non-empty string | Expected refresh interval hint, meaningful only by convention. |
| `discovered_via_id` | optional | source `id`, `[a-z0-9_-]`, ≤64 chars | The listing source via which this source was discovered. MUST resolve to a Source in this descriptor. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

Freshness policy hints live on the Source object. There is no top-level freshness map in v1.

**Firsthand sources.** First-party knowledge whose first written form is the AKB itself — runbooks, postmortems, tribal knowledge with no earlier document to point at — is RECOMMENDED to be represented as a source with `type: "firsthand"`. For a firsthand source, `uri` points at the accountable origin of the knowledge, such as a team or owner URI (illustrative material uses an `example.com` / `example.org` URI). The source object is then the citable record of who stands behind the content; `captured_at` MAY record when the knowledge was first written down.

**Feed sources and discovery.** Some sources are listing or feed pages — for example `https://www.example.com/blog/` — that surface new material over time. A listing page is itself a source and is RECOMMENDED to use `type: "feed"`; it is the natural home for the `refresh_class` and `cadence` hints.

- Each discovered item is its own Source with its own `uri` and `captured_at`, carrying `discovered_via_id` naming the listing source that surfaced it.
- Sections cite the discovered item, not the listing; an uncited listing source is valid.
- The discovery graph SHOULD be acyclic; validators MAY warn on cycles, and no error code is defined for them.

> Monitoring the feed is infrastructure's mechanism; the `discovered_via_id` edge is the declarative record it leaves.

**Capture anchoring.** `content_hash` and `capture_uri` pair with `captured_at`. Section content is often produced by a lossy transform — model cleanup, OCR — of a captured snapshot, and claim `locator.quote` values are verbatim spans of that snapshot; `captured_at` says when the snapshot was taken, and `content_hash` pins what bytes were read.

With a pinned capture, claim `locator.quote` spans can be mechanically verified — substring presence — against the exact bytes the content was generated from, even after the live resource at `uri` changes. Without `capture_uri`, that verification is available only to parties holding the snapshot.

> Capturing, retaining, and re-serving snapshots is infrastructure's mechanism; these two fields are the declarative record it leaves.

**Redacted sources.** Some sources cannot be disclosed to every audience the descriptor is served to — an internal document, a paywalled feed, an uploaded file — while the sections they ground remain servable. A source whose identity is withheld by the provider is RECOMMENDED to use `type: "redacted"`:

- `uri` points at a provider-hosted stub that acts as the accountable origin of the citation.
- `captured_at` MAY be retained as a dated marker.
- A redacted source SHOULD NOT carry identifying fields: `title`, `content_hash`, `content_length`, `capture_uri`, `refresh_class`, `cadence`, and `discovered_via_id` are omitted. Leaving `content_length` behind would still disclose the withheld source's exact captured size.
- The source keeps its `id`, so `source_ids` entries and inline `[cite:]` markers — which are baked into content bytes pinned by `content_hash` — keep resolving unchanged.

The type string `redacted` is the convention's interoperable name: a provider-specific or extended type does not receive the §5 rewrite exception and carries none of the redaction semantics.

> Withholding a source's identity, deciding which audiences see it, and serving per-audience projections are infrastructure's mechanism; the redacted source is the declarative record that mechanism leaves (§5).

### §4.3 Section fields

The Section is the atomic unit of browse, pull, and grounding. The tree is expressed by `parent_id` plus document order; `id` is position-independent so references and diffs can survive restructuring.

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Stable identity; unique in the shared source and section id space. |
| `parent_id` | optional | section `id`, `[a-z0-9_-]`, ≤64 chars | Parent section. Absence means root. |
| `title` | REQUIRED | string, 1-200 chars | Display title. |
| `description` | REQUIRED | string, 1-2000 chars | Consumer-facing browse abstract. |
| `purpose` | optional | string, max 2000 chars | Maintainer-facing statement of what the section should cover. |
| `content_uri` | optional | URI reference | Location of section content. Required when the section has content; absent for pure containers. |
| `content_type` | optional | non-empty string | Media type, default `text/markdown`; use RFC 6838 syntax by convention. |
| `content_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the decoded content bytes. |
| `content_length` | optional | integer, minimum 0 | Decoded content byte count: the same bytes `content_hash` covers. This is an advisory, untrusted size hint for pull budgeting, not a token estimate. |
| `language` | optional | language pattern `[A-Za-z0-9]+(-[A-Za-z0-9]+)*` | Per-section language override. |
| `source_ids` | conditionally REQUIRED | array of unique source ids, each `[a-z0-9_-]`, ≤64 chars | Every section with `content_uri` MUST cite at least one `source_ids` entry. |
| `provenance` | optional | array, 1-256 Claim objects | Inline claim-level provenance. |
| `provenance_uri` | optional | URI reference | Per-section provenance sidecar. |
| `provenance_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the sidecar bytes. |
| `provenance_length` | optional | integer, minimum 0 | Sidecar byte count: the same bytes `provenance_hash` covers. An advisory, untrusted size hint, like `content_length`. |
| `links` | optional | array, max 256 Link objects | Typed cross-references. |
| `refreshed_at` | optional | RFC 3339 UTC timestamp | Section last content-edit timestamp, set by maintainer or infra. It is not derived and has no required relationship to source `captured_at`. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

A section MAY have `content_uri`, child sections, or both. It MUST NOT have neither. A section with no `content_uri` MUST have at least one child section whose `parent_id` names it.

`content_hash` and `provenance_hash` use the SRI-style `<algo>-<base64>` shape. The payload is canonical base64 (RFC 4648 §4), not base64url. Validators and consumers MUST support `sha256`; publishers SHOULD stamp `sha256` digests. An unknown algorithm is an unverifiable warning, not an invalid descriptor (§7).

Language inheritance is section, then descriptor, then unspecified. Validators check the language pattern and well-formedness used by the schema; v1 does not require registry-level BCP 47 validation.

Non-normatively, parallel translations of the same material are represented as separate per-language AKBs, or as sibling sections, cross-linked with a custom rel such as `org.example:translation-of`. A controlled `translation-of` rel is a v1.1 candidate.

### §4.4 Provenance

Provenance is bound to what was written at authoring time. It terminates in raw sources. An AKB is never evidence for another AKB.

OpenAKB defines three provenance layers:

1. Section to `source_ids`, the required floor. A section with `content_uri` MUST cite at least one source id.
2. Inline citations in Markdown content, recommended for `text/markdown`.
3. Claim-level provenance, either as inline `provenance` objects or a per-section sidecar at `provenance_uri`.

The inline citation grammar is normative, and it is matched against the **raw Markdown source** rather than against rendered inline text:

- A citation is `[cite: <id-list>]` -- literal `[cite:`, the list, literal `]`.
- `<id-list>` is one or more source `id`s separated by commas: `[cite: a]`, `[cite: a, b, c]`.
- Optional horizontal whitespace is allowed after `cite:` and around each comma; each `id` matches the `[a-z0-9_-]`, ≤64 char local ID grammar, so the tokens are unambiguous.
- Each `id` MUST reference a source declared in the descriptor (checked during content verification, §7).
- Exactly five [CommonMark](https://spec.commonmark.org/) constructs suppress a marker they contain, and their spans MUST be removed from the source before matching: fenced code blocks (```` ``` ```` / `~~~`), indented code blocks, inline code spans (any backtick run length), HTML blocks, and HTML comments. The grammar is then matched literally over what remains. Nothing else affects recognition: there is no backslash escape, no character-reference decoding, and no rule about the brackets, emphasis, or other text adjacent to or enclosing a marker.
- Any bracketed text that does not match the grammar exactly — for example `[cite:]` or `[cite: Bad ID]` — is ordinary literal text. It is never a marker and never an extraction error, and a well-formed marker written beside or within it is still recognized on its own.
- There is no escape syntax in v1. `\[cite: a]` is a marker whose leading backslash is literal, `[[cite: a]]` is a marker with a literal bracket on each side, and `[cite: _a_]` cites the id `_a_` (an underscore is an id character, not emphasis). Because matching reads the raw source, a character reference does not stand in for a delimiter, so `&#91;cite: a]` — which has no literal `[` — is literal text, not a marker. To keep a literal `[cite: …]` out of provenance, place it in an inline code span or a code block.

The extraction output contract is normative. A conformant extractor reports an ordered list of citation entries, one entry per recognized marker, in document order. Each entry carries the marker's source `id` list in written order. Duplicate ids within one marker are preserved as written; a validator MAY warn on them.

Concatenated markers, such as `[cite: a][cite: b]`, are permitted and are provenance-equivalent to a combined list. That equivalence is a statement about provenance semantics only — it is not a normalization license, and extraction MUST still report one entry per marker.

Inline claim-level provenance uses the Section `provenance` array, capped at 256 claims per section; the sidecar at `provenance_uri` is the overflow path for larger claim sets. Each Claim object:

- has required `text` and `source_ids`; `source_ids` MUST contain at least one source id, its entries are unique, and each id uses the `[a-z0-9_-]`, ≤64 char local ID grammar.
- MAY include `locator` with `quote`, `page`, or `anchor`; `quote`, when present, is a non-empty string, and `page` is an integer greater than or equal to 0.
- MAY carry its own `x` extension object, as MAY the `locator`.

`locator.quote` SHOULD be a verbatim span of the cited source's captured content (§4.2), so the quote remains checkable against the capture even after the live source changes.

The provenance sidecar is a JSON object conforming to `schema/v1/provenance.schema.json`. It has optional `$schema`, required `section_id`, and required `claims`. Sidecar `section_id` and claim `source_ids` use the `[a-z0-9_-]`, ≤64 char local ID grammar; each sidecar claim's `source_ids` likewise contains at least one entry, and its entries are unique. Its shape is:

```json
{
  "$schema": "https://schema.openakb.org/v1/provenance.schema.json",
  "section_id": "configuration",
  "claims": [
    {
      "text": "Configuration keys have defaults.",
      "source_ids": ["product-docs"],
      "locator": { "anchor": "configuration-defaults" }
    }
  ]
}
```

The sidecar `section_id` MUST equal the descriptor section `id` that points at it via `provenance_uri`. Claim `source_ids` MUST reference sources declared in the descriptor. `locator.anchor` points into the cited source's internal structure; it is source-internal and is not an AKB `section_id`.

Inline claim provenance and sidecar provenance are alternative encodings. A validator MAY cross-check when both are present.

### §4.5 Links

Links are navigation, not provenance. They express related local or cross-AKB context that a reader MAY choose to resolve.

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `rel` | REQUIRED | controlled value or reverse-DNS escape | Controlled values are `see-also`, `related`, `depends-on`, `prerequisite`, `extends`, and `part-of`. Escape form is `prefix:suffix`, where `prefix` matches `[a-z0-9-]+(\.[a-z0-9-]+)+` — at least two dot-separated labels, the same reverse-DNS shape as `x` keys — and `suffix` is `[a-z0-9-]+`. |
| `section_id` | optional | section id, `[a-z0-9_-]`, ≤64 chars | On local links, MUST resolve to a section in this AKB. On cross-AKB links, names a target section in the linked AKB. |
| `akb_uri` | optional | URI reference | Target AKB descriptor for cross-AKB links. |
| `revision` | optional | string | Target AKB revision to resolve. A link is pinned if and only if it carries `revision`. |
| `content_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity hint for a pinned target section: the target section's declared `content_hash`, copied at pin time. Meaningful only alongside `section_id`. |
| `description` | optional | string, max 500 chars | Bounded description for manifest-first browsing. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

Every link MUST carry a target: `section_id`, `akb_uri`, or both. A link with neither is meaningless and is invalid (`AKB012`).

The parent tree is an acyclic forest. The link graph is a general network and MAY be cyclic.

Local links MUST resolve offline: a `section_id` with no `akb_uri` MUST name a section in this AKB. Cross-AKB links are best-effort because the target is remote and may change or disappear. Consumers MUST tolerate unresolvable or changed cross-AKB targets.

**Pinning.** `revision` and `content_hash` are meaningful only on links with `akb_uri`; they are ignored on local links. A link's `content_hash` covers the decoded content bytes of the target section named by `section_id` — the same bytes that section's own declared `content_hash` covers (§4.3, §5) — copied from the target descriptor at pin time. On a link that carries `akb_uri` but no `section_id`, `content_hash` has no defined referent and is ignored; a validator MAY warn.

A link's `content_hash` is never a hash of the target descriptor document's bytes: served descriptor bytes are not stable even within one revision, because a provider may absolutize relative references or set `base_uri` (§5), so no descriptor-byte canonicalization exists or is needed.

**Revision fetch.** Offering revision fetch is infrastructure's mechanism, and revision retention is provider policy; the spec mandates neither. A provider that does offer revision fetch is RECOMMENDED to serve pinned revisions at the URI derived from the descriptor's resolved fetch URI — for a cross-AKB link, the resolved `akb_uri` — by inserting the path segments `revisions/<revision>/` immediately before the final segment of that URI's path, with the revision value percent-encoded as a single path segment (RFC 3986). A consumer holding a pinned link can construct this URI directly from the resolved `akb_uri` and the link's `revision`, with no head fetch; the document served there is the served form of that revision, self-contained per §5.

For example, a descriptor fetched from `https://kb.example.org/widget-platform/openakb.json` serves revision `2026-06-28T12-00-00Z-a1b2c3d4` at `https://kb.example.org/widget-platform/revisions/2026-06-28T12-00-00Z-a1b2c3d4/openakb.json`.

The convention's scope:

- It is defined on the URI's path component; a fetch URI carrying a query component is outside the convention.
- A path ending in a slash ends in an empty final segment, and the insertion lands before that empty segment: a descriptor fetched from `https://kb.example.org/widget-platform/` serves the revision at `https://kb.example.org/widget-platform/revisions/<revision>/`.
- A revision the provider does not retain — or a provider that does not offer revision fetch at all — is unavailable under this convention, and the consumer treats the pin as unverifiable (§7), consistent with the best-effort rule for cross-AKB links.

**Drift detection.** A consumer MAY drift-detect by fetching the target head and comparing its top-level `revision` against the link's `revision`, or the target section's declared `content_hash` against the link's `content_hash`, then skipping or surfacing mismatches.

### §4.6 Freshness

`captured_at` is a provenance fact: when a source snapshot was captured. Freshness is represented only by optional source-level hints: `refresh_class` and `cadence`.

`refreshed_at` is separate. It records when a section's content was last edited or regenerated. It is set directly by the maintainer or infrastructure and is not derived from the section's sources.

Omitting all freshness fields leaves the AKB structurally complete.

### §4.7 Maintainer guide

The optional top-level `guide_uri` points at a maintainer-facing Markdown guide, conventionally named `AKB.md`. The guide can describe how sections are grouped, naming or taxonomy rules, and how an agent or maintainer should add, update, or remove material. It complements per-section `purpose` with global organizing guidance.

The guide is not a consumer-facing connector and is not required for conformance.

The optional top-level `guide_hash` and `guide_length` (§4.1) pin the guide's decoded bytes exactly as section `content_hash` and `content_length` pin section content, so a consumer that mirrors a descriptor plus its referenced payloads can hash-verify the guide like every other hosted payload.

### §4.8 Non-goals

A curation-only or link-hub AKB with no first-party content and no raw sources is intentionally unrepresentable. OpenAKB is provenance-first: an AKB with content cites raw sources, and empty nodes are forbidden.

This floor is structural, not adversarial. First-party knowledge with no earlier document behind it is not a workaround case: it is cited honestly through the RECOMMENDED `type: "firsthand"` source convention (§4.2). What remains out-of-contract is the fabricated phantom source — a source that stands for nothing and answers to no one, created only so a contentless link hub can satisfy the schema — even though v1 does not block it mechanically.

The 100,000-item cap on `sources` and the 10,000-item cap on `sections` are likewise deliberate bounded-manifest limits, not an oversight. Non-normatively, a corpus that outgrows them should be split into multiple AKBs joined by `part-of` links from a small index AKB. Deterministic multi-AKB composition semantics are a v1.1 candidate.

## §5 URI resolution and authoring-vs-served form

`content_uri`, `guide_uri`, `provenance_uri`, `akb_uri`, `base_uri`, and source `uri` are RFC 3986 URI references. Relative references resolve against the explicit top-level `base_uri` if present; otherwise they resolve against the location from which the descriptor was retrieved. A relative `base_uri` is itself first resolved against the retrieval URI per RFC 3986; the result then serves as the base for the descriptor's other relative references.

Providers SHOULD serve absolute, self-contained URIs, or set `base_uri`, so a descriptor remains resolvable when detached from its origin.

`content_uri` paths are opaque and unstandardized. A provider may serve content under any path scheme. The descriptor is only the mapping from sections to content. Fragments in `content_uri` are permitted but opaque to this specification: consumers dereference the full resource, and `content_hash` / `content_length` always cover the full resource bytes, never a fragment.

| Form | `content_uri` | Who uses it |
| --- | --- | --- |
| Served / canonical | absolute, self-contained | Read-side consumers. |
| Working copy | relative, with files on disk | Maintainers editing in place. |

External source URLs, meaning a Source with `type` `url`, are original web locations and are never rewritten to local paths. A Source with an unknown or extended `type` follows the `type: url` rule: its `uri` is never rewritten on serve.

One narrow exception exists for identity withholding: a provider MAY serve a redacted projection of a source whose identity is withheld from the served audience. A redacted projection MUST use the redacted source form (§4.2) — `type: "redacted"`, with `uri` rewritten to the provider-hosted stub — and this is the only case in which a source `uri` that would otherwise be carried verbatim is rewritten on serve.

Authoring form vs served form:

| Field | Authoring form | Served form | Who populates |
| --- | --- | --- | --- |
| `content_uri`, `provenance_uri`, `guide_uri` | relative to the descriptor | absolute, self-contained | provider (absolutize on serve) |
| `base_uri` | usually absent | MAY be set to the canonical root | provider |
| `revision` | absent | present (minted) | provider |
| `content_hash`, `provenance_hash` | optional/absent | stamped from the served bytes | provider (publish-time) |
| section `content_length`, `provenance_length` | optional/absent | stamped from the served bytes | provider (publish-time) |
| `guide_hash`, `guide_length` | optional/absent | stamped from the served bytes | provider (publish-time) |
| `refreshed_at` | MAY be author-set | MAY be set/updated | maintainer or provider (not derived) |
| source `uri` (`type: url`) | absolute | unchanged (never rewritten) | — |
| source `uri` (`type: file`) | relative | absolute, self-contained | provider |
| source `content_hash`, `capture_uri`, `content_length` | optional/absent; `capture_uri` MAY be a relative capture path | `capture_uri` absolute, self-contained; `content_hash` and `content_length` stamped from the captured bytes | provider (capture-time) |

Everything else (`id`, `namespace`, `title`, `sources[]`, `sections[]` structure, `source_ids`, `links`, `subject_type`, `tags`, `language`, `refresh_class`, `cadence`, `discovered_via_id`) is author-supplied — `discovered_via_id` is set by the maintainer or infrastructure at discovery time — and carried verbatim, except for a provider-served redacted projection using the redacted source form (§4.2).

A validator running on the authoring form MUST treat every serve-only field as optional.

The directory [examples/widget-platform-served/](../../examples/widget-platform-served/) illustrates served form.

### §5.1 Detach: served form to working copy

The inverse transformation — fetching a served descriptor and materializing it as an editable local working copy — is exercised by every pull, mirror, and fork workflow. The following detach procedure is RECOMMENDED so that independently written tools produce working copies that behave alike. A *hosted reference* below is a URI that resolves under the descriptor's own root: `base_uri` when set, otherwise the retrieval URI.

1. **Resolve, then relativize hosted references.** For each hosted `content_uri`, `provenance_uri`, `guide_uri`, and `type: file` source `uri`: resolve it per §5, fetch the payload to a local file, and rewrite the reference as a relative path to that file. Keeping a hosted reference absolute is NOT RECOMMENDED: after local edits, the descriptor silently keeps resolving to the remote, pre-edit content. A hosted `capture_uri` MAY be mirrored and relativized in the same way, or kept absolute: captures are immutable evidence pinned by the source `content_hash` and sized by `content_length`, so a remote reference stays truthful after local edits.
2. **Drop `base_uri`.** It exists to make the served form self-contained; carried into a working copy, it makes the copy's relative references resolve remotely.
3. **Drop `revision`.** It is provider-minted and serve-only (§5); a stale value in a working copy misdescribes the copy, and the provider mints a fresh one on the next publish.
4. **Keep stamped hashes and lengths through the pull; treat them as stale after the first edit.** `content_hash`, `content_length`, `provenance_hash`, `provenance_length`, `guide_hash`, and `guide_length` are exactly what make the just-pulled copy verifiable: verify them against the fetched bytes at detach time. After any local edit they describe bytes that no longer exist; a detach tool MAY drop them at first edit or leave them to be restamped at the next publish (§5).
5. **Keep external references verbatim.** Source `uri` values of `type: url`, of unknown or extended types — including `type: "redacted"` stubs (§4.2) — and cross-AKB `akb_uri` values are original locations; the never-rewritten rule (§5) holds in both directions. The one exception is a relative `akb_uri`: it names a remote descriptor, not a local file, so it is rewritten to its §5-resolved absolute form before step 2 removes the base it resolves against. All author-supplied fields, including `refreshed_at`, carry verbatim.

A validator running on a working copy already treats every serve-only field as optional (§5), so a partially detached descriptor remains valid throughout migration; a detached-then-edited copy simply loses its stamps until republished. The [examples/widget-platform-served/](../../examples/widget-platform-served/) and [examples/widget-platform/](../../examples/widget-platform/) directories illustrate the two ends of this round trip.

## §6 Extensions and versioning

Namespaced extensions live under `x` objects at the top level, Source level, Section level, Link level, and Claim level (including claim locators). Extension keys MUST be reverse-DNS namespaces controlled by the extension author, such as `com.example` or `org.example.tools`. Extension values MUST be objects. Extension data MUST NOT be added as unknown core fields when it can be placed under `x`.

OpenAKB uses SemVer for spec releases. MAJOR versions are breaking. MINOR versions are additive and backward-compatible. PATCH versions are editorial or corrective. Spec releases are tagged as full SemVer, such as `v1.0.0`, `v1.0.1`, and `v1.1.0`.

The descriptor `$schema` MUST be either the major-keyed URI `https://schema.openakb.org/v1/openakb.schema.json`, which tracks the latest backward-compatible 1.x schema, or, for reproducibility, an immutable SemVer pin of the form `https://schema.openakb.org/v1.X.Y/openakb.schema.json` such as `https://schema.openakb.org/v1.0.0/openakb.schema.json`. A v1 validator MUST reject any other `$schema` value as `AKB011`.

`$schema` is also the version-detection surface: a loader that encounters a different major key MUST NOT treat the document as an OpenAKB v1 descriptor, and decides for itself whether to reject or attempt a best-effort read.

The compatibility contract uses one lenient schema per major. Within a major, new minor versions add only optional core fields or optional modules. Old documents continue to validate against newer schemas because additions are optional. New-minor documents also remain usable with older schemas because core objects are lenient toward unknown members.

Strictness is a validator mode, not a second schema. The lenient default tolerates unknown core members for forward compatibility. In its optional **strict mode**, a validator flags a member outside the known core set and not under `x` as `AKB006 unknown-core-property`. How a validator exposes strict mode — a command-line flag, an API parameter, a configuration toggle — is an implementation choice outside this specification.

## §7 Validation and error codes

Structural validation runs offline on the descriptor. A provenance sidecar (§4.4) is a separate fetched artifact, so its checks are part of content verification, not structural validation. Validators MUST emit stable error codes for violations. Conformance is asserted on error codes, not only on pass/fail verdicts.

The following structural rules are normative:

- Schema-required fields MUST be present at every level: top-level, Source, Section, Link, and Claim.
- Source and Section `id`s MUST be unique across one shared id space.
- Top-level `id`, `namespace`, Source and Section `id`, and all local ID references using the local ID grammar MUST match `[a-z0-9_-]` and be ≤64 chars. This includes `parent_id`, Section `source_ids`, Source `discovered_via_id`, sidecar `section_id`, sidecar claim `source_ids`, inline `[cite:]` ids, and local link `section_id`.
- Every section MUST have `content_uri` or at least one child.
- Every section with `content_uri` MUST cite at least one `source_ids` entry.
- The `parent_id` graph MUST be acyclic.
- References MUST resolve to existing ids of the right kind.
- Local links MUST resolve. Cross-AKB links are best-effort and are not an offline structural failure.
- `rel` MUST be in the controlled vocabulary or match the reverse-DNS escape pattern.
- Every link MUST carry `section_id`, `akb_uri`, or both.
- All schema type, charset, timestamp, URI-reference, language, hash, length, cardinality, and depth constraints MUST hold.

Normative bounded-manifest caps:

| Field or shape | Cap |
| --- | --- |
| top-level `title` | <=200 chars |
| AKB `description` | <=2000 chars |
| section `title` | <=200 chars |
| section `description` | <=2000 chars |
| section `purpose` | <=2000 chars |
| source `title` | <=200 chars |
| link `description` | <=500 chars |
| `tags` | <=32 tags, each <=40 chars |
| `sections` | <=10000 |
| `sources` | <=100000 |
| per-section `links` | <=256 |
| per-section inline `provenance` | <=256 claims |
| `parent_id` depth | <=64 |

Depth is the number of nodes on the `parent_id` chain including the section itself: a root section has depth 1, its children depth 2, and so on. The deepest permitted section has depth 64.

The caps are properties of the interchange artifact, not provider capacity declarations. A provider MAY enforce stricter operational limits on what it accepts or serves; that is infrastructure policy outside this specification. A descriptor exceeding a cap is not a conformant OpenAKB descriptor even if a particular provider accepts it.

Caps are fixed within a major version — raising one is a breaking change — so a corpus that outgrows them uses the multi-AKB composition pattern (§4.8) rather than a larger manifest.

Error-code catalog:

| Code | Name | Rule (MUST) |
| ------ | ------ | ------------- |
| `AKB001` | `id-not-unique` | `id`s unique across the shared source+section id space. |
| `AKB002` | `empty-section` | Every section has a `content_uri` or ≥1 child. |
| `AKB003` | `missing-source-cite` | Every section with `content_uri` cites ≥1 `source_ids`. |
| `AKB004` | `parent-cycle` | The `parent_id` graph is acyclic. |
| `AKB005` | `cap-exceeded` | Every length and cardinality/depth cap respected. |
| `AKB006` | `unknown-core-property` | (**strict mode only**) member outside the known core set and not under `x`; valid under the lenient default. |
| `AKB007` | `unresolved-reference` | A `parent_id`, `source_ids` entry, `discovered_via_id`, inline `[cite:]` id, or local link `section_id` names an id that does not exist in the AKB. |
| `AKB008` | `unknown-rel` | `rel` in the controlled vocab or a reverse-DNS `prefix:suffix` escape. |
| `AKB009` | `missing-required-field` | Every schema-required field present, at every level: top-level, source, section, link, and claim. |
| `AKB010` | `invalid-reference-kind` | A reference resolves to an entity of the **wrong kind** (`source_ids`/`[cite:]`/`discovered_via_id` → a section; `parent_id`/local link → a source). |
| `AKB011` | `malformed-value` | Charset (`[a-z0-9_-]`), format (RFC 3339 UTC / RFC 3986), and type constraints hold. |
| `AKB012` | `link-missing-target` | Every link carries `section_id`, `akb_uri`, or both. |

The bounded-manifest caps in v1 are the caps listed above and the schema's matching max items and max lengths. The ≤64 char local ID grammar limit is a schema/global identifier constraint, not a content payload, URI, revision, source-type, claim-text, or anchor cap.

The schema is authoritative for mechanical field types and formats. Claim `text` is deliberately uncapped; the per-section claim-count cap bounds the manifest instead.

When validation is performed with the published JSON Schema, keyword violations map to codes normatively, so independent validators emit identical codes for identical documents:

| JSON Schema keyword | Code |
| --- | --- |
| `maxLength`, `maxItems` (and the non-schema depth cap) | `AKB005` |
| the section content-cite rule (the section-level `if`/`then` requiring a non-empty `source_ids` when `content_uri` is present), including its `required` and `minItems` branch errors | `AKB003` |
| the Link target rule (the link-level `anyOf` requiring `section_id` or `akb_uri`) | `AKB012` |
| `rel`'s `anyOf` (controlled value or reverse-DNS escape), including its branch errors | `AKB008` |
| `required` (elsewhere) | `AKB009` |
| `pattern`, `format`, `type`, `minimum`, `minLength`, `minItems`, `uniqueItems`, `propertyNames`, `enum` (elsewhere) | `AKB011` |

`pattern` is a JSON Schema regular expression, so it is written and matched against the ECMA-262 regex dialect. Schema patterns use explicit ASCII digit classes (`[0-9]`) rather than `\d`: ECMA-262's `\d` is already ASCII-only, but some validator implementations run patterns through a host regex engine whose `\d` matches the wider set of Unicode decimal digits by default, and `[0-9]` sidesteps that divergence so independent validators converge on identical codes for identical documents regardless of host engine. The one v1 schema pattern with a digit class, `$defs/timestamp`, uses `[0-9]` for exactly this reason.

Conformance-fixture match semantics are also normative: a validator passes an invalid fixture if and only if it emits every code listed in the fixture's `codes` array. Extra codes are permitted only when they report distinct additional violations; duplicate emissions of a code are ignored.

Deeper checks that require fetching referenced content are an opt-in **content-verification** mode, distinct from the default offline structural validation. Whether a validator offers content verification, and how it is invoked — a command-line flag, a separate API entry point, a configuration option — is an implementation choice this specification does not constrain; the spec defines only which checks run and what each yields. During content verification, inline `[cite:]` resolution failures emit `AKB007`. Section `content_hash` and `provenance_hash` can be verified against fetched bytes, and the top-level `guide_hash` likewise against the fetched guide bytes.

A fetched provenance sidecar is additionally checked against the descriptor: it MUST conform to `schema/v1/provenance.schema.json`, its `section_id` and claim `source_ids` MUST resolve to declared ids of the right kind per `AKB007`/`AKB010`, and a fetched sidecar whose `section_id` names a section other than the one referencing it via `provenance_uri` (§4.4) is a failed content check.

When a source carries `capture_uri`, the fetched capture can be verified against the source `content_hash`, and claim `locator.quote` values can be checked as substrings of the capture (§4.2). Validators and consumers MUST support `sha256` (§4.3); an unknown hash algorithm is an unverifiable warning, not an invalid descriptor.

Content checks against a `type: "redacted"` source (§4.2) are unverifiable by construction and MUST be reported as unverifiable; a validator MAY warn when a claim `locator.quote` cites a redacted source, since the quote cannot be checked against any capture.

Content checks yield three outcomes: verified, failed, and unverifiable. A validator MUST report an unresolvable or unfetchable URI — authentication required, an unsupported scheme, network unavailable, a capture not retained — as unverifiable, never as a structural failure; a conformance verdict never changes because of access. Only material that was actually fetched and fails its check is a failure, such as a hash mismatch, malformed or inconsistently bound fetched sidecar, absent quote span, fetched section content that cannot be decoded as UTF-8, or a `[cite:]` id in fetched content that resolves to nothing.

> This completes a pattern already in the spec: cross-AKB links are best-effort rather than offline structural failures (§4.5), and an unknown hash algorithm is an unverifiable warning rather than an invalid descriptor.

## §8 Security considerations (non-normative)

All AKB content is untrusted input until verified, including first-party section content and content reached through links. Descriptors and resolved payloads may be read by agents, so prompt-injection and data-exfiltration risks apply.

Resolvers and validators are advised to use URI scheme allowlists per field. For example, automated fetchers can restrict remote content to `https`, and allow `file` or relative references only in a local working copy. Unexpected schemes such as `data:`, `javascript:`, and `blob:` should not be auto-fetched. A URI skipped by a scheme allowlist is reported as unverifiable, not as a failure (§7).

URI resolution can create SSRF and local-file traversal risks. Fetching should be sandboxed and egress-restricted where possible.

Content behind URIs is unbounded payload. Consumers should apply size, time, and redirect-count limits to every fetch, and should stream with caps rather than buffering whole responses.

Integrity is not authenticity. `content_hash` and `provenance_hash` can show that fetched bytes match a stamped value; they do not prove who authored the descriptor or content. OpenAKB v1 defines no descriptor-signing or author-attestation primitive.

Redaction is projection, not protection. The redacted source form (§4.2) only withholds a source's identity from a served document; it grants no confidentiality by itself, and enforcing who may fetch which projection remains access control outside this specification. The retained fields are themselves part of the projection: the source `id` and any `captured_at` stay visible, and an `id` that describes its subject discloses that description.

## §9 Worked example

The directory [examples/widget-platform/](../../examples/widget-platform/) contains an authoring-form AKB for the fictional Widget Platform. The directory [examples/widget-platform-served/](../../examples/widget-platform-served/) contains a served-form illustration with absolute URIs, a minted `revision`, and stamped hashes and sizes.

Authoring descriptor excerpt:

```json
{
  "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
  "namespace": "example",
  "id": "widget-platform",
  "title": "Widget Platform",
  "description": "A sample knowledge base describing a fictional product, used to illustrate the OpenAKB format.",
  "language": "en",
  "guide_uri": "AKB.md",
  "sources": [
    {
      "id": "product-docs",
      "type": "url",
      "uri": "https://docs.example.com/widget-platform/",
      "captured_at": "2026-06-28T00:00:00Z",
      "refresh_class": "polled",
      "cadence": "weekly"
    }
  ],
  "sections": [
    {
      "id": "overview",
      "title": "Overview",
      "description": "What the Widget Platform is and its core model.",
      "content_uri": "sections/overview.md",
      "source_ids": ["product-docs"]
    }
  ]
}
```

One Markdown content line with an inline citation:

```markdown
The Widget Platform organizes work as a set of configurable widgets [cite: product-docs].
```

## §10 References

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)
- [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174)
- [RFC 3339](https://www.rfc-editor.org/rfc/rfc3339)
- [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986)
- [RFC 4648](https://www.rfc-editor.org/rfc/rfc4648)
- [RFC 6838](https://www.rfc-editor.org/rfc/rfc6838)
- [RFC 2606](https://www.rfc-editor.org/rfc/rfc2606)
- [BCP 47](https://www.rfc-editor.org/info/bcp47)
- [Subresource Integrity](https://www.w3.org/TR/SRI/)
- [CommonMark](https://spec.commonmark.org/)
- [ECMA-262](https://tc39.es/ecma262/)
- [JSON Schema](https://json-schema.org/specification)

This document is licensed under [CC-BY-4.0](../../LICENSE-DOCS).
