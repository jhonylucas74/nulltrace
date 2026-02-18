use nulltrace_ntml::{parse_component_file, parse_document, NtmlError};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: ntml-validate <file.ntml|file.yaml>");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  ntml-validate ui.ntml");
        eprintln!("  ntml-validate *.ntml");
        process::exit(1);
    }

    let mut exit_code = 0;
    let files: Vec<_> = args[1..].to_vec();

    for file_path in files {
        match validate_file(&file_path) {
            Ok(()) => {
                println!("✓ {} is valid", file_path);
            }
            Err(e) => {
                eprintln!("✗ {} has errors:", file_path);
                print_error(&e);
                exit_code = 1;
            }
        }
    }

    process::exit(exit_code);
}

fn validate_file(path: &str) -> Result<(), NtmlError> {
    let content = fs::read_to_string(path)
        .map_err(|e| NtmlError::ValidationError(format!("Failed to read file: {}", e)))?;

    // Component files (with "component:" key) use parse_component_file
    // Full format (with "head:") and classic use parse_document
    if content.lines().any(|l| l.trim_start().starts_with("component:")) {
        parse_component_file(&content)?;
    } else {
        parse_document(&content)?;
    }
    Ok(())
}

fn print_error(error: &NtmlError) {
    match error {
        NtmlError::ParseError {
            line,
            column,
            message,
        } => {
            eprintln!("  Parse error at line {}, column {}:", line, column);
            eprintln!("    {}", message);
        }
        NtmlError::ValidationError(msg) => {
            eprintln!("  Validation error:");
            eprintln!("    {}", msg);
        }
        NtmlError::InvalidComponent { component, reason } => {
            eprintln!("  Invalid component '{}':", component);
            eprintln!("    {}", reason);
        }
        NtmlError::InvalidProperty {
            component,
            property,
            reason,
        } => {
            eprintln!("  Invalid property '{}' for component '{}':", property, component);
            eprintln!("    {}", reason);
        }
        NtmlError::InvalidStyle { property, reason } => {
            eprintln!("  Invalid style property '{}':", property);
            eprintln!("    {}", reason);
        }
        NtmlError::InvalidColor { value, reason } => {
            eprintln!("  Invalid color value '{}':", value);
            eprintln!("    {}", reason);
        }
        NtmlError::InvalidDimension { value } => {
            eprintln!("  Invalid dimension value '{}':", value);
            eprintln!("    Must be a number or 'auto'");
        }
        NtmlError::InvalidEnum {
            property,
            value,
            expected,
        } => {
            eprintln!("  Invalid enum value '{}' for property '{}':", value, property);
            eprintln!("    Expected one of: {}", expected);
        }
        NtmlError::MissingProperty {
            component,
            property,
        } => {
            eprintln!(
                "  Missing required property '{}' for component '{}'",
                property, component
            );
        }
        NtmlError::MaxNestingDepthExceeded { max_depth } => {
            eprintln!("  Maximum nesting depth ({}) exceeded", max_depth);
            eprintln!("    Components are nested too deeply");
        }
        NtmlError::ThemeVariableNotFound { variable } => {
            eprintln!("  Theme variable '{}' not found", variable);
        }
        NtmlError::InvalidThemeReference { reference } => {
            eprintln!("  Invalid theme variable reference: {}", reference);
        }
        NtmlError::AssetNotWhitelisted { path } => {
            eprintln!("  Asset path '{}' is not whitelisted", path);
        }
        NtmlError::InvalidAction { action } => {
            eprintln!("  Invalid action: {}", action);
        }
        NtmlError::DeserializationError(msg) => {
            eprintln!("  Deserialization error:");
            eprintln!("    {}", msg);
        }
        NtmlError::YamlError(msg) => {
            eprintln!("  YAML error:");
            eprintln!("    {}", msg);
        }
        NtmlError::MultipleRootComponents => {
            eprintln!("  Multiple root components found");
            eprintln!("    NTML document must have exactly one root component");
        }
        NtmlError::EmptyDocument => {
            eprintln!("  Empty document: no components found");
        }
        NtmlError::ValueOutOfRange {
            property,
            value,
            range,
        } => {
            eprintln!("  Value out of range for '{}':", property);
            eprintln!("    Value: {}", value);
            eprintln!("    Expected range: {}", range);
        }
        // v0.2.0 errors — all displayed via their Display impl
        e => {
            eprintln!("  {}", e);
        }
    }
}
