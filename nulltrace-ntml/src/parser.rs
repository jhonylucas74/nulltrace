use crate::components::*;
use crate::document::NtmlDocument;
use crate::error::{NtmlError, NtmlResult};
use crate::head::{ComponentImport, FontImport, Head, ScriptImport};
use crate::theme::Theme;
use crate::validator::{validate_component_with_context, validate_head};
use serde_yaml::Value;

/// Parse an NTML document from YAML string (classic format only, backward compat)
pub fn parse_ntml(yaml: &str) -> NtmlResult<Component> {
    parse_ntml_with_theme(yaml, Theme::default())
}

/// Parse an NTML document with a custom theme (classic format only, backward compat)
pub fn parse_ntml_with_theme(yaml: &str, _theme: Theme) -> NtmlResult<Component> {
    let value: Value = serde_yaml::from_str(yaml)?;

    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::ValidationError("Document must be a YAML object".to_string())
    })?;

    // Reject full format when classic parse function is used
    if obj.contains_key(&Value::String("head".to_string())) {
        return Err(NtmlError::ValidationError(
            "This document uses the v0.2.0 full format (head/body). Use parse_document() instead of parse_ntml()".to_string(),
        ));
    }

    let component = parse_component_value(&value)?;
    validate_component_with_context(&component, &[])?;
    Ok(component)
}

/// Parse an NTML document — supports both classic (v0.1.0) and full (v0.2.0) formats
pub fn parse_document(yaml: &str) -> NtmlResult<NtmlDocument> {
    parse_document_with_theme(yaml, Theme::default())
}

/// Parse an NTML document with a custom theme
pub fn parse_document_with_theme(yaml: &str, _theme: Theme) -> NtmlResult<NtmlDocument> {
    let value: Value = serde_yaml::from_str(yaml)?;

    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::ValidationError("Document must be a YAML object".to_string())
    })?;

    if obj.contains_key(&Value::String("head".to_string())) {
        // --- Full format (v0.2.0) ---
        let head_value = obj
            .get(&Value::String("head".to_string()))
            .ok_or_else(|| NtmlError::MissingTitle)?;
        let head = parse_head(head_value)?;

        // Build list of import aliases for body parsing
        let import_aliases: Vec<String> = head
            .imports
            .as_ref()
            .map(|imports| imports.iter().map(|i| i.alias.clone()).collect())
            .unwrap_or_default();

        let body_value = obj
            .get(&Value::String("body".to_string()))
            .ok_or(NtmlError::MissingBody)?;
        let body = parse_component_value_ctx(body_value, &import_aliases)?;

        let font_families = head.font_families();
        validate_head(&head)?;
        validate_component_with_context(&body, &font_families)?;

        Ok(NtmlDocument::Full { head, body })
    } else {
        // --- Classic format (v0.1.0) ---
        let component = parse_component_value(&value)?;
        validate_component_with_context(&component, &[])?;
        Ok(NtmlDocument::Classic(component))
    }
}

/// Parse a YAML value into a Component — public so component_file.rs can use it
pub fn parse_component_value(value: &Value) -> NtmlResult<Component> {
    parse_component_value_ctx(value, &[])
}

