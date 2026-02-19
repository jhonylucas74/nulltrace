use crate::component_file::{is_pascal_case, ComponentFile, BUILTIN_COMPONENTS};
use crate::components::*;
use crate::error::{NtmlError, NtmlResult};
use crate::head::Head;
use crate::style::*;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

const MAX_NESTING_DEPTH: usize = 20;
const MAX_TAGS: usize = 10;
const MAX_FONTS: usize = 10;
const MAX_SCRIPTS: usize = 5;
const MAX_IMPORTS: usize = 10;

/// Validate a component tree (no custom font families)
pub fn validate_component(component: &Component) -> NtmlResult<()> {
    validate_component_with_context(component, &[])
}

/// Validate a component tree with the list of declared font families from head
pub fn validate_component_with_context(
    component: &Component,
    font_families: &[String],
) -> NtmlResult<()> {
    validate_id_uniqueness(component)?;
    validate_component_recursive(component, 0, font_families)
}

/// Validate a head section
pub fn validate_head(head: &Head) -> NtmlResult<()> {
    // Title must be non-empty
    if head.title.is_empty() {
        return Err(NtmlError::MissingTitle);
    }

    // Tags validation
    if let Some(ref tags) = head.tags {
        if tags.len() > MAX_TAGS {
            return Err(NtmlError::TagLimitExceeded { max: MAX_TAGS });
        }
        for tag in tags {
            validate_tag(tag)?;
        }
    }

    // Fonts validation
    if let Some(ref fonts) = head.fonts {
        if fonts.len() > MAX_FONTS {
            return Err(NtmlError::FontLimitExceeded { max: MAX_FONTS });
        }
        for font in fonts {
            if font.family.is_empty() {
                return Err(NtmlError::InvalidFontFamily {
                    family: font.family.clone(),
                });
            }
            if font.weights.is_empty() {
                return Err(NtmlError::ValidationError(format!(
                    "Font '{}' must have at least one weight",
                    font.family
                )));
            }
            for &weight in &font.weights {
                if weight < 100 || weight > 900 || weight % 100 != 0 {
                    return Err(NtmlError::InvalidStyle {
                        property: format!("fonts[{}].weights", font.family),
                        reason: format!(
                            "weight {} is invalid: must be 100-900 in increments of 100",
                            weight
                        ),
                    });
                }
            }
        }
    }

    // Scripts validation
    if let Some(ref scripts) = head.scripts {
        if scripts.len() > MAX_SCRIPTS {
            return Err(NtmlError::ScriptLimitExceeded { max: MAX_SCRIPTS });
        }
        for script in scripts {
            if !script.src.ends_with(".lua") {
                return Err(NtmlError::ValidationError(format!(
                    "Script src '{}' must have a .lua extension",
                    script.src
                )));
            }
        }
    }

    // Imports validation
    if let Some(ref imports) = head.imports {
        if imports.len() > MAX_IMPORTS {
            return Err(NtmlError::ImportLimitExceeded { max: MAX_IMPORTS });
        }
        for import in imports {
            // Alias must be PascalCase
            if !is_pascal_case(&import.alias) {
                return Err(NtmlError::InvalidImportAlias {
                    alias: import.alias.clone(),
                });
            }
            // Alias must not conflict with built-in components
            if BUILTIN_COMPONENTS.contains(&import.alias.as_str()) {
                return Err(NtmlError::InvalidImportAlias {
                    alias: import.alias.clone(),
                });
            }
            // Source must be a .ntml file
            if !import.src.ends_with(".ntml") {
                return Err(NtmlError::ValidationError(format!(
                    "Import src '{}' must have a .ntml extension",
                    import.src
                )));
            }
        }
    }

    Ok(())
}

/// Validate that a tag is lowercase, non-empty, and has no spaces
fn validate_tag(tag: &str) -> NtmlResult<()> {
    if tag.is_empty() || tag.contains(' ') || tag != tag.to_lowercase() {
        return Err(NtmlError::InvalidTag { tag: tag.to_string() });
    }
    Ok(())
}

/// Validate ID uniqueness across the whole component tree
pub fn validate_id_uniqueness(component: &Component) -> NtmlResult<()> {
    let mut seen_ids = HashSet::new();
    collect_ids(component, &mut seen_ids)
}

