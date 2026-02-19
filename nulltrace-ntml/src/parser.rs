use roxmltree::Node;

use crate::components::*;
use crate::document::NtmlDocument;
use crate::error::{NtmlError, NtmlResult};
use crate::head::{ComponentImport, FontImport, Head, ScriptImport};
use crate::style::*;
use crate::theme::Theme;
use crate::validator::{validate_component_with_context, validate_head};
use std::collections::HashMap;

/// Synthetic root tag used to wrap NTML content (allows multiple root siblings like head+body)
const WRAPPER: &str = "__ntml_root__";

fn wrap(xml: &str) -> String {
    format!("<{0}>{1}</{0}>", WRAPPER, xml)
}


// ─── Public parse functions ──────────────────────────────────────────────────

/// Parse NTML classic format only (no head/body). Returns error if full format is used.
pub fn parse_ntml(xml: &str) -> NtmlResult<Component> {
    parse_ntml_with_theme(xml, Theme::default())
}

/// Parse NTML classic format with a custom theme.
pub fn parse_ntml_with_theme(xml: &str, _theme: Theme) -> NtmlResult<Component> {
    let wrapped = wrap(xml);
    let doc = roxmltree::Document::parse(&wrapped)?;
    let root = doc.root_element();

    let children: Vec<_> = element_children(root).collect();

    if children.is_empty() {
        return Err(NtmlError::EmptyDocument);
    }

    if children.len() > 1 {
        return Err(NtmlError::MultipleRootComponents);
    }

    let first = children.into_iter().next().unwrap();

    if first.tag_name().name() == "head" {
        return Err(NtmlError::ValidationError(
            "This document uses the full format (head/body). Use parse_document() instead."
                .to_string(),
        ));
    }

    let component = parse_component_node(first, &[])?;
    validate_component_with_context(&component, &[])?;
    Ok(component)
}

/// Parse an NTML document — supports both classic and full format.
pub fn parse_document(xml: &str) -> NtmlResult<NtmlDocument> {
    parse_document_with_theme(xml, Theme::default())
}

/// Parse an NTML document with a custom theme.
pub fn parse_document_with_theme(xml: &str, _theme: Theme) -> NtmlResult<NtmlDocument> {
    let wrapped = wrap(xml);
    let doc = roxmltree::Document::parse(&wrapped)?;
    let root = doc.root_element();

    let first = element_children(root)
        .next()
        .ok_or(NtmlError::EmptyDocument)?;

    if first.tag_name().name() == "head" {
        // ── Full format ──────────────────────────────────────────────────
        let head = parse_head_node(first)?;

        let body_elem = element_children(root)
            .find(|n| n.tag_name().name() == "body")
            .ok_or(NtmlError::MissingBody)?;

        let body_component_elem = element_children(body_elem)
            .next()
            .ok_or(NtmlError::EmptyDocument)?;

        let import_aliases: Vec<String> = head
            .imports
            .as_ref()
            .map(|imports| imports.iter().map(|i| i.alias.clone()).collect())
            .unwrap_or_default();

        let body = parse_component_node(body_component_elem, &import_aliases)?;

        let font_families = head.font_families();
        validate_head(&head)?;
        validate_component_with_context(&body, &font_families)?;

        Ok(NtmlDocument::Full { head, body })
    } else {
        // ── Classic format ───────────────────────────────────────────────
        let component = parse_component_node(first, &[])?;
        validate_component_with_context(&component, &[])?;
        Ok(NtmlDocument::Classic(component))
    }
}

// ─── Head parsing ────────────────────────────────────────────────────────────

fn parse_head_node(node: Node) -> NtmlResult<Head> {
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut author: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut fonts: Vec<FontImport> = Vec::new();
    let mut scripts: Vec<ScriptImport> = Vec::new();
    let mut imports: Vec<ComponentImport> = Vec::new();

    for child in element_children(node) {
        match child.tag_name().name() {
            "title" => {
                title = child.text().map(|s| s.trim().to_string());
            }
            "description" => {
                description = child.text().map(|s| s.trim().to_string());
            }
            "author" => {
                author = child.text().map(|s| s.trim().to_string());
            }
            "tags" => {
                if let Some(text) = child.text() {
                    tags = text
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                }
            }
            "font" => {
                let family = child
                    .attribute("family")
                    .ok_or_else(|| {
                        NtmlError::ValidationError(
                            "<font>: missing required attribute 'family'".to_string(),
                        )
                    })?
                    .to_string();

                let weights_str = child.attribute("weights").ok_or_else(|| {
                    NtmlError::ValidationError(
                        "<font>: missing required attribute 'weights'".to_string(),
                    )
                })?;

                let weights = weights_str
                    .split(',')
                    .map(|w| {
                        w.trim().parse::<u16>().map_err(|_| {
                            NtmlError::ValidationError(format!(
                                "<font>: invalid weight '{}' — must be a number like 400",
                                w.trim()
                            ))
                        })
                    })
                    .collect::<NtmlResult<Vec<u16>>>()?;

                fonts.push(FontImport { family, weights });
            }
            "script" => {
                let src = child
                    .attribute("src")
                    .ok_or_else(|| {
                        NtmlError::ValidationError(
                            "<script>: missing required attribute 'src'".to_string(),
                        )
                    })?
                    .to_string();
                scripts.push(ScriptImport { src });
            }
            "import" => {
                let src = child
                    .attribute("src")
                    .ok_or_else(|| {
                        NtmlError::ValidationError(
                            "<import>: missing required attribute 'src'".to_string(),
                        )
                    })?
                    .to_string();
                let alias = child
                    .attribute("as")
                    .ok_or_else(|| {
                        NtmlError::ValidationError(
                            "<import>: missing required attribute 'as'".to_string(),
                        )
                    })?
                    .to_string();
                imports.push(ComponentImport { src, alias });
            }
            other => {
                return Err(NtmlError::ValidationError(format!(
                    "Unknown element in <head>: <{}>",
                    other
                )));
            }
        }
    }

    Ok(Head {
        title: title.ok_or(NtmlError::MissingTitle)?,
        description,
        author,
        tags: if tags.is_empty() { None } else { Some(tags) },
        fonts: if fonts.is_empty() { None } else { Some(fonts) },
        scripts: if scripts.is_empty() {
            None
        } else {
            Some(scripts)
        },
        imports: if imports.is_empty() {
            None
        } else {
            Some(imports)
        },
    })
}

