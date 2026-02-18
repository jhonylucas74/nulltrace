use serde::{Deserialize, Serialize};
use crate::components::Component;
use crate::head::Head;

/// The result of parsing an NTML document
///
/// Supports two formats:
/// - `Classic`: v0.1.0 format — single root component with no head/body
/// - `Full`: v0.2.0 format — explicit head and body sections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NtmlDocument {
    /// Classic v0.1.0 format: a single root component
    Classic(Component),
    /// Full v0.2.0 format: head metadata + body component
    Full {
        head: Head,
        body: Component,
    },
}

impl NtmlDocument {
    /// Returns a reference to the root component regardless of format
    pub fn root_component(&self) -> &Component {
        match self {
            NtmlDocument::Classic(c) => c,
            NtmlDocument::Full { body, .. } => body,
        }
    }

    /// Returns the head section if this is a Full document
    pub fn head(&self) -> Option<&Head> {
        match self {
            NtmlDocument::Classic(_) => None,
            NtmlDocument::Full { head, .. } => Some(head),
        }
    }

    /// Returns true if this document uses the v0.2.0 Full format
    pub fn is_full(&self) -> bool {
        matches!(self, NtmlDocument::Full { .. })
    }
}