fn collect_ids(component: &Component, seen: &mut HashSet<String>) -> NtmlResult<()> {
    let id = get_component_id(component);
    if let Some(id_str) = id {
        if !seen.insert(id_str.clone()) {
            return Err(NtmlError::DuplicateId { id: id_str });
        }
    }
    // Recurse into children
    if let Some(children) = get_component_children(component) {
        for child in children {
            collect_ids(child, seen)?;
        }
    }
    Ok(())
}

fn get_component_id(component: &Component) -> Option<String> {
    match component {
        Component::Container(c) => c.id.clone(),
        Component::Flex(c) => c.id.clone(),
        Component::Grid(c) => c.id.clone(),
        Component::Stack(c) => c.id.clone(),
        Component::Row(c) => c.id.clone(),
        Component::Column(c) => c.id.clone(),
        Component::Text(c) => c.id.clone(),
        Component::Image(c) => c.id.clone(),
        Component::Icon(c) => c.id.clone(),
        Component::Button(c) => c.id.clone(),
        Component::Input(c) => c.id.clone(),
        Component::Checkbox(c) => c.id.clone(),
        Component::Radio(c) => c.id.clone(),
        Component::Select(c) => c.id.clone(),
        Component::ProgressBar(c) => c.id.clone(),
        Component::Badge(c) => c.id.clone(),
        Component::Divider(c) => c.id.clone(),
        Component::Spacer(_) => None,
        Component::Link(c) => c.id.clone(),
        Component::Code(c) => c.id.clone(),
        Component::Markdown(c) => c.id.clone(),
        Component::List(c) => c.id.clone(),
        Component::ListItem(c) => c.id.clone(),
        Component::Heading(c) => c.id.clone(),
        Component::Table(c) => c.id.clone(),
        Component::Blockquote(c) => c.id.clone(),
        Component::Pre(c) => c.id.clone(),
        Component::Details(c) => c.id.clone(),
        Component::ImportedComponent(c) => c.id.clone(),
    }
}

fn get_component_children(component: &Component) -> Option<&Vec<Component>> {
    match component {
        Component::Container(c) => c.children.as_ref(),
        Component::Flex(c) => c.children.as_ref(),
        Component::Grid(c) => c.children.as_ref(),
        Component::Stack(c) => c.children.as_ref(),
        Component::Row(c) => c.children.as_ref(),
        Component::Column(c) => c.children.as_ref(),
        Component::Button(c) => c.children.as_ref(),
        Component::Link(c) => c.children.as_ref(),
        Component::List(c) => c.children.as_ref(),
        Component::ListItem(c) => c.children.as_ref(),
        Component::Blockquote(c) => c.children.as_ref(),
        Component::Details(c) => c.children.as_ref(),
        _ => None,
    }
}

fn validate_component_recursive(
    component: &Component,
    depth: usize,
    font_families: &[String],
) -> NtmlResult<()> {
    if depth > MAX_NESTING_DEPTH {
        return Err(NtmlError::MaxNestingDepthExceeded {
            max_depth: MAX_NESTING_DEPTH,
        });
    }

    match component {
        Component::Container(c) => validate_container(c, depth, font_families),
        Component::Flex(c) => validate_flex(c, depth, font_families),
        Component::Grid(c) => validate_grid(c, depth, font_families),
        Component::Stack(c) => validate_stack(c, depth, font_families),
        Component::Row(c) => validate_row(c, depth, font_families),
        Component::Column(c) => validate_column(c, depth, font_families),
        Component::Text(c) => validate_text(c, font_families),
        Component::Image(c) => validate_image(c, font_families),
        Component::Icon(c) => validate_icon(c, font_families),
        Component::Button(c) => validate_button(c, depth, font_families),
        Component::Input(c) => validate_input(c, font_families),
        Component::Checkbox(c) => validate_checkbox(c, font_families),
        Component::Radio(c) => validate_radio(c, font_families),
        Component::Select(c) => validate_select(c, font_families),
        Component::ProgressBar(c) => validate_progress_bar(c, font_families),
        Component::Badge(c) => validate_badge(c, font_families),
        Component::Divider(c) => validate_divider(c, font_families),
        Component::Spacer(c) => validate_spacer(c),
        Component::Link(c) => validate_link(c, depth, font_families),
        Component::Code(c) => validate_code(c, font_families),
        Component::Markdown(c) => validate_markdown(c, font_families),
        Component::List(c) => validate_list(c, depth, font_families),
        Component::ListItem(c) => validate_list_item(c, depth, font_families),
        Component::Heading(c) => validate_heading(c, font_families),
        Component::Table(c) => validate_table(c, font_families),
        Component::Blockquote(c) => validate_blockquote(c, depth, font_families),
        Component::Pre(c) => validate_pre(c, font_families),
        Component::Details(c) => validate_details(c, depth, font_families),
        Component::ImportedComponent(_) => Ok(()), // props validated at runtime
    }
}