// ─── Component dispatch ───────────────────────────────────────────────────────

/// Parse a component XML element into a Component.
pub fn parse_component_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Component> {
    let name = node.tag_name().name();

    match name {
        "Container" => {
            parse_container_node(node, import_aliases).map(Component::Container)
        }
        "Flex" => parse_flex_node(node, import_aliases).map(Component::Flex),
        "Grid" => parse_grid_node(node, import_aliases).map(Component::Grid),
        "Stack" => parse_stack_node(node, import_aliases).map(Component::Stack),
        "Row" => parse_row_node(node, import_aliases).map(Component::Row),
        "Column" => parse_column_node(node, import_aliases).map(Component::Column),
        "Text" => parse_text_node(node).map(Component::Text),
        "Image" => parse_image_node(node).map(Component::Image),
        "Icon" => parse_icon_node(node).map(Component::Icon),
        "Button" => parse_button_node(node, import_aliases).map(Component::Button),
        "Input" => parse_input_node(node).map(Component::Input),
        "Checkbox" => parse_checkbox_node(node).map(Component::Checkbox),
        "Radio" => parse_radio_node(node).map(Component::Radio),
        "Select" => parse_select_node(node).map(Component::Select),
        "ProgressBar" => parse_progress_bar_node(node).map(Component::ProgressBar),
        "Badge" => parse_badge_node(node).map(Component::Badge),
        "Divider" => parse_divider_node(node).map(Component::Divider),
        "Spacer" => parse_spacer_node(node).map(Component::Spacer),
        "Link" => parse_link_node(node, import_aliases).map(Component::Link),
        "Code" => parse_code_node(node).map(Component::Code),
        "Markdown" => parse_markdown_node(node).map(Component::Markdown),
        "List" => parse_list_node(node, import_aliases).map(Component::List),
        "ListItem" => parse_list_item_node(node, import_aliases).map(Component::ListItem),
        "Heading" => parse_heading_node(node).map(Component::Heading),
        "Table" => parse_table_node(node).map(Component::Table),
        "Blockquote" => {
            parse_blockquote_node(node, import_aliases).map(Component::Blockquote)
        }
        "Pre" => parse_pre_node(node).map(Component::Pre),
        "Details" => parse_details_node(node, import_aliases).map(Component::Details),
        other => {
            if import_aliases.iter().any(|a| a == other) {
                parse_imported_component_node(other, node)
                    .map(Component::ImportedComponent)
            } else {
                Err(NtmlError::InvalidComponent {
                    component: other.to_string(),
                    reason: format!(
                        "Unknown component type '{}'. If this is an imported component, declare it in <head> with <import>.",
                        other
                    ),
                })
            }
        }
    }
}

// ─── Children & utility helpers ───────────────────────────────────────────────

/// Iterator over element children (skips text/CDATA/comment nodes).
fn element_children<'a>(node: Node<'a, 'a>) -> impl Iterator<Item = Node<'a, 'a>> {
    node.children().filter(|n| n.is_element())
}

/// Parse element children as Component children.
fn parse_children_nodes<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Option<Vec<Component>>> {
    let children: Vec<_> = element_children(node).collect();
    if children.is_empty() {
        return Ok(None);
    }
    let mut result = Vec::new();
    for child in children {
        result.push(parse_component_node(child, import_aliases)?);
    }
    Ok(Some(result))
}

/// Read `id` attribute.
fn get_id(node: Node) -> Option<String> {
    node.attribute("id").map(|s| s.to_string())
}

/// Collect `data-*` attributes.
fn parse_data_attributes(node: Node) -> NtmlResult<DataAttributes> {
    let mut data = HashMap::new();
    for attr in node.attributes() {
        if attr.name().starts_with("data-") {
            data.insert(attr.name().to_string(), attr.value().to_string());
        }
    }
    Ok(data)
}

/// Parse `class` and `style` attributes into an optional Style.
fn parse_style_from_node(node: Node) -> NtmlResult<Option<Style>> {
    let class = node.attribute("class");
    let style_attr = node.attribute("style");

    if class.is_none() && style_attr.is_none() {
        return Ok(None);
    }

    let mut style = Style::default();

    if let Some(classes) = class {
        style.classes = Some(classes.to_string());
    }
    if let Some(s) = style_attr {
        apply_style_string(&mut style, s)?;
    }

    Ok(Some(style))
}

/// Apply `"key:val; key2:val2"` style string to a mutable Style.
fn apply_style_string(style: &mut Style, s: &str) -> NtmlResult<()> {
    for entry in s.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let colon = entry.find(':').ok_or_else(|| NtmlError::InvalidStyle {
            property: entry.to_string(),
            reason: "expected 'property:value' format".to_string(),
        })?;
        let key = entry[..colon].trim();
        let val = entry[colon + 1..].trim();
        apply_single_style(style, key, val)?;
    }
    Ok(())
}

