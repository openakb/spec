//! Stable code catalog.

use std::fmt;

use serde::ser::{Serialize, Serializer};

/// Maximum allowed depth for parent chains.
pub const PARENT_DEPTH_MAX: usize = 64;

/// Maximum structural nesting depth a descriptor may reach before
/// [`crate::validate`] declines the recursive schema walk and reports `AKB011`
/// instead of risking a stack-overflow abort.
///
/// [`serde_json::from_str`] caps deserialization nesting at 128, so every
/// descriptor obtained by parsing text has a structural depth of at most 128.
/// This cap sits an order of magnitude above that, so no parsed descriptor can
/// ever reach it (zero false positives on real input), while staying far below
/// the empirically observed ~150,000-level depth at which the recursive
/// `jsonschema` structural walk overflows the stack and aborts. The wide margin
/// on both sides keeps validation total without rejecting any realistic input.
pub const STRUCTURAL_DEPTH_MAX: usize = 1024;

/// Maximum allowed length for local identifiers.
pub const LOCAL_ID_MAX_LENGTH: usize = 64;

/// Local identifier characters (spec §7). The hyphen is last so this string
/// can also be used directly as a regex character-class body.
pub const LOCAL_ID_CHARSET: &str = "abcdefghijklmnopqrstuvwxyz0123456789_-";

/// Stable OpenAKB diagnostic code.
#[allow(missing_docs)] // Variant meanings are exposed through Code::name and fixed by the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Code {
    Akb001,
    Akb002,
    Akb003,
    Akb004,
    Akb005,
    Akb006,
    Akb007,
    Akb008,
    Akb009,
    Akb010,
    Akb011,
    Akb012,
}

impl Code {
    /// All stable diagnostic codes in ascending order.
    pub const ALL: [Self; 12] = [
        Self::Akb001,
        Self::Akb002,
        Self::Akb003,
        Self::Akb004,
        Self::Akb005,
        Self::Akb006,
        Self::Akb007,
        Self::Akb008,
        Self::Akb009,
        Self::Akb010,
        Self::Akb011,
        Self::Akb012,
    ];

    /// Returns the wire spelling of this diagnostic code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Akb001 => "AKB001",
            Self::Akb002 => "AKB002",
            Self::Akb003 => "AKB003",
            Self::Akb004 => "AKB004",
            Self::Akb005 => "AKB005",
            Self::Akb006 => "AKB006",
            Self::Akb007 => "AKB007",
            Self::Akb008 => "AKB008",
            Self::Akb009 => "AKB009",
            Self::Akb010 => "AKB010",
            Self::Akb011 => "AKB011",
            Self::Akb012 => "AKB012",
        }
    }

    /// Returns the stable human-readable name for this diagnostic code.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Akb001 => "id-not-unique",
            Self::Akb002 => "empty-section",
            Self::Akb003 => "missing-source-cite",
            Self::Akb004 => "parent-cycle",
            Self::Akb005 => "cap-exceeded",
            Self::Akb006 => "unknown-core-property",
            Self::Akb007 => "unresolved-reference",
            Self::Akb008 => "unknown-rel",
            Self::Akb009 => "missing-required-field",
            Self::Akb010 => "invalid-reference-kind",
            Self::Akb011 => "malformed-value",
            Self::Akb012 => "link-missing-target",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for Code {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
