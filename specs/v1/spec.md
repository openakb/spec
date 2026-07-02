# OpenAKB v1 Specification

Status: Draft

## §1 Conformance language

The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, NOT RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as described in BCP 14 [RFC 2119] [RFC 8174] when, and only when, they appear in all capitals, as shown here.

## §2 Scope

OpenAKB standardizes the declarative description of an agentic knowledge base (AKB). It defines a portable vocabulary for expressing an AKB's identity, sources, section tree, content locations, provenance, and links. It defines no mechanism for storage, serving, search, freshness, authentication, authorization, or transport.

| Concern | In the spec (declaration) | Infra's job (mechanism) |
| --- | --- | --- |
| Provenance | The binding: section to `source_ids`; source to `captured_at`. | Capturing or snapshotting sources, generating the binding, re-serving raw bytes. |
| Freshness | Thin descriptive hints: `refresh_class`, `cadence`, and a last-edit `refreshed_at`. | The refresh loop, change detection, health polling, and honoring cadence. |
| Verification | Per-section `purpose` and the structural rules a validator asserts. | Running judges or refetch validation. |
| Addressing | The `namespace/id` grammar and URI references. | Resolving names, registry authority, and mirroring. |
| Revisions | The `revision` marker as declarative desired state. | Copy-on-write builds, diffing, changelogs, and live-head flips. |
| Auth / transport | Nothing. | Credentials, tokens, signed URLs, and fetching. |

## §3 Naming conventions

Descriptor keys use `snake_case` and lowercase. Timestamps end in `_at` and are RFC 3339 UTC values using `Z` or `+00:00`.

Related-resource URIs end in `_uri`, such as `content_uri`, `guide_uri`, `provenance_uri`, `base_uri`, and `akb_uri`. A bare `uri` is used only when the object itself is a pointer, as with a Source.

ID references end in `_id` or `_ids`, such as `parent_id`, `source_ids`, and `section_id`. An object's own identity is `id`. Arrays of embedded objects use the plural noun, such as `sources`, `sections`, and `links`.

Boolean fields use `is_` or `has_` prefixes. Enum values are lowercase strings.

## §4 Descriptor model

An OpenAKB descriptor is a JSON object, conventionally named `openakb.json`. The descriptor is the bounded manifest; section content and provenance sidecars are payloads behind URIs.

### §4.1 Top-level fields

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `$schema` | REQUIRED | URI | Major-keyed schema URI such as `https://schema.openakb.org/v1/openakb.schema.json`, or an immutable SemVer pin such as `https://schema.openakb.org/v1.0.0/openakb.schema.json`. |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Human-readable AKB id. |
| `namespace` | optional | `[a-z0-9_-]`, ≤64 chars | Owner or grouping segment. |
| `title` | REQUIRED | string, 1-100 chars | Display name. |
| `description` | REQUIRED | string, 1-600 chars | Bounded AKB abstract. |
| `subject_type` | optional | non-empty string | Open subject classification. |
| `tags` | optional | array, max 32 items; each `[a-z0-9-]`, max 40 chars | Discovery and filtering labels. |
| `language` | optional | language pattern `[A-Za-z0-9]+(-[A-Za-z0-9]+)*` | Primary language of AKB content. Sections MAY override. |
| `guide_uri` | optional | URI reference | Maintainer guide, conventionally `AKB.md`. |
| `revision` | optional | string | Infra-minted revision marker, normally present on served descriptors. |
| `base_uri` | optional | URI reference | Explicit base for resolving relative references. |
| `created_at` | optional | RFC 3339 UTC timestamp | Descriptor creation timestamp. |
| `updated_at` | optional | RFC 3339 UTC timestamp | Descriptor update timestamp. |
| `sources` | REQUIRED | array, 1-10000 Source objects | Raw-evidence registry. |
| `sections` | REQUIRED | array, 1-10000 Section objects | Section forest. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

`namespace/id` is a self-declared canonical name. It is not registry-enforced and is not resolvable on its own. Resolution is always by URI, such as the URL or file path from which a descriptor was fetched. Two unrelated AKBs MAY assert the same `namespace/id`. A consumer MUST treat the fetch URI as the identity of record and MUST NOT assume `namespace/id` is globally unique.