fn apply_single_style(style: &mut Style, key: &str, val: &str) -> NtmlResult<()> {
    match key {
        "width" => style.width = Some(parse_dimension(val)?),
        "height" => style.height = Some(parse_dimension(val)?),
        "minWidth" => style.min_width = Some(parse_f64(val, key)?),
        "maxWidth" => style.max_width = Some(parse_f64(val, key)?),
        "minHeight" => style.min_height = Some(parse_f64(val, key)?),
        "maxHeight" => style.max_height = Some(parse_f64(val, key)?),
        "padding" => style.padding = Some(parse_f64(val, key)?),
        "paddingVertical" => style.padding_vertical = Some(parse_f64(val, key)?),
        "paddingHorizontal" => style.padding_horizontal = Some(parse_f64(val, key)?),
        "paddingTop" => style.padding_top = Some(parse_f64(val, key)?),
        "paddingRight" => style.padding_right = Some(parse_f64(val, key)?),
        "paddingBottom" => style.padding_bottom = Some(parse_f64(val, key)?),
        "paddingLeft" => style.padding_left = Some(parse_f64(val, key)?),
        "margin" => style.margin = Some(parse_f64(val, key)?),
        "marginVertical" => style.margin_vertical = Some(parse_f64(val, key)?),
        "marginHorizontal" => style.margin_horizontal = Some(parse_f64(val, key)?),
        "marginTop" => style.margin_top = Some(parse_f64(val, key)?),
        "marginRight" => style.margin_right = Some(parse_f64(val, key)?),
        "marginBottom" => style.margin_bottom = Some(parse_f64(val, key)?),
        "marginLeft" => style.margin_left = Some(parse_f64(val, key)?),
        "color" => style.color = Some(val.to_string()),
        "backgroundColor" => style.background_color = Some(val.to_string()),
        "borderColor" => style.border_color = Some(val.to_string()),
        "opacity" => style.opacity = Some(parse_f64(val, key)?),
        "fontSize" => style.font_size = Some(parse_f64(val, key)?),
        "fontWeight" => style.font_weight = Some(parse_font_weight(val)?),
        "fontFamily" => style.font_family = Some(parse_font_family(val)),
        "textAlign" => style.text_align = Some(parse_text_align(val, key)?),
        "textTransform" => style.text_transform = Some(parse_text_transform(val, key)?),
        "letterSpacing" => style.letter_spacing = Some(parse_f64(val, key)?),
        "lineHeight" => style.line_height = Some(parse_f64(val, key)?),
        "textDecoration" => style.text_decoration = Some(parse_text_decoration(val, key)?),
        "borderWidth" => style.border_width = Some(parse_f64(val, key)?),
        "borderTopWidth" => style.border_top_width = Some(parse_f64(val, key)?),
        "borderRightWidth" => style.border_right_width = Some(parse_f64(val, key)?),
        "borderBottomWidth" => style.border_bottom_width = Some(parse_f64(val, key)?),
        "borderLeftWidth" => style.border_left_width = Some(parse_f64(val, key)?),
        "borderStyle" => style.border_style = Some(parse_border_style(val, key)?),
        "borderRadius" => style.border_radius = Some(parse_f64(val, key)?),
        "borderTopLeftRadius" => style.border_top_left_radius = Some(parse_f64(val, key)?),
        "borderTopRightRadius" => {
            style.border_top_right_radius = Some(parse_f64(val, key)?)
        }
        "borderBottomLeftRadius" => {
            style.border_bottom_left_radius = Some(parse_f64(val, key)?)
        }
        "borderBottomRightRadius" => {
            style.border_bottom_right_radius = Some(parse_f64(val, key)?)
        }
        "shadow" => style.shadow = Some(parse_shadow_preset(val, key)?),
        "position" => style.position = Some(parse_position(val, key)?),
        "top" => style.top = Some(parse_f64(val, key)?),
        "right" => style.right = Some(parse_f64(val, key)?),
        "bottom" => style.bottom = Some(parse_f64(val, key)?),
        "left" => style.left = Some(parse_f64(val, key)?),
        "zIndex" => style.z_index = Some(parse_i32(val, key)?),
        "flex" => style.flex = Some(parse_f64(val, key)?),
        "alignSelf" => style.align_self = Some(parse_alignment(val, key)?),
        "display" => style.display = Some(parse_display(val, key)?),
        "overflow" => style.overflow = Some(parse_overflow(val, key)?),
        "cursor" => style.cursor = Some(parse_cursor(val, key)?),
        "classes" => style.classes = Some(val.to_string()),
        other => {
            return Err(NtmlError::InvalidStyle {
                property: other.to_string(),
                reason: format!("Unknown style property '{}'", other),
            });
        }
    }
    Ok(())
}

// ─── Value parsers ────────────────────────────────────────────────────────────

fn parse_f64(s: &str, prop: &str) -> NtmlResult<f64> {
    s.parse::<f64>().map_err(|_| NtmlError::InvalidStyle {
        property: prop.to_string(),
        reason: format!("expected a number, got '{}'", s),
    })
}

fn parse_i32(s: &str, prop: &str) -> NtmlResult<i32> {
    s.parse::<i32>().map_err(|_| NtmlError::InvalidStyle {
        property: prop.to_string(),
        reason: format!("expected an integer, got '{}'", s),
    })
}

fn parse_bool(s: &str, prop: &str) -> NtmlResult<bool> {
    match s {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(NtmlError::InvalidProperty {
            component: String::new(),
            property: prop.to_string(),
            reason: format!("expected 'true' or 'false', got '{}'", other),
        }),
    }
}

