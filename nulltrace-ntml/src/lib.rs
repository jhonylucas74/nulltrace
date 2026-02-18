//! # NullTrace UI Markup Language (NTML) Parser
//!
//! A secure, YAML-based UI description language for NullTrace game.
//!
//! ## Features
//! - Type-safe component parsing
//! - Comprehensive validation with detailed error messages
//! - Support for all NTML components and styles
//! - Theme system with variable interpolation
//! - v0.2.0: head/body format, Lua script declarations, importable components, component IDs
//!
//! ## Example — Classic format (v0.1.0)
//! ```ignore
//! use nulltrace_ntml::parse_ntml;
//!
//! let yaml = r#"
//! Container:
//!   style:
//!     padding: 16
//!     backgroundColor: "#1a1a1a"
//!   children:
//!     - Text:
//!         text: "Hello World"
//! "#;
//!
//! let component = parse_ntml(yaml).expect("Failed to parse NTML");
//! ```
//!
//! ## Example — Full format (v0.2.0)
//! ```ignore
//! use nulltrace_ntml::parse_document;
//!
//! let yaml = r#"
//! head:
//!   title: "My Page"
//!   scripts:
//!     - src: "scripts/main.lua"
//!
//! body:
//!   Text:
//!     id: "greeting"
//!     text: "Hello from v0.2.0"
//! "#;
//!
//! let doc = parse_document(yaml).expect("Failed to parse NTML document");
//! let root = doc.root_component();
//! ```

pub mod component_file;
pub mod components;
pub mod document;
pub mod error;
pub mod head;
pub mod parser;
pub mod style;
pub mod theme;
pub mod validator;

// --- Core types ---
pub use components::Component;
pub use document::NtmlDocument;
pub use error::{NtmlError, NtmlResult};
pub use head::{ComponentImport, FontImport, Head, ScriptImport};
pub use style::Style;
pub use theme::Theme;

// --- Component file types ---
pub use component_file::{ComponentFile, PropDefault, PropDef, PropType};

/// Parse an NTML document — classic format only (v0.1.0 backward compat)
///
/// Returns an error if the document uses the v0.2.0 head/body format.
/// Use [`parse_document`] to support both formats.
pub fn parse_ntml(yaml: &str) -> NtmlResult<Component> {
    parser::parse_ntml(yaml)
}

/// Parse an NTML document — classic format only, with theme
pub fn parse_ntml_with_theme(yaml: &str, theme: Theme) -> NtmlResult<Component> {
    parser::parse_ntml_with_theme(yaml, theme)
}

/// Parse an NTML document — supports both classic (v0.1.0) and full (v0.2.0) formats
pub fn parse_document(yaml: &str) -> NtmlResult<NtmlDocument> {
    parser::parse_document(yaml)
}

/// Parse an NTML document with a custom theme
pub fn parse_document_with_theme(yaml: &str, theme: Theme) -> NtmlResult<NtmlDocument> {
    parser::parse_document_with_theme(yaml, theme)
}

/// Parse an NTML component file (a reusable component definition)
pub fn parse_component_file(yaml: &str) -> NtmlResult<ComponentFile> {
    component_file::parse_component_file(yaml)
}

/// Parse and validate an NTML document (classic format, backward compat alias)
pub fn parse(yaml: &str) -> NtmlResult<Component> {
    parse_ntml(yaml)
}

/// Parse and validate an NTML document with custom theme (backward compat alias)
pub fn parse_with_theme(yaml: &str, theme: Theme) -> NtmlResult<Component> {
    parse_ntml_with_theme(yaml, theme)
}
