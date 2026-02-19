//! Converts NTML to safe HTML for iframe srcDoc.
//! No script, no inline event handlers; only structure and styles.

/// Tailwind-compatible utility CSS for NTML docs (embedded at compile time).
const NTML_TAILWIND_CSS: &str = include_str!("../ntml_tailwind.css");

use nulltrace_ntml::components::*;
use nulltrace_ntml::style::{
    Alignment, BorderStyle, Cursor, Dimension, Display, FontFamily, FontWeight, Overflow,
    Position, Shadow, TextAlign, TextDecoration, TextTransform,
};
use nulltrace_ntml::{parse_component_file, parse_document, Component, ComponentFile, Style};
use std::collections::HashMap;
use std::fmt::Write;

/// Override for a component by id. Only fields that are set are applied.
#[derive(Default, Clone)]
pub struct PatchOverride {
    pub text: Option<String>,
    pub visible: Option<bool>,
    pub value: Option<f64>,
    pub disabled: Option<bool>,
}

/// Build PatchOverride map from ntml_runtime patches.
pub fn patches_to_map(patches: &[crate::ntml_runtime::Patch]) -> HashMap<String, PatchOverride> {
    let mut map: HashMap<String, PatchOverride> = HashMap::new();
    for p in patches {
        match p {
            crate::ntml_runtime::Patch::SetText { id, text } => {
                map.entry(id.clone()).or_default().text = Some(text.clone());
            }
            crate::ntml_runtime::Patch::SetVisible { id, visible } => {
                map.entry(id.clone()).or_default().visible = Some(*visible);
            }
            crate::ntml_runtime::Patch::SetValue { id, value } => {
                map.entry(id.clone()).or_default().value = Some(*value);
            }
            crate::ntml_runtime::Patch::SetDisabled { id, disabled } => {
                map.entry(id.clone()).or_default().disabled = Some(*disabled);
            }
        }
    }
    map
}

/// Converts NTML string to safe HTML. Returns error message on parse failure.
pub fn ntml_to_html(yaml: &str) -> Result<String, String> {
    ntml_to_html_with_imports(yaml, &[], None)
}

/// Import definition: alias and component file content.
pub struct NtmlImport {
    pub alias: String,
    pub content: String,
}

/// Converts NTML to safe HTML with optional component imports resolved.
pub fn ntml_to_html_with_imports(
    yaml: &str,
    imports: &[NtmlImport],
    base_url: Option<&str>,
) -> Result<String, String> {
    ntml_to_html_with_imports_and_patches(yaml, imports, &[], base_url)
}

/// Converts NTML to safe HTML with imports and patches applied.
pub fn ntml_to_html_with_imports_and_patches(
    yaml: &str,
    imports: &[NtmlImport],
    patches: &[crate::ntml_runtime::Patch],
    base_url: Option<&str>,
) -> Result<String, String> {
    let doc = parse_document(yaml).map_err(|e| e.to_string())?;

    let title = doc.head().map(|h| h.title.as_str()).unwrap_or("Page");
    let root = doc.root_component();

    let import_map: HashMap<String, ComponentFile> = imports
        .iter()
        .filter_map(|i| {
            parse_component_file(&i.content)
                .ok()
                .map(|f| (i.alias.clone(), f))
        })
        .collect();

    let patch_map = patches_to_map(patches);

    let mut html = String::new();
    write!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
<style>{}</style>
</head>
<body>
"#,
        escape_html(title),
        NTML_TAILWIND_CSS
    )
    .map_err(|e| e.to_string())?;

    component_to_html_with_imports(root, &mut html, &import_map, &patch_map, base_url)
        .map_err(|e| e.to_string())?;

    write!(html, "\n</body>\n</html>").map_err(|e| e.to_string())?;
    Ok(html)
}

fn substitute_props(s: &str, props: &HashMap<String, String>) -> String {
    let prefix = "{props.";
    let suffix = "}";
    if s.starts_with(prefix) && s.ends_with(suffix) && s.len() > prefix.len() + suffix.len() {
        let key = &s[prefix.len()..s.len() - suffix.len()];
        props
            .get(key)
            .cloned()
            .unwrap_or_else(|| s.to_string())
    } else {
        s.to_string()
    }
}

fn substitute_props_in_style(
    style: &Style,
    props: &HashMap<String, String>,
) -> Style {
    let mut out = style.clone();
    out.color = out.color.as_ref().map(|c| substitute_props(c, props));
    out.background_color = out
        .background_color
        .as_ref()
        .map(|c| substitute_props(c, props));
    out.border_color = out.border_color.as_ref().map(|c| substitute_props(c, props));
    out
}

