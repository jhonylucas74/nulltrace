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

    #[error("YAML error: {0}")]
    YamlError(String),

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
}

impl From<serde_yaml::Error> for NtmlError {
    fn from(err: serde_yaml::Error) -> Self {
        if let Some(location) = err.location() {
            NtmlError::ParseError {
                line: location.line(),
                column: location.column(),
                message: err.to_string(),
            }
        } else {
            NtmlError::YamlError(err.to_string())
        }
    }
}
