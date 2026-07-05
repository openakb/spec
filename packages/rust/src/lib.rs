//! Validator for OpenAKB agent knowledge base descriptors.
//!
//! Validates a parsed descriptor ([`serde_json::Value`]) against the embedded
//! OpenAKB v1 JSON Schema plus the spec's semantic and strict rules, reporting
//! stable diagnostics `AKB001`-`AKB012`. Optional content checks verify
//! hashes, provenance sidecars, citations, and quotes through an async
//! `Resolver`.

mod catalog;
mod result;
mod schema;
mod validator;

pub use catalog::*;
pub use result::*;
pub use validator::*;