fn substitute_props_in_component(
    c: &Component,
    props: &HashMap<String, String>,
) -> Component {
    match c {
        Component::Text(t) => Component::Text(Text {
            text: substitute_props(&t.text, props),
            style: t.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..t.clone()
        }),
        Component::Container(ct) => Component::Container(Container {
            style: ct
                .style
                .as_ref()
                .map(|s| substitute_props_in_style(s, props)),
            children: ct.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..ct.clone()
        }),
        Component::Flex(f) => Component::Flex(Flex {
            style: f.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: f.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..f.clone()
        }),
        Component::Grid(g) => Component::Grid(Grid {
            style: g.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: g.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..g.clone()
        }),
        Component::Stack(s) => Component::Stack(Stack {
            style: s.style.as_ref().map(|st| substitute_props_in_style(st, props)),
            children: s.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..s.clone()
        }),
        Component::Row(r) => Component::Row(Row {
            style: r.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: r.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..r.clone()
        }),
        Component::Column(col) => Component::Column(Column {
            style: col.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: col.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..col.clone()
        }),
        Component::Image(img) => Component::Image(Image {
            src: substitute_props(&img.src, props),
            style: img.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..img.clone()
        }),
        Component::Icon(ic) => Component::Icon(Icon {
            style: ic.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..ic.clone()
        }),
        Component::Button(b) => Component::Button(Button {
            style: b.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: b.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..b.clone()
        }),
        Component::Input(inp) => Component::Input(Input {
            placeholder: inp.placeholder.as_ref().map(|p| substitute_props(p, props)),
            value: inp.value.as_ref().map(|v| substitute_props(v, props)),
            style: inp.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..inp.clone()
        }),
        Component::Checkbox(cb) => Component::Checkbox(Checkbox {
            label: cb.label.as_ref().map(|l| substitute_props(l, props)),
            style: cb.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..cb.clone()
        }),
        Component::Radio(r) => Component::Radio(Radio {
            label: r.label.as_ref().map(|l| substitute_props(l, props)),
            style: r.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..r.clone()
        }),
        Component::Select(sel) => Component::Select(Select {
            style: sel.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..sel.clone()
        }),
        Component::ProgressBar(pb) => Component::ProgressBar(ProgressBar {
            style: pb.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..pb.clone()
        }),
        Component::Badge(b) => Component::Badge(Badge {
            text: substitute_props(&b.text, props),
            style: b.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..b.clone()
        }),
        Component::Divider(d) => Component::Divider(Divider {
            style: d.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..d.clone()
        }),
        Component::Spacer(sp) => Component::Spacer(sp.clone()),
        Component::Link(lnk) => Component::Link(Link {
            href: substitute_props(&lnk.href, props),
            style: lnk.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: lnk.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..lnk.clone()
        }),
        Component::Code(co) => Component::Code(Code {
            style: co.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..co.clone()
        }),
        Component::Markdown(m) => Component::Markdown(Markdown {
            style: m.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..m.clone()
        }),
        Component::List(l) => Component::List(List {
            style: l.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: l.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..l.clone()
        }),
        Component::ListItem(li) => Component::ListItem(ListItem {
            style: li.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: li.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..li.clone()
        }),
        Component::Heading(h) => Component::Heading(Heading {
            style: h.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..h.clone()
        }),
        Component::Table(t) => Component::Table(t.clone()),
        Component::Blockquote(bq) => Component::Blockquote(Blockquote {
            style: bq.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: bq.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..bq.clone()
        }),
        Component::Pre(p) => Component::Pre(Pre {
            style: p.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            ..p.clone()
        }),
        Component::Details(d) => Component::Details(Details {
            style: d.style.as_ref().map(|s| substitute_props_in_style(s, props)),
            children: d.children.as_ref().map(|ch| {
                ch.iter()
                    .map(|c| substitute_props_in_component(c, props))
                    .collect()
            }),
            ..d.clone()
        }),
        Component::ImportedComponent(_) => c.clone(),
    }
}