/// Parse a YAML value into a Component with optional import alias context
pub fn parse_component_value_ctx(value: &Value, import_aliases: &[String]) -> NtmlResult<Component> {
    let obj = value
        .as_mapping()
        .ok_or_else(|| NtmlError::ValidationError("Component must be an object".to_string()))?;

    if obj.is_empty() {
        return Err(NtmlError::EmptyDocument);
    }

    if obj.len() > 1 {
        return Err(NtmlError::MultipleRootComponents);
    }

    let (component_type, component_props) = obj
        .iter()
        .next()
        .ok_or_else(|| NtmlError::EmptyDocument)?;

    let component_name = component_type.as_str().ok_or_else(|| {
        NtmlError::ValidationError("Component type must be a string".to_string())
    })?;

    match component_name {
        "Container" => parse_container(component_props, import_aliases).map(Component::Container),
        "Flex" => parse_flex(component_props, import_aliases).map(Component::Flex),
        "Grid" => parse_grid(component_props, import_aliases).map(Component::Grid),
        "Stack" => parse_stack(component_props, import_aliases).map(Component::Stack),
        "Row" => parse_row(component_props, import_aliases).map(Component::Row),
        "Column" => parse_column(component_props, import_aliases).map(Component::Column),
        "Text" => parse_text(component_props).map(Component::Text),
        "Image" => parse_image(component_props).map(Component::Image),
        "Icon" => parse_icon(component_props).map(Component::Icon),
        "Button" => parse_button(component_props, import_aliases).map(Component::Button),
        "Input" => parse_input(component_props).map(Component::Input),
        "Checkbox" => parse_checkbox(component_props).map(Component::Checkbox),
        "Radio" => parse_radio(component_props).map(Component::Radio),
        "Select" => parse_select(component_props).map(Component::Select),
        "ProgressBar" => parse_progress_bar(component_props).map(Component::ProgressBar),
        "Badge" => parse_badge(component_props).map(Component::Badge),
        "Divider" => parse_divider(component_props).map(Component::Divider),
        "Spacer" => parse_spacer(component_props).map(Component::Spacer),
        "Link" => parse_link(component_props, import_aliases).map(Component::Link),
        "Code" => parse_code(component_props).map(Component::Code),
        "Markdown" => parse_markdown(component_props).map(Component::Markdown),
        "List" => parse_list(component_props, import_aliases).map(Component::List),
        "ListItem" => parse_list_item(component_props, import_aliases).map(Component::ListItem),
        "Heading" => parse_heading(component_props).map(Component::Heading),
        "Table" => parse_table(component_props).map(Component::Table),
        "Blockquote" => parse_blockquote(component_props, import_aliases).map(Component::Blockquote),
        "Pre" => parse_pre(component_props).map(Component::Pre),
        "Details" => parse_details(component_props, import_aliases).map(Component::Details),
        other => {
            if import_aliases.iter().any(|a| a == other) {
                parse_imported_component(other, component_props).map(Component::ImportedComponent)
            } else {
                Err(NtmlError::InvalidComponent {
                    component: other.to_string(),
                    reason: format!("Unknown component type '{}'. If this is an imported component, declare it in head.imports.", other),
                })
            }
        }
    }
}

/// Parse children array into Vec<Component>
fn parse_children(value: &Value, import_aliases: &[String]) -> NtmlResult<Option<Vec<Component>>> {
    if value.is_null() {
        return Ok(None);
    }

    let children_array = value
        .as_sequence()
        .ok_or_else(|| NtmlError::ValidationError("children must be an array".to_string()))?;

    let mut children = Vec::new();
    for child_value in children_array {
        let component = parse_component_value_ctx(child_value, import_aliases)?;
        children.push(component);
    }

    Ok(Some(children))
}

/// Parse an imported component instance
fn parse_imported_component(
    name: &str,
    value: &Value,
) -> NtmlResult<ImportedComponentInstance> {
    use std::collections::HashMap;

    let id;
    let mut props = HashMap::new();

    if value.is_null() {
        id = None;
    } else if let Some(obj) = value.as_mapping() {
        id = parse_id(obj);
        for (key, val) in obj {
            if let Some(key_str) = key.as_str() {
                if key_str != "id" {
                    props.insert(key_str.to_string(), val.clone());
                }
            }
        }
    } else {
        id = None;
    }

    Ok(ImportedComponentInstance {
        id,
        name: name.to_string(),
        props,
    })
}