fn validate_children(
    children: &Option<Vec<Component>>,
    depth: usize,
    font_families: &[String],
) -> NtmlResult<()> {
    if let Some(children) = children {
        for child in children {
            validate_component_recursive(child, depth + 1, font_families)?;
        }
    }
    Ok(())
}

fn validate_style(style: &Option<Style>, font_families: &[String]) -> NtmlResult<()> {
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

        // Validate fontFamily — custom strings must be declared in head.fonts
        if let Some(ref font_family) = style.font_family {
            if let FontFamily::Custom(ref name) = font_family {
                if !font_families.iter().any(|f| f == name) {
                    return Err(NtmlError::InvalidStyle {
                        property: "fontFamily".to_string(),
                        reason: format!(
                            "custom font family '{}' is not declared in head.fonts",
                            name
                        ),
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

        // Validate classes (safe chars: alphanumeric, space, -, _, :, /, ., [, ] for Tailwind arbitrary values)
        if let Some(ref classes) = style.classes {
            static CLASSES_REGEX: OnceLock<Regex> = OnceLock::new();
            let re = CLASSES_REGEX.get_or_init(|| {
                Regex::new(r"^[a-zA-Z0-9_\-\s:./\[\]]+$").unwrap()
            });
            if !re.is_match(classes) {
                return Err(NtmlError::InvalidStyle {
                    property: "classes".to_string(),
                    reason: "must contain only safe characters (alphanumeric, spaces, -, _, :, /, ., [, ])".to_string(),
                });
            }
        }
    }
    Ok(())
}

pub fn validate_color(color: &str, _property: &str) -> NtmlResult<()> {
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

/// Validate a component file definition
pub fn validate_component_file(file: &ComponentFile) -> NtmlResult<()> {
    if !is_pascal_case(&file.component) {
        return Err(NtmlError::InvalidComponent {
            component: file.component.clone(),
            reason: "component name must be PascalCase".to_string(),
        });
    }
    if BUILTIN_COMPONENTS.contains(&file.component.as_str()) {
        return Err(NtmlError::InvalidComponent {
            component: file.component.clone(),
            reason: format!(
                "'{}' conflicts with a built-in component name",
                file.component
            ),
        });
    }

    // Validate prop names are unique and camelCase
    let mut seen_props = HashSet::new();
    for prop in &file.props {
        if prop.name.is_empty() {
            return Err(NtmlError::ValidationError(
                "Prop name must not be empty".to_string(),
            ));
        }
        if !seen_props.insert(prop.name.clone()) {
            return Err(NtmlError::ValidationError(format!(
                "Duplicate prop name '{}'",
                prop.name
            )));
        }
    }

    // Validate the body component tree (no custom fonts in component files)
    validate_component_with_context(&file.body, &[])
}

/// Validate data-* attribute key names (values are unconstrained strings)
fn validate_data_attributes(data: &std::collections::HashMap<String, String>) -> NtmlResult<()> {
    static DATA_KEY_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = DATA_KEY_REGEX
        .get_or_init(|| Regex::new(r"^data-[a-z][a-z0-9-]*$").unwrap());

    for key in data.keys() {
        if !re.is_match(key) {
            return Err(NtmlError::InvalidDataAttribute {
                key: key.clone(),
                reason: "must match pattern data-[a-z][a-z0-9-]* (lowercase, starts with a letter after the hyphen)".to_string(),
            });
        }
    }
    Ok(())
}

// --- Component validators ---

fn validate_container(
    container: &Container,
    depth: usize,
    font_families: &[String],
) -> NtmlResult<()> {
    validate_data_attributes(&container.data)?;
    validate_style(&container.style, font_families)?;
    validate_children(&container.children, depth, font_families)
}

fn validate_flex(flex: &Flex, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&flex.data)?;
    if let Some(gap) = flex.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Flex".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&flex.style, font_families)?;
    validate_children(&flex.children, depth, font_families)
}

fn validate_grid(grid: &Grid, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&grid.data)?;
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

    validate_style(&grid.style, font_families)?;
    validate_children(&grid.children, depth, font_families)
}

fn validate_stack(stack: &Stack, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&stack.data)?;
    validate_style(&stack.style, font_families)?;
    validate_children(&stack.children, depth, font_families)
}

fn validate_row(row: &Row, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&row.data)?;
    if let Some(gap) = row.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Row".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&row.style, font_families)?;
    validate_children(&row.children, depth, font_families)
}

fn validate_column(column: &Column, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&column.data)?;
    if let Some(gap) = column.gap {
        if gap < 0.0 {
            return Err(NtmlError::InvalidProperty {
                component: "Column".to_string(),
                property: "gap".to_string(),
                reason: "must be non-negative".to_string(),
            });
        }
    }
    validate_style(&column.style, font_families)?;
    validate_children(&column.children, depth, font_families)
}