The `x` extension object MAY appear at the top level, on sources, on sections, and on links. Extension keys MUST be reverse-DNS names, such as `com.example` or `org.example.tools`, and each extension value is an object.

### §4.2 Source fields

A Source is raw provenance material a section can be grounded in. Sources are not other AKBs; cross-AKB relationships are represented as links.

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Citation target; unique in the shared source and section id space. |
| `type` | REQUIRED | non-empty string | Source kind, such as `url` or `file`; extensible. |
| `uri` | REQUIRED | URI reference | Location of the evidence. External URLs remain absolute. |
| `title` | optional | string, max 200 chars | Display title. |
| `captured_at` | optional | RFC 3339 UTC timestamp | When the source snapshot was captured. |
| `refresh_class` | optional | non-empty string | Freshness policy hint, such as `static`, `polled`, `event`, or `streaming`; descriptive only. |
| `cadence` | optional | non-empty string | Expected refresh interval hint, meaningful only by convention. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

Freshness policy hints live on the Source object. There is no top-level freshness map in v1.

### §4.3 Section fields

The Section is the atomic unit of browse, pull, and grounding. The tree is expressed by `parent_id` plus document order; `id` is position-independent so references and diffs can survive restructuring.

| Field | Req? | Type / rule | Notes |
| --- | --- | --- | --- |
| `id` | REQUIRED | `[a-z0-9_-]`, ≤64 chars | Stable identity; unique in the shared source and section id space. |
| `parent_id` | optional | section `id`, `[a-z0-9_-]`, ≤64 chars | Parent section. Absence means root. |
| `title` | REQUIRED | string, 1-100 chars | Display title. |
| `description` | REQUIRED | string, 1-400 chars | Consumer-facing browse abstract. |
| `purpose` | optional | string, max 400 chars | Maintainer-facing statement of what the section should cover. |
| `content_uri` | optional | URI reference | Location of section content. Required when the section has content; absent for pure containers. |
| `content_type` | optional | non-empty string | Media type, default `text/markdown`; use RFC 6838 syntax by convention. |
| `content_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the decoded content bytes. |
| `content_length` | optional | integer, minimum 0 | Decoded content byte count: the same bytes `content_hash` covers. This is an advisory, untrusted size hint for pull budgeting, not a token estimate. |
| `language` | optional | language pattern `[A-Za-z0-9]+(-[A-Za-z0-9]+)*` | Per-section language override. |
| `source_ids` | conditionally REQUIRED | array of source ids, each `[a-z0-9_-]`, ≤64 chars | Every section with `content_uri` MUST cite at least one `source_ids` entry. |
| `provenance` | optional | array of Claim objects | Inline claim-level provenance. |
| `provenance_uri` | optional | URI reference | Per-section provenance sidecar. |
| `provenance_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity of the sidecar bytes. |
| `links` | optional | array, max 256 Link objects | Typed cross-references. |
| `refreshed_at` | optional | RFC 3339 UTC timestamp | Section last content-edit timestamp, set by maintainer or infra. It is not derived and has no required relationship to source `captured_at`. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

A section MAY have `content_uri`, child sections, or both. It MUST NOT have neither. A section with no `content_uri` MUST have at least one child section whose `parent_id` names it.

Language inheritance is section, then descriptor, then unspecified. Validators check the language pattern and well-formedness used by the schema; v1 does not require registry-level BCP 47 validation.

### §4.4 Provenance

Provenance is bound to what was written at authoring time. It terminates in raw sources. An AKB is never evidence for another AKB.

OpenAKB defines three provenance layers:

1. Section to `source_ids`, the required floor. A section with `content_uri` MUST cite at least one source id.
2. Inline citations in Markdown content, recommended for `text/markdown`.
3. Claim-level provenance, either as inline `provenance` objects or a per-section sidecar at `provenance_uri`.

The inline citation grammar is normative:

