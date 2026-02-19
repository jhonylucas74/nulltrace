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
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_ok(), "valid-simple.ntml should be valid: {:?}", result.err());
}

#[test]
fn test_valid_complex_example() {
    let path = get_example_path("valid-complex.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_ok(), "valid-complex.ntml should be valid: {:?}", result.err());
}

#[test]
fn test_game_hud_example() {
    let path = get_example_path("game-hud.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_ok(), "game-hud.ntml should be valid: {:?}", result.err());
}

#[test]
fn test_terminal_ui_example() {
    let path = get_example_path("terminal-ui.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_ok(), "terminal-ui.ntml should be valid: {:?}", result.err());
}

#[test]
fn test_mission_briefing_example() {
    let path = get_example_path("mission-briefing.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_ok(), "mission-briefing.ntml should be valid: {:?}", result.err());
}

// Test invalid examples
#[test]
fn test_invalid_color_example() {
    let path = get_example_path("invalid-color.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_err(), "invalid-color.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::InvalidColor { .. }));
}

#[test]
fn test_invalid_component_example() {
    let path = get_example_path("invalid-component.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_err(), "invalid-component.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::InvalidComponent { .. }));
}

#[test]
fn test_invalid_missing_property_example() {
    let path = get_example_path("invalid-missing-property.ntml");
    let xml = fs::read_to_string(&path).unwrap();
    let result = parse_ntml(&xml);
    assert!(result.is_err(), "invalid-missing-property.ntml should be invalid");
    assert!(matches!(result.unwrap_err(), NtmlError::MissingProperty { .. }));
}

// Component tests
#[test]
fn test_text_component() {
    let xml = r#"<Text text="Hello World" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok());
    if let Ok(Component::Text(text)) = result {
        assert_eq!(text.text, "Hello World");
    } else {
        panic!("Expected Text component");
    }
}

#[test]
fn test_button_component() {
    let xml = r#"<Button action="hack_system" variant="primary" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok());
}

#[test]
fn test_progress_bar_component() {
    let xml = r#"<ProgressBar value="75" max="100" variant="danger" />"#;
    let result = parse_ntml(xml);
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
    let xml = r#"<Text text="" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

#[test]
fn test_negative_gap_validation() {
    let xml = r#"<Flex gap="-10"><Text text="Test" /></Flex>"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

#[test]
fn test_opacity_out_of_range() {
    let xml = r#"<Text text="Test" style="opacity:1.5" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

// Color validation tests
#[test]
fn test_valid_hex_colors() {
    let colors = vec!["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff"];

    for color in colors {
        let xml = format!(r#"<Text text="Test" style="color:{}" />"#, color);
        let result = parse_ntml(&xml);
        assert!(result.is_ok(), "Failed for color: {}", color);
    }
}

#[test]
fn test_valid_named_colors() {
    let colors = vec!["red", "blue", "green", "white", "black", "transparent"];

    for color in colors {
        let xml = format!(r#"<Text text="Test" style="color:{}" />"#, color);
        let result = parse_ntml(&xml);
        assert!(result.is_ok(), "Failed for color: {}", color);
    }
}

#[test]
fn test_invalid_hex_colors() {
    let colors = vec!["#fff", "#12345", "#gggggg", "123456"];

    for color in colors {
        let xml = format!(r#"<Text text="Test" style="color:{}" />"#, color);
        let result = parse_ntml(&xml);
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
    let xml = r#"<Grid columns="0"></Grid>"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

// Progress bar tests
#[test]
fn test_progress_bar_value_validation() {
    let xml = r#"<ProgressBar value="150" max="100" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

// Icon tests
#[test]
fn test_icon_negative_size() {
    let xml = r#"<Icon name="heart" size="-10" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_err());
}

// Dimension style tests (width/height: number or "auto")
#[test]
fn test_dimension_auto_parses() {
    let xml = r#"
<Container style="width:200; height:auto">
  <Text text="Hello" />
</Container>
"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok(), "height:auto should parse: {:?}", result.err());
}

// Multiple roots test
#[test]
fn test_multiple_root_components() {
    let xml = r#"<Text text="First" /><Container></Container>"#;
    let result = parse_ntml(xml);
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
        let xml = fs::read_to_string(&path).unwrap();
        let result = parse_ntml(&xml);
        assert!(result.is_ok(), "{} should be valid: {:?}", example, result.err());
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
        let xml = fs::read_to_string(&path).unwrap();
        let result = parse_ntml(&xml);
        assert!(result.is_err(), "{} should be invalid", example);
    }
}

// --- v0.2.0 tests ---

#[test]
fn test_parse_document_classic_format() {
    let xml = r#"
<Container style="padding:16">
  <Text text="Hello" />
</Container>
"#;
    let result = parse_document(xml);
    assert!(result.is_ok(), "Classic format should parse as NtmlDocument::Classic");
    assert!(matches!(result.unwrap(), NtmlDocument::Classic(_)));
}

#[test]
fn test_parse_document_full_format_minimal() {
    let xml = r#"
<head>
  <title>Test Page</title>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(result.is_ok(), "Full format should parse: {:?}", result.err());
    if let NtmlDocument::Full { head, .. } = result.unwrap() {
        assert_eq!(head.title, "Test Page");
    } else {
        panic!("Expected NtmlDocument::Full");
    }
}

#[test]
fn test_parse_document_full_format_with_all_head_fields() {
    let xml = r#"
<head>
  <title>Dashboard</title>
  <description>Main dashboard page</description>
  <author>Player One</author>
  <tags>hud dashboard system</tags>
  <font family="Roboto Mono" weights="400,700" />
  <script src="scripts/main.lua" />
  <import src="components/nav-bar.ntml" as="NavBar" />
</head>
<body>
  <Column gap="16">
    <Text id="title" text="Dashboard" />
  </Column>
</body>
"#;
    let result = parse_document(xml);
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
    let xml = r#"
<head>
  <title>Test</title>
</head>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::MissingBody)));
}

#[test]
fn test_full_format_missing_title() {
    let xml = r#"
<head>
  <description>No title</description>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::MissingTitle)));
}

#[test]
fn test_parse_ntml_rejects_full_format() {
    let xml = r#"
<head>
  <title>Test</title>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_ntml(xml);
    assert!(result.is_err(), "parse_ntml should reject full format");
}

#[test]
fn test_component_id_parsing() {
    let xml = r#"
<Container id="root">
  <Text id="greeting" text="Hello" />
  <Button id="btn-submit" action="game:submit" />
</Container>
"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok(), "Should parse with ids: {:?}", result.err());
    if let Component::Container(c) = result.unwrap() {
        assert_eq!(c.id.as_deref(), Some("root"));
    }
}

#[test]
fn test_duplicate_id_rejected() {
    let xml = r#"
<Container id="same-id">
  <Text id="same-id" text="Hello" />
</Container>
"#;
    let result = parse_ntml(xml);
    assert!(
        matches!(result, Err(NtmlError::DuplicateId { .. })),
        "Duplicate id should be rejected, got: {:?}",
        result
    );
}

#[test]
fn test_custom_font_family_with_head() {
    let xml = r#"
<head>
  <title>Fonts Test</title>
  <font family="Roboto Mono" weights="400,700" />
</head>
<body>
  <Text text="Hello" style="fontFamily:Roboto Mono" />
</body>
"#;
    let result = parse_document(xml);
    assert!(
        result.is_ok(),
        "Custom font declared in head should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_custom_font_family_not_declared_rejected() {
    let xml = r#"
<head>
  <title>Fonts Test</title>
</head>
<body>
  <Text text="Hello" style="fontFamily:Undeclared Font" />
</body>
"#;
    let result = parse_document(xml);
    assert!(
        result.is_err(),
        "Custom font not declared in head should fail"
    );
}

#[test]
fn test_invalid_tag_uppercase() {
    let xml = r#"
<head>
  <title>Test</title>
  <tags>UPPERCASE</tags>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::InvalidTag { .. })));
}

#[test]
fn test_invalid_tag_mixed_case() {
    let xml = r#"
<head>
  <title>Test</title>
  <tags>tag-With-Caps</tags>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::InvalidTag { .. })));
}