/// Read optional id field from a component properties mapping
fn parse_id(obj: &serde_yaml::Mapping) -> Option<String> {
    obj.get(&Value::String("id".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract all data-* fields from a component properties mapping
fn parse_data_attributes(
    obj: &serde_yaml::Mapping,
) -> crate::error::NtmlResult<std::collections::HashMap<String, String>> {
    let mut data = std::collections::HashMap::new();
    for (key, val) in obj {
        if let Some(key_str) = key.as_str() {
            if key_str.starts_with("data-") {
                let value = val.as_str().ok_or_else(|| NtmlError::InvalidDataAttribute {
                    key: key_str.to_string(),
                    reason: "value must be a string".to_string(),
                })?;
                data.insert(key_str.to_string(), value.to_string());
            }
        }
    }
    Ok(data)
}

/// Parse the head section of a v0.2.0 document
fn parse_head(value: &Value) -> NtmlResult<Head> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::ValidationError("'head' must be an object".to_string())
    })?;

    let title = obj
        .get(&Value::String("title".to_string()))
        .and_then(|v| v.as_str())
        .ok_or(NtmlError::MissingTitle)?
        .to_string();

    let description = obj
        .get(&Value::String("description".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let author = obj
        .get(&Value::String("author".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tags = if let Some(tags_value) = obj.get(&Value::String("tags".to_string())) {
        Some(parse_tags(tags_value)?)
    } else {
        None
    };

    let fonts = if let Some(fonts_value) = obj.get(&Value::String("fonts".to_string())) {
        Some(parse_fonts(fonts_value)?)
    } else {
        None
    };

    let scripts = if let Some(scripts_value) = obj.get(&Value::String("scripts".to_string())) {
        Some(parse_scripts(scripts_value)?)
    } else {
        None
    };

    let imports = if let Some(imports_value) = obj.get(&Value::String("imports".to_string())) {
        Some(parse_imports(imports_value)?)
    } else {
        None
    };

    Ok(Head {
        title,
        description,
        author,
        tags,
        fonts,
        scripts,
        imports,
    })
}

fn parse_tags(value: &Value) -> NtmlResult<Vec<String>> {
    let arr = value
        .as_sequence()
        .ok_or_else(|| NtmlError::ValidationError("'tags' must be an array".to_string()))?;

    arr.iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| NtmlError::ValidationError("Each tag must be a string".to_string()))
        })
        .collect()
}

fn parse_fonts(value: &Value) -> NtmlResult<Vec<FontImport>> {
    let arr = value
        .as_sequence()
        .ok_or_else(|| NtmlError::ValidationError("'fonts' must be an array".to_string()))?;

    arr.iter()
        .map(|item| {
            let obj = item.as_mapping().ok_or_else(|| {
                NtmlError::ValidationError("Each font must be an object".to_string())
            })?;

            let family = obj
                .get(&Value::String("family".to_string()))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NtmlError::ValidationError("Font must have a 'family' field".to_string())
                })?
                .to_string();

            let weights_value = obj
                .get(&Value::String("weights".to_string()))
                .ok_or_else(|| {
                    NtmlError::ValidationError("Font must have a 'weights' field".to_string())
                })?;

            let weights_arr = weights_value.as_sequence().ok_or_else(|| {
                NtmlError::ValidationError("Font 'weights' must be an array".to_string())
            })?;

            let weights: Result<Vec<u16>, _> = weights_arr
                .iter()
                .map(|w| {
                    w.as_u64()
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| {
                            NtmlError::ValidationError(
                                "Font weight must be a positive integer".to_string(),
                            )
                        })
                })
                .collect();

            Ok(FontImport {
                family,
                weights: weights?,
            })
        })
        .collect()
}

fn parse_scripts(value: &Value) -> NtmlResult<Vec<ScriptImport>> {
    let arr = value
        .as_sequence()
        .ok_or_else(|| NtmlError::ValidationError("'scripts' must be an array".to_string()))?;

    arr.iter()
        .map(|item| {
            let obj = item.as_mapping().ok_or_else(|| {
                NtmlError::ValidationError("Each script must be an object".to_string())
            })?;

            let src = obj
                .get(&Value::String("src".to_string()))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NtmlError::ValidationError("Script must have a 'src' field".to_string())
                })?
                .to_string();

            Ok(ScriptImport { src })
        })
        .collect()
}

fn parse_imports(value: &Value) -> NtmlResult<Vec<ComponentImport>> {
    let arr = value
        .as_sequence()
        .ok_or_else(|| NtmlError::ValidationError("'imports' must be an array".to_string()))?;

    arr.iter()
        .map(|item| {
            let obj = item.as_mapping().ok_or_else(|| {
                NtmlError::ValidationError("Each import must be an object".to_string())
            })?;

            let src = obj
                .get(&Value::String("src".to_string()))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NtmlError::ValidationError("Import must have a 'src' field".to_string())
                })?
                .to_string();

            let alias = obj
                .get(&Value::String("as".to_string()))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    NtmlError::ValidationError("Import must have an 'as' field".to_string())
                })?
                .to_string();

            Ok(ComponentImport { src, alias })
        })
        .collect()
}

// --- Individual component parsers ---

fn parse_container(value: &Value, import_aliases: &[String]) -> NtmlResult<Container> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Container".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Container { id, style, children, data })
}

