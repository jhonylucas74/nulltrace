use crate::components::*;
use crate::error::{NtmlError, NtmlResult};
use crate::style::*;
use regex::Regex;
use std::sync::OnceLock;

const MAX_NESTING_DEPTH: usize = 20;

/// Validate a component tree
pub fn validate_component(component: &Component) -> NtmlResult<()> {
    validate_component_recursive(component, 0)
}

fn validate_component_recursive(component: &Component, depth: usize) -> NtmlResult<()> {
    if depth > MAX_NESTING_DEPTH {
        return Err(NtmlError::MaxNestingDepthExceeded {
            max_depth: MAX_NESTING_DEPTH,
        });
    }

    match component {
        Component::Container(c) => validate_container(c, depth),
        Component::Flex(c) => validate_flex(c, depth),
        Component::Grid(c) => validate_grid(c, depth),
        Component::Stack(c) => validate_stack(c, depth),
        Component::Row(c) => validate_row(c, depth),
        Component::Column(c) => validate_column(c, depth),
        Component::Text(c) => validate_text(c),
        Component::Image(c) => validate_image(c),
        Component::Icon(c) => validate_icon(c),
        Component::Button(c) => validate_button(c, depth),
        Component::Input(c) => validate_input(c),
        Component::Checkbox(c) => validate_checkbox(c),
        Component::Radio(c) => validate_radio(c),
        Component::Select(c) => validate_select(c),
        Component::ProgressBar(c) => validate_progress_bar(c),
        Component::Badge(c) => validate_badge(c),
        Component::Divider(c) => validate_divider(c),
        Component::Spacer(c) => validate_spacer(c),
    }
}

fn validate_children(children: &Option<Vec<Component>>, depth: usize) -> NtmlResult<()> {
    if let Some(children) = children {
        for child in children {
            validate_component_recursive(child, depth + 1)?;
        }
    }
    Ok(())
}

fn validate_style(style: &Option<Style>) -> NtmlResult<()> {
    if let Some(style) = style {
        // Validate colors
        if let Some(ref color) = style.color {
            validate_color(color, "color")?;
        }
        if let Some(ref bg_color) = style.background_color {
            validate_color(bg_color, "backgroundColor")?;
        }
        if let Some(ref border_color) = style.border_color {
            validate_color(border_color, "borderColor")?;
        }

        // Validate opacity
        if let Some(opacity) = style.opacity {
            validate_range(opacity, 0.0, 1.0, "opacity")?;
        }

        // Validate font weight
        if let Some(ref font_weight) = style.font_weight {
            if let FontWeight::Number(weight) = font_weight {
                if *weight < 100 || *weight > 900 || weight % 100 != 0 {
                    return Err(NtmlError::InvalidStyle {
                        property: "fontWeight".to_string(),
                        reason: "must be between 100 and 900 in increments of 100".to_string(),
                    });
                }
            }
        }

        // Validate line height
        if let Some(line_height) = style.line_height {
            if line_height < 0.0 {
                return Err(NtmlError::InvalidStyle {
                    property: "lineHeight".to_string(),
                    reason: "must be non-negative".to_string(),
                });
            }
        }

        // Validate flex
        if let Some(flex) = style.flex {
            if flex < 0.0 {
                return Err(NtmlError::InvalidStyle {
                    property: "flex".to_string(),
                    reason: "must be non-negative".to_string(),
                });
            }
        }
    }
    Ok(())
}

fn validate_color(color: &str, _property: &str) -> NtmlResult<()> {
    static HEX_COLOR_REGEX: OnceLock<Regex> = OnceLock::new();
    let hex_regex = HEX_COLOR_REGEX.get_or_init(|| Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap());

    const NAMED_COLORS: &[&str] = &[
        "red", "blue", "green", "white", "black", "transparent", "yellow", "orange", "purple",
        "pink", "gray", "grey",
    ];

    if hex_regex.is_match(color) || NAMED_COLORS.contains(&color) {
        Ok(())
    } else {
        Err(NtmlError::InvalidColor {
            value: color.to_string(),
            reason: format!(
                "must be a valid hex color (e.g., #ff0000) or named color ({})",
                NAMED_COLORS.join(", ")
            ),
        })
    }
}

fn validate_range(value: f64, min: f64, max: f64, property: &str) -> NtmlResult<()> {
    if value < min || value > max {
        Err(NtmlError::ValueOutOfRange {
            property: property.to_string(),
            value: value.to_string(),
            range: format!("{} to {}", min, max),
        })
    } else {
        Ok(())
    }
}

// Component validators

fn validate_container(container: &Container, depth: usize) -> NtmlResult<()> {
    validate_style(&container.style)?;
    validate_children(&container.children, depth)
}

fn validate_flex(flex: &Flex, depth: usize) -> NtmlResult<()> {
    if let Some(gap) = flex.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Flex".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&flex.style)?;
    validate_children(&flex.children, depth)
}

