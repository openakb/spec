//! Validator for OpenAKB agent knowledge base descriptors.
//!
//! Validates a parsed descriptor ([`serde_json::Value`]) against the embedded
//! OpenAKB v1 JSON Schema plus descriptor-local semantic rules. Strict mode
//! adds `AKB006` unknown-core-property findings. Optional async content checks
//! verify referenced bytes where a caller supplies a [`Resolver`].

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
