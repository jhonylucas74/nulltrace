use nulltrace_ntml::{
    parse_component_file, parse_document, parse_ntml, Component, NtmlDocument, NtmlError, Theme,
};
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

// --- v0.2.0 tests ---

#[test]
fn test_parse_document_classic_format() {
    let yaml = r#"
Container:
  style:
    padding: 16
  children:
    - Text:
        text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(result.is_ok(), "Classic format should parse as NtmlDocument::Classic");
    assert!(matches!(result.unwrap(), NtmlDocument::Classic(_)));
}

#[test]
fn test_parse_document_full_format_minimal() {
    let yaml = r#"
head:
  title: "Test Page"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(result.is_ok(), "Full format should parse: {:?}", result.err());
    if let NtmlDocument::Full { head, .. } = result.unwrap() {
        assert_eq!(head.title, "Test Page");
    } else {
        panic!("Expected NtmlDocument::Full");
    }
}

#[test]
fn test_parse_document_full_format_with_all_head_fields() {
    let yaml = r#"
head:
  title: "Dashboard"
  description: "Main dashboard page"
  author: "Player One"
  tags: [hud, dashboard, system]
  fonts:
    - family: "Roboto Mono"
      weights: [400, 700]
  scripts:
    - src: "scripts/main.lua"
  imports:
    - src: "components/nav-bar.ntml"
      as: "NavBar"

body:
  Column:
    gap: 16
    children:
      - Text:
          id: "title"
          text: "Dashboard"
"#;
    let result = parse_document(yaml);
    assert!(result.is_ok(), "Full format with all head fields: {:?}", result.err());
    if let NtmlDocument::Full { head, body } = result.unwrap() {
        assert_eq!(head.title, "Dashboard");
        assert_eq!(head.description.as_deref(), Some("Main dashboard page"));
        assert_eq!(head.author.as_deref(), Some("Player One"));
        assert_eq!(head.tags.as_ref().unwrap().len(), 3);
        assert_eq!(head.fonts.as_ref().unwrap().len(), 1);
        assert_eq!(head.fonts.as_ref().unwrap()[0].family, "Roboto Mono");
        assert_eq!(head.scripts.as_ref().unwrap().len(), 1);
        assert_eq!(head.imports.as_ref().unwrap().len(), 1);
        assert_eq!(head.imports.as_ref().unwrap()[0].alias, "NavBar");
        // Body should be a Column component
        assert!(matches!(body, Component::Column(_)));
    } else {
        panic!("Expected NtmlDocument::Full");
    }
}

#[test]
fn test_full_format_missing_body() {
    let yaml = r#"
head:
  title: "Test"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::MissingBody)));
}

#[test]
fn test_full_format_missing_title() {
    let yaml = r#"
head:
  description: "No title"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::MissingTitle)));
}

#[test]
fn test_parse_ntml_rejects_full_format() {
    let yaml = r#"
head:
  title: "Test"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_err(), "parse_ntml should reject full format");
}

#[test]
fn test_component_id_parsing() {
    let yaml = r#"
Container:
  id: "root"
  children:
    - Text:
        id: "greeting"
        text: "Hello"
    - Button:
        id: "btn-submit"
        action: "game:submit"
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_ok(), "Should parse with ids: {:?}", result.err());
    if let Component::Container(c) = result.unwrap() {
        assert_eq!(c.id.as_deref(), Some("root"));
    }
}

#[test]
fn test_duplicate_id_rejected() {
    let yaml = r#"
Container:
  id: "same-id"
  children:
    - Text:
        id: "same-id"
        text: "Hello"
"#;
    let result = parse_ntml(yaml);
    assert!(
        matches!(result, Err(NtmlError::DuplicateId { .. })),
        "Duplicate id should be rejected, got: {:?}",
        result
    );
}