- A citation is `[cite: <id-list>]` -- literal `[cite:`, the list, literal `]`.
- `<id-list>` is one or more source `id`s separated by commas: `[cite: a]`, `[cite: a, b, c]`.
- Optional horizontal whitespace is allowed after `cite:` and around each comma; each `id` matches the `[a-z0-9_-]`, ≤64 char local ID grammar, so the tokens are unambiguous.
- Each `id` MUST reference a source declared in the descriptor (checked under `--check-content`).
- The marker is recognized only in Markdown prose; occurrences inside fenced code blocks (```` ``` ```` / `~~~`) or inline code spans (`` ` ``) are literal text and MUST be ignored.
- There is no escape syntax in v1; to write a literal `[cite: …]` in prose, place it in a code span.

Concatenated markers, such as `[cite: a][cite: b]`, are equivalent to a combined list and are permitted.

Inline claim-level provenance uses the Section `provenance` array. Each Claim object has required `text` and `source_ids`; `source_ids` MUST contain at least one source id using the `[a-z0-9_-]`, ≤64 char local ID grammar. A Claim MAY include `locator` with `quote`, `page`, or `anchor`. `page` is an integer greater than or equal to 0.

The provenance sidecar is a JSON object conforming to `schema/v1/provenance.schema.json`. It has optional `$schema`, required `section_id`, and required `claims`. Sidecar `section_id` and claim `source_ids` use the `[a-z0-9_-]`, ≤64 char local ID grammar. Its shape is:

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
| `rel` | REQUIRED | controlled value or reverse-DNS escape | Controlled values are `see-also`, `related`, `depends-on`, `prerequisite`, `extends`, and `part-of`. Escape form is `prefix:suffix`, where `prefix` matches `[a-z0-9-]+(\.[a-z0-9-]+)*` and `suffix` is `[a-z0-9-]+`. |
| `section_id` | optional | section id, `[a-z0-9_-]`, ≤64 chars | On local links, MUST resolve to a section in this AKB. On cross-AKB links, names a target section in the linked AKB. |
| `akb_uri` | optional | URI reference | Target AKB descriptor for cross-AKB links. |
| `revision` | optional | string | Target AKB revision to resolve. A link is pinned if and only if it carries `revision`. |
| `content_hash` | optional | SRI-style hash, `<algo>-<base64>` | Integrity hint for a pinned target. |
| `description` | optional | string, max 200 chars | Bounded description for manifest-first browsing. |
| `x` | optional | reverse-DNS extension object | Namespaced extensions. |

The parent tree is an acyclic forest. The link graph is a general network and MAY be cyclic.

Local links MUST resolve offline: a `section_id` with no `akb_uri` MUST name a section in this AKB. Cross-AKB links are best-effort because the target is remote and may change or disappear. Consumers MUST tolerate unresolvable or changed cross-AKB targets.

`revision` and `content_hash` are meaningful only on links with `akb_uri`; they are ignored on local links. The spec defines no retrieval mechanism for a given `revision`. Non-normatively, infrastructure can offer revision fetch. A consumer MAY drift-detect by fetching the target head and comparing its top-level `revision` or fetched content hash against the link's pin, then skipping or surfacing mismatches.

### §4.6 Freshness

`captured_at` is a provenance fact: when a source snapshot was captured. Freshness is represented only by optional source-level hints: `refresh_class` and `cadence`.

`refreshed_at` is separate. It records when a section's content was last edited or regenerated. It is set directly by the maintainer or infrastructure and is not derived from the section's sources.

Omitting all freshness fields leaves the AKB structurally complete.

### §4.7 Maintainer guide

The optional top-level `guide_uri` points at a maintainer-facing Markdown guide, conventionally named `AKB.md`. The guide can describe how sections are grouped, naming or taxonomy rules, and how an agent or maintainer should add, update, or remove material. It complements per-section `purpose` with global organizing guidance.

The guide is not a consumer-facing connector and is not required for conformance.

### §4.8 Non-goals

A curation-only or link-hub AKB with no first-party content and no raw sources is intentionally unrepresentable. OpenAKB is provenance-first: an AKB with content cites raw sources, and empty nodes are forbidden.

This floor is structural, not adversarial. A determined author can create a phantom source and a stub section that satisfy the schema. Such phantom-source link hubs are out-of-contract even though v1 does not block them mechanically.

## §5 URI resolution and authoring-vs-served form

