//! # NullTrace UI Markup Language (NTML) Parser
//!
//! A secure, YAML-based UI description language for NullTrace game.
//!
//! ## Features
//! - Type-safe component parsing
//! - Comprehensive validation with detailed error messages
//! - Support for all NTML components and styles
//! - Theme system with variable interpolation
//!
//! ## Example
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

pub mod components;
pub mod error;
pub mod parser;
pub mod style;
pub mod theme;
pub mod validator;

pub use components::Component;
pub use error::{NtmlError, NtmlResult};
pub use parser::parse_ntml;
pub use style::Style;
pub use theme::Theme;

/// Parse and validate an NTML document
pub fn parse(yaml: &str) -> NtmlResult<Component> {
    parse_ntml(yaml)
}

/// Parse and validate an NTML document with custom theme
pub fn parse_with_theme(yaml: &str, theme: Theme) -> NtmlResult<Component> {
    parser::parse_ntml_with_theme(yaml, theme)
}