fn parse_flex(value: &Value, import_aliases: &[String]) -> NtmlResult<Flex> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Flex".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let direction = if let Some(dir_value) = obj.get(&Value::String("direction".to_string())) {
        Some(serde_yaml::from_value(dir_value.clone())?)
    } else {
        None
    };

    let justify = if let Some(just_value) = obj.get(&Value::String("justify".to_string())) {
        Some(serde_yaml::from_value(just_value.clone())?)
    } else {
        None
    };

    let align = if let Some(align_value) = obj.get(&Value::String("align".to_string())) {
        Some(serde_yaml::from_value(align_value.clone())?)
    } else {
        None
    };

    let gap = if let Some(gap_value) = obj.get(&Value::String("gap".to_string())) {
        Some(gap_value.as_f64().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Flex".to_string(),
            property: "gap".to_string(),
            reason: "must be a number".to_string(),
        })?)
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Flex".to_string(),
            property: "wrap".to_string(),
            reason: "must be a boolean".to_string(),
        })?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Flex { id, direction, justify, align, gap, wrap, style, children, data })
}

fn parse_grid(value: &Value, import_aliases: &[String]) -> NtmlResult<Grid> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Grid".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let columns_value = obj
        .get(&Value::String("columns".to_string()))
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Grid".to_string(),
            property: "columns".to_string(),
        })?;
    let columns = serde_yaml::from_value(columns_value.clone())?;

    let rows = if let Some(rows_value) = obj.get(&Value::String("rows".to_string())) {
        Some(serde_yaml::from_value(rows_value.clone())?)
    } else {
        None
    };

    let gap = if let Some(gap_value) = obj.get(&Value::String("gap".to_string())) {
        Some(serde_yaml::from_value(gap_value.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Grid { id, columns, rows, gap, style, children, data })
}

fn parse_stack(value: &Value, import_aliases: &[String]) -> NtmlResult<Stack> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Stack".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let alignment = if let Some(align_value) = obj.get(&Value::String("alignment".to_string())) {
        Some(serde_yaml::from_value(align_value.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Stack { id, alignment, style, children, data })
}

fn parse_row(value: &Value, import_aliases: &[String]) -> NtmlResult<Row> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Row".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let justify = if let Some(just_value) = obj.get(&Value::String("justify".to_string())) {
        Some(serde_yaml::from_value(just_value.clone())?)
    } else {
        None
    };

    let align = if let Some(align_value) = obj.get(&Value::String("align".to_string())) {
        Some(serde_yaml::from_value(align_value.clone())?)
    } else {
        None
    };

    let gap = if let Some(gap_value) = obj.get(&Value::String("gap".to_string())) {
        Some(gap_value.as_f64().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Row".to_string(),
            property: "gap".to_string(),
            reason: "must be a number".to_string(),
        })?)
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Row".to_string(),
            property: "wrap".to_string(),
            reason: "must be a boolean".to_string(),
        })?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Row { id, justify, align, gap, wrap, style, children, data })
}

fn parse_column(value: &Value, import_aliases: &[String]) -> NtmlResult<Column> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Column".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let justify = if let Some(just_value) = obj.get(&Value::String("justify".to_string())) {
        Some(serde_yaml::from_value(just_value.clone())?)
    } else {
        None
    };

    let align = if let Some(align_value) = obj.get(&Value::String("align".to_string())) {
        Some(serde_yaml::from_value(align_value.clone())?)
    } else {
        None
    };

    let gap = if let Some(gap_value) = obj.get(&Value::String("gap".to_string())) {
        Some(gap_value.as_f64().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Column".to_string(),
            property: "gap".to_string(),
            reason: "must be a number".to_string(),
        })?)
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| NtmlError::InvalidProperty {
            component: "Column".to_string(),
            property: "wrap".to_string(),
            reason: "must be a boolean".to_string(),
        })?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Column { id, justify, align, gap, wrap, style, children, data })
}

fn parse_text(value: &Value) -> NtmlResult<Text> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Text".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let text = obj
        .get(&Value::String("text".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Text".to_string(),
            property: "text".to_string(),
        })?
        .to_string();

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Text { id, text, style, data })
}