fn validate_grid(grid: &Grid, depth: usize) -> NtmlResult<()> {
    // Validate columns
    match &grid.columns {
        GridSize::Count(count) => {
            if *count == 0 {
                return Err(NtmlError::InvalidProperty {
                    component: "Grid".to_string(),
                    property: "columns".to_string(),
                    reason: "must be greater than 0".to_string(),
                });
            }
        }
        GridSize::Definitions(defs) => {
            if defs.is_empty() {
                return Err(NtmlError::InvalidProperty {
                    component: "Grid".to_string(),
                    property: "columns".to_string(),
                    reason: "must not be empty".to_string(),
                });
            }
        }
    }

    // Validate gap
    if let Some(gap) = &grid.gap {
        match gap {
            GridGap::Single(g) => {
                if *g < 0.0 {
                    return Err(NtmlError::InvalidProperty {
                        component: "Grid".to_string(),
                        property: "gap".to_string(),
                        reason: "must be non-negative".to_string(),
                    });
                }
            }
            GridGap::Separate { row, column } => {
                if *row < 0.0 || *column < 0.0 {
                    return Err(NtmlError::InvalidProperty {
                        component: "Grid".to_string(),
                        property: "gap".to_string(),
                        reason: "row and column must be non-negative".to_string(),
                    });
                }
            }
        }
    }

    validate_style(&grid.style)?;
    validate_children(&grid.children, depth)
}

fn validate_stack(stack: &Stack, depth: usize) -> NtmlResult<()> {
    validate_style(&stack.style)?;
    validate_children(&stack.children, depth)
}

fn validate_row(row: &Row, depth: usize) -> NtmlResult<()> {
    if let Some(gap) = row.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Row".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&row.style)?;
    validate_children(&row.children, depth)
}

fn validate_column(column: &Column, depth: usize) -> NtmlResult<()> {
    if let Some(gap) = column.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Column".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&column.style)?;
    validate_children(&column.children, depth)
}

fn validate_text(text: &Text) -> NtmlResult<()> {
    if text.text.is_empty() {
        return Err(NtmlError::ValidationError(
            "Text component must have non-empty text".to_string(),
        ));
    }
    validate_style(&text.style)
}

fn validate_image(image: &Image) -> NtmlResult<()> {
    if image.src.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Image".to_string(),
            property: "src".to_string(),
        });
    }
    // Note: Asset whitelisting would be done at runtime with actual whitelist
    validate_style(&image.style)
}

fn validate_icon(icon: &Icon) -> NtmlResult<()> {
    if icon.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Icon".to_string(),
            property: "name".to_string(),
        });
    }
    if let Some(size) = icon.size {
        if size <= 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Icon".to_string(),
                property: "size".to_string(),
                reason: "must be positive".to_string(),
            });
        }
    }
    validate_style(&icon.style)
}

fn validate_button(button: &Button, depth: usize) -> NtmlResult<()> {
    if button.action.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Button".to_string(),
            property: "action".to_string(),
        });
    }
    validate_style(&button.style)?;
    validate_children(&button.children, depth)
}

fn validate_input(input: &Input) -> NtmlResult<()> {
    if input.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Input".to_string(),
            property: "name".to_string(),
        });
    }
    validate_style(&input.style)
}

fn validate_checkbox(checkbox: &Checkbox) -> NtmlResult<()> {
    if checkbox.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Checkbox".to_string(),
            property: "name".to_string(),
        });
    }
    validate_style(&checkbox.style)
}

fn validate_radio(radio: &Radio) -> NtmlResult<()> {
    if radio.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Radio".to_string(),
            property: "name".to_string(),
        });
    }
    if radio.value.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Radio".to_string(),
            property: "value".to_string(),
        });
    }
    validate_style(&radio.style)
}

fn validate_select(select: &Select) -> NtmlResult<()> {
    if select.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Select".to_string(),
            property: "name".to_string(),
        });
    }
    if select.options.is_empty() {
        return Err(NtmlError::ValidationError(
            "Select component must have at least one option".to_string(),
        ));
    }
    validate_style(&select.style)
}

fn validate_progress_bar(progress_bar: &ProgressBar) -> NtmlResult<()> {
    let max = progress_bar.max.unwrap_or(100.0);
    validate_range(progress_bar.value, 0.0, max, "value")?;
    if let Some(max_val) = progress_bar.max {
        if max_val <= 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "ProgressBar".to_string(),
                property: "max".to_string(),
                reason: "must be positive".to_string(),
            });
        }
    }
    validate_style(&progress_bar.style)
}

fn validate_badge(badge: &Badge) -> NtmlResult<()> {
    if badge.text.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Badge".to_string(),
            property: "text".to_string(),
        });
    }
    validate_style(&badge.style)
}

fn validate_divider(divider: &Divider) -> NtmlResult<()> {
    validate_style(&divider.style)
}

fn validate_spacer(_spacer: &Spacer) -> NtmlResult<()> {
    // Spacer validation is minimal - size can be any number or "auto"
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_color() {
        assert!(validate_color("#ff0000", "color").is_ok());
        assert!(validate_color("#FF0000", "color").is_ok());
        assert!(validate_color("red", "color").is_ok());
        assert!(validate_color("transparent", "color").is_ok());
        assert!(validate_color("#ff00", "color").is_err());
        assert!(validate_color("invalid", "color").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(0.5, 0.0, 1.0, "opacity").is_ok());
        assert!(validate_range(0.0, 0.0, 1.0, "opacity").is_ok());
        assert!(validate_range(1.0, 0.0, 1.0, "opacity").is_ok());
        assert!(validate_range(-0.1, 0.0, 1.0, "opacity").is_err());
        assert!(validate_range(1.1, 0.0, 1.0, "opacity").is_err());
    }
}