fn parse_usize(s: &str, prop: &str) -> NtmlResult<usize> {
    s.parse::<usize>().map_err(|_| NtmlError::InvalidProperty {
        component: String::new(),
        property: prop.to_string(),
        reason: format!("expected a non-negative integer, got '{}'", s),
    })
}

fn parse_dimension(s: &str) -> NtmlResult<Dimension> {
    if s == "auto" {
        Ok(Dimension::Auto)
    } else if let Ok(n) = s.parse::<f64>() {
        Ok(Dimension::Pixels(n))
    } else {
        Ok(Dimension::Custom(s.to_string()))
    }
}

fn parse_font_weight(s: &str) -> NtmlResult<FontWeight> {
    match s {
        "normal" => Ok(FontWeight::Named(FontWeightNamed::Normal)),
        "bold" => Ok(FontWeight::Named(FontWeightNamed::Bold)),
        other => {
            let n = other.parse::<u16>().map_err(|_| NtmlError::InvalidStyle {
                property: "fontWeight".to_string(),
                reason: format!("expected 100-900 or 'normal'/'bold', got '{}'", other),
            })?;
            Ok(FontWeight::Number(n))
        }
    }
}

fn parse_font_family(s: &str) -> FontFamily {
    match s {
        "sans" => FontFamily::Named(FontFamilyNamed::Sans),
        "serif" => FontFamily::Named(FontFamilyNamed::Serif),
        "monospace" => FontFamily::Named(FontFamilyNamed::Monospace),
        "game" => FontFamily::Named(FontFamilyNamed::Game),
        other => FontFamily::Custom(other.to_string()),
    }
}

fn parse_text_align(s: &str, prop: &str) -> NtmlResult<TextAlign> {
    match s {
        "left" => Ok(TextAlign::Left),
        "center" => Ok(TextAlign::Center),
        "right" => Ok(TextAlign::Right),
        "justify" => Ok(TextAlign::Justify),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "left, center, right, justify".to_string(),
        }),
    }
}

fn parse_text_transform(s: &str, prop: &str) -> NtmlResult<TextTransform> {
    match s {
        "none" => Ok(TextTransform::None),
        "uppercase" => Ok(TextTransform::Uppercase),
        "lowercase" => Ok(TextTransform::Lowercase),
        "capitalize" => Ok(TextTransform::Capitalize),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "none, uppercase, lowercase, capitalize".to_string(),
        }),
    }
}

fn parse_text_decoration(s: &str, prop: &str) -> NtmlResult<TextDecoration> {
    match s {
        "none" => Ok(TextDecoration::None),
        "underline" => Ok(TextDecoration::Underline),
        "line-through" => Ok(TextDecoration::LineThrough),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "none, underline, line-through".to_string(),
        }),
    }
}

fn parse_border_style(s: &str, prop: &str) -> NtmlResult<BorderStyle> {
    match s {
        "solid" => Ok(BorderStyle::Solid),
        "dashed" => Ok(BorderStyle::Dashed),
        "dotted" => Ok(BorderStyle::Dotted),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "solid, dashed, dotted".to_string(),
        }),
    }
}

fn parse_shadow_preset(s: &str, prop: &str) -> NtmlResult<Shadow> {
    match s {
        "small" => Ok(Shadow::Preset(ShadowPreset::Small)),
        "medium" => Ok(Shadow::Preset(ShadowPreset::Medium)),
        "large" => Ok(Shadow::Preset(ShadowPreset::Large)),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "small, medium, large".to_string(),
        }),
    }
}

fn parse_position(s: &str, prop: &str) -> NtmlResult<Position> {
    match s {
        "relative" => Ok(Position::Relative),
        "absolute" => Ok(Position::Absolute),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "relative, absolute".to_string(),
        }),
    }
}

fn parse_alignment(s: &str, prop: &str) -> NtmlResult<Alignment> {
    match s {
        "start" => Ok(Alignment::Start),
        "center" => Ok(Alignment::Center),
        "end" => Ok(Alignment::End),
        "stretch" => Ok(Alignment::Stretch),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "start, center, end, stretch".to_string(),
        }),
    }
}

fn parse_display(s: &str, prop: &str) -> NtmlResult<Display> {
    match s {
        "flex" => Ok(Display::Flex),
        "none" => Ok(Display::None),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "flex, none".to_string(),
        }),
    }
}

fn parse_overflow(s: &str, prop: &str) -> NtmlResult<Overflow> {
    match s {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        "scroll" => Ok(Overflow::Scroll),
        "auto" => Ok(Overflow::Auto),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "visible, hidden, scroll, auto".to_string(),
        }),
    }
}

fn parse_cursor(s: &str, prop: &str) -> NtmlResult<Cursor> {
    match s {
        "default" => Ok(Cursor::Default),
        "pointer" => Ok(Cursor::Pointer),
        "not-allowed" => Ok(Cursor::NotAllowed),
        "text" => Ok(Cursor::Text),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "default, pointer, not-allowed, text".to_string(),
        }),
    }
}

fn parse_flex_direction(s: &str, prop: &str) -> NtmlResult<FlexDirection> {
    match s {
        "row" => Ok(FlexDirection::Row),
        "column" => Ok(FlexDirection::Column),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "row, column".to_string(),
        }),
    }
}