fn component_to_html_with_imports(
    c: &Component,
    out: &mut String,
    imports: &HashMap<String, ComponentFile>,
    patches: &HashMap<String, PatchOverride>,
    base_url: Option<&str>,
) -> std::fmt::Result {
    match c {
        Component::ImportedComponent(inst) => {
            if let Some(file) = imports.get(&inst.name) {
                let expanded = substitute_props_in_component(&file.body, &inst.props);
                component_to_html_with_imports(&expanded, out, imports, patches, base_url)
            } else {
                write!(out, "<div style=\"color:#999;\">[Imported component {} - not found]</div>", escape_html(&inst.name))
            }
        }
        _ => component_to_html(c, out, patches, base_url),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Allowed HTML tags for markdown output (safe subset).
const MARKDOWN_ALLOWED_TAGS: &[&str] = &[
    "h1", "h2", "h3", "h4", "h5", "h6", "p", "ul", "ol", "li", "table", "thead", "tbody", "tr", "th", "td",
    "a", "strong", "em", "code", "pre", "blockquote", "hr", "br", "span", "div",
];

/// Render markdown to sanitized HTML (safe tags only, no script URLs).
fn markdown_to_sanitized_html(md: &str) -> String {
    use pulldown_cmark::{Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    // Remove script URLs from href and strip disallowed tags
    let html = html.replace("javascript:", "");
    sanitize_html_fragment(&html)
}

/// Keep only allowed tags; escape others so they display as text.
fn sanitize_html_fragment(html: &str) -> String {
    let allowed: std::collections::HashSet<&str> = MARKDOWN_ALLOWED_TAGS.iter().copied().collect();
    let mut out = String::new();
    let mut i = 0;
    let bytes = html.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let start = i;
            i += 1;
            if i >= bytes.len() {
                out.push_str(&escape_html(std::str::from_utf8(&bytes[start..]).unwrap_or("")));
                break;
            }
            let closing = bytes[i] == b'/';
            if closing {
                i += 1;
            }
            let tag_start = i;
            while i < bytes.len() && bytes[i] != b'>' && bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            let tag = std::str::from_utf8(&bytes[tag_start..i]).unwrap_or("").to_lowercase();
            let tag_clean = tag.trim_end_matches('/');
            if allowed.contains(&tag_clean) {
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                out.push_str(std::str::from_utf8(&bytes[start..i]).unwrap_or(""));
                continue;
            }
            out.push_str("&lt;");
            i = start + 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Build id and class attributes from component id and style (for Tailwind etc.).
fn build_attrs(id: Option<&str>, style: Option<&Style>) -> String {
    let id_part = id
        .map(|s| format!(" id=\"{}\"", escape_html(s)))
        .unwrap_or_default();
    let class_part = style
        .and_then(|s| s.classes.as_ref())
        .map(|c| format!(" class=\"{}\"", escape_html(c)))
        .unwrap_or_default();
    format!("{}{}", id_part, class_part)
}

fn style_to_css(style: &Style) -> String {
    let mut css = String::new();

    // Padding: shorthand takes precedence; else use individual/compound
    if let Some(p) = style.padding {
        css.push_str(&format!("padding:{}px;", p));
    } else {
        if let Some(v) = style.padding_vertical {
            css.push_str(&format!("padding-top:{}px;padding-bottom:{}px;", v, v));
        }
        if let Some(h) = style.padding_horizontal {
            css.push_str(&format!("padding-left:{}px;padding-right:{}px;", h, h));
        }
        if let Some(v) = style.padding_top {
            css.push_str(&format!("padding-top:{}px;", v));
        }
        if let Some(v) = style.padding_right {
            css.push_str(&format!("padding-right:{}px;", v));
        }
        if let Some(v) = style.padding_bottom {
            css.push_str(&format!("padding-bottom:{}px;", v));
        }
        if let Some(v) = style.padding_left {
            css.push_str(&format!("padding-left:{}px;", v));
        }
    }

    // Margin: same pattern
    if let Some(m) = style.margin {
        css.push_str(&format!("margin:{}px;", m));
    } else {
        if let Some(v) = style.margin_vertical {
            css.push_str(&format!("margin-top:{}px;margin-bottom:{}px;", v, v));
        }
        if let Some(h) = style.margin_horizontal {
            css.push_str(&format!("margin-left:{}px;margin-right:{}px;", h, h));
        }
        if let Some(v) = style.margin_top {
            css.push_str(&format!("margin-top:{}px;", v));
        }
        if let Some(v) = style.margin_right {
            css.push_str(&format!("margin-right:{}px;", v));
        }
        if let Some(v) = style.margin_bottom {
            css.push_str(&format!("margin-bottom:{}px;", v));
        }
        if let Some(v) = style.margin_left {
            css.push_str(&format!("margin-left:{}px;", v));
        }
    }

    // Colors
    if let Some(c) = &style.background_color {
        css.push_str(&format!("background-color:{};", c));
    }
    if let Some(c) = &style.color {
        css.push_str(&format!("color:{};", c));
    }
    if let Some(o) = style.opacity {
        css.push_str(&format!("opacity:{};", o));
    }

    // Typography
    if let Some(s) = style.font_size {
        css.push_str(&format!("font-size:{}px;", s));
    }
    if let Some(ff) = &style.font_family {
        let v = match ff {
            FontFamily::Named(named) => match named {
                nulltrace_ntml::style::FontFamilyNamed::Sans => "sans-serif".to_string(),
                nulltrace_ntml::style::FontFamilyNamed::Serif => "serif".to_string(),
                nulltrace_ntml::style::FontFamilyNamed::Monospace => "monospace".to_string(),
                nulltrace_ntml::style::FontFamilyNamed::Game => "monospace".to_string(),
            },
            FontFamily::Custom(s) => format!("\"{}\"", escape_css_string(s)),
        };
        css.push_str(&format!("font-family:{};", v));
    }
    if let Some(fw) = &style.font_weight {
        let v = match fw {
            FontWeight::Number(n) => n.to_string(),
            FontWeight::Named(named) => match named {
                nulltrace_ntml::style::FontWeightNamed::Normal => "normal".to_string(),
                nulltrace_ntml::style::FontWeightNamed::Bold => "bold".to_string(),
            },
        };
        css.push_str(&format!("font-weight:{};", v));
    }
    if let Some(a) = &style.text_align {
        let v = match a {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
            TextAlign::Justify => "justify",
        };
        css.push_str(&format!("text-align:{};", v));
    }
    if let Some(tt) = &style.text_transform {
        let v = match tt {
            TextTransform::None => "none",
            TextTransform::Uppercase => "uppercase",
            TextTransform::Lowercase => "lowercase",
            TextTransform::Capitalize => "capitalize",
        };
        css.push_str(&format!("text-transform:{};", v));
    }
    if let Some(ls) = style.letter_spacing {
        css.push_str(&format!("letter-spacing:{}px;", ls));
    }
    if let Some(lh) = style.line_height {
        css.push_str(&format!("line-height:{};", lh));
    }
    if let Some(td) = &style.text_decoration {
        let v = match td {
            TextDecoration::None => "none",
            TextDecoration::Underline => "underline",
            TextDecoration::LineThrough => "line-through",
        };
        css.push_str(&format!("text-decoration:{};", v));
    }

    // Dimensions
    if let Some(w) = &style.width {
        css.push_str(&format!("width:{};", dimension_to_css(w)));
    }
    if let Some(h) = &style.height {
        css.push_str(&format!("height:{};", dimension_to_css(h)));
    }
    if let Some(v) = style.min_width {
        css.push_str(&format!("min-width:{}px;", v));
    }
    if let Some(v) = style.max_width {
        css.push_str(&format!("max-width:{}px;", v));
    }
    if let Some(v) = style.min_height {
        css.push_str(&format!("min-height:{}px;", v));
    }
    if let Some(v) = style.max_height {
        css.push_str(&format!("max-height:{}px;", v));
    }

    // Borders
    if let Some(w) = style.border_width {
        css.push_str(&format!("border-width:{}px;", w));
    }
    if let Some(w) = style.border_top_width {
        css.push_str(&format!("border-top-width:{}px;", w));
    }
    if let Some(w) = style.border_right_width {
        css.push_str(&format!("border-right-width:{}px;", w));
    }
    if let Some(w) = style.border_bottom_width {
        css.push_str(&format!("border-bottom-width:{}px;", w));
    }
    if let Some(w) = style.border_left_width {
        css.push_str(&format!("border-left-width:{}px;", w));
    }
    if let Some(c) = &style.border_color {
        css.push_str(&format!("border-color:{};", c));
    }
    if let Some(bs) = &style.border_style {
        let v = match bs {
            BorderStyle::Solid => "solid",
            BorderStyle::Dashed => "dashed",
            BorderStyle::Dotted => "dotted",
        };
        css.push_str(&format!("border-style:{};", v));
    }
    if let Some(r) = style.border_radius {
        css.push_str(&format!("border-radius:{}px;", r));
    }
    if let Some(r) = style.border_top_left_radius {
        css.push_str(&format!("border-top-left-radius:{}px;", r));
    }
    if let Some(r) = style.border_top_right_radius {
        css.push_str(&format!("border-top-right-radius:{}px;", r));
    }
    if let Some(r) = style.border_bottom_left_radius {
        css.push_str(&format!("border-bottom-left-radius:{}px;", r));
    }
    if let Some(r) = style.border_bottom_right_radius {
        css.push_str(&format!("border-bottom-right-radius:{}px;", r));
    }

    // Shadow
    if let Some(sh) = &style.shadow {
        let v = match sh {
            Shadow::Preset(preset) => match preset {
                nulltrace_ntml::style::ShadowPreset::Small => "0 1px 2px rgba(0,0,0,0.15)",
                nulltrace_ntml::style::ShadowPreset::Medium => "0 4px 6px rgba(0,0,0,0.2)",
                nulltrace_ntml::style::ShadowPreset::Large => "0 10px 15px rgba(0,0,0,0.25)",
            }
            .to_string(),
            Shadow::Custom {
                color,
                offset,
                blur,
                opacity,
            } => format!(
                "{}px {}px {}px {}",
                offset.x, offset.y, blur, rgba_from_hex(color, *opacity)
            ),
        };
        css.push_str(&format!("box-shadow:{};", v));
    }

    // Position
    if let Some(p) = &style.position {
        let v = match p {
            Position::Relative => "relative",
            Position::Absolute => "absolute",
        };
        css.push_str(&format!("position:{};", v));
    }
    if let Some(v) = style.top {
        css.push_str(&format!("top:{}px;", v));
    }
    if let Some(v) = style.right {
        css.push_str(&format!("right:{}px;", v));
    }
    if let Some(v) = style.bottom {
        css.push_str(&format!("bottom:{}px;", v));
    }
    if let Some(v) = style.left {
        css.push_str(&format!("left:{}px;", v));
    }
    if let Some(z) = style.z_index {
        css.push_str(&format!("z-index:{};", z));
    }

    // Flex item
    if let Some(f) = style.flex {
        css.push_str(&format!("flex:{};", f));
    }
    if let Some(a) = &style.align_self {
        let v = match a {
            Alignment::Start => "flex-start",
            Alignment::Center => "center",
            Alignment::End => "flex-end",
            Alignment::Stretch => "stretch",
        };
        css.push_str(&format!("align-self:{};", v));
    }

    // Display
    if let Some(d) = &style.display {
        let v = match d {
            Display::Flex => "flex",
            Display::None => "none",
        };
        css.push_str(&format!("display:{};", v));
    }
    if let Some(o) = &style.overflow {
        let v = match o {
            Overflow::Visible => "visible",
            Overflow::Hidden => "hidden",
            Overflow::Scroll => "scroll",
            Overflow::Auto => "auto",
        };
        css.push_str(&format!("overflow:{};", v));
    }
    if let Some(c) = &style.cursor {
        let v = match c {
            Cursor::Default => "default",
            Cursor::Pointer => "pointer",
            Cursor::NotAllowed => "not-allowed",
            Cursor::Text => "text",
        };
        css.push_str(&format!("cursor:{};", v));
    }

    css
}

/// Escapes a string for use inside CSS (e.g. font-family value)
fn escape_css_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Converts hex color + opacity to rgba() string
fn rgba_from_hex(hex: &str, opacity: f64) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return format!("rgba({},{},{},{})", r, g, b, opacity);
        }
    }
    format!("rgba(0,0,0,{})", opacity)
}

fn dimension_to_css(d: &Dimension) -> String {
    match d {
        Dimension::Pixels(p) => format!("{}px", p),
        Dimension::Auto => "auto".to_string(),
        Dimension::Custom(s) => s.clone(),
    }
}

fn justify_to_css(j: &JustifyContent) -> &'static str {
    match j {
        JustifyContent::Start => "flex-start",
        JustifyContent::Center => "center",
        JustifyContent::End => "flex-end",
        JustifyContent::SpaceBetween => "space-between",
        JustifyContent::SpaceAround => "space-around",
        JustifyContent::SpaceEvenly => "space-evenly",
    }
}