#[test]
fn test_too_many_scripts() {
    let xml = r#"
<head>
  <title>Test</title>
  <script src="a.lua" />
  <script src="b.lua" />
  <script src="c.lua" />
  <script src="d.lua" />
  <script src="e.lua" />
  <script src="f.lua" />
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::ScriptLimitExceeded { .. })));
}

#[test]
fn test_script_invalid_extension() {
    let xml = r#"
<head>
  <title>Test</title>
  <script src="scripts/main.js" />
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(result.is_err(), "Non-.lua script should be rejected");
}

#[test]
fn test_import_alias_not_pascal_case() {
    let xml = r#"
<head>
  <title>Test</title>
  <import src="components/foo.ntml" as="lowerCase" />
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::InvalidImportAlias { .. })));
}

#[test]
fn test_import_alias_conflicts_with_builtin() {
    let xml = r#"
<head>
  <title>Test</title>
  <import src="components/foo.ntml" as="Container" />
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(matches!(result, Err(NtmlError::InvalidImportAlias { .. })));
}

#[test]
fn test_invalid_font_weight() {
    let xml = r#"
<head>
  <title>Test</title>
  <font family="Roboto Mono" weights="150" />
</head>
<body>
  <Text text="Hello" />
</body>
"#;
    let result = parse_document(xml);
    assert!(result.is_err(), "Invalid font weight should be rejected");
}

