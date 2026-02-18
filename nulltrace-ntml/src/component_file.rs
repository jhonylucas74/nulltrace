use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::components::Component;
use crate::error::{NtmlError, NtmlResult};
use crate::parser::parse_component_value;

/// The set of built-in component names that cannot be used as component aliases
pub const BUILTIN_COMPONENTS: &[&str] = &[
    "Container", "Flex", "Grid", "Stack", "Row", "Column",
    "Text", "Image", "Icon", "Button", "Input", "Checkbox",
    "Radio", "Select", "ProgressBar", "Badge", "Divider", "Spacer",
];

/// A parsed NTML component file (reusable component definition)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentFile {
    /// PascalCase name of this component
    pub component: String,
    /// Prop definitions (may be empty)
    pub props: Vec<PropDef>,
    /// The component tree for this component's body
    pub body: Component,
}

/// A prop definition in a component file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropDef {
    /// camelCase prop name
    pub name: String,
    /// The prop's type
    pub prop_type: PropType,
    /// Default value (if absent, the prop is required)
    pub default: Option<PropDefault>,
}

/// Supported prop types for importable components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropType {
    String,
    Number,
    Boolean,
    Color,
}

/// A default value for a prop
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropDefault {
    String(std::string::String),
    Number(f64),
    Boolean(bool),
}

/// Parse an NTML component file from a YAML string
pub fn parse_component_file(yaml: &str) -> NtmlResult<ComponentFile> {
    let value: Value = serde_yaml::from_str(yaml)?;

    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::ValidationError("Component file must be a YAML object".to_string())
    })?;

    // Component files must NOT have a head section
    if obj.contains_key(&Value::String("head".to_string())) {
        return Err(NtmlError::ComponentFileHasHead {
            path: "<unknown>".to_string(),
        });
    }

    // Must have a "component" key
    let component_name = obj
        .get(&Value::String("component".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            NtmlError::ValidationError(
                "Component file must have a 'component' key with a PascalCase name".to_string(),
            )
        })?
        .to_string();

    // Validate component name is PascalCase and not a built-in
    if !is_pascal_case(&component_name) {
        return Err(NtmlError::InvalidComponent {
            component: component_name.clone(),
            reason: "component name must be PascalCase".to_string(),
        });
    }
    if BUILTIN_COMPONENTS.contains(&component_name.as_str()) {
        return Err(NtmlError::InvalidComponent {
            component: component_name.clone(),
            reason: format!(
                "'{}' conflicts with a built-in component name",
                component_name
            ),
        });
    }

    // Parse props (optional section)
    let props = if let Some(props_value) = obj.get(&Value::String("props".to_string())) {
        parse_props(props_value)?
    } else {
        vec![]
    };

    // Parse body (required)
    let body_value = obj
        .get(&Value::String("body".to_string()))
        .ok_or_else(|| NtmlError::ValidationError(
            "Component file must have a 'body' section".to_string(),
        ))?;
    let body = parse_component_value(body_value)?;

    Ok(ComponentFile {
        component: component_name,
        props,
        body,
    })
}

/// Parse the props array from a component file
fn parse_props(value: &Value) -> NtmlResult<Vec<PropDef>> {
    let arr = value.as_sequence().ok_or_else(|| {
        NtmlError::ValidationError("'props' must be an array".to_string())
    })?;

    let mut prop_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for item in arr {
        let obj = item.as_mapping().ok_or_else(|| {
            NtmlError::ValidationError("Each prop must be an object".to_string())
        })?;

        let name = obj
            .get(&Value::String("name".to_string()))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NtmlError::ValidationError("Each prop must have a 'name' field".to_string())
            })?
            .to_string();

        if prop_names.contains(&name) {
            return Err(NtmlError::ValidationError(format!(
                "Duplicate prop name '{}'",
                name
            )));
        }
        prop_names.insert(name.clone());

        let type_str = obj
            .get(&Value::String("type".to_string()))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NtmlError::ValidationError(format!(
                    "Prop '{}' must have a 'type' field",
                    name
                ))
            })?;

        let prop_type = match type_str {
            "string" => PropType::String,
            "number" => PropType::Number,
            "boolean" => PropType::Boolean,
            "color" => PropType::Color,
            other => {
                return Err(NtmlError::InvalidPropType {
                    component: "<component file>".to_string(),
                    prop: name.clone(),
                    expected: format!(
                        "one of: string, number, boolean, color — got '{}'",
                        other
                    ),
                })
            }
        };

        let default = if let Some(def_value) = obj.get(&Value::String("default".to_string())) {
            Some(parse_prop_default(def_value, &prop_type, &name)?)
        } else {
            None
        };

        result.push(PropDef {
            name,
            prop_type,
            default,
        });
    }

    Ok(result)
}

fn parse_prop_default(value: &Value, prop_type: &PropType, name: &str) -> NtmlResult<PropDefault> {
    match prop_type {
        PropType::String | PropType::Color => {
            let s = value.as_str().ok_or_else(|| NtmlError::InvalidPropType {
                component: "<component file>".to_string(),
                prop: name.to_string(),
                expected: "string default value".to_string(),
            })?;
            Ok(PropDefault::String(s.to_string()))
        }
        PropType::Number => {
            let n = value.as_f64().ok_or_else(|| NtmlError::InvalidPropType {
                component: "<component file>".to_string(),
                prop: name.to_string(),
                expected: "number default value".to_string(),
            })?;
            Ok(PropDefault::Number(n))
        }
        PropType::Boolean => {
            let b = value.as_bool().ok_or_else(|| NtmlError::InvalidPropType {
                component: "<component file>".to_string(),
                prop: name.to_string(),
                expected: "boolean default value".to_string(),
            })?;
            Ok(PropDefault::Boolean(b))
        }
    }
}

/// Check if a string is PascalCase (starts with uppercase, alphanumeric only)
pub fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => chars.all(|c| c.is_ascii_alphanumeric()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("NavBar"));
        assert!(is_pascal_case("MyComponent"));
        assert!(!is_pascal_case("navBar"));
        assert!(!is_pascal_case("nav-bar"));
        assert!(!is_pascal_case(""));
    }

    #[test]
    fn test_parse_component_file_basic() {
        let yaml = r#"
component: NavBar
props:
  - name: title
    type: string
    default: "Navigation"
body:
  Text:
    text: "{props.title}"
"#;
        let result = parse_component_file(yaml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.component, "NavBar");
        assert_eq!(file.props.len(), 1);
        assert_eq!(file.props[0].name, "title");
    }

    #[test]
    fn test_component_file_rejects_head() {
        let yaml = r#"
component: NavBar
head:
  title: "Oops"
body:
  Text:
    text: "Hello"
"#;
        let result = parse_component_file(yaml);
        assert!(matches!(
            result,
            Err(NtmlError::ComponentFileHasHead { .. })
        ));
    }

    #[test]
    fn test_component_file_requires_body() {
        let yaml = r#"
component: NavBar
"#;
        let result = parse_component_file(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_component_file_rejects_builtin_name() {
        let yaml = r#"
component: Container
body:
  Text:
    text: "Hello"
"#;
        let result = parse_component_file(yaml);
        assert!(matches!(
            result,
            Err(NtmlError::InvalidComponent { .. })
        ));
    }
}