fn align_items_to_css(a: &AlignItems) -> &'static str {
    match a {
        AlignItems::Start => "flex-start",
        AlignItems::Center => "center",
        AlignItems::End => "flex-end",
        AlignItems::Stretch => "stretch",
    }
}

fn grid_size_to_css(size: &GridSize) -> String {
    match size {
        GridSize::Count(n) => format!("repeat({}, 1fr)", n),
        GridSize::Definitions(defs) => defs.join(" "),
    }
}

fn stack_alignment_to_css(a: &StackAlignment) -> (&'static str, &'static str) {
    match a {
        StackAlignment::TopLeft => ("flex-start", "flex-start"),
        StackAlignment::TopCenter => ("center", "flex-start"),
        StackAlignment::TopRight => ("flex-end", "flex-start"),
        StackAlignment::CenterLeft => ("flex-start", "center"),
        StackAlignment::Center => ("center", "center"),
        StackAlignment::CenterRight => ("flex-end", "center"),
        StackAlignment::BottomLeft => ("flex-start", "flex-end"),
        StackAlignment::BottomCenter => ("center", "flex-end"),
        StackAlignment::BottomRight => ("flex-end", "flex-end"),
    }
}

fn get_component_id(c: &Component) -> Option<&str> {
    match c {
        Component::Container(x) => x.id.as_deref(),
        Component::Flex(x) => x.id.as_deref(),
        Component::Grid(x) => x.id.as_deref(),
        Component::Stack(x) => x.id.as_deref(),
        Component::Row(x) => x.id.as_deref(),
        Component::Column(x) => x.id.as_deref(),
        Component::Text(x) => x.id.as_deref(),
        Component::Image(x) => x.id.as_deref(),
        Component::Icon(x) => x.id.as_deref(),
        Component::Button(x) => x.id.as_deref(),
        Component::Input(x) => x.id.as_deref(),
        Component::Checkbox(x) => x.id.as_deref(),
        Component::Radio(x) => x.id.as_deref(),
        Component::Select(x) => x.id.as_deref(),
        Component::ProgressBar(x) => x.id.as_deref(),
        Component::Badge(x) => x.id.as_deref(),
        Component::Divider(x) => x.id.as_deref(),
        Component::Spacer(_) => None,
        Component::Link(x) => x.id.as_deref(),
        Component::Code(x) => x.id.as_deref(),
        Component::Markdown(x) => x.id.as_deref(),
        Component::List(x) => x.id.as_deref(),
        Component::ListItem(x) => x.id.as_deref(),
        Component::Heading(x) => x.id.as_deref(),
        Component::Table(x) => x.id.as_deref(),
        Component::Blockquote(x) => x.id.as_deref(),
        Component::Pre(x) => x.id.as_deref(),
        Component::Details(x) => x.id.as_deref(),
        Component::ImportedComponent(x) => x.id.as_deref(),
    }
}

