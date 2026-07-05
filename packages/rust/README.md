# openakb-validate

`openakb-validate` is the Rust validator crate for OpenAKB descriptor documents. It performs
schema, descriptor-local semantic, and strict-profile structural validation, and it provides
opt-in async content checks for descriptor-related bytes supplied through a resolver.

Status: pre-release.

## Installation

```bash
cargo add openakb-validate
```

## Quick Start

```rust
use openakb_validate::{Mode, validate};
use serde_json::json;

let descriptor = json!({
    "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
    "id": "example-akb",
    "title": "Example AKB",
    "description": "A small descriptor for validation examples.",
    "sources": [
        {
            "id": "primary-source",
            "type": "url",
            "uri": "https://docs.example.com/source/"
        }
    ],
    "sections": [
        {
            "id": "overview",
            "title": "Overview",
            "description": "A section grounded in the primary source.",
            "content_uri": "sections/overview.md",
            "source_ids": ["primary-source"]
        }
    ]
});

let result = validate(&descriptor, Mode::Strict);
assert!(result.ok());
```

## Content Checks

Content checks are explicit and resolver-backed. `LocalFileResolver` resolves scheme-less
relative descriptor references under a local base directory.

```rust,no_run
use openakb_validate::{LocalFileResolver, Mode, validate_with_content};
use serde_json::Value;

# async fn run(descriptor: Value) {
let resolver = LocalFileResolver::new("examples/example-akb");
let report = validate_with_content(&descriptor, &resolver, Mode::Strict).await;

assert!(report.ok());
# }
```

## Compatibility

The minimum supported Rust version is 1.88.

## Specification

See the repository [specification](../../specs/v1/spec.md), [schemas](../../schema/v1/),
and [conformance fixtures](../../conformance/README.md).

## License

Apache-2.0. See [LICENSE](../../LICENSE).