fn validate_text(text: &Text, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&text.data)?;
    // Empty text is allowed (e.g. for cells updated via ui.set_text)
    validate_style(&text.style, font_families)
}

fn validate_image(image: &Image, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&image.data)?;
    if image.src.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Image".to_string(),
            property: "src".to_string(),
        });
    }
    validate_style(&image.style, font_families)
}

fn validate_icon(icon: &Icon, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&icon.data)?;
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
    validate_style(&icon.style, font_families)
}

fn validate_button(button: &Button, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&button.data)?;
    if button.action.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Button".to_string(),
            property: "action".to_string(),
        });
    }
    validate_style(&button.style, font_families)?;
    validate_children(&button.children, depth, font_families)
}

fn validate_lua_action_name(name: &str, component: &str, property: &str) -> NtmlResult<()> {
    if name.is_empty() {
        return Err(NtmlError::InvalidProperty {
            component: component.to_string(),
            property: property.to_string(),
            reason: "must be non-empty".to_string(),
        });
    }
    if name.contains(':') {
        return Err(NtmlError::InvalidAction {
            action: name.to_string(),
        });
    }
    Ok(())
}

fn validate_input(input: &Input, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&input.data)?;
    if input.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Input".to_string(),
            property: "name".to_string(),
        });
    }
    if let Some(ref oc) = input.onchange {
        validate_lua_action_name(oc, "Input", "onchange")?;
    }
    validate_style(&input.style, font_families)
}

fn validate_checkbox(checkbox: &Checkbox, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&checkbox.data)?;
    if checkbox.name.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Checkbox".to_string(),
            property: "name".to_string(),
        });
    }
    if let Some(ref oc) = checkbox.onchange {
        validate_lua_action_name(oc, "Checkbox", "onchange")?;
    }
    validate_style(&checkbox.style, font_families)
}

fn validate_radio(radio: &Radio, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&radio.data)?;
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
    if let Some(ref oc) = radio.onchange {
        validate_lua_action_name(oc, "Radio", "onchange")?;
    }
    validate_style(&radio.style, font_families)
}

fn validate_select(select: &Select, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&select.data)?;
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
    if let Some(ref oc) = select.onchange {
        validate_lua_action_name(oc, "Select", "onchange")?;
    }
    validate_style(&select.style, font_families)
}

fn validate_progress_bar(
    progress_bar: &ProgressBar,
    font_families: &[String],
) -> NtmlResult<()> {
    validate_data_attributes(&progress_bar.data)?;
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
    validate_style(&progress_bar.style, font_families)
}

fn validate_badge(badge: &Badge, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&badge.data)?;
    if badge.text.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Badge".to_string(),
            property: "text".to_string(),
        });
    }
    validate_style(&badge.style, font_families)
}

fn validate_divider(divider: &Divider, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&divider.data)?;
    validate_style(&divider.style, font_families)
}

fn validate_spacer(spacer: &Spacer) -> NtmlResult<()> {
    validate_data_attributes(&spacer.data)
}

fn validate_link(link: &Link, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&link.data)?;
    if link.href.is_empty() {
        return Err(NtmlError::InvalidProperty {
            component: "Link".to_string(),
            property: "href".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    validate_style(&link.style, font_families)?;
    validate_children(&link.children, depth, font_families)
}

fn validate_code(code: &Code, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&code.data)?;
    validate_style(&code.style, font_families)?;
    Ok(())
}

fn validate_markdown(markdown: &Markdown, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&markdown.data)?;
    validate_style(&markdown.style, font_families)?;
    Ok(())
}

fn validate_list(list: &List, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&list.data)?;
    validate_style(&list.style, font_families)?;
    validate_children(&list.children, depth, font_families)
}

