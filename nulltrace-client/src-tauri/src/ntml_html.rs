//! Converts NTML to safe HTML for iframe srcDoc.
//! No script, no inline event handlers; only structure and styles.

/// Base document styles (html, body). The Rust Tailwind engine generates only utility classes.
const NTML_BASE_STYLES: &str = "html,body{margin:0;min-height:100vh;background:#09090b;color:#e4e4e7;}\
.ntml-markdown{line-height:1.7;color:#d4d4d8;}\
.ntml-markdown h1,.ntml-markdown h2,.ntml-markdown h3,.ntml-markdown h4{color:#f4f4f5;font-weight:700;margin:1.5em 0 0.5em;line-height:1.25;}\
.ntml-markdown h1{font-size:2rem;}\
.ntml-markdown h2{font-size:1.5rem;border-bottom:1px solid #3f3f46;padding-bottom:0.3em;}\
.ntml-markdown h3{font-size:1.2rem;}\
.ntml-markdown p{margin:0.75em 0;}\
.ntml-markdown a{color:#f59e0b;text-decoration:underline;}\
.ntml-markdown code{font-family:monospace;font-size:0.875em;background:#27272a;color:#fbbf24;padding:0.15em 0.4em;border-radius:4px;}\
.ntml-markdown pre{background:#18181b;border:1px solid #3f3f46;border-radius:8px;padding:1rem 1.25rem;overflow-x:auto;margin:1em 0;}\
.ntml-markdown pre code{background:none;color:#d4d4d8;padding:0;font-size:0.82rem;}\
.ntml-markdown ul,.ntml-markdown ol{padding-left:1.5em;margin:0.75em 0;}\
.ntml-markdown li{margin:0.35em 0;}\
.ntml-markdown blockquote{border-left:3px solid #f59e0b;margin:1em 0;padding:0.5em 1em;background:#27272a;color:#a1a1aa;border-radius:0 4px 4px 0;}\
.ntml-markdown table{border-collapse:collapse;width:100%;margin:1em 0;}\
.ntml-markdown th,.ntml-markdown td{border:1px solid #3f3f46;padding:0.5em 0.75em;text-align:left;}\
.ntml-markdown th{background:#27272a;font-weight:600;color:#f4f4f5;}\
.ntml-markdown hr{border:none;border-top:1px solid #3f3f46;margin:2em 0;}\
.ntml-markdown strong{color:#f4f4f5;font-weight:700;}\
.ntml-markdown em{font-style:italic;color:#a1a1aa;}";

use nulltrace_ntml::components::*;
use nulltrace_ntml::tailwind;
use nulltrace_ntml::style::{
    Alignment, BorderStyle, Cursor, Dimension, Display, FontFamily, FontWeight, Overflow,
    Position, Shadow, TextAlign, TextDecoration, TextTransform,
};
use nulltrace_ntml::{parse_component_file, parse_document, Component, ComponentFile, Style};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::OnceLock;