fn component_to_html(
    c: &Component,
    out: &mut String,
    patches: &HashMap<String, PatchOverride>,
    base_url: Option<&str>,
) -> std::fmt::Result {
    if let Some(id) = get_component_id(c) {
        if let Some(ov) = patches.get(id) {
            if ov.visible == Some(false) {
                return Ok(());
            }
        }
    }
    match c {
        Component::Container(ct) => {
            let style = ct
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(ct.id.as_deref(), ct.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            if let Some(children) = &ct.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div>")?;
        }
        Component::Flex(f) => {
            let mut style = f
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.push_str("display:flex;");
            if let Some(d) = &f.direction {
                let dir = match d {
                    FlexDirection::Row => "row",
                    FlexDirection::Column => "column",
                };
                style.push_str(&format!("flex-direction:{};", dir));
            }
            if let Some(j) = &f.justify {
                style.push_str(&format!("justify-content:{};", justify_to_css(j)));
            }
            if let Some(a) = &f.align {
                style.push_str(&format!("align-items:{};", align_items_to_css(a)));
            }
            if let Some(w) = f.wrap {
                style.push_str(&format!("flex-wrap:{};", if w { "wrap" } else { "nowrap" }));
            }
            if let Some(g) = f.gap {
                style.push_str(&format!("gap:{}px;", g));
            }
            let attrs = build_attrs(f.id.as_deref(), f.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            if let Some(children) = &f.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div>")?;
        }
        Component::Text(t) => {
            let text = t
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.text.as_ref())
                .map(|s| s.as_str())
                .unwrap_or(&t.text);
            let style = t
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(t.id.as_deref(), t.style.as_ref());
            write!(
                out,
                "<span{} style=\"{}\">{}</span>",
                attrs,
                style,
                escape_html(text)
            )?;
        }
        Component::Button(b) => {
            let disabled = b
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.disabled)
                .or(b.disabled)
                .unwrap_or(false);
            let style = b
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(b.id.as_deref(), b.style.as_ref());
            let has_lua_action = !disabled && !b.action.contains(':');
            let data_action = if has_lua_action {
                format!(" data-action=\"{}\"", escape_html(&b.action))
            } else {
                String::new()
            };
            let onclick = if has_lua_action {
                " onclick=\"var a=this.getAttribute('data-action');if(a)window.parent.postMessage({type:'ntml-action',action:a},'*')\""
                    .to_string()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            write!(
                out,
                "<button{} type=\"button\"{}{} style=\"{}\"{}>",
                attrs, data_action, onclick, style, disabled_attr
            )?;
            if let Some(children) = &b.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</button>")?;
        }
        Component::Image(img) => {
            let src = if img.src.is_empty() {
                String::new()
            } else if let Some(base) = base_url {
                let path = img.src.trim_start_matches('/');
                let base_trimmed = base.trim_end_matches('/');
                format!("{}/{}", base_trimmed, path)
            } else {
                img.src.clone()
            };
            let fit_css = img.fit.as_ref().map(|f| {
                let v = match f {
                    ImageFit::Cover => "cover",
                    ImageFit::Contain => "contain",
                    ImageFit::Fill => "fill",
                    ImageFit::None => "none",
                    ImageFit::ScaleDown => "scale-down",
                };
                format!("object-fit:{};", v)
            }).unwrap_or_default();
            let mut style = img
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.insert_str(0, &fit_css);
            let attrs = build_attrs(img.id.as_deref(), img.style.as_ref());
            let alt = img
                .alt
                .as_ref()
                .map(|s| escape_html(s))
                .unwrap_or_else(|| "".to_string());
            write!(
                out,
                "<img{} src=\"{}\" alt=\"{}\" style=\"{}\">",
                attrs,
                escape_html(&src),
                alt,
                style
            )?;
        }
        Component::Link(lnk) => {
            let href_resolved = if lnk.href.starts_with("http://") || lnk.href.starts_with("https://") {
                lnk.href.clone()
            } else if let Some(base) = base_url {
                let path = lnk.href.trim_start_matches('/');
                let base_trimmed = base.trim_end_matches('/');
                format!("{}/{}", base_trimmed, path)
            } else {
                lnk.href.clone()
            };
            let target_val = if matches!(lnk.target, Some(LinkTarget::New)) {
                "new"
            } else {
                "same"
            };
            let style = lnk
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(lnk.id.as_deref(), lnk.style.as_ref());
            write!(
                out,
                "<a{} href=\"{}\" data-ntml-url=\"{}\" data-ntml-target=\"{}\" style=\"color:inherit;text-decoration:underline;cursor:pointer;{}\" onclick=\"event.preventDefault();var u=this.getAttribute('data-ntml-url');var t=this.getAttribute('data-ntml-target');if(u)window.parent.postMessage({{type:'ntml-navigate',url:u,target:t||'same'}},'*')\">",
                attrs,
                escape_html(&href_resolved),
                escape_html(&href_resolved),
                target_val,
                style
            )?;
            if let Some(children) = &lnk.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            } else {
                write!(out, "{}", escape_html(&lnk.href))?;
            }
            write!(out, "</a>")?;
        }
        Component::Row(r) => {
            let mut style = r
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.push_str("display:flex;flex-direction:row;");
            if let Some(j) = &r.justify {
                style.push_str(&format!("justify-content:{};", justify_to_css(j)));
            }
            if let Some(a) = &r.align {
                style.push_str(&format!("align-items:{};", align_items_to_css(a)));
            }
            if let Some(w) = r.wrap {
                style.push_str(&format!("flex-wrap:{};", if w { "wrap" } else { "nowrap" }));
            }
            if let Some(g) = r.gap {
                style.push_str(&format!("gap:{}px;", g));
            }
            let attrs = build_attrs(r.id.as_deref(), r.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            if let Some(children) = &r.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div>")?;
        }
        Component::Column(col) => {
            let mut style = col
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.push_str("display:flex;flex-direction:column;");
            if let Some(j) = &col.justify {
                style.push_str(&format!("justify-content:{};", justify_to_css(j)));
            }
            if let Some(a) = &col.align {
                style.push_str(&format!("align-items:{};", align_items_to_css(a)));
            }
            if let Some(w) = col.wrap {
                style.push_str(&format!("flex-wrap:{};", if w { "wrap" } else { "nowrap" }));
            }
            if let Some(g) = col.gap {
                style.push_str(&format!("gap:{}px;", g));
            }
            let attrs = build_attrs(col.id.as_deref(), col.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            if let Some(children) = &col.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div>")?;
        }
        Component::Grid(g) => {
            let mut style = g
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.push_str("display:grid;");
            style.push_str(&format!(
                "grid-template-columns:{};",
                grid_size_to_css(&g.columns)
            ));
            if let Some(rows) = &g.rows {
                style.push_str(&format!("grid-template-rows:{};", grid_size_to_css(rows)));
            }
            if let Some(gap) = &g.gap {
                match gap {
                    GridGap::Single(v) => {
                        style.push_str(&format!("gap:{}px;", v));
                    }
                    GridGap::Separate { row, column } => {
                        style.push_str(&format!("row-gap:{}px;column-gap:{}px;", row, column));
                    }
                }
            }
            let attrs = build_attrs(g.id.as_deref(), g.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            if let Some(children) = &g.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div>")?;
        }
        Component::Stack(s) => {
            let mut style = s
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            style.insert_str(0, "position:relative;");
            let inner_style = if let Some(a) = &s.alignment {
                let (justify, align) = stack_alignment_to_css(a);
                format!(
                    "position:absolute;inset:0;display:flex;justify-content:{};align-items:{};",
                    justify, align
                )
            } else {
                "position:absolute;inset:0;display:flex;".to_string()
            };
            let attrs = build_attrs(s.id.as_deref(), s.style.as_ref());
            write!(out, "<div{} style=\"{}\">", attrs, style)?;
            write!(out, "<div style=\"{}\">", inner_style)?;
            if let Some(children) = &s.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</div></div>")?;
        }
        Component::Divider(d) => {
            let style = d
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let (base_style, tag) = match d.orientation.as_ref() {
                Some(DividerOrientation::Vertical) => (
                    "display:inline-block;width:0;height:100%;min-height:1em;border:none;border-left:1px solid #ccc;margin:0 8px;vertical-align:middle;",
                    "div"
                ),
                _ => (
                    "border:none;border-top:1px solid #ccc;margin:8px 0;",
                    "hr"
                ),
            };
            let attrs = build_attrs(d.id.as_deref(), d.style.as_ref());
            let combined = if style.is_empty() {
                base_style.to_string()
            } else {
                format!("{}{}", base_style, style)
            };
            if tag == "hr" {
                write!(out, "<hr{} style=\"{}\">", attrs, combined)?;
            } else {
                write!(out, "<div{} style=\"{}\"></div>", attrs, combined)?;
            }
        }
        Component::Spacer(sp) => {
            match &sp.size {
                SpacerSize::Fixed(v) => {
                    write!(out, "<div style=\"height:{}px\"></div>", v)?;
                }
                SpacerSize::Auto(_) => {
                    write!(out, "<div style=\"flex:1;min-width:0;min-height:0\"></div>")?;
                }
            }
        }
        Component::ProgressBar(pb) => {
            let value = pb
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.value)
                .unwrap_or(pb.value);
            let max = pb.max.unwrap_or(100.0);
            let pct = (value / max * 100.0).min(100.0).max(0.0);
            let style = pb
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(pb.id.as_deref(), pb.style.as_ref());
            write!(
                out,
                "<div{} style=\"background:#eee;border-radius:4px;overflow:hidden;{}\"><div style=\"width:{}%;height:20px;background:#4a9;{}\"></div></div>",
                attrs, style, pct, style
            )?;
        }
        Component::Badge(b) => {
            let text = b
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.text.as_ref())
                .map(|s| s.as_str())
                .unwrap_or(&b.text);
            let style = b
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(b.id.as_deref(), b.style.as_ref());
            write!(
                out,
                "<span{} style=\"display:inline-block;padding:2px 8px;border-radius:4px;font-size:12px;{}\">{}</span>",
                attrs,
                style,
                escape_html(text)
            )?;
        }
        Component::Input(inp) => {
            let style = inp
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(inp.id.as_deref(), inp.style.as_ref());
            let ph = inp
                .placeholder
                .as_ref()
                .map(|s| escape_html(s))
                .unwrap_or_else(|| "".to_string());
            let val = inp
                .value
                .as_ref()
                .map(|s| escape_html(s))
                .unwrap_or_else(|| "".to_string());
            write!(
                out,
                "<input{} type=\"text\" name=\"{}\" placeholder=\"{}\" value=\"{}\" style=\"{}\" readonly disabled>",
                attrs,
                escape_html(&inp.name),
                ph,
                val,
                style
            )?;
        }
        Component::Checkbox(cb) => {
            let style = cb
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(cb.id.as_deref(), cb.style.as_ref());
            let checked = cb.checked.unwrap_or(false);
            write!(
                out,
                "<label{} style=\"{}\"><input type=\"checkbox\" name=\"{}\" {} disabled> {}</label>",
                attrs,
                style,
                escape_html(&cb.name),
                if checked { "checked" } else { "" },
                escape_html(cb.label.as_deref().unwrap_or(""))
            )?;
        }
        Component::Radio(r) => {
            let style = r
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(r.id.as_deref(), r.style.as_ref());
            let checked = r.checked.unwrap_or(false);
            write!(
                out,
                "<label{} style=\"{}\"><input type=\"radio\" name=\"{}\" value=\"{}\" {} disabled> {}</label>",
                attrs,
                style,
                escape_html(&r.name),
                escape_html(&r.value),
                if checked { "checked" } else { "" },
                escape_html(r.label.as_deref().unwrap_or(""))
            )?;
        }
        Component::Select(sel) => {
            let style = sel
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(sel.id.as_deref(), sel.style.as_ref());
            write!(out, "<select{} name=\"{}\" style=\"{}\" disabled>", attrs, escape_html(&sel.name), style)?;
            for opt in &sel.options {
                write!(
                    out,
                    "<option value=\"{}\">{}</option>",
                    escape_html(&opt.value),
                    escape_html(&opt.label)
                )?;
            }
            write!(out, "</select>")?;
        }
        Component::Icon(ic) => {
            let size = ic.size.unwrap_or(16.0);
            let size_px = size as i32;
            let style = ic
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(ic.id.as_deref(), ic.style.as_ref());
            let title_attr = if ic.name.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", escape_html(&ic.name))
            };
            let mut combined = format!(
                "display:inline-block;width:{}px;height:{}px;flex-shrink:0;vertical-align:middle;",
                size, size
            );
            combined.push_str(&style);
            // data-lucide: frontend resolves this to Lucide icon SVG
            write!(
                out,
                "<span data-lucide=\"{}\" data-size=\"{}\"{}{} style=\"{}\"></span>",
                escape_html(&ic.name),
                size_px,
                attrs,
                title_attr,
                combined
            )?;
        }
        Component::Code(co) => {
            let style_css = co
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(co.id.as_deref(), co.style.as_ref());
            let lang_class = co
                .language
                .as_ref()
                .map(|l| format!(" class=\"language-{}\"", escape_html(l)))
                .unwrap_or_default();
            let text_esc = escape_html(&co.text);
            if co.block == Some(true) {
                write!(out, "<pre{} style=\"{}\"><code{}>{}</code></pre>", attrs, style_css, lang_class, text_esc)?;
            } else {
                write!(out, "<code{} style=\"{}\">{}</code>", attrs, style_css, text_esc)?;
            }
        }
        Component::Markdown(m) => {
            let style_css = m
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(m.id.as_deref(), m.style.as_ref());
            let inner = markdown_to_sanitized_html(&m.content);
            write!(out, "<div{} class=\"ntml-markdown\" style=\"{}\">{}</div>", attrs, style_css, inner)?;
        }
        Component::List(list) => {
            let style_css = list
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(list.id.as_deref(), list.style.as_ref());
            let tag = if list.ordered == Some(true) { "ol" } else { "ul" };
            write!(out, "<{}{} style=\"{}\">", tag, attrs, style_css)?;
            if let Some(children) = &list.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</{}>", tag)?;
        }
        Component::ListItem(li) => {
            let style_css = li
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(li.id.as_deref(), li.style.as_ref());
            write!(out, "<li{} style=\"{}\">", attrs, style_css)?;
            if let Some(children) = &li.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</li>")?;
        }
        Component::Heading(h) => {
            let style_css = h
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(h.id.as_deref(), h.style.as_ref());
            let tag = match h.level {
                1 => "h1",
                2 => "h2",
                _ => "h3",
            };
            write!(out, "<{}{} style=\"{}\">{}</{}>", tag, attrs, style_css, escape_html(&h.text), tag)?;
        }
        Component::Table(t) => {
            let style_css = t
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(t.id.as_deref(), t.style.as_ref());
            write!(out, "<table{} style=\"{}\">", attrs, style_css)?;
            if !t.headers.is_empty() {
                write!(out, "<thead><tr>")?;
                for hdr in &t.headers {
                    write!(out, "<th>{}</th>", escape_html(hdr))?;
                }
                write!(out, "</tr></thead>")?;
            }
            write!(out, "<tbody>")?;
            for row in &t.rows {
                write!(out, "<tr>")?;
                for cell in row {
                    write!(out, "<td>{}</td>", escape_html(cell))?;
                }
                write!(out, "</tr>")?;
            }
            write!(out, "</tbody></table>")?;
        }
        Component::Blockquote(bq) => {
            let style_css = bq
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(bq.id.as_deref(), bq.style.as_ref());
            write!(out, "<blockquote{} style=\"{}\">", attrs, style_css)?;
            if let Some(children) = &bq.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</blockquote>")?;
        }
        Component::Pre(pre) => {
            let style_css = pre
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(pre.id.as_deref(), pre.style.as_ref());
            write!(out, "<pre{} style=\"{}\">{}</pre>", attrs, style_css, escape_html(&pre.text))?;
        }
        Component::Details(d) => {
            let style_css = d
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let attrs = build_attrs(d.id.as_deref(), d.style.as_ref());
            let open_attr = if d.open == Some(true) { " open" } else { "" };
            write!(out, "<details{}{} style=\"{}\">", attrs, open_attr, style_css)?;
            write!(out, "<summary>{}</summary>", escape_html(&d.summary))?;
            if let Some(children) = &d.children {
                for ch in children {
                    component_to_html(ch, out, patches, base_url)?;
                }
            }
            write!(out, "</details>")?;
        }
        Component::ImportedComponent(_) => {
            write!(out, "<div style=\"color:#999;\">[Imported component - not rendered]</div>")?;
        }
    }
    Ok(())
}
