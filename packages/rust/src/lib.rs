//! Validator for OpenAKB agent knowledge base descriptors.
//!
//! Validates a parsed descriptor ([`serde_json::Value`]) against the embedded
//! OpenAKB v1 JSON Schema plus descriptor-local semantic rules. Strict mode
//! adds `AKB006` unknown-core-property findings; content checks are reserved
//! for later phases.

mod catalog;
mod citations;
mod result;
mod schema;
mod semantic;
mod shape;
mod strict;
mod validator;

pub use catalog::*;
pub use citations::*;
pub use result::*;
pub use validator::*;
