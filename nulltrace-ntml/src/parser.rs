use crate::components::*;
use crate::error::{NtmlError, NtmlResult};
use crate::theme::Theme;
use crate::validator::validate_component;
use serde_yaml::Value;

/// Parse an NTML document from YAML string
pub fn parse_ntml(yaml: &str) -> NtmlResult<Component> {
    parse_ntml_with_theme(yaml, Theme::default())
}

/// Parse an NTML document with a custom theme
pub fn parse_ntml_with_theme(yaml: &str, _theme: Theme) -> NtmlResult<Component> {
    // Parse YAML into generic Value
    let value: Value = serde_yaml::from_str(yaml)?;

    // Extract root component
    let component = parse_component(&value)?;

    // Validate the component tree
    validate_component(&component)?;

    Ok(component)
}

/// Parse a YAML value into a Component
fn parse_component(value: &Value) -> NtmlResult<Component> {
    let obj = value
        .as_mapping()
        .ok_or_else(|| NtmlError::ValidationError("Component must be an object".to_string()))?;

    if obj.is_empty() {
        return Err(NtmlError::EmptyDocument);
    }

    if obj.len() > 1 {
        return Err(NtmlError::MultipleRootComponents);
    }

    // Get the single key-value pair (component type and its properties)
    let (component_type, component_props) = obj
        .iter()
        .next()
        .ok_or_else(|| NtmlError::EmptyDocument)?;

    let component_name = component_type
        .as_str()
        .ok_or_else(|| {
            NtmlError::ValidationError("Component type must be a string".to_string())
        })?;

    // Parse based on component type
    match component_name {
        "Container" => parse_container(component_props).map(Component::Container),
        "Flex" => parse_flex(component_props).map(Component::Flex),
        "Grid" => parse_grid(component_props).map(Component::Grid),
        "Stack" => parse_stack(component_props).map(Component::Stack),
        "Row" => parse_row(component_props).map(Component::Row),
        "Column" => parse_column(component_props).map(Component::Column),
        "Text" => parse_text(component_props).map(Component::Text),
        "Image" => parse_image(component_props).map(Component::Image),
        "Icon" => parse_icon(component_props).map(Component::Icon),
        "Button" => parse_button(component_props).map(Component::Button),
        "Input" => parse_input(component_props).map(Component::Input),
        "Checkbox" => parse_checkbox(component_props).map(Component::Checkbox),
        "Radio" => parse_radio(component_props).map(Component::Radio),
        "Select" => parse_select(component_props).map(Component::Select),
        "ProgressBar" => parse_progress_bar(component_props).map(Component::ProgressBar),
        "Badge" => parse_badge(component_props).map(Component::Badge),
        "Divider" => parse_divider(component_props).map(Component::Divider),
        "Spacer" => parse_spacer(component_props).map(Component::Spacer),
        _ => Err(NtmlError::InvalidComponent {
            component: component_name.to_string(),
            reason: format!("Unknown component type '{}'", component_name),
        }),
    }
}

/// Parse children array into Vec<Component>
fn parse_children(value: &Value) -> NtmlResult<Option<Vec<Component>>> {
    if value.is_null() {
        return Ok(None);
    }

    let children_array = value.as_sequence().ok_or_else(|| {
        NtmlError::ValidationError("children must be an array".to_string())
    })?;

    let mut children = Vec::new();
    for child_value in children_array {
        let component = parse_component(child_value)?;
        children.push(component);
    }

    Ok(Some(children))
}

// Individual component parsers

fn parse_container(value: &Value) -> NtmlResult<Container> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Container".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    let children = if let Some(children_value) = obj.get(&Value::String("children".to_string())) {
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Container { style, children })
}

fn parse_flex(value: &Value) -> NtmlResult<Flex> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Flex".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        Some(
            gap_value
                .as_f64()
                .ok_or_else(|| NtmlError::InvalidProperty {
                    component: "Flex".to_string(),
                    property: "gap".to_string(),
                    reason: "must be a number".to_string(),
                })?,
        )
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| {
            NtmlError::InvalidProperty {
                component: "Flex".to_string(),
                property: "wrap".to_string(),
                reason: "must be a boolean".to_string(),
            }
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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Flex {
        direction,
        justify,
        align,
        gap,
        wrap,
        style,
        children,
    })
}