`content_uri`, `guide_uri`, `provenance_uri`, `akb_uri`, `base_uri`, and source `uri` are RFC 3986 URI references. Relative references resolve against the explicit top-level `base_uri` if present; otherwise they resolve against the location from which the descriptor was retrieved.

Providers SHOULD serve absolute, self-contained URIs, or set `base_uri`, so a descriptor remains resolvable when detached from its origin.

`content_uri` paths are opaque and unstandardized. A provider may serve content under any path scheme. The descriptor is only the mapping from sections to content.

| Form | `content_uri` | Who uses it |
| --- | --- | --- |
| Served / canonical | absolute, self-contained | Read-side consumers. |
| Working copy | relative, with files on disk | Maintainers editing in place. |

External source URLs, meaning a Source with `type` `url`, are original web locations and are never rewritten to local paths.

Authoring form vs served form:

| Field | Authoring form | Served form | Who populates |
| --- | --- | --- | --- |
| `content_uri`, `provenance_uri`, `guide_uri` | relative to the descriptor | absolute, self-contained | provider (absolutize on serve) |
| `base_uri` | usually absent | MAY be set to the canonical root | provider |
| `revision` | absent | present (minted) | provider |
| `content_hash`, `provenance_hash` | optional/absent | stamped from the served bytes | provider (publish-time) |
| `content_length` | optional/absent | stamped from the served bytes | provider (publish-time) |
| `refreshed_at` | MAY be author-set | MAY be set/updated | maintainer or provider (not derived) |
| source `uri` (`type: url`) | absolute | unchanged (never rewritten) | — |
| source `uri` (`type: file`) | relative | absolute, self-contained | provider |

Everything else (`id`, `namespace`, `title`, `sources[]`, `sections[]` structure, `source_ids`, `links`, `subject_type`, `tags`, `language`, `refresh_class`, `cadence`) is author-supplied and carried verbatim. A validator running on the authoring form MUST treat every serve-only field as optional.

The directory [examples/widget-platform-served/](../../examples/widget-platform-served/) illustrates served form.

## §6 Extensions and versioning

Namespaced extensions live under `x` objects at the top level, Source level, Section level, and Link level. Extension keys MUST be reverse-DNS namespaces controlled by the extension author, such as `com.example` or `org.example.tools`. Extension values MUST be objects. Extension data MUST NOT be added as unknown core fields when it can be placed under `x`.

OpenAKB uses SemVer for spec releases. MAJOR versions are breaking. MINOR versions are additive and backward-compatible. PATCH versions are editorial or corrective. Spec releases are tagged as full SemVer, such as `v1.0.0`, `v1.0.1`, and `v1.1.0`.

The descriptor `$schema` is major-keyed by default, such as `https://schema.openakb.org/v1/openakb.schema.json`, and tracks the latest backward-compatible 1.x schema. For reproducibility, a descriptor MAY point at an immutable pin such as `https://schema.openakb.org/v1.0.0/openakb.schema.json`.

The compatibility contract uses one lenient schema per major. Within a major, new minor versions add only optional core fields or optional modules. Old documents continue to validate against newer schemas because additions are optional. New-minor documents also remain usable with older schemas because core objects are lenient toward unknown members.

Strictness is a validator mode, not a second schema. The lenient default tolerates unknown core members for forward compatibility. A validator `--strict` lint flags a member outside the known core set and not under `x` as `AKB006 unknown-core-property`.

## §7 Validation and error codes

Structural validation runs offline on the descriptor and, when present, sidecar files. Validators MUST emit stable error codes for violations. Conformance is asserted on error codes, not only on pass/fail verdicts.

The following structural rules are normative:

- Required top-level, Source, and Section fields MUST be present.
- Source and Section `id`s MUST be unique across one shared id space.
- Top-level `id`, `namespace`, Source and Section `id`, and all local ID references using the local ID grammar MUST match `[a-z0-9_-]` and be ≤64 chars. This includes `parent_id`, Section `source_ids`, sidecar `section_id`, sidecar claim `source_ids`, inline `[cite:]` ids, and local link `section_id`.
- Every section MUST have `content_uri` or at least one child.
- Every section with `content_uri` MUST cite at least one `source_ids` entry.
- The `parent_id` graph MUST be acyclic.
- References MUST resolve to existing ids of the right kind.
- Local links MUST resolve. Cross-AKB links are best-effort and are not an offline structural failure.
- `rel` MUST be in the controlled vocabulary or match the reverse-DNS escape pattern.
- All schema type, charset, timestamp, URI-reference, language, hash, length, cardinality, and depth constraints MUST hold.

