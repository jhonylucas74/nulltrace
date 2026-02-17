use nulltrace_ntml::{parse_ntml, Component, NtmlError, Theme};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn get_example_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path.push(filename);
    path
}

// Test valid examples
#[test]
fn test_valid_simple_example() {
    let path = get_example_path("valid-simple.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_ok(), "valid-simple.ntml should be valid");
}

#[test]
fn test_valid_complex_example() {
    let path = get_example_path("valid-complex.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_ok(), "valid-complex.ntml should be valid");
}

#[test]
fn test_game_hud_example() {
    let path = get_example_path("game-hud.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_ok(), "game-hud.ntml should be valid");
}

#[test]
fn test_terminal_ui_example() {
    let path = get_example_path("terminal-ui.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_ok(), "terminal-ui.ntml should be valid");
}

#[test]
fn test_mission_briefing_example() {
    let path = get_example_path("mission-briefing.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_ok(), "mission-briefing.ntml should be valid");
}

// Test invalid examples
#[test]
fn test_invalid_color_example() {
    let path = get_example_path("invalid-color.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_err(), "invalid-color.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::InvalidColor { .. }));
}

#[test]
fn test_invalid_component_example() {
    let path = get_example_path("invalid-component.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_err(), "invalid-component.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::InvalidComponent { .. }));
}

#[test]
fn test_invalid_missing_property_example() {
    let path = get_example_path("invalid-missing-property.ntml");
    let yaml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&yaml);
    assert!(result.is_err(), "invalid-missing-property.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::MissingProperty { .. }));
}

// Component tests
#[test]
fn test_text_component() {
    let yaml = "Text:\n  text: \"Hello World\"";
    let result = parse_ntml(yaml);
    assert!(result.is_ok());
    if let Ok(Component::Text(text)) = result {
        assert_eq!(text.text, "Hello World");
    } else {
        panic!("Expected Text component");
    }
}

#[test]
fn test_button_component() {
    let yaml = "Button:\n  action: \"hack_system\"\n  variant: primary";
    let result = parse_ntml(yaml);
    assert!(result.is_ok());
}

#[test]
fn test_progress_bar_component() {
    let yaml = "ProgressBar:\n  value: 75\n  max: 100\n  variant: danger";
    let result = parse_ntml(yaml);
    assert!(result.is_ok());
}

// Validation tests
#[test]
fn test_empty_document() {
    let result = parse_ntml("");
    assert!(result.is_err());
}

#[test]
fn test_empty_text_validation() {
    let yaml = "Text:\n  text: \"\"";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

#[test]
fn test_negative_gap_validation() {
    let yaml = "Flex:\n  gap: -10\n  children:\n    - Text:\n        text: \"Test\"";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

#[test]
fn test_opacity_out_of_range() {
    let yaml = "Text:\n  text: \"Test\"\n  style:\n    opacity: 1.5";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

// Color validation tests
#[test]
fn test_valid_hex_colors() {
    let colors = vec!["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff"];

    for color in colors {
        let yaml = format!("Text:\n  text: \"Test\"\n  style:\n    color: \"{}\"", color);
        let result = parse_ntml(&yaml);
        assert!(result.is_ok(), "Failed for color: {}", color);
    }
}

#[test]
fn test_valid_named_colors() {
    let colors = vec!["red", "blue", "green", "white", "black", "transparent"];

    for color in colors {
        let yaml = format!("Text:\n  text: \"Test\"\n  style:\n    color: {}", color);
        let result = parse_ntml(&yaml);
        assert!(result.is_ok(), "Failed for color: {}", color);
    }
}

#[test]
fn test_invalid_hex_colors() {
    let colors = vec!["#fff", "#12345", "#gggggg", "123456"];

    for color in colors {
        let yaml = format!("Text:\n  text: \"Test\"\n  style:\n    color: \"{}\"", color);
        let result = parse_ntml(&yaml);
        assert!(result.is_err(), "Should fail for color: {}", color);
    }
}

// Theme tests
#[test]
fn test_theme_color_resolution() {
    let mut theme = Theme::new();
    let mut colors = HashMap::new();
    colors.insert("primary".to_string(), "#4a90e2".to_string());
    theme.colors = Some(colors);

    assert_eq!(
        theme.resolve("$theme.colors.primary"),
        Some("#4a90e2".to_string())
    );
}

#[test]
fn test_theme_spacing_resolution() {
    let mut theme = Theme::new();
    let mut spacing = HashMap::new();
    spacing.insert("medium".to_string(), 16.0);
    theme.spacing = Some(spacing);

    assert_eq!(
        theme.resolve("$theme.spacing.medium"),
        Some("16".to_string())
    );
}

#[test]
fn test_theme_unknown_variable() {
    let theme = Theme::new();
    assert_eq!(theme.resolve("$theme.colors.unknown"), None);
}

// Grid tests
#[test]
fn test_grid_zero_columns() {
    let yaml = "Grid:\n  columns: 0\n  children: []";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

// Progress bar tests
#[test]
fn test_progress_bar_value_validation() {
    let yaml = "ProgressBar:\n  value: 150\n  max: 100";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

// Icon tests
#[test]
fn test_icon_negative_size() {
    let yaml = "Icon:\n  name: \"heart\"\n  size: -10";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
}

// Multiple roots test
#[test]
fn test_multiple_root_components() {
    let yaml = "Text:\n  text: \"First\"\nContainer:\n  children: []";
    let result = parse_ntml(yaml);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), NtmlError::MultipleRootComponents));
}

// Comprehensive test
#[test]
fn test_all_valid_examples() {
    let examples = vec![
        "valid-simple.ntml",
        "valid-complex.ntml",
        "game-hud.ntml",
        "terminal-ui.ntml",
        "mission-briefing.ntml",
    ];

    for example in examples {
        let path = get_example_path(example);
        let yaml = fs::read_to_string(&path).unwrap();
        let result = parse_ntml(&yaml);
        assert!(result.is_ok(), "{} should be valid", example);
    }
}

#[test]
fn test_all_invalid_examples() {
    let examples = vec![
        "invalid-color.ntml",
        "invalid-component.ntml",
        "invalid-missing-property.ntml",
    ];

    for example in examples {
        let path = get_example_path(example);
        let yaml = fs::read_to_string(&path).unwrap();
        let result = parse_ntml(&yaml);
        assert!(result.is_err(), "{} should be invalid", example);
    }
}