fn parse_image(value: &Value) -> NtmlResult<Image> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Image".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let src = obj
        .get(&Value::String("src".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Image".to_string(),
            property: "src".to_string(),
        })?
        .to_string();

    let alt = obj
        .get(&Value::String("alt".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let fit = if let Some(fit_value) = obj.get(&Value::String("fit".to_string())) {
        Some(serde_yaml::from_value(fit_value.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Image { id, src, alt, fit, style, data })
}

fn parse_icon(value: &Value) -> NtmlResult<Icon> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Icon".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let name = obj
        .get(&Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Icon".to_string(),
            property: "name".to_string(),
        })?
        .to_string();

    let size = obj
        .get(&Value::String("size".to_string()))
        .and_then(|v| v.as_f64());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Icon { id, name, size, style, data })
}

fn parse_button(value: &Value, import_aliases: &[String]) -> NtmlResult<Button> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Button".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let action = obj
        .get(&Value::String("action".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Button".to_string(),
            property: "action".to_string(),
        })?
        .to_string();

    let variant = if let Some(variant_value) = obj.get(&Value::String("variant".to_string())) {
        Some(serde_yaml::from_value(variant_value.clone())?)
    } else {
        None
    };

    let disabled = obj
        .get(&Value::String("disabled".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Button { id, action, variant, disabled, style, children, data })
}

fn parse_input(value: &Value) -> NtmlResult<Input> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Input".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let name = obj
        .get(&Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Input".to_string(),
            property: "name".to_string(),
        })?
        .to_string();

    let placeholder = obj
        .get(&Value::String("placeholder".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let value_str = obj
        .get(&Value::String("value".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let input_type = if let Some(type_value) = obj.get(&Value::String("type".to_string())) {
        Some(serde_yaml::from_value(type_value.clone())?)
    } else {
        None
    };

    let max_length = obj
        .get(&Value::String("maxLength".to_string()))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let disabled = obj
        .get(&Value::String("disabled".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Input { id, name, placeholder, value: value_str, input_type, max_length, disabled, style, data })
}

fn parse_checkbox(value: &Value) -> NtmlResult<Checkbox> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Checkbox".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let name = obj
        .get(&Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Checkbox".to_string(),
            property: "name".to_string(),
        })?
        .to_string();

    let label = obj
        .get(&Value::String("label".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let checked = obj
        .get(&Value::String("checked".to_string()))
        .and_then(|v| v.as_bool());

    let disabled = obj
        .get(&Value::String("disabled".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Checkbox { id, name, label, checked, disabled, style, data })
}

fn parse_radio(value: &Value) -> NtmlResult<Radio> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Radio".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let name = obj
        .get(&Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Radio".to_string(),
            property: "name".to_string(),
        })?
        .to_string();

    let value_str = obj
        .get(&Value::String("value".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Radio".to_string(),
            property: "value".to_string(),
        })?
        .to_string();

    let label = obj
        .get(&Value::String("label".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let checked = obj
        .get(&Value::String("checked".to_string()))
        .and_then(|v| v.as_bool());

    let disabled = obj
        .get(&Value::String("disabled".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Radio { id, name, value: value_str, label, checked, disabled, style, data })
}

fn parse_select(value: &Value) -> NtmlResult<Select> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Select".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let name = obj
        .get(&Value::String("name".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Select".to_string(),
            property: "name".to_string(),
        })?
        .to_string();

    let options_value = obj
        .get(&Value::String("options".to_string()))
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Select".to_string(),
            property: "options".to_string(),
        })?;
    let options: Vec<SelectOption> = serde_yaml::from_value(options_value.clone())?;

    let value_str = obj
        .get(&Value::String("value".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let disabled = obj
        .get(&Value::String("disabled".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Select { id, name, options, value: value_str, disabled, style, data })
}

fn parse_progress_bar(value: &Value) -> NtmlResult<ProgressBar> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "ProgressBar".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let value_num = obj
        .get(&Value::String("value".to_string()))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "ProgressBar".to_string(),
            property: "value".to_string(),
        })?;

    let max = obj
        .get(&Value::String("max".to_string()))
        .and_then(|v| v.as_f64());

    let variant = if let Some(variant_value) = obj.get(&Value::String("variant".to_string())) {
        Some(serde_yaml::from_value(variant_value.clone())?)
    } else {
        None
    };

    let show_label = obj
        .get(&Value::String("showLabel".to_string()))
        .and_then(|v| v.as_bool());

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(ProgressBar { id, value: value_num, max, variant, show_label, style, data })
}

fn parse_badge(value: &Value) -> NtmlResult<Badge> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Badge".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let text = obj
        .get(&Value::String("text".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Badge".to_string(),
            property: "text".to_string(),
        })?
        .to_string();

    let variant = if let Some(variant_value) = obj.get(&Value::String("variant".to_string())) {
        Some(serde_yaml::from_value(variant_value.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Badge { id, text, variant, style, data })
}

fn parse_divider(value: &Value) -> NtmlResult<Divider> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Divider".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let orientation =
        if let Some(orient_value) = obj.get(&Value::String("orientation".to_string())) {
            Some(serde_yaml::from_value(orient_value.clone())?)
        } else {
            None
        };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Divider { id, orientation, style, data })
}

fn parse_spacer(value: &Value) -> NtmlResult<Spacer> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Spacer".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let data = parse_data_attributes(obj)?;

    let size_value = obj
        .get(&Value::String("size".to_string()))
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Spacer".to_string(),
            property: "size".to_string(),
        })?;

    let size = if let Some(num) = size_value.as_f64() {
        SpacerSize::Fixed(num)
    } else if let Some(s) = size_value.as_str() {
        if s == "auto" {
            SpacerSize::Auto(s.to_string())
        } else {
            return Err(NtmlError::InvalidProperty {
                component: "Spacer".to_string(),
                property: "size".to_string(),
                reason: "must be a number or 'auto'".to_string(),
            });
        }
    } else {
        return Err(NtmlError::InvalidProperty {
            component: "Spacer".to_string(),
            property: "size".to_string(),
            reason: "must be a number or 'auto'".to_string(),
        });
    };

    Ok(Spacer { size, data })
}

