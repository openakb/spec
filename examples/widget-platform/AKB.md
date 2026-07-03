# Maintaining the Widget Platform AKB

This guide is for maintainers of this example knowledge base. It complements the per-section
`purpose` fields with the global organizing logic.

## Structure

- **Overview** — the root section; what the platform is and its core model.
- **Configuration** — a child of Overview; how settings layer and resolve.

## Conventions

- One source of record (`product-docs`); cite it inline with `[cite: product-docs]` wherever a
  claim is grounded in it.
- The product blog is monitored as a `type: "feed"` listing source (`blog-index`). Each post
  worth grounding a section in becomes its own source carrying
  `discovered_via_id: "blog-index"`; sections cite the post, never the index.
- Add a new section as a child of the most specific existing section, and give every content
  section at least one `source_ids` entry.
