use thiserror::Error;

pub type NtmlResult<T> = Result<T, NtmlError>;

#[derive(Error, Debug, Clone)]
pub enum NtmlError {
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Invalid component: {component}. {reason}")]
    InvalidComponent { component: String, reason: String },

    #[error("Invalid property '{property}' for component '{component}': {reason}")]
    InvalidProperty {
        component: String,
        property: String,
        reason: String,
    },

    #[error("Invalid style property '{property}': {reason}")]
    InvalidStyle { property: String, reason: String },

    #[error("Invalid color value '{value}': {reason}")]
    InvalidColor { value: String, reason: String },

    #[error("Invalid dimension value '{value}': must be a number or 'auto'")]
    InvalidDimension { value: String },

    #[error("Invalid enum value '{value}' for property '{property}'. Expected one of: {expected}")]
    InvalidEnum {
        property: String,
        value: String,
        expected: String,
    },

    #[error("Missing required property '{property}' for component '{component}'")]
    MissingProperty {
        component: String,
        property: String,
    },

    #[error("Maximum nesting depth ({max_depth}) exceeded")]
    MaxNestingDepthExceeded { max_depth: usize },

    #[error("Theme variable '{variable}' not found")]
    ThemeVariableNotFound { variable: String },

    #[error("Invalid theme variable reference: {reference}")]
    InvalidThemeReference { reference: String },

    #[error("Asset path '{path}' is not whitelisted")]
    AssetNotWhitelisted { path: String },

    #[error("Invalid action '{action}'")]
    InvalidAction { action: String },

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("XML parse error: {0}")]
    XmlError(String),

    #[error("Multiple root components found. NTML document must have exactly one root component")]
    MultipleRootComponents,

    #[error("Empty document: no components found")]
    EmptyDocument,

    #[error("Value out of range for '{property}': {value}. Expected range: {range}")]
    ValueOutOfRange {
        property: String,
        value: String,
        range: String,
    },

    // --- v0.2.0 head errors ---

    #[error("Document has a 'head' section but is missing a 'body' section")]
    MissingBody,

    #[error("Document head is missing required field 'title'")]
    MissingTitle,

    #[error("Invalid font family name '{family}': must be a non-empty string")]
    InvalidFontFamily { family: String },

    #[error("Too many scripts: maximum {max} scripts per document")]
    ScriptLimitExceeded { max: usize },

    #[error("Too many imports: maximum {max} imports per document")]
    ImportLimitExceeded { max: usize },

    #[error("Too many fonts: maximum {max} fonts per document")]
    FontLimitExceeded { max: usize },

    #[error("Invalid import alias '{alias}': must be PascalCase and not conflict with built-in component names")]
    InvalidImportAlias { alias: String },

    #[error("Invalid tag '{tag}': tags must be non-empty, lowercase, and contain no spaces")]
    InvalidTag { tag: String },

    #[error("Too many tags: maximum {max} tags per document")]
    TagLimitExceeded { max: usize },

    // --- v0.2.0 importable component errors ---

    #[error("Unknown imported component '{name}': not declared in head.imports")]
    UnknownImportedComponent { name: String },

    #[error("Missing required prop '{prop}' for component '{component}'")]
    MissingRequiredProp { component: String, prop: String },

    #[error("Invalid type for prop '{prop}' in component '{component}': expected {expected}")]
    InvalidPropType {
        component: String,
        prop: String,
        expected: String,
    },

    #[error("Unknown prop '{prop}' for component '{component}'")]
    UnknownProp { component: String, prop: String },

    #[error("Circular component import detected: '{path}'")]
    CircularComponentImport { path: String },

    #[error("Component file '{path}' cannot have a 'head' section")]
    ComponentFileHasHead { path: String },

    // --- v0.2.0 Lua errors ---

    #[error("Script '{src}' exceeds maximum line limit of {max_lines} lines")]
    LuaScriptTooLong { src: String, max_lines: usize },

    #[error("Lua handler '{handler}' timed out after {timeout_ms}ms")]
    LuaHandlerTimeout { handler: String, timeout_ms: u64 },

    #[error("Lua runtime error in '{src}': {message}")]
    LuaRuntimeError { src: String, message: String },

    #[error("Lua function '{action}' not found: button action references a function that is not defined in any imported script")]
    LuaFunctionNotFound { action: String },

    // --- v0.2.0 ID errors ---

    #[error("Duplicate id '{id}': component ids must be unique within the document")]
    DuplicateId { id: String },

    // --- data-* attribute errors ---

    #[error("Invalid data attribute '{key}': {reason}")]
    InvalidDataAttribute { key: String, reason: String },
}

impl From<roxmltree::Error> for NtmlError {
    fn from(err: roxmltree::Error) -> Self {
        NtmlError::XmlError(err.to_string())
    }
}