fn parse_link(value: &Value, import_aliases: &[String]) -> NtmlResult<Link> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Link".to_string(),
        reason: "properties must be an object".to_string(),
    })?;

    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;

    let href = obj
        .get(&Value::String("href".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Link".to_string(),
            property: "href".to_string(),
        })?
        .to_string();

    let target = if let Some(t) = obj.get(&Value::String("target".to_string())) {
        Some(serde_yaml::from_value(t.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value, import_aliases)?
    } else {
        None
    };

    Ok(Link {
        id,
        href,
        target,
        style,
        children,
        data,
    })
}

fn parse_code(value: &Value) -> NtmlResult<Code> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Code".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let text = obj
        .get(&Value::String("text".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Code".to_string(),
            property: "text".to_string(),
        })?
        .to_string();
    let language = obj.get(&Value::String("language".to_string())).and_then(|v| v.as_str()).map(String::from);
    let block = obj.get(&Value::String("block".to_string())).and_then(|v| v.as_bool());
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    Ok(Code { id, text, language, block, style, data })
}

fn parse_markdown(value: &Value) -> NtmlResult<Markdown> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Markdown".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let content = obj
        .get(&Value::String("content".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Markdown".to_string(),
            property: "content".to_string(),
        })?
        .to_string();
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    Ok(Markdown { id, content, style, data })
}

fn parse_list(value: &Value, import_aliases: &[String]) -> NtmlResult<List> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "List".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let ordered = obj.get(&Value::String("ordered".to_string())).and_then(|v| v.as_bool());
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    let children = obj.get(&Value::String("children".to_string())).and_then(|c| parse_children(c, import_aliases).ok()).flatten();
    Ok(List { id, ordered, style, children, data })
}

fn parse_list_item(value: &Value, import_aliases: &[String]) -> NtmlResult<ListItem> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "ListItem".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    let children = obj.get(&Value::String("children".to_string())).and_then(|c| parse_children(c, import_aliases).ok()).flatten();
    Ok(ListItem { id, style, children, data })
}

fn parse_heading(value: &Value) -> NtmlResult<Heading> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Heading".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let level = obj
        .get(&Value::String("level".to_string()))
        .and_then(|v| v.as_u64())
        .map(|n| n as u8)
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
        })?;
    if level < 1 || level > 3 {
        return Err(NtmlError::InvalidProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
            reason: "must be 1, 2, or 3".to_string(),
        });
    }
    let text = obj
        .get(&Value::String("text".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Heading".to_string(),
            property: "text".to_string(),
        })?
        .to_string();
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    Ok(Heading { id, level, text, style, data })
}

