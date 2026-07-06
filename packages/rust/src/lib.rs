//! Validator for OpenAKB agent knowledge base descriptors.
//!
//! The crate provides structural validation for parsed descriptors
//! ([`serde_json::Value`]) against the embedded OpenAKB v1 JSON Schema plus
//! descriptor-local semantic rules. [`Mode::Strict`] adds `AKB006`
//! unknown-core-property findings. Opt-in async content checks verify referenced
//! bytes when a caller supplies a [`Resolver`].
//!
//! ```
//! use openakb_validate::{Mode, validate};
//! use serde_json::json;
//!
//! let descriptor = json!({
//!     "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
//!     "id": "example-akb",
//!     "title": "Example AKB",
//!     "description": "A tiny descriptor for documentation examples.",
//!     "sources": [
//!         {
//!             "id": "primary-source",
//!             "type": "url",
//!             "uri": "https://docs.example.com/source/"
//!         }
//!     ],
//!     "sections": [
//!         {
//!             "id": "overview",
//!             "title": "Overview",
//!             "description": "A section grounded in the primary source.",
//!             "content_uri": "sections/overview.md",
//!             "source_ids": ["primary-source"]
//!         }
//!     ]
//! });
//!
//! let result = validate(&descriptor, Mode::Strict);
//! assert!(result.ok());
//! ```

mod catalog;
mod citations;
mod content;
mod result;
mod schema;
mod semantic;
mod shape;
mod strict;
mod validator;

pub use catalog::*;
pub use citations::*;
pub use content::*;
pub use result::*;
pub use validator::*;
