//! # NullTrace UI Markup Language (NTML) Parser
//!
//! A secure, XML-based UI description language for the NullTrace game.
//!
//! ## Features
//! - Type-safe component parsing from XML
//! - Comprehensive validation with detailed error messages
//! - Support for all NTML components and styles
//! - Theme system with variable interpolation
//! - Full format: head/body structure, Lua script declarations, importable components, component IDs
//!
//! ## Example — Classic format (single component)
//! ```ignore
//! use nulltrace_ntml::parse_ntml;
//!
//! let xml = r#"
//! <Container style="padding:16; backgroundColor:#1a1a1a">
//!   <Text text="Hello World" />
//! </Container>
//! "#;
//!
//! let component = parse_ntml(xml).expect("Failed to parse NTML");
//! ```
//!
//! ## Example — Full format (head + body)
//! ```ignore
//! use nulltrace_ntml::parse_document;
//!
//! let xml = r#"
//! <head>
//!   <title>My Page</title>
//!   <script src="scripts/main.lua" />
//! </head>
//! <body>
//!   <Text id="greeting" text="Hello from NTML" />
//! </body>
//! "#;
//!
//! let doc = parse_document(xml).expect("Failed to parse NTML document");
//! let root = doc.root_component();
//! ```

pub mod component_file;
pub mod components;
pub mod document;
pub mod error;
pub mod head;
pub mod parser;
pub mod style;
pub mod tailwind;
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

/// Parse an NTML document — classic format only (single root component, no head/body).
///
/// Returns an error if the document uses the full head/body format.
/// Use [`parse_document`] to support both formats.
pub fn parse_ntml(xml: &str) -> NtmlResult<Component> {
    parser::parse_ntml(xml)
}

/// Parse an NTML document — classic format only, with theme
pub fn parse_ntml_with_theme(xml: &str, theme: Theme) -> NtmlResult<Component> {
    parser::parse_ntml_with_theme(xml, theme)
}

/// Parse an NTML document — supports both classic (single component) and full (head + body) formats
pub fn parse_document(xml: &str) -> NtmlResult<NtmlDocument> {
    parser::parse_document(xml)
}

/// Parse an NTML document with a custom theme
pub fn parse_document_with_theme(xml: &str, theme: Theme) -> NtmlResult<NtmlDocument> {
    parser::parse_document_with_theme(xml, theme)
}

/// Parse an NTML component file (a reusable component definition)
pub fn parse_component_file(xml: &str) -> NtmlResult<ComponentFile> {
    component_file::parse_component_file(xml)
}

/// Parse and validate an NTML document (classic format, backward compat alias)
pub fn parse(xml: &str) -> NtmlResult<Component> {
    parse_ntml(xml)
}

/// Parse and validate an NTML document with custom theme (backward compat alias)
pub fn parse_with_theme(xml: &str, theme: Theme) -> NtmlResult<Component> {
    parse_ntml_with_theme(xml, theme)
}