#[test]
fn test_custom_font_family_with_head() {
    let yaml = r#"
head:
  title: "Fonts Test"
  fonts:
    - family: "Roboto Mono"
      weights: [400, 700]
body:
  Text:
    text: "Hello"
    style:
      fontFamily: "Roboto Mono"
"#;
    let result = parse_document(yaml);
    assert!(
        result.is_ok(),
        "Custom font declared in head should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_custom_font_family_not_declared_rejected() {
    let yaml = r#"
head:
  title: "Fonts Test"
body:
  Text:
    text: "Hello"
    style:
      fontFamily: "Undeclared Font"
"#;
    let result = parse_document(yaml);
    assert!(
        result.is_err(),
        "Custom font not declared in head should fail"
    );
}

#[test]
fn test_invalid_tag_with_spaces() {
    let yaml = r#"
head:
  title: "Test"
  tags: ["has space"]
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::InvalidTag { .. })));
}

#[test]
fn test_invalid_tag_uppercase() {
    let yaml = r#"
head:
  title: "Test"
  tags: ["UPPERCASE"]
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::InvalidTag { .. })));
}

#[test]
fn test_too_many_scripts() {
    let yaml = r#"
head:
  title: "Test"
  scripts:
    - src: "a.lua"
    - src: "b.lua"
    - src: "c.lua"
    - src: "d.lua"
    - src: "e.lua"
    - src: "f.lua"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::ScriptLimitExceeded { .. })));
}

#[test]
fn test_script_invalid_extension() {
    let yaml = r#"
head:
  title: "Test"
  scripts:
    - src: "scripts/main.js"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(result.is_err(), "Non-.lua script should be rejected");
}

#[test]
fn test_import_alias_not_pascal_case() {
    let yaml = r#"
head:
  title: "Test"
  imports:
    - src: "components/foo.ntml"
      as: "lowerCase"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::InvalidImportAlias { .. })));
}

#[test]
fn test_import_alias_conflicts_with_builtin() {
    let yaml = r#"
head:
  title: "Test"
  imports:
    - src: "components/foo.ntml"
      as: "Container"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(matches!(result, Err(NtmlError::InvalidImportAlias { .. })));
}

#[test]
fn test_invalid_font_weight() {
    let yaml = r#"
head:
  title: "Test"
  fonts:
    - family: "Roboto Mono"
      weights: [150]
body:
  Text:
    text: "Hello"
"#;
    let result = parse_document(yaml);
    assert!(result.is_err(), "Invalid font weight should be rejected");
}

#[test]
fn test_parse_component_file_basic() {
    let yaml = r#"
component: StatCard

props:
  - name: label
    type: string
  - name: value
    type: number
  - name: unit
    type: string
    default: ""

body:
  Container:
    children:
      - Text:
          text: "{props.label}"
"#;
    let result = parse_component_file(yaml);
    assert!(result.is_ok(), "Component file should parse: {:?}", result.err());
    let file = result.unwrap();
    assert_eq!(file.component, "StatCard");
    assert_eq!(file.props.len(), 3);
    assert_eq!(file.props[0].name, "label");
    assert_eq!(file.props[2].name, "unit");
    assert!(file.props[2].default.is_some());
}

#[test]
fn test_parse_component_file_rejects_head() {
    let yaml = r#"
component: NavBar
head:
  title: "Should fail"
body:
  Text:
    text: "Hello"
"#;
    let result = parse_component_file(yaml);
    assert!(matches!(result, Err(NtmlError::ComponentFileHasHead { .. })));
}

#[test]
fn test_parse_component_file_rejects_builtin_name() {
    let yaml = r#"
component: Text
body:
  Container:
    children: []
"#;
    let result = parse_component_file(yaml);
    assert!(matches!(result, Err(NtmlError::InvalidComponent { .. })));
}

#[test]
fn test_parse_page_with_head_example() {
    let path = {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("examples");
        p.push("page-with-head.ntml");
        p
    };
    if path.exists() {
        let yaml = fs::read_to_string(&path).unwrap();
        let result = parse_document(&yaml);
        assert!(
            result.is_ok(),
            "page-with-head.ntml should be valid: {:?}",
            result.err()
        );
        assert!(
            matches!(result.unwrap(), NtmlDocument::Full { .. }),
            "Should be Full format"
        );
    }
}