fn parse_justify(s: &str, prop: &str) -> NtmlResult<JustifyContent> {
    match s {
        "start" => Ok(JustifyContent::Start),
        "center" => Ok(JustifyContent::Center),
        "end" => Ok(JustifyContent::End),
        "spaceBetween" => Ok(JustifyContent::SpaceBetween),
        "spaceAround" => Ok(JustifyContent::SpaceAround),
        "spaceEvenly" => Ok(JustifyContent::SpaceEvenly),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "start, center, end, spaceBetween, spaceAround, spaceEvenly".to_string(),
        }),
    }
}

fn parse_align(s: &str, prop: &str) -> NtmlResult<AlignItems> {
    match s {
        "start" => Ok(AlignItems::Start),
        "center" => Ok(AlignItems::Center),
        "end" => Ok(AlignItems::End),
        "stretch" => Ok(AlignItems::Stretch),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "start, center, end, stretch".to_string(),
        }),
    }
}

fn parse_stack_alignment(s: &str, prop: &str) -> NtmlResult<StackAlignment> {
    match s {
        "topLeft" => Ok(StackAlignment::TopLeft),
        "topCenter" => Ok(StackAlignment::TopCenter),
        "topRight" => Ok(StackAlignment::TopRight),
        "centerLeft" => Ok(StackAlignment::CenterLeft),
        "center" => Ok(StackAlignment::Center),
        "centerRight" => Ok(StackAlignment::CenterRight),
        "bottomLeft" => Ok(StackAlignment::BottomLeft),
        "bottomCenter" => Ok(StackAlignment::BottomCenter),
        "bottomRight" => Ok(StackAlignment::BottomRight),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "topLeft, topCenter, topRight, centerLeft, center, centerRight, bottomLeft, bottomCenter, bottomRight".to_string(),
        }),
    }
}

fn parse_image_fit(s: &str, prop: &str) -> NtmlResult<ImageFit> {
    match s {
        "cover" => Ok(ImageFit::Cover),
        "contain" => Ok(ImageFit::Contain),
        "fill" => Ok(ImageFit::Fill),
        "none" => Ok(ImageFit::None),
        "scaleDown" => Ok(ImageFit::ScaleDown),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "cover, contain, fill, none, scaleDown".to_string(),
        }),
    }
}

fn parse_link_target(s: &str, prop: &str) -> NtmlResult<LinkTarget> {
    match s {
        "same" => Ok(LinkTarget::Same),
        "new" => Ok(LinkTarget::New),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "same, new".to_string(),
        }),
    }
}

fn parse_button_variant(s: &str, prop: &str) -> NtmlResult<ButtonVariant> {
    match s {
        "primary" => Ok(ButtonVariant::Primary),
        "secondary" => Ok(ButtonVariant::Secondary),
        "danger" => Ok(ButtonVariant::Danger),
        "ghost" => Ok(ButtonVariant::Ghost),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "primary, secondary, danger, ghost".to_string(),
        }),
    }
}

fn parse_input_type(s: &str, prop: &str) -> NtmlResult<InputType> {
    match s {
        "text" => Ok(InputType::Text),
        "password" => Ok(InputType::Password),
        "number" => Ok(InputType::Number),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "text, password, number".to_string(),
        }),
    }
}

fn parse_progress_bar_variant(s: &str, prop: &str) -> NtmlResult<ProgressBarVariant> {
    match s {
        "default" => Ok(ProgressBarVariant::Default),
        "success" => Ok(ProgressBarVariant::Success),
        "warning" => Ok(ProgressBarVariant::Warning),
        "danger" => Ok(ProgressBarVariant::Danger),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "default, success, warning, danger".to_string(),
        }),
    }
}

fn parse_badge_variant(s: &str, prop: &str) -> NtmlResult<BadgeVariant> {
    match s {
        "default" => Ok(BadgeVariant::Default),
        "primary" => Ok(BadgeVariant::Primary),
        "success" => Ok(BadgeVariant::Success),
        "warning" => Ok(BadgeVariant::Warning),
        "danger" => Ok(BadgeVariant::Danger),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "default, primary, success, warning, danger".to_string(),
        }),
    }
}

fn parse_divider_orientation(s: &str, prop: &str) -> NtmlResult<DividerOrientation> {
    match s {
        "horizontal" => Ok(DividerOrientation::Horizontal),
        "vertical" => Ok(DividerOrientation::Vertical),
        other => Err(NtmlError::InvalidEnum {
            property: prop.to_string(),
            value: other.to_string(),
            expected: "horizontal, vertical".to_string(),
        }),
    }
}

/// Collect all text content from a node (text nodes + CDATA).
fn node_text_content(node: Node) -> String {
    let mut s = String::new();
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                s.push_str(t);
            }
        }
    }
    s
}

// ─── Component parsers ────────────────────────────────────────────────────────