fn validate_list_item(list_item: &ListItem, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&list_item.data)?;
    validate_style(&list_item.style, font_families)?;
    validate_children(&list_item.children, depth, font_families)
}

fn validate_heading(heading: &Heading, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&heading.data)?;
    if heading.level < 1 || heading.level > 3 {
        return Err(NtmlError::InvalidProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
            reason: "must be 1, 2, or 3".to_string(),
        });
    }
    if heading.text.is_empty() {
        return Err(NtmlError::InvalidProperty {
            component: "Heading".to_string(),
            property: "text".to_string(),
            reason: "must not be empty".to_string(),
        });
    }
    validate_style(&heading.style, font_families)?;
    Ok(())
}

fn validate_table(table: &Table, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&table.data)?;
    validate_style(&table.style, font_families)?;
    Ok(())
}

fn validate_blockquote(blockquote: &Blockquote, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&blockquote.data)?;
    validate_style(&blockquote.style, font_families)?;
    validate_children(&blockquote.children, depth, font_families)
}

fn validate_pre(pre: &Pre, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&pre.data)?;
    validate_style(&pre.style, font_families)?;
    Ok(())
}

fn validate_details(details: &Details, depth: usize, font_families: &[String]) -> NtmlResult<()> {
    validate_data_attributes(&details.data)?;
    validate_style(&details.style, font_families)?;
    validate_children(&details.children, depth, font_families)
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

    #[test]
    fn test_validate_tag() {
        assert!(validate_tag("hud").is_ok());
        assert!(validate_tag("my-tag").is_ok());
        assert!(validate_tag("").is_err());
        assert!(validate_tag("my tag").is_err());
        assert!(validate_tag("MyTag").is_err());
        assert!(validate_tag("UPPER").is_err());
    }

    #[test]
    fn test_validate_head_valid() {
        let head = Head {
            title: "Test Page".to_string(),
            description: Some("A test".to_string()),
            author: None,
            tags: Some(vec!["hud".to_string(), "system".to_string()]),
            fonts: None,
            scripts: None,
            imports: None,
        };
        assert!(validate_head(&head).is_ok());
    }

    #[test]
    fn test_validate_head_missing_title() {
        let head = Head {
            title: "".to_string(),
            description: None,
            author: None,
            tags: None,
            fonts: None,
            scripts: None,
            imports: None,
        };
        assert!(matches!(validate_head(&head), Err(NtmlError::MissingTitle)));
    }

    #[test]
    fn test_validate_head_tag_with_uppercase() {
        let head = Head {
            title: "Test".to_string(),
            description: None,
            author: None,
            tags: Some(vec!["ValidTag".to_string()]),
            fonts: None,
            scripts: None,
            imports: None,
        };
        assert!(matches!(
            validate_head(&head),
            Err(NtmlError::InvalidTag { .. })
        ));
    }

    #[test]
    fn test_validate_head_too_many_tags() {
        let head = Head {
            title: "Test".to_string(),
            description: None,
            author: None,
            tags: Some(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string(),
                "g".to_string(),
                "h".to_string(),
                "i".to_string(),
                "j".to_string(),
                "k".to_string(),
            ]),
            fonts: None,
            scripts: None,
            imports: None,
        };
        assert!(matches!(
            validate_head(&head),
            Err(NtmlError::TagLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_validate_id_uniqueness() {
        let comp = Component::Container(Container {
            id: Some("foo".to_string()),
            visible: None,
            style: None,
            children: Some(vec![
                Component::Text(Text {
                    id: Some("foo".to_string()), // duplicate!
                    text: "Hello".to_string(),
                    style: None,
                    data: Default::default(),
                }),
            ]),
            data: Default::default(),
        });
        assert!(matches!(
            validate_id_uniqueness(&comp),
            Err(NtmlError::DuplicateId { .. })
        ));
    }

    #[test]
    fn test_validate_custom_font_not_declared() {
        let style = Some(Style {
            font_family: Some(FontFamily::Custom("Roboto Mono".to_string())),
            ..Default::default()
        });
        let result = validate_style(&style, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_custom_font_declared() {
        let style = Some(Style {
            font_family: Some(FontFamily::Custom("Roboto Mono".to_string())),
            ..Default::default()
        });
        let font_families = vec!["Roboto Mono".to_string()];
        let result = validate_style(&style, &font_families);
        assert!(result.is_ok());
    }
}
