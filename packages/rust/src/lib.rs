//! Validator for OpenAKB agent knowledge base descriptors.
//!
//! Validates a parsed descriptor ([`serde_json::Value`]) against the embedded
//! OpenAKB v1 JSON Schema and reports stable diagnostics `AKB001`-`AKB012`.
//! Semantic, strict-profile, and content checks are reserved for later phases.

mod catalog;
mod result;
mod schema;
mod validator;

pub use catalog::*;
pub use result::*;
pub use validator::*;