#[test]
fn test_parse_nav_bar_component_example() {
    let path = {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("examples");
        p.push("components");
        p.push("nav-bar.ntml");
        p
    };
    if path.exists() {
        let yaml = fs::read_to_string(&path).unwrap();
        let result = parse_component_file(&yaml);
        assert!(
            result.is_ok(),
            "nav-bar.ntml component file should be valid: {:?}",
            result.err()
        );
    }
}

// === data-* attribute tests ===

#[test]
fn test_data_attr_valid_on_container() {
    let yaml = r#"
Container:
  data-test: "fulano"
  data-user-id: "42"
  children: []
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_ok(), "Container with data-* attrs should be valid: {:?}", result.err());
    if let Component::Container(c) = result.unwrap() {
        assert_eq!(c.data.get("data-test"), Some(&"fulano".to_string()));
        assert_eq!(c.data.get("data-user-id"), Some(&"42".to_string()));
    } else {
        panic!("Expected Container");
    }
}

#[test]
fn test_data_attr_valid_on_button() {
    let yaml = r#"
Button:
  action: "click"
  data-analytics: "btn-submit"
  data-env: "production"
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_ok(), "Button with data-* attrs should be valid: {:?}", result.err());
    if let Component::Button(b) = result.unwrap() {
        assert_eq!(b.data.get("data-analytics"), Some(&"btn-submit".to_string()));
        assert_eq!(b.data.get("data-env"), Some(&"production".to_string()));
    } else {
        panic!("Expected Button");
    }
}

#[test]
fn test_data_attr_valid_on_text() {
    let yaml = r#"
Text:
  text: "hello"
  data-role: "label"
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_ok(), "Text with data-* attr should be valid: {:?}", result.err());
    if let Component::Text(t) = result.unwrap() {
        assert_eq!(t.data.get("data-role"), Some(&"label".to_string()));
    } else {
        panic!("Expected Text");
    }
}

#[test]
fn test_data_attr_no_attrs_yields_empty_map() {
    let yaml = r#"
Text:
  text: "hello"
"#;
    let result = parse_ntml(yaml);
    assert!(result.is_ok());
    if let Component::Text(t) = result.unwrap() {
        assert!(t.data.is_empty());
    } else {
        panic!("Expected Text");
    }
}

#[test]
fn test_data_attr_invalid_value_not_string() {
    // YAML integer value must be rejected
    let yaml = r#"
Text:
  text: "hello"
  data-count: 42
"#;
    let result = parse_ntml(yaml);
    assert!(
        result.is_err(),
        "data-* attribute with non-string value should fail"
    );
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}

#[test]
fn test_data_attr_invalid_name_uppercase() {
    // Key has uppercase letters after "data-" — should be rejected by validator
    let yaml = r#"
Text:
  text: "hello"
  data-MyAttr: "x"
"#;
    let result = parse_ntml(yaml);
    assert!(
        result.is_err(),
        "data-* with uppercase key should fail validation"
    );
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}

#[test]
fn test_data_attr_invalid_name_empty_suffix() {
    // "data-" with nothing after it — invalid
    let yaml = "Text:\n  text: hello\n  data-: x\n";
    let result = parse_ntml(yaml);
    assert!(result.is_err(), "data- with no suffix should fail validation");
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}

#[test]
fn test_data_attr_multiple_components_in_tree() {
    let yaml = r#"
Container:
  data-section: "main"
  children:
    - Text:
        text: "hello"
        data-label: "greeting"
    - Button:
        action: "close"
        data-target: "modal"
"#;
    let result = parse_ntml(yaml);
    assert!(
        result.is_ok(),
        "Tree with data-* attrs should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_data_attr_valid_with_hyphens_in_name() {
    // data-user-profile-id is a valid name
    let yaml = r#"
Text:
  text: "hello"
  data-user-profile-id: "abc123"
"#;
    let result = parse_ntml(yaml);
    assert!(
        result.is_ok(),
        "data-user-profile-id should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_data_attr_invalid_name_starts_with_digit() {
    // data-1item starts with a digit after "data-" — invalid per regex
    let yaml = "Text:\n  text: hello\n  data-1item: x\n";
    let result = parse_ntml(yaml);
    assert!(
        result.is_err(),
        "data-1item should fail (digit after data-)"
    );
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}