Normative bounded-manifest caps:

| Field or shape | Cap |
| --- | --- |
| top-level `title` | <=100 chars |
| AKB `description` | <=600 chars |
| section `title` | <=100 chars |
| section `description` | <=400 chars |
| section `purpose` | <=400 chars |
| source `title` | <=200 chars |
| link `description` | <=200 chars |
| `tags` | <=32 tags, each <=40 chars |
| `sections` | <=10000 |
| `sources` | <=10000 |
| per-section `links` | <=256 |
| `parent_id` depth | <=64 |

Error-code catalog:

| Code | Name | Rule (MUST) |
| ------ | ------ | ------------- |
| `AKB001` | `id-not-unique` | `id`s unique across the shared source+section id space. |
| `AKB002` | `empty-section` | Every section has a `content_uri` or ≥1 child. |
| `AKB003` | `missing-source-cite` | Every section with `content_uri` cites ≥1 `source_ids`. |
| `AKB004` | `parent-cycle` | The `parent_id` graph is acyclic. |
| `AKB005` | `cap-exceeded` | Every length and cardinality/depth cap respected. |
| `AKB006` | `unknown-core-property` | (**`--strict` only**) member outside the known core set and not under `x`; valid under the lenient default. |
| `AKB007` | `unresolved-reference` | A `parent_id`, `source_ids` entry, inline `[cite:]` id, or local link `section_id` names an id that does not exist in the AKB. |
| `AKB008` | `unknown-rel` | `rel` in the controlled vocab or a reverse-DNS `prefix:suffix` escape. |
| `AKB009` | `missing-required-field` | Every required top-level/source/section field present. |
| `AKB010` | `invalid-reference-kind` | A reference resolves to an entity of the **wrong kind** (`source_ids`/`[cite:]` → a section; `parent_id`/local link → a source). |
| `AKB011` | `malformed-value` | Charset (`[a-z0-9_-]`), format (RFC 3339 UTC / RFC 3986), and type constraints hold. |

The bounded-manifest caps in v1 are the caps listed above and the schema's matching max items and max lengths. The ≤64 char local ID grammar limit is a schema/global identifier constraint, not a content payload, URI, revision, source-type, claim-text, or anchor cap. The schema is authoritative for mechanical field types and formats.

Deeper checks that require fetching content are opt-in under `--check-content`. Under `--check-content`, inline `[cite:]` resolution failures emit `AKB007`. `content_hash` and `provenance_hash` can be verified against fetched bytes. An unknown hash algorithm is an unverifiable warning, not an invalid descriptor.

## §8 Security considerations (non-normative)

All AKB content is untrusted input until verified, including first-party section content and content reached through links. Descriptors and resolved payloads may be read by agents, so prompt-injection and data-exfiltration risks apply.

Resolvers and validators are advised to use URI scheme allowlists per field. For example, automated fetchers can restrict remote content to `https`, and allow `file` or relative references only in a local working copy. Unexpected schemes such as `data:`, `javascript:`, and `blob:` should not be auto-fetched.

URI resolution can create SSRF and local-file traversal risks. Fetching should be sandboxed and egress-restricted where possible.

Content behind URIs is unbounded payload. Consumers should apply size, time, and redirect-count limits to every fetch, and should stream with caps rather than buffering whole responses.

Integrity is not authenticity. `content_hash` and `provenance_hash` can show that fetched bytes match a stamped value; they do not prove who authored the descriptor or content. OpenAKB v1 defines no descriptor-signing or author-attestation primitive.

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
- [RFC 6838](https://www.rfc-editor.org/rfc/rfc6838)
- [RFC 2606](https://www.rfc-editor.org/rfc/rfc2606)
- [BCP 47](https://www.rfc-editor.org/info/bcp47)
- [Subresource Integrity](https://www.w3.org/TR/SRI/)

This document is licensed under [CC-BY-4.0](../../LICENSE-DOCS).