fn parse_table(value: &Value) -> NtmlResult<Table> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Table".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let headers = obj
        .get(&Value::String("headers".to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let rows = obj
        .get(&Value::String("rows".to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_sequence())
                .map(|row| row.iter().filter_map(|c| c.as_str().map(String::from)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    Ok(Table { id, headers, rows, style, data })
}

fn parse_blockquote(value: &Value, import_aliases: &[String]) -> NtmlResult<Blockquote> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Blockquote".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    let children = obj.get(&Value::String("children".to_string())).and_then(|c| parse_children(c, import_aliases).ok()).flatten();
    Ok(Blockquote { id, style, children, data })
}

fn parse_pre(value: &Value) -> NtmlResult<Pre> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Pre".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let text = obj
        .get(&Value::String("text".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Pre".to_string(),
            property: "text".to_string(),
        })?
        .to_string();
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    Ok(Pre { id, text, style, data })
}

fn parse_details(value: &Value, import_aliases: &[String]) -> NtmlResult<Details> {
    let obj = value.as_mapping().ok_or_else(|| NtmlError::InvalidComponent {
        component: "Details".to_string(),
        reason: "properties must be an object".to_string(),
    })?;
    let id = parse_id(obj);
    let data = parse_data_attributes(obj)?;
    let summary = obj
        .get(&Value::String("summary".to_string()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Details".to_string(),
            property: "summary".to_string(),
        })?
        .to_string();
    let open = obj.get(&Value::String("open".to_string())).and_then(|v| v.as_bool());
    let style = obj.get(&Value::String("style".to_string())).and_then(|s| serde_yaml::from_value(s.clone()).ok());
    let children = obj.get(&Value::String("children".to_string())).and_then(|c| parse_children(c, import_aliases).ok()).flatten();
    Ok(Details { id, summary, open, style, children, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let yaml = r#"
Text:
  text: "Hello World"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_text_with_id() {
        let yaml = r#"
Text:
  id: "my-text"
  text: "Hello World"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_ok());
        if let Component::Text(t) = result.unwrap() {
            assert_eq!(t.id, Some("my-text".to_string()));
        } else {
            panic!("Expected Text component");
        }
    }

    #[test]
    fn test_parse_container_with_children() {
        let yaml = r#"
Container:
  style:
    padding: 16
    backgroundColor: red
  children:
    - Text:
        text: "Hello"
    - Text:
        text: "World"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_flex_layout() {
        let yaml = r#"
Flex:
  direction: column
  gap: 12
  align: center
  children:
    - Text:
        text: "Item 1"
    - Text:
        text: "Item 2"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_color() {
        let yaml = r#"
Text:
  text: "Test"
  style:
    color: "invalid-color"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_property() {
        let yaml = r#"
Button:
  variant: primary
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_document_classic() {
        let yaml = r#"
Text:
  text: "Hello"
"#;
        let result = parse_document(yaml);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), NtmlDocument::Classic(_)));
    }

    #[test]
    fn test_parse_document_full() {
        let yaml = r#"
head:
  title: "My Page"
  description: "A test page"
  tags: [test, ntml]

body:
  Text:
    text: "Hello from v0.2.0"
"#;
        let result = parse_document(yaml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        if let NtmlDocument::Full { head, .. } = result.unwrap() {
            assert_eq!(head.title, "My Page");
            assert_eq!(head.description, Some("A test page".to_string()));
        } else {
            panic!("Expected Full document");
        }
    }

    #[test]
    fn test_parse_ntml_rejects_full_format() {
        let yaml = r#"
head:
  title: "My Page"
body:
  Text:
    text: "Hello"
"#;
        let result = parse_ntml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_format_requires_body() {
        let yaml = r#"
head:
  title: "My Page"
"#;
        let result = parse_document(yaml);
        assert!(matches!(result, Err(NtmlError::MissingBody)));
    }

    #[test]
    fn test_full_format_requires_title() {
        let yaml = r#"
head:
  description: "No title here"
body:
  Text:
    text: "Hello"
"#;
        let result = parse_document(yaml);
        assert!(matches!(result, Err(NtmlError::MissingTitle)));
    }
}