fn parse_container_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Container> {
    Ok(Container {
        id: get_id(node),
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_flex_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Flex> {
    Ok(Flex {
        id: get_id(node),
        direction: node
            .attribute("direction")
            .map(|s| parse_flex_direction(s, "direction"))
            .transpose()?,
        justify: node
            .attribute("justify")
            .map(|s| parse_justify(s, "justify"))
            .transpose()?,
        align: node
            .attribute("align")
            .map(|s| parse_align(s, "align"))
            .transpose()?,
        gap: node
            .attribute("gap")
            .map(|s| parse_f64(s, "gap"))
            .transpose()?,
        wrap: node
            .attribute("wrap")
            .map(|s| parse_bool(s, "wrap"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_grid_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Grid> {
    let columns_str = node
        .attribute("columns")
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Grid".to_string(),
            property: "columns".to_string(),
        })?;

    let columns = if let Ok(n) = columns_str.parse::<usize>() {
        GridSize::Count(n)
    } else {
        GridSize::Definitions(
            columns_str
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
        )
    };

    let rows = node.attribute("rows").map(|s| {
        if let Ok(n) = s.parse::<usize>() {
            GridSize::Count(n)
        } else {
            GridSize::Definitions(s.split_whitespace().map(|t| t.to_string()).collect())
        }
    });

    let gap = node.attribute("gap").map(|s| {
        let parts: Vec<f64> = s
            .split_whitespace()
            .filter_map(|t| t.parse::<f64>().ok())
            .collect();
        match parts.as_slice() {
            [row, col] => GridGap::Separate {
                row: *row,
                column: *col,
            },
            [single] => GridGap::Single(*single),
            _ => GridGap::Single(0.0),
        }
    });

    Ok(Grid {
        id: get_id(node),
        columns,
        rows,
        gap,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_stack_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Stack> {
    Ok(Stack {
        id: get_id(node),
        alignment: node
            .attribute("alignment")
            .map(|s| parse_stack_alignment(s, "alignment"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_row_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Row> {
    Ok(Row {
        id: get_id(node),
        justify: node
            .attribute("justify")
            .map(|s| parse_justify(s, "justify"))
            .transpose()?,
        align: node
            .attribute("align")
            .map(|s| parse_align(s, "align"))
            .transpose()?,
        gap: node
            .attribute("gap")
            .map(|s| parse_f64(s, "gap"))
            .transpose()?,
        wrap: node
            .attribute("wrap")
            .map(|s| parse_bool(s, "wrap"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_column_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Column> {
    Ok(Column {
        id: get_id(node),
        justify: node
            .attribute("justify")
            .map(|s| parse_justify(s, "justify"))
            .transpose()?,
        align: node
            .attribute("align")
            .map(|s| parse_align(s, "align"))
            .transpose()?,
        gap: node
            .attribute("gap")
            .map(|s| parse_f64(s, "gap"))
            .transpose()?,
        wrap: node
            .attribute("wrap")
            .map(|s| parse_bool(s, "wrap"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_text_node(node: Node) -> NtmlResult<Text> {
    // text= attribute takes priority; fall back to element text content
    let text = node
        .attribute("text")
        .map(|s| s.to_string())
        .or_else(|| {
            let content = node_text_content(node);
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Text".to_string(),
            property: "text".to_string(),
        })?;

    Ok(Text {
        id: get_id(node),
        text,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_image_node(node: Node) -> NtmlResult<Image> {
    Ok(Image {
        id: get_id(node),
        src: node
            .attribute("src")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Image".to_string(),
                property: "src".to_string(),
            })?
            .to_string(),
        alt: node.attribute("alt").map(|s| s.to_string()),
        fit: node
            .attribute("fit")
            .map(|s| parse_image_fit(s, "fit"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_icon_node(node: Node) -> NtmlResult<Icon> {
    Ok(Icon {
        id: get_id(node),
        name: node
            .attribute("name")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Icon".to_string(),
                property: "name".to_string(),
            })?
            .to_string(),
        size: node
            .attribute("size")
            .map(|s| parse_f64(s, "size"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_button_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Button> {
    Ok(Button {
        id: get_id(node),
        action: node
            .attribute("action")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Button".to_string(),
                property: "action".to_string(),
            })?
            .to_string(),
        variant: node
            .attribute("variant")
            .map(|s| parse_button_variant(s, "variant"))
            .transpose()?,
        disabled: node
            .attribute("disabled")
            .map(|s| parse_bool(s, "disabled"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_input_node(node: Node) -> NtmlResult<Input> {
    Ok(Input {
        id: get_id(node),
        name: node
            .attribute("name")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Input".to_string(),
                property: "name".to_string(),
            })?
            .to_string(),
        placeholder: node.attribute("placeholder").map(|s| s.to_string()),
        value: node.attribute("value").map(|s| s.to_string()),
        input_type: node
            .attribute("type")
            .map(|s| parse_input_type(s, "type"))
            .transpose()?,
        max_length: node
            .attribute("maxLength")
            .map(|s| parse_usize(s, "maxLength"))
            .transpose()?,
        disabled: node
            .attribute("disabled")
            .map(|s| parse_bool(s, "disabled"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_checkbox_node(node: Node) -> NtmlResult<Checkbox> {
    Ok(Checkbox {
        id: get_id(node),
        name: node
            .attribute("name")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Checkbox".to_string(),
                property: "name".to_string(),
            })?
            .to_string(),
        label: node.attribute("label").map(|s| s.to_string()),
        checked: node
            .attribute("checked")
            .map(|s| parse_bool(s, "checked"))
            .transpose()?,
        disabled: node
            .attribute("disabled")
            .map(|s| parse_bool(s, "disabled"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_radio_node(node: Node) -> NtmlResult<Radio> {
    Ok(Radio {
        id: get_id(node),
        name: node
            .attribute("name")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Radio".to_string(),
                property: "name".to_string(),
            })?
            .to_string(),
        value: node
            .attribute("value")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Radio".to_string(),
                property: "value".to_string(),
            })?
            .to_string(),
        label: node.attribute("label").map(|s| s.to_string()),
        checked: node
            .attribute("checked")
            .map(|s| parse_bool(s, "checked"))
            .transpose()?,
        disabled: node
            .attribute("disabled")
            .map(|s| parse_bool(s, "disabled"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_select_node(node: Node) -> NtmlResult<Select> {
    let mut options: Vec<SelectOption> = Vec::new();

    for child in element_children(node) {
        if child.tag_name().name() == "option" {
            let value = child
                .attribute("value")
                .ok_or_else(|| {
                    NtmlError::ValidationError(
                        "<option>: missing required attribute 'value'".to_string(),
                    )
                })?
                .to_string();
            let label = child
                .attribute("label")
                .ok_or_else(|| {
                    NtmlError::ValidationError(
                        "<option>: missing required attribute 'label'".to_string(),
                    )
                })?
                .to_string();
            options.push(SelectOption { value, label });
        }
    }

    if options.is_empty() {
        return Err(NtmlError::MissingProperty {
            component: "Select".to_string(),
            property: "options".to_string(),
        });
    }

    Ok(Select {
        id: get_id(node),
        name: node
            .attribute("name")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Select".to_string(),
                property: "name".to_string(),
            })?
            .to_string(),
        options,
        value: node.attribute("value").map(|s| s.to_string()),
        disabled: node
            .attribute("disabled")
            .map(|s| parse_bool(s, "disabled"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_progress_bar_node(node: Node) -> NtmlResult<ProgressBar> {
    Ok(ProgressBar {
        id: get_id(node),
        value: node
            .attribute("value")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "ProgressBar".to_string(),
                property: "value".to_string(),
            })
            .and_then(|s| parse_f64(s, "value"))?,
        max: node
            .attribute("max")
            .map(|s| parse_f64(s, "max"))
            .transpose()?,
        variant: node
            .attribute("variant")
            .map(|s| parse_progress_bar_variant(s, "variant"))
            .transpose()?,
        show_label: node
            .attribute("showLabel")
            .map(|s| parse_bool(s, "showLabel"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_badge_node(node: Node) -> NtmlResult<Badge> {
    let text = node
        .attribute("text")
        .map(|s| s.to_string())
        .or_else(|| {
            let c = node_text_content(node);
            let t = c.trim();
            if !t.is_empty() {
                Some(t.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Badge".to_string(),
            property: "text".to_string(),
        })?;

    Ok(Badge {
        id: get_id(node),
        text,
        variant: node
            .attribute("variant")
            .map(|s| parse_badge_variant(s, "variant"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_divider_node(node: Node) -> NtmlResult<Divider> {
    Ok(Divider {
        id: get_id(node),
        orientation: node
            .attribute("orientation")
            .map(|s| parse_divider_orientation(s, "orientation"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_spacer_node(node: Node) -> NtmlResult<Spacer> {
    let size_str = node
        .attribute("size")
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Spacer".to_string(),
            property: "size".to_string(),
        })?;

    let size = if size_str == "auto" {
        SpacerSize::Auto("auto".to_string())
    } else {
        let n = parse_f64(size_str, "size")?;
        SpacerSize::Fixed(n)
    };

    Ok(Spacer {
        size,
        data: parse_data_attributes(node)?,
    })
}

fn parse_link_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<Link> {
    Ok(Link {
        id: get_id(node),
        href: node
            .attribute("href")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Link".to_string(),
                property: "href".to_string(),
            })?
            .to_string(),
        target: node
            .attribute("target")
            .map(|s| parse_link_target(s, "target"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_code_node(node: Node) -> NtmlResult<Code> {
    // text= attribute OR element text/CDATA content
    let text = node
        .attribute("text")
        .map(|s| s.to_string())
        .or_else(|| {
            let c = node_text_content(node);
            // Trim only leading/trailing newlines, preserve internal whitespace
            let trimmed = c.trim_matches('\n');
            // Also trim a single leading newline that may follow the opening tag
            if !trimmed.is_empty() {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Code".to_string(),
            property: "text".to_string(),
        })?;

    Ok(Code {
        id: get_id(node),
        text,
        language: node.attribute("language").map(|s| s.to_string()),
        block: node
            .attribute("block")
            .map(|s| parse_bool(s, "block"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_markdown_node(node: Node) -> NtmlResult<Markdown> {
    let content = node
        .attribute("content")
        .map(|s| s.to_string())
        .or_else(|| {
            let c = node_text_content(node);
            let trimmed = c.trim_matches('\n');
            if !trimmed.is_empty() {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Markdown".to_string(),
            property: "content".to_string(),
        })?;

    Ok(Markdown {
        id: get_id(node),
        content,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_list_node<'a>(node: Node<'a, 'a>, import_aliases: &[String]) -> NtmlResult<List> {
    Ok(List {
        id: get_id(node),
        ordered: node
            .attribute("ordered")
            .map(|s| parse_bool(s, "ordered"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_list_item_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<ListItem> {
    Ok(ListItem {
        id: get_id(node),
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_heading_node(node: Node) -> NtmlResult<Heading> {
    let level_str = node
        .attribute("level")
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
        })?;

    let level = level_str
        .parse::<u8>()
        .map_err(|_| NtmlError::InvalidProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
            reason: "must be 1, 2, or 3".to_string(),
        })?;

    if level < 1 || level > 3 {
        return Err(NtmlError::InvalidProperty {
            component: "Heading".to_string(),
            property: "level".to_string(),
            reason: "must be 1, 2, or 3".to_string(),
        });
    }

    let text = node
        .attribute("text")
        .map(|s| s.to_string())
        .or_else(|| {
            let c = node_text_content(node);
            let t = c.trim();
            if !t.is_empty() {
                Some(t.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Heading".to_string(),
            property: "text".to_string(),
        })?;

    Ok(Heading {
        id: get_id(node),
        level,
        text,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_table_node(node: Node) -> NtmlResult<Table> {
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for child in element_children(node) {
        match child.tag_name().name() {
            "header" => {
                let text = child.text().map(|s| s.trim().to_string()).unwrap_or_default();
                headers.push(text);
            }
            "row" => {
                let cells: Vec<String> = element_children(child)
                    .filter(|n| n.tag_name().name() == "cell")
                    .map(|n| n.text().map(|s| s.trim().to_string()).unwrap_or_default())
                    .collect();
                rows.push(cells);
            }
            _ => {}
        }
    }

    Ok(Table {
        id: get_id(node),
        headers,
        rows,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_blockquote_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Blockquote> {
    Ok(Blockquote {
        id: get_id(node),
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_pre_node(node: Node) -> NtmlResult<Pre> {
    let text = node
        .attribute("text")
        .map(|s| s.to_string())
        .or_else(|| {
            let c = node_text_content(node);
            let trimmed = c.trim_matches('\n');
            if !trimmed.is_empty() {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| NtmlError::MissingProperty {
            component: "Pre".to_string(),
            property: "text".to_string(),
        })?;

    Ok(Pre {
        id: get_id(node),
        text,
        style: parse_style_from_node(node)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_details_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Details> {
    Ok(Details {
        id: get_id(node),
        summary: node
            .attribute("summary")
            .ok_or_else(|| NtmlError::MissingProperty {
                component: "Details".to_string(),
                property: "summary".to_string(),
            })?
            .to_string(),
        open: node
            .attribute("open")
            .map(|s| parse_bool(s, "open"))
            .transpose()?,
        style: parse_style_from_node(node)?,
        children: parse_children_nodes(node, import_aliases)?,
        data: parse_data_attributes(node)?,
    })
}

fn parse_imported_component_node(name: &str, node: Node) -> NtmlResult<ImportedComponentInstance> {
    let id = get_id(node);
    let mut props = HashMap::new();

    for attr in node.attributes() {
        let attr_name = attr.name();
        if attr_name != "id"
            && attr_name != "class"
            && attr_name != "style"
            && !attr_name.starts_with("data-")
        {
            props.insert(attr_name.to_string(), attr.value().to_string());
        }
    }

    // Also pass class and style as props in case the component uses them
    if let Some(c) = node.attribute("class") {
        props.insert("class".to_string(), c.to_string());
    }

    Ok(ImportedComponentInstance {
        id,
        name: name.to_string(),
        props,
    })
}

// ─── Used by component_file.rs ───────────────────────────────────────────────

/// Parse a single component XML element (used by component_file.rs).
pub fn parse_component_value_from_node<'a>(
    node: Node<'a, 'a>,
    import_aliases: &[String],
) -> NtmlResult<Component> {
    parse_component_node(node, import_aliases)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let xml = r#"<Text text="Hello World" />"#;
        let result = parse_ntml(xml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_text_with_id() {
        let xml = r#"<Text id="my-text" text="Hello World" />"#;
        let result = parse_ntml(xml);
        assert!(result.is_ok());
        if let Component::Text(t) = result.unwrap() {
            assert_eq!(t.id, Some("my-text".to_string()));
        } else {
            panic!("Expected Text component");
        }
    }

    #[test]
    fn test_parse_container_with_children() {
        let xml = r#"
<Container style="padding:16; backgroundColor:red">
  <Text text="Hello" />
  <Text text="World" />
</Container>
"#;
        let result = parse_ntml(xml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_parse_flex_layout() {
        let xml = r#"
<Flex direction="column" gap="12" align="center">
  <Text text="Item 1" />
  <Text text="Item 2" />
</Flex>
"#;
        let result = parse_ntml(xml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_invalid_color() {
        let xml = r#"<Text text="Test" style="color:invalid-color" />"#;
        // Color validation happens in the validator, after parsing
        // The parse itself succeeds; validation catches invalid colors
        let result = parse_ntml(xml);
        assert!(result.is_err(), "Expected error for invalid color");
    }

    #[test]
    fn test_missing_required_property() {
        let xml = r#"<Button variant="primary" />"#;
        let result = parse_ntml(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_document_classic() {
        let xml = r#"<Text text="Hello" />"#;
        let result = parse_document(xml);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), NtmlDocument::Classic(_)));
    }

    #[test]
    fn test_parse_document_full() {
        let xml = r#"
<head>
  <title>My Page</title>
  <description>A test page</description>
</head>
<body>
  <Text text="Hello from full format" />
</body>
"#;
        let result = parse_document(xml);
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
        let xml = r#"
<head>
  <title>My Page</title>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
        let result = parse_ntml(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_format_requires_body() {
        let xml = r#"<head><title>My Page</title></head>"#;
        let result = parse_document(xml);
        assert!(matches!(result, Err(NtmlError::MissingBody)));
    }

    #[test]
    fn test_full_format_requires_title() {
        let xml = r#"
<head>
  <description>No title here</description>
</head>
<body>
  <Text text="Hello" />
</body>
"#;
        let result = parse_document(xml);
        assert!(matches!(result, Err(NtmlError::MissingTitle)));
    }

    #[test]
    fn test_class_attribute_becomes_style_classes() {
        let xml = r#"<Container class="p-4 bg-zinc-900"><Text text="Hi" /></Container>"#;
        let result = parse_ntml(xml);
        assert!(result.is_ok());
        if let Component::Container(c) = result.unwrap() {
            assert_eq!(
                c.style.unwrap().classes,
                Some("p-4 bg-zinc-900".to_string())
            );
        }
    }

    #[test]
    fn test_code_cdata() {
        let xml = "<Code language=\"xml\" block=\"true\"><![CDATA[<Text text=\"Hello\" />]]></Code>";
        let result = parse_ntml(xml);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        if let Component::Code(c) = result.unwrap() {
            assert!(c.text.contains("<Text"));
        }
    }
}