#[test]
fn test_parse_component_file_basic() {
    let xml = r#"
<props>
  <prop name="label" type="string" />
  <prop name="value" type="number" />
  <prop name="unit" type="string" default="" />
</props>
<body>
  <Container>
    <Text text="{props.label}" />
  </Container>
</body>
"#;
    let result = parse_component_file(xml);
    assert!(result.is_ok(), "Component file should parse: {:?}", result.err());
    let file = result.unwrap();
    // Component name is inferred ("Component") — actual name comes from import alias
    assert_eq!(file.props.len(), 3);
    assert_eq!(file.props[0].name, "label");
    assert_eq!(file.props[2].name, "unit");
    assert!(file.props[2].default.is_some());
}

#[test]
fn test_parse_component_file_rejects_head() {
    let xml = r#"<head><title>Should fail</title></head><body><Text text="Hello" /></body>"#;
    let result = parse_component_file(xml);
    assert!(matches!(result, Err(NtmlError::ComponentFileHasHead { .. })));
}

#[test]
fn test_parse_component_file_no_body_fails() {
    let xml = r#"
<props>
  <prop name="title" type="string" />
</props>
"#;
    let result = parse_component_file(xml);
    assert!(result.is_err(), "Component file without <body> should fail");
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
        let xml = fs::read_to_string(&path).unwrap();
        let result = parse_document(&xml);
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
        let xml = fs::read_to_string(&path).unwrap();
        let result = parse_component_file(&xml);
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
    let xml = r#"
<Container data-test="fulano" data-user-id="42">
</Container>
"#;
    let result = parse_ntml(xml);
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
    let xml = r#"<Button action="click" data-analytics="btn-submit" data-env="production" />"#;
    let result = parse_ntml(xml);
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
    let xml = r#"<Text text="hello" data-role="label" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok(), "Text with data-* attr should be valid: {:?}", result.err());
    if let Component::Text(t) = result.unwrap() {
        assert_eq!(t.data.get("data-role"), Some(&"label".to_string()));
    } else {
        panic!("Expected Text");
    }
}

#[test]
fn test_data_attr_no_attrs_yields_empty_map() {
    let xml = r#"<Text text="hello" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_ok());
    if let Component::Text(t) = result.unwrap() {
        assert!(t.data.is_empty());
    } else {
        panic!("Expected Text");
    }
}

#[test]
fn test_data_attr_string_value_is_valid() {
    // In XML all attribute values are strings — "42" is a valid string data-* value
    let xml = r#"<Text text="hello" data-count="42" />"#;
    let result = parse_ntml(xml);
    assert!(
        result.is_ok(),
        "data-* attribute with numeric string value should be valid in XML"
    );
}

#[test]
fn test_data_attr_invalid_name_uppercase() {
    // Key has uppercase letters after "data-" — should be rejected by validator
    let xml = r#"<Text text="hello" data-MyAttr="x" />"#;
    let result = parse_ntml(xml);
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
    // In XML, "data-" is a valid attribute name syntactically, but NTML validator should reject it
    let xml = r#"<Text text="hello" data-="x" />"#;
    let result = parse_ntml(xml);
    assert!(result.is_err(), "data- with no suffix should fail validation");
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}

#[test]
fn test_data_attr_multiple_components_in_tree() {
    let xml = r#"
<Container data-section="main">
  <Text text="hello" data-label="greeting" />
  <Button action="close" data-target="modal" />
</Container>
"#;
    let result = parse_ntml(xml);
    assert!(
        result.is_ok(),
        "Tree with data-* attrs should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_data_attr_valid_with_hyphens_in_name() {
    // data-user-profile-id is a valid name
    let xml = r#"<Text text="hello" data-user-profile-id="abc123" />"#;
    let result = parse_ntml(xml);
    assert!(
        result.is_ok(),
        "data-user-profile-id should be valid: {:?}",
        result.err()
    );
}

#[test]
fn test_data_attr_invalid_name_starts_with_digit() {
    // data-1item starts with a digit after "data-" — invalid per regex
    let xml = r#"<Text text="hello" data-1item="x" />"#;
    let result = parse_ntml(xml);
    assert!(
        result.is_err(),
        "data-1item should fail (digit after data-)"
    );
    assert!(
        matches!(result, Err(NtmlError::InvalidDataAttribute { .. })),
        "Should be InvalidDataAttribute error"
    );
}
