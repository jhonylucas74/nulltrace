use serde::{Deserialize, Serialize};

use crate::components::Component;
use crate::error::{NtmlError, NtmlResult};
use crate::parser::parse_component_value_from_node;

/// The set of built-in component names that cannot be used as component aliases
pub const BUILTIN_COMPONENTS: &[&str] = &[
    "Container", "Flex", "Grid", "Stack", "Row", "Column",
    "Text", "Image", "Icon", "Button", "Input", "Checkbox",
    "Radio", "Select", "ProgressBar", "Badge", "Divider", "Spacer", "Link",
    "Code", "Markdown", "List", "ListItem", "Heading", "Table", "Blockquote", "Pre", "Details",
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

/// Parse an NTML component file from an XML string.
///
/// Component files use this format:
/// ```xml
/// <props>
///   <prop name="title" type="string" default="Navigation" />
///   <prop name="accentColor" type="color" />
/// </props>
/// <body>
///   <Column>
///     <Text text="{props.title}" style="color:{props.accentColor}" />
///   </Column>
/// </body>
/// ```
///
/// The `<props>` element is optional. The `<body>` element is required and
/// must contain exactly one root component element. The component's name is
/// determined by the `as` alias in the importing document's `<import>` tag.
pub fn parse_component_file(xml: &str) -> NtmlResult<ComponentFile> {
    const WRAPPER: &str = "__ntml_root__";
    let wrapped = format!("<{0}>{1}</{0}>", WRAPPER, xml);

    let doc = roxmltree::Document::parse(&wrapped)?;
    let root = doc.root_element();

    // Component files must NOT have a <head> section
    if root
        .children()
        .filter(|n| n.is_element())
        .any(|n| n.tag_name().name() == "head")
    {
        return Err(NtmlError::ComponentFileHasHead {
            path: "<unknown>".to_string(),
        });
    }

    let mut props: Vec<PropDef> = Vec::new();
    let mut body_node: Option<roxmltree::Node> = None;
    let mut component_name: Option<String> = None;

    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "props" => {
                props = parse_props_node(child)?;
            }
            "body" => {
                // <body> wraps the root component — same as in the full document format
                body_node = Some(child);
            }
            name => {
                // Support the old-style where the component name is a PascalCase root element
                // containing the body directly (for simpler files without <body> wrapper)
                if is_pascal_case(name) && !BUILTIN_COMPONENTS.contains(&name) {
                    if component_name.is_some() {
                        return Err(NtmlError::MultipleRootComponents);
                    }
                    component_name = Some(name.to_string());
                    body_node = Some(child);
                } else {
                    return Err(NtmlError::ValidationError(format!(
                        "Unexpected element <{}> in component file. Expected <props>, <body>, or a PascalCase component name element.",
                        name
                    )));
                }
            }
        }
    }

    let body_container = body_node.ok_or_else(|| {
        NtmlError::ValidationError(
            "Component file must have a <body> element containing the root component".to_string(),
        )
    })?;

    // Determine the actual body element and component name
    let (name, root_component_node) = if let Some(cname) = component_name {
        // Old style: <NavBar>...</NavBar> — component name is the tag, body is the children
        // This shouldn't happen for built-ins; the component name element IS the wrapper
        // In this style, the component name element wraps the actual component tree
        // but wait — if we use <NavBar> as wrapper, then its children are the body.
        // Actually this is confusing. Let's just require <body> style.
        return Err(NtmlError::ValidationError(format!(
            "Component file: use <body>...</body> to wrap the root component. Got <{}>.",
            cname
        )));
    } else {
        // Standard style: <body><Column>...</Column></body>
        let first_child = body_container
            .children()
            .filter(|n| n.is_element())
            .next()
            .ok_or_else(|| {
                NtmlError::ValidationError("<body> element is empty".to_string())
            })?;

        // The component name comes from the filename, but we need at least something.
        // Since XML doesn't have a "component name declaration" here, we derive it from
        // the component's implied context. We use a placeholder — the actual name comes
        // from the import alias when the file is loaded.
        let inferred_name = "Component".to_string();
        (inferred_name, first_child)
    };

    let body = parse_component_value_from_node(root_component_node, &[])?;

    Ok(ComponentFile {
        component: name,
        props,
        body,
    })
}

/// Parse the <props> element into PropDef list.
fn parse_props_node(node: roxmltree::Node) -> NtmlResult<Vec<PropDef>> {
    let mut prop_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() != "prop" {
            return Err(NtmlError::ValidationError(format!(
                "<props>: unexpected element <{}>; only <prop> is allowed",
                child.tag_name().name()
            )));
        }

        let name = child
            .attribute("name")
            .ok_or_else(|| {
                NtmlError::ValidationError(
                    "<prop>: missing required attribute 'name'".to_string(),
                )
            })?
            .to_string();

        if prop_names.contains(&name) {
            return Err(NtmlError::ValidationError(format!(
                "Duplicate prop name '{}'",
                name
            )));
        }
        prop_names.insert(name.clone());

        let type_str = child
            .attribute("type")
            .ok_or_else(|| {
                NtmlError::ValidationError(format!(
                    "<prop name=\"{}\">: missing required attribute 'type'",
                    name
                ))
            })?;

        let prop_type = match type_str {
            "string" | "text" => PropType::String,
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

        let default = if let Some(def_str) = child.attribute("default") {
            Some(parse_prop_default_str(def_str, &prop_type, &name)?)
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

fn parse_prop_default_str(
    s: &str,
    prop_type: &PropType,
    name: &str,
) -> NtmlResult<PropDefault> {
    match prop_type {
        PropType::String | PropType::Color => Ok(PropDefault::String(s.to_string())),
        PropType::Number => {
            let n = s.parse::<f64>().map_err(|_| NtmlError::InvalidPropType {
                component: "<component file>".to_string(),
                prop: name.to_string(),
                expected: "number default value".to_string(),
            })?;
            Ok(PropDefault::Number(n))
        }
        PropType::Boolean => {
            let b = match s {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(NtmlError::InvalidPropType {
                        component: "<component file>".to_string(),
                        prop: name.to_string(),
                        expected: "boolean default value (true or false)".to_string(),
                    })
                }
            };
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
        let xml = r#"
<props>
  <prop name="title" type="string" default="Navigation" />
</props>
<body>
  <Column>
    <Text text="{props.title}" />
  </Column>
</body>
"#;
        let result = parse_component_file(xml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let cf = result.unwrap();
        assert_eq!(cf.props.len(), 1);
        assert_eq!(cf.props[0].name, "title");
    }

    #[test]
    fn test_component_file_rejects_head() {
        let xml = r#"<head><title>Nope</title></head><body><Text text="hi" /></body>"#;
        let result = parse_component_file(xml);
        assert!(result.is_err());
    }
}