fn parse_grid(value: &Value) -> NtmlResult<Grid> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Grid".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Grid {
        columns,
        rows,
        gap,
        style,
        children,
    })
}

fn parse_stack(value: &Value) -> NtmlResult<Stack> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Stack".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Stack {
        alignment,
        style,
        children,
    })
}

fn parse_row(value: &Value) -> NtmlResult<Row> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Row".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        Some(gap_value.as_f64().ok_or_else(|| {
            NtmlError::InvalidProperty {
                component: "Row".to_string(),
                property: "gap".to_string(),
                reason: "must be a number".to_string(),
            }
        })?)
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| {
            NtmlError::InvalidProperty {
                component: "Row".to_string(),
                property: "wrap".to_string(),
                reason: "must be a boolean".to_string(),
            }
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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Row {
        justify,
        align,
        gap,
        wrap,
        style,
        children,
    })
}

fn parse_column(value: &Value) -> NtmlResult<Column> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Column".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        Some(gap_value.as_f64().ok_or_else(|| {
            NtmlError::InvalidProperty {
                component: "Column".to_string(),
                property: "gap".to_string(),
                reason: "must be a number".to_string(),
            }
        })?)
    } else {
        None
    };

    let wrap = if let Some(wrap_value) = obj.get(&Value::String("wrap".to_string())) {
        Some(wrap_value.as_bool().ok_or_else(|| {
            NtmlError::InvalidProperty {
                component: "Column".to_string(),
                property: "wrap".to_string(),
                reason: "must be a boolean".to_string(),
            }
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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Column {
        justify,
        align,
        gap,
        wrap,
        style,
        children,
    })
}

fn parse_text(value: &Value) -> NtmlResult<Text> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Text".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Text { text, style })
}

fn parse_image(value: &Value) -> NtmlResult<Image> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Image".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Image {
        src,
        alt,
        fit,
        style,
    })
}

fn parse_icon(value: &Value) -> NtmlResult<Icon> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Icon".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Icon { name, size, style })
}

fn parse_button(value: &Value) -> NtmlResult<Button> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Button".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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
        parse_children(children_value)?
    } else {
        None
    };

    Ok(Button {
        action,
        variant,
        disabled,
        style,
        children,
    })
}

fn parse_input(value: &Value) -> NtmlResult<Input> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Input".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Input {
        name,
        placeholder,
        value: value_str,
        input_type,
        max_length,
        disabled,
        style,
    })
}

fn parse_checkbox(value: &Value) -> NtmlResult<Checkbox> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Checkbox".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Checkbox {
        name,
        label,
        checked,
        disabled,
        style,
    })
}

fn parse_radio(value: &Value) -> NtmlResult<Radio> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Radio".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Radio {
        name,
        value: value_str,
        label,
        checked,
        disabled,
        style,
    })
}

fn parse_select(value: &Value) -> NtmlResult<Select> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Select".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Select {
        name,
        options,
        value: value_str,
        disabled,
        style,
    })
}

fn parse_progress_bar(value: &Value) -> NtmlResult<ProgressBar> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "ProgressBar".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(ProgressBar {
        value: value_num,
        max,
        variant,
        show_label,
        style,
    })
}

fn parse_badge(value: &Value) -> NtmlResult<Badge> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Badge".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Badge {
        text,
        variant,
        style,
    })
}

fn parse_divider(value: &Value) -> NtmlResult<Divider> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Divider".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

    let orientation = if let Some(orient_value) = obj.get(&Value::String("orientation".to_string()))
    {
        Some(serde_yaml::from_value(orient_value.clone())?)
    } else {
        None
    };

    let style = if let Some(style_value) = obj.get(&Value::String("style".to_string())) {
        Some(serde_yaml::from_value(style_value.clone())?)
    } else {
        None
    };

    Ok(Divider { orientation, style })
}

fn parse_spacer(value: &Value) -> NtmlResult<Spacer> {
    let obj = value.as_mapping().ok_or_else(|| {
        NtmlError::InvalidComponent {
            component: "Spacer".to_string(),
            reason: "properties must be an object".to_string(),
        }
    })?;

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

    Ok(Spacer { size })
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
}