/// Override for a component by id. Only fields that are set are applied.
#[derive(Default, Clone)]
pub struct PatchOverride {
    pub text: Option<String>,
    pub visible: Option<bool>,
    pub value: Option<f64>,
    /// Input value (string) for Input components.
    pub input_value: Option<String>,
    pub disabled: Option<bool>,
    /// Full class replacement (set_class). If set, replaces style.classes entirely.
    pub class_replace: Option<String>,
    /// Class tokens to append (add_class).
    pub class_add: Vec<String>,
    /// Class tokens to remove (remove_class).
    pub class_remove: Vec<String>,
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
            crate::ntml_runtime::Patch::SetInputValue { id, value } => {
                map.entry(id.clone()).or_default().input_value = Some(value.clone());
            }
            crate::ntml_runtime::Patch::SetDisabled { id, disabled } => {
                map.entry(id.clone()).or_default().disabled = Some(*disabled);
            }
            crate::ntml_runtime::Patch::SetClass { id, class } => {
                let po = map.entry(id.clone()).or_default();
                po.class_replace = Some(class.clone());
                // Reset add/remove when a full replace is issued
                po.class_add.clear();
                po.class_remove.clear();
            }
            crate::ntml_runtime::Patch::AddClass { id, class } => {
                map.entry(id.clone()).or_default().class_add.push(class.clone());
            }
            crate::ntml_runtime::Patch::RemoveClass { id, class } => {
                map.entry(id.clone()).or_default().class_remove.push(class.clone());
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
    ntml_to_html_with_imports_and_patches(yaml, imports, &HashMap::new(), &[], base_url)
}

/// Converts NTML to safe HTML with imports and patches applied.
/// `markdowns` maps external src paths (e.g. `/content/welcome.md`) to their fetched content.
pub fn ntml_to_html_with_imports_and_patches(
    yaml: &str,
    imports: &[NtmlImport],
    markdowns: &HashMap<String, String>,
    patches: &[crate::ntml_runtime::Patch],
    base_url: Option<&str>,
) -> Result<String, String> {
    let doc = parse_document(yaml).map_err(|e| e.to_string())?;

    let title = doc.head().map(|h| h.title.as_str()).unwrap_or("Page");
    let root = doc.root_component();

    // Pre-process: resolve external Markdown src references into inline content
    let resolved_root;
    let render_root = if markdowns.is_empty() {
        root
    } else {
        resolved_root = resolve_markdown_srcs(root, markdowns);
        &resolved_root
    };

    let import_map: HashMap<String, ComponentFile> = imports
        .iter()
        .filter_map(|i| {
            parse_component_file(&i.content)
                .ok()
                .map(|f| (i.alias.clone(), f))
        })
        .collect();

    let patch_map = patches_to_map(patches);

    // Build body HTML first so we can extract classes and generate CSS
    let mut body_html = String::new();
    component_to_html_with_imports(render_root, &mut body_html, &import_map, &patch_map, base_url)
        .map_err(|e| e.to_string())?;

    let generated_css = tailwind::generate_css(&body_html);
    let syntect_css = syntect_highlight_css();
    let css = if generated_css.is_empty() {
        format!("{}{}", NTML_BASE_STYLES, syntect_css)
    } else {
        format!("{}{}{}", NTML_BASE_STYLES, generated_css, syntect_css)
    };

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
{}
</body>
</html>
"#,
        escape_html(title),
        css,
        body_html
    )
    .map_err(|e| e.to_string())?;

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
            visible: ct.visible,
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

/// Pre-processes the component tree: replaces `Markdown { src }` nodes with
/// `Markdown { content }` using the fetched markdown contents map.
fn resolve_markdown_srcs(
    c: &Component,
    markdowns: &HashMap<String, String>,
) -> Component {
    match c {
        Component::Markdown(m) => {
            if let Some(src) = &m.src {
                let content = markdowns.get(src.as_str()).cloned().unwrap_or_default();
                Component::Markdown(Markdown {
                    content: Some(content),
                    src: None,
                    ..m.clone()
                })
            } else {
                c.clone()
            }
        }
        Component::Container(ct) => Component::Container(Container {
            children: ct.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..ct.clone()
        }),
        Component::Flex(f) => Component::Flex(Flex {
            children: f.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..f.clone()
        }),
        Component::Grid(g) => Component::Grid(Grid {
            children: g.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..g.clone()
        }),
        Component::Stack(s) => Component::Stack(Stack {
            children: s.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..s.clone()
        }),
        Component::Row(r) => Component::Row(Row {
            children: r.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..r.clone()
        }),
        Component::Column(col) => Component::Column(Column {
            children: col.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..col.clone()
        }),
        Component::Button(b) => Component::Button(Button {
            children: b.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..b.clone()
        }),
        Component::Link(lnk) => Component::Link(Link {
            children: lnk.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..lnk.clone()
        }),
        Component::List(l) => Component::List(List {
            children: l.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..l.clone()
        }),
        Component::ListItem(li) => Component::ListItem(ListItem {
            children: li.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..li.clone()
        }),
        Component::Blockquote(bq) => Component::Blockquote(Blockquote {
            children: bq.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..bq.clone()
        }),
        Component::Details(d) => Component::Details(Details {
            children: d.children.as_ref().map(|ch| {
                ch.iter().map(|c| resolve_markdown_srcs(c, markdowns)).collect()
            }),
            ..d.clone()
        }),
        _ => c.clone(),
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
/// Code blocks (```ntml, ```lua, etc.) are syntax-highlighted via syntect.
fn markdown_to_sanitized_html(md: &str) -> String {
    use pulldown_cmark::{Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    // Remove script URLs from href
    let html = html.replace("javascript:", "");
    // Apply syntax highlighting to code blocks
    let html = replace_code_blocks_with_highlighted(&html);
    sanitize_html_fragment(&html)
}

/// Unescape HTML entities in code block content (reverse of escape_html).
fn unescape_html_content(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

/// Map markdown language token to syntect syntax token.
fn syntect_lang_for(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "ntml" => "xml", // NTML is XML-based
        "lua" | "luau" => "lua",
        "css" => "css",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "html" => "html",
        _ => lang,
    }
}

/// Class style for syntect (prefix avoids CSS conflicts).
fn syntect_class_style() -> syntect::html::ClassStyle {
    syntect::html::ClassStyle::SpacedPrefixed { prefix: "hl-" }
}

static SYNTRACT_CSS: OnceLock<String> = OnceLock::new();

/// Get syntect theme CSS for code block highlighting (cached).
fn syntect_highlight_css() -> &'static str {
    SYNTRACT_CSS.get_or_init(|| {
        use syntect::highlighting::ThemeSet;
        use syntect::html::css_for_theme_with_class_style;

        let ts = ThemeSet::load_defaults();
        let theme = ts
            .themes
            .get("base16-ocean.dark")
            .or_else(|| ts.themes.get("InspiredGitHub"))
            .or_else(|| ts.themes.values().next())
            .expect("at least one theme");
        css_for_theme_with_class_style(theme, syntect_class_style()).unwrap_or_default()
    })
}

/// Syntax-highlight a code block using syntect. Returns HTML with span elements.
fn highlight_code_block(lang: &str, source: &str) -> String {
    use syntect::html::ClassedHTMLGenerator;
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    let ss = SyntaxSet::load_defaults_newlines();
    let syntect_lang = syntect_lang_for(lang);
    let syntax = ss
        .find_syntax_by_token(syntect_lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut html_gen =
        ClassedHTMLGenerator::new_with_class_style(syntax, &ss, syntect_class_style());
    for line in LinesWithEndings::from(source) {
        if html_gen.parse_html_for_line_which_includes_newline(line).is_err() {
            return escape_html(source);
        }
    }
    html_gen.finalize()
}

/// Find and replace <pre><code class="language-xxx">...</code></pre> blocks with highlighted HTML.
fn replace_code_blocks_with_highlighted(html: &str) -> String {
    const OPEN: &str = "<pre><code class=\"language-";
    const OPEN_END: &str = "\">";
    const CLOSE: &str = "</code></pre>";

    let mut result = String::with_capacity(html.len());
    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        if let Some(start) = find_substring(bytes, i, OPEN.as_bytes()) {
            let lang_start = start + OPEN.len();
            let lang_end = match bytes[lang_start..].iter().position(|&b| b == b'"') {
                Some(p) => lang_start + p,
                None => {
                    i = start + 1;
                    continue;
                }
            };
            let lang = std::str::from_utf8(&bytes[lang_start..lang_end]).unwrap_or("");
            let content_start = lang_end + OPEN_END.len();
            if let Some(content_end) = find_substring(bytes, content_start, CLOSE.as_bytes()) {
                let content = std::str::from_utf8(&bytes[content_start..content_end]).unwrap_or("");
                let raw = unescape_html_content(content);
                let highlighted = highlight_code_block(lang, &raw);
                result.push_str(std::str::from_utf8(&bytes[i..start]).unwrap_or(""));
                result.push_str(OPEN);
                result.push_str(lang);
                result.push_str(OPEN_END);
                result.push_str(&highlighted);
                result.push_str(CLOSE);
                i = content_end + CLOSE.len();
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn find_substring(haystack: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || start + needle.len() > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| start + p)
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

/// Compute the final class string for an element, merging style + patch overrides.
/// Priority: set_class replaces style.classes; add_class appends; remove_class removes tokens.
fn compute_class(
    id: Option<&str>,
    style: Option<&Style>,
    patches: &HashMap<String, PatchOverride>,
) -> Option<String> {
    let po = id.and_then(|i| patches.get(i));
    // Base: class_replace takes priority over style.classes
    let base: Option<&str> = po
        .and_then(|p| p.class_replace.as_deref())
        .or_else(|| style.and_then(|s| s.classes.as_deref()));

    let has_mutations = po.map(|p| !p.class_add.is_empty() || !p.class_remove.is_empty()).unwrap_or(false);

    if !has_mutations {
        return base.map(|s| s.to_string());
    }

    let po = po.unwrap();
    // Build ordered token list from base (dedup via seen set)
    let mut tokens: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(b) = base {
        for tok in b.split_whitespace() {
            if seen.insert(tok.to_string()) {
                tokens.push(tok.to_string());
            }
        }
    }
    // Add tokens
    for chunk in &po.class_add {
        for tok in chunk.split_whitespace() {
            if seen.insert(tok.to_string()) {
                tokens.push(tok.to_string());
            }
        }
    }
    // Remove tokens
    let mut remove_set: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for chunk in &po.class_remove {
        for tok in chunk.split_whitespace() {
            remove_set.insert(tok);
        }
    }
    tokens.retain(|t| !remove_set.contains(t.as_str()));

    if tokens.is_empty() { None } else { Some(tokens.join(" ")) }
}

/// Build id and class attributes.
fn build_attrs(id: Option<&str>, class: Option<&str>) -> String {
    let id_part = id
        .map(|s| format!(" id=\"{}\"", escape_html(s)))
        .unwrap_or_default();
    let class_part = class
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
        let patch_visible = patches.get(id).and_then(|ov| ov.visible);
        let attr_visible = match c {
            Component::Container(ct) => ct.visible,
            _ => None,
        };
        let visible = patch_visible.or(attr_visible).unwrap_or(true);
        if !visible {
            return Ok(());
        }
    }
    match c {
        Component::Container(ct) => {
            let style = ct
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(ct.id.as_deref(), ct.style.as_ref(), patches);
            let attrs = build_attrs(ct.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(f.id.as_deref(), f.style.as_ref(), patches);
            let attrs = build_attrs(f.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(t.id.as_deref(), t.style.as_ref(), patches);
            let attrs = build_attrs(t.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(b.id.as_deref(), b.style.as_ref(), patches);
            let attrs = build_attrs(b.id.as_deref(), cls.as_deref());
            let has_lua_action = !disabled && !b.action.contains(':');
            let mut data_attrs = String::new();
            if has_lua_action {
                data_attrs.push_str(&format!(" data-action=\"{}\"", escape_html(&b.action)));
            }
            for (k, v) in &b.data {
                if k.starts_with("data-") {
                    data_attrs.push_str(&format!(" {}=\"{}\"", k, escape_html(v)));
                }
            }
            let onclick = if has_lua_action {
                " onclick=\"var a=this.getAttribute('data-action');if(a){var ed={};for(var i=0;i<this.attributes.length;i++){var x=this.attributes[i];if(x.name.indexOf('data-')===0)ed[x.name.slice(5)]=x.value}window.parent.postMessage({type:'ntml-action',action:a,eventData:ed},'*')}\""
                    .to_string()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            write!(
                out,
                "<button{} type=\"button\"{}{} style=\"{}\"{}>",
                attrs, data_attrs, onclick, style, disabled_attr
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
            let cls = compute_class(img.id.as_deref(), img.style.as_ref(), patches);
            let attrs = build_attrs(img.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(lnk.id.as_deref(), lnk.style.as_ref(), patches);
            let attrs = build_attrs(lnk.id.as_deref(), cls.as_deref());
            write!(
                out,
                "<a{} href=\"{}\" data-ntml-url=\"{}\" data-ntml-target=\"{}\" style=\"color:inherit;cursor:pointer;text-decoration:none;{}\" onclick=\"event.preventDefault();var u=this.getAttribute('data-ntml-url');var t=this.getAttribute('data-ntml-target');if(u)window.parent.postMessage({{type:'ntml-navigate',url:u,target:t||'same'}},'*')\">",
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
            let cls = compute_class(r.id.as_deref(), r.style.as_ref(), patches);
            let attrs = build_attrs(r.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(col.id.as_deref(), col.style.as_ref(), patches);
            let attrs = build_attrs(col.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(g.id.as_deref(), g.style.as_ref(), patches);
            let attrs = build_attrs(g.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(s.id.as_deref(), s.style.as_ref(), patches);
            let attrs = build_attrs(s.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(d.id.as_deref(), d.style.as_ref(), patches);
            let attrs = build_attrs(d.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(pb.id.as_deref(), pb.style.as_ref(), patches);
            let attrs = build_attrs(pb.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(b.id.as_deref(), b.style.as_ref(), patches);
            let attrs = build_attrs(b.id.as_deref(), cls.as_deref());
            write!(
                out,
                "<span{} style=\"display:inline-block;padding:2px 8px;border-radius:4px;font-size:12px;{}\">{}</span>",
                attrs,
                style,
                escape_html(text)
            )?;
        }
        Component::Input(inp) => {
            let disabled = inp
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.disabled)
                .or(inp.disabled)
                .unwrap_or(false);
            let style = inp
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(inp.id.as_deref(), inp.style.as_ref(), patches);
            let attrs = build_attrs(inp.id.as_deref(), cls.as_deref());
            let ph = inp
                .placeholder
                .as_ref()
                .map(|s| escape_html(s))
                .unwrap_or_else(|| "".to_string());
            let val = inp
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.input_value.as_ref())
                .map(|s| escape_html(s))
                .or_else(|| inp.value.as_ref().map(|s| escape_html(s)))
                .unwrap_or_else(|| "".to_string());
            let input_type = inp
                .input_type
                .as_ref()
                .map(|t| match t {
                    nulltrace_ntml::components::InputType::Text => "text",
                    nulltrace_ntml::components::InputType::Password => "password",
                    nulltrace_ntml::components::InputType::Number => "number",
                })
                .unwrap_or("text");
            let onchange_attr = if !disabled {
                inp.onchange
                    .as_ref()
                    .filter(|a| !a.is_empty() && !a.contains(':'))
                    .map(|a| {
                        format!(
                            " onchange=\"window.parent.postMessage({{type:'ntml-action',action:'{}'}},'*')\"",
                            escape_html(a)
                        )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            let maxlength_attr = inp
                .max_length
                .map(|n| format!(" maxlength=\"{}\"", n))
                .unwrap_or_default();
            write!(
                out,
                "<input{} type=\"{}\" name=\"{}\" placeholder=\"{}\" value=\"{}\" style=\"{}\"{}{}{}>",
                attrs,
                input_type,
                escape_html(&inp.name),
                ph,
                val,
                style,
                onchange_attr,
                disabled_attr,
                maxlength_attr
            )?;
        }
        Component::Checkbox(cb) => {
            let disabled = cb
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.disabled)
                .or(cb.disabled)
                .unwrap_or(false);
            let style = cb
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(cb.id.as_deref(), cb.style.as_ref(), patches);
            let attrs = build_attrs(cb.id.as_deref(), cls.as_deref());
            let checked = cb.checked.unwrap_or(false);
            let onchange_attr = if !disabled {
                cb.onchange
                    .as_ref()
                    .filter(|a| !a.is_empty() && !a.contains(':'))
                    .map(|a| {
                        format!(
                            " onchange=\"window.parent.postMessage({{type:'ntml-action',action:'{}'}},'*')\"",
                            escape_html(a)
                        )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            write!(
                out,
                "<label{} style=\"{}\"><input type=\"checkbox\" name=\"{}\" {}{}{}> {}</label>",
                attrs,
                style,
                escape_html(&cb.name),
                if checked { "checked " } else { "" },
                onchange_attr,
                disabled_attr,
                escape_html(cb.label.as_deref().unwrap_or(""))
            )?;
        }
        Component::Radio(r) => {
            let disabled = r
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.disabled)
                .or(r.disabled)
                .unwrap_or(false);
            let style = r
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(r.id.as_deref(), r.style.as_ref(), patches);
            let attrs = build_attrs(r.id.as_deref(), cls.as_deref());
            let checked = r.checked.unwrap_or(false);
            let onchange_attr = if !disabled {
                r.onchange
                    .as_ref()
                    .filter(|a| !a.is_empty() && !a.contains(':'))
                    .map(|a| {
                        format!(
                            " onchange=\"window.parent.postMessage({{type:'ntml-action',action:'{}'}},'*')\"",
                            escape_html(a)
                        )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            write!(
                out,
                "<label{} style=\"{}\"><input type=\"radio\" name=\"{}\" value=\"{}\" {}{}{}> {}</label>",
                attrs,
                style,
                escape_html(&r.name),
                escape_html(&r.value),
                if checked { "checked " } else { "" },
                onchange_attr,
                disabled_attr,
                escape_html(r.label.as_deref().unwrap_or(""))
            )?;
        }
        Component::Select(sel) => {
            let disabled = sel
                .id
                .as_ref()
                .and_then(|id| patches.get(id))
                .and_then(|ov| ov.disabled)
                .or(sel.disabled)
                .unwrap_or(false);
            let style = sel
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(sel.id.as_deref(), sel.style.as_ref(), patches);
            let attrs = build_attrs(sel.id.as_deref(), cls.as_deref());
            let onchange_attr = if !disabled {
                sel.onchange
                    .as_ref()
                    .filter(|a| !a.is_empty() && !a.contains(':'))
                    .map(|a| {
                        format!(
                            " onchange=\"window.parent.postMessage({{type:'ntml-action',action:'{}'}},'*')\"",
                            escape_html(a)
                        )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let disabled_attr = if disabled { " disabled" } else { "" };
            let val = sel.value.as_deref().unwrap_or("");
            write!(out, "<select{} name=\"{}\" style=\"{}\"{}{}>", attrs, escape_html(&sel.name), style, onchange_attr, disabled_attr)?;
            for opt in &sel.options {
                let sel_attr = if opt.value == val { " selected" } else { "" };
                write!(
                    out,
                    "<option value=\"{}\"{}>{}</option>",
                    escape_html(&opt.value),
                    sel_attr,
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
            let cls = compute_class(ic.id.as_deref(), ic.style.as_ref(), patches);
            let attrs = build_attrs(ic.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(co.id.as_deref(), co.style.as_ref(), patches);
            let attrs = build_attrs(co.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(m.id.as_deref(), m.style.as_ref(), patches);
            let attrs = build_attrs(m.id.as_deref(), cls.as_deref());
            let inner = markdown_to_sanitized_html(m.content.as_deref().unwrap_or(""));
            write!(out, "<div{} class=\"ntml-markdown\" style=\"{}\">{}</div>", attrs, style_css, inner)?;
        }
        Component::List(list) => {
            let style_css = list
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(list.id.as_deref(), list.style.as_ref(), patches);
            let attrs = build_attrs(list.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(li.id.as_deref(), li.style.as_ref(), patches);
            let attrs = build_attrs(li.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(h.id.as_deref(), h.style.as_ref(), patches);
            let attrs = build_attrs(h.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(t.id.as_deref(), t.style.as_ref(), patches);
            let attrs = build_attrs(t.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(bq.id.as_deref(), bq.style.as_ref(), patches);
            let attrs = build_attrs(bq.id.as_deref(), cls.as_deref());
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
            let cls = compute_class(pre.id.as_deref(), pre.style.as_ref(), patches);
            let attrs = build_attrs(pre.id.as_deref(), cls.as_deref());
            write!(out, "<pre{} style=\"{}\">{}</pre>", attrs, style_css, escape_html(&pre.text))?;
        }
        Component::Details(d) => {
            let style_css = d
                .style
                .as_ref()
                .map(style_to_css)
                .unwrap_or_default();
            let cls = compute_class(d.id.as_deref(), d.style.as_ref(), patches);
            let attrs = build_attrs(d.id.as_deref(), cls.as_deref());
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
