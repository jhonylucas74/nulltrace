use super::colors;
use super::spacing;

/// A single resolved CSS rule.
#[derive(Debug, Clone)]
pub struct CssRule {
    /// Full CSS selector, e.g. `.flex`, `.w-1\/2`, `.space-x-4 > * + *`
    pub selector: String,
    /// CSS declarations, e.g. `[("display", "flex")]`
    pub declarations: Vec<(String, String)>,
    /// Optional wrapping media query, e.g. `@media (min-width: 768px)` (Phase 7)
    pub media_query: Option<String>,
    /// Optional @keyframes block to emit before rules (Phase 12: animate-spin, etc.)
    pub keyframes: Option<String>,
}

impl CssRule {
    fn new(class: &str, props: &[(&str, &str)]) -> Self {
        Self {
            selector: format!(".{}", escape_selector(class)),
            declarations: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            media_query: None,
            keyframes: None,
        }
    }

    fn dynamic(class: &str, props: Vec<(String, String)>) -> Self {
        Self {
            selector: format!(".{}", escape_selector(class)),
            declarations: props,
            media_query: None,
            keyframes: None,
        }
    }

    fn with_compound_selector(selector: String, props: Vec<(String, String)>) -> Self {
        Self {
            selector,
            declarations: props,
            media_query: None,
            keyframes: None,
        }
    }

    fn with_keyframes(mut self, kf: impl Into<String>) -> Self {
        self.keyframes = Some(kf.into());
        self
    }
}

/// Escapes CSS special characters in a class name so it can be used in a selector.
fn escape_selector(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 4);
    for ch in class.chars() {
        match ch {
            '.' | '/' | '[' | ']' | '(' | ')' | '%' | '#' | ':' | '@' | '!' | ',' | '~'
            | '^' | '$' | '&' | '+' | '=' | '<' | '>' | '|' | '\'' | '"' | ';' | '{'
            | '}' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ─── Public resolver ────────────────────────────────────────────────────────

/// Resolves a single Tailwind utility class string to a CSS rule.
///
/// Returns `None` if the class is not recognised (no rule emitted).
/// Handles variant prefixes (hover:, sm:, dark:, etc.) by resolving the base
/// class and applying the variant transformation.
pub fn resolve_class(class: &str) -> Option<CssRule> {
    if let Some((variant, base)) = super::variants::parse_variant(class) {
        if let Some(rule) = resolve_class_inner(base) {
            return Some(super::variants::apply_variant(rule, variant, class));
        }
        return None;
    }
    resolve_class_inner(class)
}

/// Inner resolver without variant handling. Used by resolve_class.
fn resolve_class_inner(class: &str) -> Option<CssRule> {
    resolve_display(class)
        .or_else(|| resolve_position(class))
        .or_else(|| resolve_inset(class))
        .or_else(|| resolve_z_index(class))
        .or_else(|| resolve_float_clear(class))
        .or_else(|| resolve_overflow(class))
        .or_else(|| resolve_visibility(class))
        .or_else(|| resolve_object(class))
        .or_else(|| resolve_aspect(class))
        .or_else(|| resolve_columns(class))
        .or_else(|| resolve_break(class))
        .or_else(|| resolve_container(class))
        .or_else(|| resolve_box_misc(class))
        .or_else(|| resolve_grid(class))
        .or_else(|| resolve_flex(class))
        .or_else(|| resolve_alignment(class))
        .or_else(|| resolve_spacing(class))
        .or_else(|| resolve_sizing(class))
        .or_else(|| resolve_font_family(class))
        .or_else(|| resolve_font_size(class))
        .or_else(|| resolve_font_weight(class))
        .or_else(|| resolve_font_style(class))
        .or_else(|| resolve_font_smoothing(class))
        .or_else(|| resolve_letter_spacing(class))
        .or_else(|| resolve_line_height(class))
        .or_else(|| resolve_text_align(class))
        .or_else(|| resolve_text_decoration(class))
        .or_else(|| resolve_text_transform(class))
        .or_else(|| resolve_text_overflow(class))
        .or_else(|| resolve_text_wrap(class))
        .or_else(|| resolve_text_indent(class))
        .or_else(|| resolve_vertical_align(class))
        .or_else(|| resolve_whitespace(class))
        .or_else(|| resolve_word_break(class))
        .or_else(|| resolve_line_clamp(class))
        .or_else(|| resolve_list_style(class))
        .or_else(|| resolve_font_variant_numeric(class))
        .or_else(|| resolve_border_width(class))
        .or_else(|| resolve_border_color(class))
        .or_else(|| resolve_border_style(class))
        .or_else(|| resolve_border_radius(class))
        .or_else(|| resolve_outline(class))
        .or_else(|| resolve_ring(class))
        .or_else(|| resolve_divide(class))
        .or_else(|| resolve_box_shadow(class))
        .or_else(|| resolve_opacity(class))
        .or_else(|| resolve_mix_blend_mode(class))
        .or_else(|| resolve_bg_blend_mode(class))
        .or_else(|| resolve_filter(class))
        .or_else(|| resolve_backdrop_filter(class))
        .or_else(|| resolve_transition(class))
        .or_else(|| resolve_duration(class))
        .or_else(|| resolve_ease(class))
        .or_else(|| resolve_delay(class))
        .or_else(|| resolve_animation(class))
        .or_else(|| resolve_scale(class))
        .or_else(|| resolve_rotate(class))
        .or_else(|| resolve_translate(class))
        .or_else(|| resolve_skew(class))
        .or_else(|| resolve_transform_origin(class))
        .or_else(|| resolve_perspective(class))
        .or_else(|| resolve_cursor(class))
        .or_else(|| resolve_pointer_events(class))
        .or_else(|| resolve_resize(class))
        .or_else(|| resolve_user_select(class))
        .or_else(|| resolve_scroll(class))
        .or_else(|| resolve_touch_action(class))
        .or_else(|| resolve_will_change(class))
        .or_else(|| resolve_appearance(class))
        .or_else(|| resolve_caret_color(class))
        .or_else(|| resolve_accent_color(class))
        .or_else(|| resolve_table(class))
        .or_else(|| resolve_content(class))
        .or_else(|| resolve_overscroll(class))
        .or_else(|| resolve_bg_image(class))
        .or_else(|| resolve_bg_size(class))
        .or_else(|| resolve_bg_position(class))
        .or_else(|| resolve_bg_repeat(class))
        .or_else(|| resolve_bg_attachment(class))
        .or_else(|| resolve_bg_clip(class))
        .or_else(|| resolve_bg_origin(class))
        .or_else(|| resolve_gradient_stops(class))
        .or_else(|| resolve_bg_color(class))
        .or_else(|| resolve_text_color(class))
}

// ─── Position ───────────────────────────────────────────────────────────────

fn resolve_position(class: &str) -> Option<CssRule> {
    let pos = match class {
        "static"   => "static",
        "fixed"    => "fixed",
        "absolute" => "absolute",
        "relative" => "relative",
        "sticky"   => "sticky",
        _ => return None,
    };
    Some(CssRule::new(class, &[("position", pos)]))
}

// ─── Inset (top / right / bottom / left) ────────────────────────────────────

fn resolve_inset(class: &str) -> Option<CssRule> {
    // Optional negative prefix for -top-*, -inset-*, etc.
    let (neg, rest) = if let Some(r) = class.strip_prefix('-') {
        (true, r)
    } else {
        (false, class)
    };

    let (props, val_str): (&[&str], &str) =
        if let Some(v) = rest.strip_prefix("inset-x-") {
            (&["left", "right"], v)
        } else if let Some(v) = rest.strip_prefix("inset-y-") {
            (&["top", "bottom"], v)
        } else if let Some(v) = rest.strip_prefix("inset-") {
            (&["top", "right", "bottom", "left"], v)
        } else if let Some(v) = rest.strip_prefix("top-") {
            (&["top"], v)
        } else if let Some(v) = rest.strip_prefix("right-") {
            (&["right"], v)
        } else if let Some(v) = rest.strip_prefix("bottom-") {
            (&["bottom"], v)
        } else if let Some(v) = rest.strip_prefix("left-") {
            (&["left"], v)
        } else {
            return None;
        };

    // resolve_size_value handles: auto, px, full (100%), fractions, arbitrary, spacing scale
    let base_val = resolve_size_value(val_str, SizeContext::Width)?;
    let css_val = if neg && base_val != "0" {
        format!("-{}", base_val)
    } else {
        base_val
    };

    let decls = props
        .iter()
        .map(|p| (p.to_string(), css_val.clone()))
        .collect();
    Some(CssRule::dynamic(class, decls))
}

// ─── Z-Index ─────────────────────────────────────────────────────────────────

fn resolve_z_index(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("z-")?;
    let css_val = match val {
        "auto" => "auto".to_string(),
        _ if val.starts_with('[') && val.ends_with(']') => val[1..val.len() - 1].to_string(),
        _ => {
            // Accept any integer (positive or negative)
            val.parse::<i32>().ok()?;
            val.to_string()
        }
    };
    Some(CssRule::dynamic(class, vec![("z-index".into(), css_val)]))
}

// ─── Float & Clear ───────────────────────────────────────────────────────────

fn resolve_float_clear(class: &str) -> Option<CssRule> {
    if let Some(val) = class.strip_prefix("float-") {
        let css_val = match val {
            "start" => "inline-start",
            "end"   => "inline-end",
            "left"  => "left",
            "right" => "right",
            "none"  => "none",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("float", css_val)]));
    }
    if let Some(val) = class.strip_prefix("clear-") {
        let css_val = match val {
            "start" => "inline-start",
            "end"   => "inline-end",
            "left"  => "left",
            "right" => "right",
            "both"  => "both",
            "none"  => "none",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("clear", css_val)]));
    }
    None
}

// ─── Overflow ────────────────────────────────────────────────────────────────

fn resolve_overflow(class: &str) -> Option<CssRule> {
    let (prop, val_str) = if let Some(v) = class.strip_prefix("overflow-x-") {
        ("overflow-x", v)
    } else if let Some(v) = class.strip_prefix("overflow-y-") {
        ("overflow-y", v)
    } else if let Some(v) = class.strip_prefix("overflow-") {
        ("overflow", v)
    } else {
        return None;
    };

    let css_val = match val_str {
        "auto"    => "auto",
        "hidden"  => "hidden",
        "clip"    => "clip",
        "visible" => "visible",
        "scroll"  => "scroll",
        _ => return None,
    };

    Some(CssRule::new(class, &[(prop, css_val)]))
}

// ─── Visibility ──────────────────────────────────────────────────────────────

fn resolve_visibility(class: &str) -> Option<CssRule> {
    let val = match class {
        "visible"   => "visible",
        "invisible" => "hidden",
        "collapse"  => "collapse",
        _ => return None,
    };
    Some(CssRule::new(class, &[("visibility", val)]))
}

// ─── Object Fit & Position ───────────────────────────────────────────────────

fn resolve_object(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("object-")?;

    // object-fit
    let fit = match val {
        "contain"    => Some("contain"),
        "cover"      => Some("cover"),
        "fill"       => Some("fill"),
        "none"       => Some("none"),
        "scale-down" => Some("scale-down"),
        _ => None,
    };
    if let Some(f) = fit {
        return Some(CssRule::new(class, &[("object-fit", f)]));
    }

    // object-position
    let pos = match val {
        "center"       => "center",
        "top"          => "top",
        "bottom"       => "bottom",
        "left"         => "left",
        "right"        => "right",
        "left-top"     => "left top",
        "left-bottom"  => "left bottom",
        "right-top"    => "right top",
        "right-bottom" => "right bottom",
        _ => return None,
    };
    Some(CssRule::new(class, &[("object-position", pos)]))
}

// ─── Aspect Ratio ────────────────────────────────────────────────────────────

fn resolve_aspect(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("aspect-")?;
    let css_val = match val {
        "auto"   => "auto".to_string(),
        "square" => "1 / 1".to_string(),
        "video"  => "16 / 9".to_string(),
        _ if val.starts_with('[') && val.ends_with(']') => {
            val[1..val.len() - 1].replace('_', " ")
        }
        _ => return None,
    };
    Some(CssRule::dynamic(class, vec![("aspect-ratio".into(), css_val)]))
}

// ─── Columns ─────────────────────────────────────────────────────────────────

fn resolve_columns(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("columns-")?;
    let css_val = match val {
        "auto" => "auto".to_string(),
        // Container size scale
        "3xs" => "16rem".to_string(),
        "2xs" => "18rem".to_string(),
        "xs"  => "20rem".to_string(),
        "sm"  => "24rem".to_string(),
        "md"  => "28rem".to_string(),
        "lg"  => "32rem".to_string(),
        "xl"  => "36rem".to_string(),
        "2xl" => "42rem".to_string(),
        "3xl" => "48rem".to_string(),
        "4xl" => "56rem".to_string(),
        "5xl" => "64rem".to_string(),
        "6xl" => "72rem".to_string(),
        "7xl" => "80rem".to_string(),
        _ => {
            let n: u32 = val.parse().ok()?;
            if n < 1 || n > 12 { return None; }
            n.to_string()
        }
    };
    Some(CssRule::dynamic(class, vec![("columns".into(), css_val)]))
}

// ─── Break ───────────────────────────────────────────────────────────────────

fn resolve_break(class: &str) -> Option<CssRule> {
    let (prop, val_str) = if let Some(v) = class.strip_prefix("break-before-") {
        ("break-before", v)
    } else if let Some(v) = class.strip_prefix("break-inside-") {
        ("break-inside", v)
    } else if let Some(v) = class.strip_prefix("break-after-") {
        ("break-after", v)
    } else {
        return None;
    };

    let css_val = match val_str {
        "auto"         => "auto",
        "avoid"        => "avoid",
        "all"          => "all",
        "page"         => "page",
        "column"       => "column",
        "avoid-page"   => "avoid-page",
        "avoid-column" => "avoid-column",
        _ => return None,
    };

    Some(CssRule::new(class, &[(prop, css_val)]))
}

// ─── Container ───────────────────────────────────────────────────────────────
// Base rule only. Responsive max-width breakpoints are emitted in Phase 7
// when media-query variant support is added.

fn resolve_container(class: &str) -> Option<CssRule> {
    if class != "container" {
        return None;
    }
    Some(CssRule::dynamic(
        class,
        vec![
            ("width".into(), "100%".into()),
            ("margin-left".into(), "auto".into()),
            ("margin-right".into(), "auto".into()),
        ],
    ))
}

// ─── Box Decoration Break / Box Sizing / Isolation ───────────────────────────

fn resolve_box_misc(class: &str) -> Option<CssRule> {
    match class {
        // Box Decoration Break
        "box-decoration-clone" => Some(CssRule::new(class, &[
            ("-webkit-box-decoration-break", "clone"),
            ("box-decoration-break", "clone"),
        ])),
        "box-decoration-slice" => Some(CssRule::new(class, &[
            ("-webkit-box-decoration-break", "slice"),
            ("box-decoration-break", "slice"),
        ])),
        // Box Sizing
        "box-border"   => Some(CssRule::new(class, &[("box-sizing", "border-box")])),
        "box-content"  => Some(CssRule::new(class, &[("box-sizing", "content-box")])),
        // Isolation
        "isolate"          => Some(CssRule::new(class, &[("isolation", "isolate")])),
        "isolation-auto"   => Some(CssRule::new(class, &[("isolation", "auto")])),
        _ => None,
    }
}

// ─── Grid ────────────────────────────────────────────────────────────────────

fn resolve_grid(class: &str) -> Option<CssRule> {
    // ── 3.1 grid-template-columns ──
    if let Some(val) = class.strip_prefix("grid-cols-") {
        let css_val = match val {
            "none"    => "none".to_string(),
            "subgrid" => "subgrid".to_string(),
            _ if val.starts_with('[') && val.ends_with(']') => {
                val[1..val.len() - 1].replace('_', " ")
            }
            _ => {
                let n: u32 = val.parse().ok()?;
                if n < 1 || n > 12 { return None; }
                format!("repeat({}, minmax(0, 1fr))", n)
            }
        };
        return Some(CssRule::dynamic(class, vec![("grid-template-columns".into(), css_val)]));
    }

    // ── 3.2 grid-template-rows ──
    if let Some(val) = class.strip_prefix("grid-rows-") {
        let css_val = match val {
            "none"    => "none".to_string(),
            "subgrid" => "subgrid".to_string(),
            _ if val.starts_with('[') && val.ends_with(']') => {
                val[1..val.len() - 1].replace('_', " ")
            }
            _ => {
                let n: u32 = val.parse().ok()?;
                if n < 1 || n > 12 { return None; }
                format!("repeat({}, minmax(0, 1fr))", n)
            }
        };
        return Some(CssRule::dynamic(class, vec![("grid-template-rows".into(), css_val)]));
    }

    // ── 3.3 column span / start / end ──
    if class == "col-auto" {
        return Some(CssRule::new(class, &[("grid-column", "auto")]));
    }
    if let Some(val) = class.strip_prefix("col-span-") {
        let css_val = match val {
            "full" => "1 / -1".to_string(),
            _ => {
                let n: u32 = val.parse().ok()?;
                if n < 1 || n > 12 { return None; }
                format!("span {} / span {}", n, n)
            }
        };
        return Some(CssRule::dynamic(class, vec![("grid-column".into(), css_val)]));
    }
    if let Some(val) = class.strip_prefix("col-start-") {
        let css_val = match val {
            "auto" => "auto".to_string(),
            _ => { val.parse::<u32>().ok()?; val.to_string() }
        };
        return Some(CssRule::dynamic(class, vec![("grid-column-start".into(), css_val)]));
    }
    if let Some(val) = class.strip_prefix("col-end-") {
        let css_val = match val {
            "auto" => "auto".to_string(),
            _ => { val.parse::<u32>().ok()?; val.to_string() }
        };
        return Some(CssRule::dynamic(class, vec![("grid-column-end".into(), css_val)]));
    }

    // ── 3.4 row span / start / end ──
    if class == "row-auto" {
        return Some(CssRule::new(class, &[("grid-row", "auto")]));
    }
    if let Some(val) = class.strip_prefix("row-span-") {
        let css_val = match val {
            "full" => "1 / -1".to_string(),
            _ => {
                let n: u32 = val.parse().ok()?;
                if n < 1 || n > 12 { return None; }
                format!("span {} / span {}", n, n)
            }
        };
        return Some(CssRule::dynamic(class, vec![("grid-row".into(), css_val)]));
    }
    if let Some(val) = class.strip_prefix("row-start-") {
        let css_val = match val {
            "auto" => "auto".to_string(),
            _ => { val.parse::<u32>().ok()?; val.to_string() }
        };
        return Some(CssRule::dynamic(class, vec![("grid-row-start".into(), css_val)]));
    }
    if let Some(val) = class.strip_prefix("row-end-") {
        let css_val = match val {
            "auto" => "auto".to_string(),
            _ => { val.parse::<u32>().ok()?; val.to_string() }
        };
        return Some(CssRule::dynamic(class, vec![("grid-row-end".into(), css_val)]));
    }

    // ── 3.5 grid-auto-flow ──
    if let Some(val) = class.strip_prefix("grid-flow-") {
        let css_val = match val {
            "row"       => "row",
            "col"       => "column",
            "dense"     => "dense",
            "row-dense" => "row dense",
            "col-dense" => "column dense",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("grid-auto-flow", css_val)]));
    }

    // ── 3.6 auto-cols / auto-rows ──
    if let Some(val) = class.strip_prefix("auto-cols-") {
        let css_val = match val {
            "auto" => "auto",
            "min"  => "min-content",
            "max"  => "max-content",
            "fr"   => "minmax(0, 1fr)",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("grid-auto-columns", css_val)]));
    }
    if let Some(val) = class.strip_prefix("auto-rows-") {
        let css_val = match val {
            "auto" => "auto",
            "min"  => "min-content",
            "max"  => "max-content",
            "fr"   => "minmax(0, 1fr)",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("grid-auto-rows", css_val)]));
    }

    None
}

// ─── Display ────────────────────────────────────────────────────────────────

fn resolve_display(class: &str) -> Option<CssRule> {
    let display = match class {
        "block"              => "block",
        "inline-block"       => "inline-block",
        "inline"             => "inline",
        "flex"               => "flex",
        "inline-flex"        => "inline-flex",
        "grid"               => "grid",
        "inline-grid"        => "inline-grid",
        "contents"           => "contents",
        "flow-root"          => "flow-root",
        "table"              => "table",
        "inline-table"       => "inline-table",
        "table-caption"      => "table-caption",
        "table-cell"         => "table-cell",
        "table-column"       => "table-column",
        "table-column-group" => "table-column-group",
        "table-footer-group" => "table-footer-group",
        "table-header-group" => "table-header-group",
        "table-row-group"    => "table-row-group",
        "table-row"          => "table-row",
        "list-item"          => "list-item",
        "hidden"             => "none",
        _ => return None,
    };
    Some(CssRule::new(class, &[("display", display)]))
}

// ─── Flexbox ────────────────────────────────────────────────────────────────

fn resolve_flex(class: &str) -> Option<CssRule> {
    // Static flex classes
    let rule = match class {
        // Direction
        "flex-row"          => CssRule::new(class, &[("flex-direction", "row")]),
        "flex-row-reverse"  => CssRule::new(class, &[("flex-direction", "row-reverse")]),
        "flex-col"          => CssRule::new(class, &[("flex-direction", "column")]),
        "flex-col-reverse"  => CssRule::new(class, &[("flex-direction", "column-reverse")]),
        // Wrap
        "flex-wrap"         => CssRule::new(class, &[("flex-wrap", "wrap")]),
        "flex-wrap-reverse" => CssRule::new(class, &[("flex-wrap", "wrap-reverse")]),
        "flex-nowrap"       => CssRule::new(class, &[("flex-wrap", "nowrap")]),
        // Flex shorthand
        "flex-1"            => CssRule::new(class, &[("flex", "1 1 0%")]),
        "flex-auto"         => CssRule::new(class, &[("flex", "1 1 auto")]),
        "flex-initial"      => CssRule::new(class, &[("flex", "0 1 auto")]),
        "flex-none"         => CssRule::new(class, &[("flex", "none")]),
        // Grow
        "grow"              => CssRule::new(class, &[("flex-grow", "1")]),
        "grow-0"            => CssRule::new(class, &[("flex-grow", "0")]),
        // Shrink
        "shrink"            => CssRule::new(class, &[("flex-shrink", "1")]),
        "shrink-0"          => CssRule::new(class, &[("flex-shrink", "0")]),
        // Order keywords
        "order-first"       => CssRule::new(class, &[("order", "-9999")]),
        "order-last"        => CssRule::new(class, &[("order", "9999")]),
        "order-none"        => CssRule::new(class, &[("order", "0")]),
        _ => {
            // Dynamic: order-{n}
            if let Some(n_str) = class.strip_prefix("order-") {
                if let Ok(n) = n_str.parse::<i32>() {
                    return Some(CssRule::dynamic(class, vec![("order".into(), n.to_string())]));
                }
            }
            // Dynamic: basis-{value}
            if let Some(val) = class.strip_prefix("basis-") {
                if let Some(size) = resolve_size_value(val, SizeContext::Width) {
                    return Some(CssRule::dynamic(class, vec![("flex-basis".into(), size)]));
                }
            }
            // Dynamic: grow-{n}
            if let Some(n_str) = class.strip_prefix("grow-") {
                if let Ok(n) = n_str.parse::<u32>() {
                    return Some(CssRule::dynamic(class, vec![("flex-grow".into(), n.to_string())]));
                }
            }
            // Dynamic: shrink-{n}
            if let Some(n_str) = class.strip_prefix("shrink-") {
                if let Ok(n) = n_str.parse::<u32>() {
                    return Some(CssRule::dynamic(class, vec![("flex-shrink".into(), n.to_string())]));
                }
            }
            return None;
        }
    };
    Some(rule)
}

// ─── Alignment & Gap ────────────────────────────────────────────────────────

fn resolve_alignment(class: &str) -> Option<CssRule> {
    // Static alignment classes
    let rule = match class {
        // justify-content
        "justify-normal"         => CssRule::new(class, &[("justify-content", "normal")]),
        "justify-start"          => CssRule::new(class, &[("justify-content", "flex-start")]),
        "justify-end"            => CssRule::new(class, &[("justify-content", "flex-end")]),
        "justify-center"         => CssRule::new(class, &[("justify-content", "center")]),
        "justify-between"        => CssRule::new(class, &[("justify-content", "space-between")]),
        "justify-around"         => CssRule::new(class, &[("justify-content", "space-around")]),
        "justify-evenly"         => CssRule::new(class, &[("justify-content", "space-evenly")]),
        "justify-stretch"        => CssRule::new(class, &[("justify-content", "stretch")]),
        // align-items
        "items-start"            => CssRule::new(class, &[("align-items", "flex-start")]),
        "items-end"              => CssRule::new(class, &[("align-items", "flex-end")]),
        "items-center"           => CssRule::new(class, &[("align-items", "center")]),
        "items-baseline"         => CssRule::new(class, &[("align-items", "baseline")]),
        "items-stretch"          => CssRule::new(class, &[("align-items", "stretch")]),
        // align-self
        "self-auto"              => CssRule::new(class, &[("align-self", "auto")]),
        "self-start"             => CssRule::new(class, &[("align-self", "flex-start")]),
        "self-end"               => CssRule::new(class, &[("align-self", "flex-end")]),
        "self-center"            => CssRule::new(class, &[("align-self", "center")]),
        "self-stretch"           => CssRule::new(class, &[("align-self", "stretch")]),
        "self-baseline"          => CssRule::new(class, &[("align-self", "baseline")]),
        // align-content
        "content-normal"         => CssRule::new(class, &[("align-content", "normal")]),
        "content-start"          => CssRule::new(class, &[("align-content", "flex-start")]),
        "content-end"            => CssRule::new(class, &[("align-content", "flex-end")]),
        "content-center"         => CssRule::new(class, &[("align-content", "center")]),
        "content-between"        => CssRule::new(class, &[("align-content", "space-between")]),
        "content-around"         => CssRule::new(class, &[("align-content", "space-around")]),
        "content-evenly"         => CssRule::new(class, &[("align-content", "space-evenly")]),
        "content-baseline"       => CssRule::new(class, &[("align-content", "baseline")]),
        "content-stretch"        => CssRule::new(class, &[("align-content", "stretch")]),
        // place-content
        "place-content-center"   => CssRule::new(class, &[("place-content", "center")]),
        "place-content-start"    => CssRule::new(class, &[("place-content", "start")]),
        "place-content-end"      => CssRule::new(class, &[("place-content", "end")]),
        "place-content-between"  => CssRule::new(class, &[("place-content", "space-between")]),
        "place-content-around"   => CssRule::new(class, &[("place-content", "space-around")]),
        "place-content-evenly"   => CssRule::new(class, &[("place-content", "space-evenly")]),
        "place-content-baseline" => CssRule::new(class, &[("place-content", "baseline")]),
        "place-content-stretch"  => CssRule::new(class, &[("place-content", "stretch")]),
        // place-items
        "place-items-start"      => CssRule::new(class, &[("place-items", "start")]),
        "place-items-end"        => CssRule::new(class, &[("place-items", "end")]),
        "place-items-center"     => CssRule::new(class, &[("place-items", "center")]),
        "place-items-baseline"   => CssRule::new(class, &[("place-items", "baseline")]),
        "place-items-stretch"    => CssRule::new(class, &[("place-items", "stretch")]),
        // place-self
        "place-self-auto"        => CssRule::new(class, &[("place-self", "auto")]),
        "place-self-start"       => CssRule::new(class, &[("place-self", "start")]),
        "place-self-end"         => CssRule::new(class, &[("place-self", "end")]),
        "place-self-center"      => CssRule::new(class, &[("place-self", "center")]),
        "place-self-stretch"     => CssRule::new(class, &[("place-self", "stretch")]),
        _ => {
            // justify-items-{value}
            if let Some(val) = class.strip_prefix("justify-items-") {
                let cv = match val {
                    "start" | "end" | "center" | "stretch" | "normal" => val,
                    _ => return None,
                };
                return Some(CssRule::new(class, &[("justify-items", cv)]));
            }
            // justify-self-{value}
            if let Some(val) = class.strip_prefix("justify-self-") {
                let cv = match val {
                    "auto" | "start" | "end" | "center" | "stretch" => val,
                    _ => return None,
                };
                return Some(CssRule::new(class, &[("justify-self", cv)]));
            }
            // gap-x-{n}
            if let Some(val) = class.strip_prefix("gap-x-") {
                if let Some(sp) = resolve_spacing_or_arbitrary(val) {
                    return Some(CssRule::dynamic(class, vec![("column-gap".into(), sp)]));
                }
                return None;
            }
            // gap-y-{n}
            if let Some(val) = class.strip_prefix("gap-y-") {
                if let Some(sp) = resolve_spacing_or_arbitrary(val) {
                    return Some(CssRule::dynamic(class, vec![("row-gap".into(), sp)]));
                }
                return None;
            }
            // gap-{n}
            if let Some(val) = class.strip_prefix("gap-") {
                if let Some(sp) = resolve_spacing_or_arbitrary(val) {
                    return Some(CssRule::dynamic(class, vec![("gap".into(), sp)]));
                }
                return None;
            }
            // space-x-{n}  →  .space-x-{n} > * + *
            if let Some(val) = class.strip_prefix("space-x-") {
                if let Some(sp) = resolve_spacing_or_arbitrary(val) {
                    let sel = format!(".{} > * + *", escape_selector(class));
                    return Some(CssRule::with_compound_selector(
                        sel,
                        vec![("margin-left".into(), sp)],
                    ));
                }
                return None;
            }
            // space-y-{n}  →  .space-y-{n} > * + *
            if let Some(val) = class.strip_prefix("space-y-") {
                if let Some(sp) = resolve_spacing_or_arbitrary(val) {
                    let sel = format!(".{} > * + *", escape_selector(class));
                    return Some(CssRule::with_compound_selector(
                        sel,
                        vec![("margin-top".into(), sp)],
                    ));
                }
                return None;
            }
            return None;
        }
    };
    Some(rule)
}

// ─── Spacing (padding & margin) ─────────────────────────────────────────────

fn resolve_spacing(class: &str) -> Option<CssRule> {
    // Detect negative prefix (negative margin only)
    let (neg, rest) = if let Some(r) = class.strip_prefix('-') {
        (true, r)
    } else {
        (false, class)
    };

    // Match prefix → list of CSS properties to set
    let (props, val_str): (&[&str], &str) = if let Some(v) = rest.strip_prefix("p-") {
        (&["padding"], v)
    } else if let Some(v) = rest.strip_prefix("px-") {
        (&["padding-left", "padding-right"], v)
    } else if let Some(v) = rest.strip_prefix("py-") {
        (&["padding-top", "padding-bottom"], v)
    } else if let Some(v) = rest.strip_prefix("pt-") {
        (&["padding-top"], v)
    } else if let Some(v) = rest.strip_prefix("pr-") {
        (&["padding-right"], v)
    } else if let Some(v) = rest.strip_prefix("pb-") {
        (&["padding-bottom"], v)
    } else if let Some(v) = rest.strip_prefix("pl-") {
        (&["padding-left"], v)
    } else if let Some(v) = rest.strip_prefix("ps-") {
        (&["padding-inline-start"], v)
    } else if let Some(v) = rest.strip_prefix("pe-") {
        (&["padding-inline-end"], v)
    } else if let Some(v) = rest.strip_prefix("m-") {
        (&["margin"], v)
    } else if let Some(v) = rest.strip_prefix("mx-") {
        (&["margin-left", "margin-right"], v)
    } else if let Some(v) = rest.strip_prefix("my-") {
        (&["margin-top", "margin-bottom"], v)
    } else if let Some(v) = rest.strip_prefix("mt-") {
        (&["margin-top"], v)
    } else if let Some(v) = rest.strip_prefix("mr-") {
        (&["margin-right"], v)
    } else if let Some(v) = rest.strip_prefix("mb-") {
        (&["margin-bottom"], v)
    } else if let Some(v) = rest.strip_prefix("ml-") {
        (&["margin-left"], v)
    } else if let Some(v) = rest.strip_prefix("ms-") {
        (&["margin-inline-start"], v)
    } else if let Some(v) = rest.strip_prefix("me-") {
        (&["margin-inline-end"], v)
    } else {
        return None;
    };

    // Negative padding doesn't exist
    if neg && props.iter().any(|p| p.starts_with("padding")) {
        return None;
    }

    // "auto" (margins only)
    if val_str == "auto" {
        if props.iter().any(|p| p.starts_with("padding")) {
            return None;
        }
        let decls = props
            .iter()
            .map(|p| (p.to_string(), "auto".to_string()))
            .collect();
        return Some(CssRule::dynamic(class, decls));
    }

    // Resolve value (spacing scale or arbitrary)
    let base_val = resolve_spacing_or_arbitrary(val_str)?;
    let css_val = if neg && base_val != "0" {
        format!("-{}", base_val)
    } else {
        base_val
    };

    let decls = props
        .iter()
        .map(|p| (p.to_string(), css_val.clone()))
        .collect();
    Some(CssRule::dynamic(class, decls))
}

// ─── Sizing ─────────────────────────────────────────────────────────────────

fn resolve_sizing(class: &str) -> Option<CssRule> {
    // width
    if let Some(val) = class.strip_prefix("w-") {
        let css_val = width_viewport_special(val)
            .or_else(|| resolve_size_value(val, SizeContext::Width))?;
        return Some(CssRule::dynamic(class, vec![("width".into(), css_val)]));
    }
    // height
    if let Some(val) = class.strip_prefix("h-") {
        let css_val = height_viewport_special(val)
            .or_else(|| resolve_size_value(val, SizeContext::Height))?;
        return Some(CssRule::dynamic(class, vec![("height".into(), css_val)]));
    }
    // min-width
    if let Some(val) = class.strip_prefix("min-w-") {
        let css_val = match val {
            "0" => "0".to_string(),
            _ => resolve_size_value(val, SizeContext::Width)?,
        };
        return Some(CssRule::dynamic(class, vec![("min-width".into(), css_val)]));
    }
    // max-width
    if let Some(val) = class.strip_prefix("max-w-") {
        let css_val = match val {
            "none"       => "none".to_string(),
            "screen-sm"  => "640px".to_string(),
            "screen-md"  => "768px".to_string(),
            "screen-lg"  => "1024px".to_string(),
            "screen-xl"  => "1280px".to_string(),
            "screen-2xl" => "1536px".to_string(),
            _ => resolve_size_value(val, SizeContext::Width)?,
        };
        return Some(CssRule::dynamic(class, vec![("max-width".into(), css_val)]));
    }
    // min-height
    if let Some(val) = class.strip_prefix("min-h-") {
        let css_val = height_viewport_special(val)
            .or_else(|| resolve_size_value(val, SizeContext::Height))?;
        return Some(CssRule::dynamic(class, vec![("min-height".into(), css_val)]));
    }
    // max-height
    if let Some(val) = class.strip_prefix("max-h-") {
        let css_val = match val {
            "none" => "none".to_string(),
            _ => height_viewport_special(val)
                .or_else(|| resolve_size_value(val, SizeContext::Height))?,
        };
        return Some(CssRule::dynamic(class, vec![("max-height".into(), css_val)]));
    }
    // size (width + height simultaneously)
    if let Some(val) = class.strip_prefix("size-") {
        let css_val = resolve_size_value(val, SizeContext::Width)?;
        return Some(CssRule::dynamic(
            class,
            vec![("width".into(), css_val.clone()), ("height".into(), css_val)],
        ));
    }
    None
}

/// Width-specific viewport keyword overrides.
fn width_viewport_special(val: &str) -> Option<String> {
    Some(match val {
        "screen" => "100vw",
        "dvw"    => "100dvw",
        "svw"    => "100svw",
        "lvw"    => "100lvw",
        "dvh"    => "100dvh",
        "svh"    => "100svh",
        "lvh"    => "100lvh",
        _ => return None,
    }.to_string())
}

/// Height-specific viewport keyword overrides.
fn height_viewport_special(val: &str) -> Option<String> {
    Some(match val {
        "screen" => "100vh",
        "dvh"    => "100dvh",
        "svh"    => "100svh",
        "lvh"    => "100lvh",
        "dvw"    => "100dvw",
        "svw"    => "100svw",
        "lvw"    => "100lvw",
        _ => return None,
    }.to_string())
}

/// Resolves a size/dimension token to a CSS value.
///
/// Handles: spacing scale, special keywords (auto, full, min, max, fit, screen, prose),
/// container sizes (xs, sm … 7xl), fractions (1/2, 1/3 …), and arbitrary `[value]`.
#[derive(Clone, Copy)]
enum SizeContext {
    Width,
    Height,
}

fn resolve_size_value(val: &str, ctx: SizeContext) -> Option<String> {
    // Special keywords
    match val {
        "auto"  => return Some("auto".into()),
        "px"    => return Some("1px".into()),
        "full"  => return Some("100%".into()),
        "min"   => return Some("min-content".into()),
        "max"   => return Some("max-content".into()),
        "fit"   => return Some("fit-content".into()),
        "prose" => return Some("65ch".into()),
        // viewport shortcut handled by caller, but cover "screen" here as fallback
        "screen" => return Some(match ctx {
            SizeContext::Width  => "100vw",
            SizeContext::Height => "100vh",
        }.into()),
        // Container sizes
        "3xs" => return Some("16rem".into()),
        "2xs" => return Some("18rem".into()),
        "xs"  => return Some("20rem".into()),
        "sm"  => return Some("24rem".into()),
        "md"  => return Some("28rem".into()),
        "lg"  => return Some("32rem".into()),
        "xl"  => return Some("36rem".into()),
        "2xl" => return Some("42rem".into()),
        "3xl" => return Some("48rem".into()),
        "4xl" => return Some("56rem".into()),
        "5xl" => return Some("64rem".into()),
        "6xl" => return Some("72rem".into()),
        "7xl" => return Some("80rem".into()),
        _ => {}
    }

    // Fraction: "1/2", "2/3", etc.
    if let Some(slash) = val.find('/') {
        let num: f64 = val[..slash].parse().ok()?;
        let den: f64 = val[slash + 1..].parse().ok()?;
        if den == 0.0 {
            return None;
        }
        let pct = num / den * 100.0;
        let s = format!("{:.6}", pct);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        return Some(format!("{}%", trimmed));
    }

    // Arbitrary value: "[100px]", "[50%]", etc.
    if val.starts_with('[') && val.ends_with(']') {
        return Some(val[1..val.len() - 1].replace('_', " "));
    }

    // Spacing scale
    spacing::spacing_value(val)
}

// ─── Font Family (Typography 7.1) ───────────────────────────────────────────

/// Tailwind default font stacks (ui-sans-serif, ui-serif, ui-monospace).
const FONT_SANS: &str = "ui-sans-serif, system-ui, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Segoe UI Symbol\", \"Noto Color Emoji\"";
const FONT_SERIF: &str = "ui-serif, Georgia, Cambria, \"Times New Roman\", Times, serif";
const FONT_MONO: &str = "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace";

fn resolve_font_family(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("font-")?;

    // Built-in: font-sans, font-serif, font-mono
    let font_stack = match val {
        "sans" => FONT_SANS,
        "serif" => FONT_SERIF,
        "mono" => FONT_MONO,
        _ => {
            // Arbitrary: font-[Inter], font-[Open_Sans] (underscores → spaces)
            let inner = val.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?;
            let inner = inner.replace('_', " ");
            // Quote font names with spaces for valid CSS (e.g. "Open Sans")
            let css_val = if inner.contains(' ') {
                format!("\"{}\"", inner)
            } else {
                inner
            };
            return Some(CssRule::dynamic(class, vec![("font-family".into(), css_val)]));
        }
    };

    Some(CssRule::new(class, &[("font-family", font_stack)]))
}

// ─── Font Size (Typography 7.2) ──────────────────────────────────────────────

fn resolve_font_size(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("text-")?;

    // Built-in scale: text-xs, text-sm, text-base, text-lg, text-xl, text-2xl … text-9xl
    let (font_size, line_height) = match val {
        "xs"   => ("0.75rem", "1rem"),
        "sm"   => ("0.875rem", "1.25rem"),
        "base" => ("1rem", "1.5rem"),
        "lg"   => ("1.125rem", "1.75rem"),
        "xl"   => ("1.25rem", "1.75rem"),
        "2xl"  => ("1.5rem", "2rem"),
        "3xl"  => ("1.875rem", "2.25rem"),
        "4xl"  => ("2.25rem", "2.5rem"),
        "5xl"  => ("3rem", "1"),
        "6xl"  => ("3.75rem", "1"),
        "7xl"  => ("4.5rem", "1"),
        "8xl"  => ("6rem", "1"),
        "9xl"  => ("8rem", "1"),
        _ => {
            // Arbitrary: text-[1rem], text-[14px], etc.
            let inner = val.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?;
            let inner = inner.replace('_', " ");
            return Some(CssRule::dynamic(
                class,
                vec![("font-size".into(), inner)],
            ));
        }
    };

    Some(CssRule::dynamic(
        class,
        vec![
            ("font-size".into(), font_size.into()),
            ("line-height".into(), line_height.into()),
        ],
    ))
}

// ─── Font Weight (Typography 7.3) ──────────────────────────────────────────

fn resolve_font_weight(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("font-")?;
    let weight = match val {
        "thin"       => "100",
        "extralight" => "200",
        "light"      => "300",
        "normal"     => "400",
        "medium"     => "500",
        "semibold"   => "600",
        "bold"       => "700",
        "extrabold"  => "800",
        "black"      => "900",
        _ => return None,
    };
    Some(CssRule::new(class, &[("font-weight", weight)]))
}

// ─── Font Style (Typography 7.4) ─────────────────────────────────────────────

fn resolve_font_style(class: &str) -> Option<CssRule> {
    let (prop, val) = match class {
        "italic"     => ("font-style", "italic"),
        "not-italic" => ("font-style", "normal"),
        _ => return None,
    };
    Some(CssRule::new(class, &[(prop, val)]))
}

// ─── Font Smoothing (Typography 7.5) ────────────────────────────────────────

fn resolve_font_smoothing(class: &str) -> Option<CssRule> {
    let decls = match class {
        "antialiased" => vec![
            ("-webkit-font-smoothing".into(), "antialiased".into()),
            ("-moz-osx-font-smoothing".into(), "grayscale".into()),
        ],
        "subpixel-antialiased" => vec![
            ("-webkit-font-smoothing".into(), "auto".into()),
            ("-moz-osx-font-smoothing".into(), "auto".into()),
        ],
        _ => return None,
    };
    Some(CssRule::dynamic(class, decls))
}

// ─── Letter Spacing (Typography 7.6) ───────────────────────────────────────────

fn resolve_letter_spacing(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("tracking-")?;
    let letter_spacing = match val {
        "tighter" => "-0.05em",
        "tight"   => "-0.025em",
        "normal"  => "0em",
        "wide"    => "0.025em",
        "wider"   => "0.05em",
        "widest"  => "0.1em",
        _ => return None,
    };
    Some(CssRule::new(class, &[("letter-spacing", letter_spacing)]))
}

// ─── Line Height (Typography 7.7) ─────────────────────────────────────────────

fn resolve_line_height(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("leading-")?;
    let line_height: String = match val {
        "none"    => "1".into(),
        "tight"   => "1.25".into(),
        "snug"    => "1.375".into(),
        "normal"  => "1.5".into(),
        "relaxed" => "1.625".into(),
        "loose"   => "2".into(),
        _ => {
            // leading-{n} → spacing scale (calc(0.25rem * n))
            resolve_spacing_or_arbitrary(val)?
        }
    };
    Some(CssRule::dynamic(class, vec![("line-height".into(), line_height)]))
}

// ─── Text Align (Typography 7.8) ────────────────────────────────────────────

fn resolve_text_align(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("text-")?;
    let align = match val {
        "left"    => "left",
        "center"  => "center",
        "right"   => "right",
        "justify" => "justify",
        "start"   => "start",
        "end"     => "end",
        _ => return None,
    };
    Some(CssRule::new(class, &[("text-align", align)]))
}

// ─── Text Decoration (Typography 7.10) ──────────────────────────────────────

fn resolve_text_decoration(class: &str) -> Option<CssRule> {
    // text-decoration-line: underline, overline, line-through, no-underline
    if let Some(line) = match class {
        "underline"     => Some("underline"),
        "overline"      => Some("overline"),
        "line-through"   => Some("line-through"),
        "no-underline"  => Some("none"),
        _ => None,
    } {
        return Some(CssRule::new(class, &[("text-decoration-line", line)]));
    }

    // decoration-{color}
    if let Some(val) = class.strip_prefix("decoration-") {
        if let Some(rule) = resolve_color_value(val, class, "text-decoration-color") {
            return Some(rule);
        }
    }

    // decoration-{style}: solid, double, dotted, dashed, wavy
    if let Some(val) = class.strip_prefix("decoration-") {
        let style = match val {
            "solid"  => Some("solid"),
            "double" => Some("double"),
            "dotted" => Some("dotted"),
            "dashed" => Some("dashed"),
            "wavy"   => Some("wavy"),
            _ => None,
        };
        if let Some(s) = style {
            return Some(CssRule::new(class, &[("text-decoration-style", s)]));
        }
    }

    // decoration-{thickness}: auto, from-font, 0, 1, 2, 4, 8
    if let Some(val) = class.strip_prefix("decoration-") {
        let thickness = match val {
            "auto"       => Some("auto"),
            "from-font"  => Some("from-font"),
            "0"          => Some("0px"),
            "1"          => Some("1px"),
            "2"          => Some("2px"),
            "4"          => Some("4px"),
            "8"          => Some("8px"),
            _ => None,
        };
        if let Some(t) = thickness {
            return Some(CssRule::new(class, &[("text-decoration-thickness", t)]));
        }
    }

    // underline-offset-*
    if let Some(val) = class.strip_prefix("underline-offset-") {
        let offset: Option<String> = match val {
            "auto" => Some("auto".into()),
            "0"    => Some("0px".into()),
            "1"    => Some("1px".into()),
            "2"    => Some("2px".into()),
            "4"    => Some("4px".into()),
            "8"    => Some("8px".into()),
            _ if val.starts_with('[') && val.ends_with(']') => {
                Some(val[1..val.len() - 1].replace('_', " "))
            }
            _ => resolve_spacing_or_arbitrary(val),
        };
        if let Some(o) = offset {
            return Some(CssRule::dynamic(class, vec![("text-underline-offset".into(), o)]));
        }
    }

    None
}

// ─── Text Transform (Typography 7.11) ────────────────────────────────────────

fn resolve_text_transform(class: &str) -> Option<CssRule> {
    let transform = match class {
        "uppercase"   => "uppercase",
        "lowercase"   => "lowercase",
        "capitalize"  => "capitalize",
        "normal-case" => "none",
        _ => return None,
    };
    Some(CssRule::new(class, &[("text-transform", transform)]))
}

// ─── Text Overflow (Typography 7.12) ────────────────────────────────────────

fn resolve_text_overflow(class: &str) -> Option<CssRule> {
    match class {
        "truncate" => Some(CssRule::dynamic(
            class,
            vec![
                ("overflow".into(), "hidden".into()),
                ("text-overflow".into(), "ellipsis".into()),
                ("white-space".into(), "nowrap".into()),
            ],
        )),
        "text-ellipsis" => Some(CssRule::new(class, &[("text-overflow", "ellipsis")])),
        "text-clip" => Some(CssRule::new(class, &[("text-overflow", "clip")])),
        _ => None,
    }
}

// ─── Text Wrap (Typography 7.13) ──────────────────────────────────────────────

fn resolve_text_wrap(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("text-")?;
    let wrap = match val {
        "wrap"    => "wrap",
        "nowrap"  => "nowrap",
        "balance" => "balance",
        "pretty"  => "pretty",
        _ => return None,
    };
    Some(CssRule::new(class, &[("text-wrap", wrap)]))
}

// ─── Text Indent (Typography 7.14) ──────────────────────────────────────────

fn resolve_text_indent(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("indent-")?;
    let indent = match val {
        "px" => "1px".to_string(),
        _ => resolve_spacing_or_arbitrary(val)?,
    };
    Some(CssRule::dynamic(class, vec![("text-indent".into(), indent)]))
}

// ─── Vertical Align (Typography 7.15) ───────────────────────────────────────

fn resolve_vertical_align(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("align-")?;
    let align = match val {
        "baseline"       => "baseline",
        "top"            => "top",
        "middle"         => "middle",
        "bottom"         => "bottom",
        "text-top"       => "text-top",
        "text-bottom"    => "text-bottom",
        "sub"            => "sub",
        "super"          => "super",
        _ => return None,
    };
    Some(CssRule::new(class, &[("vertical-align", align)]))
}

// ─── White Space (Typography 7.16) ──────────────────────────────────────────

fn resolve_whitespace(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("whitespace-")?;
    let ws = match val {
        "normal"        => "normal",
        "nowrap"        => "nowrap",
        "pre"           => "pre",
        "pre-line"      => "pre-line",
        "pre-wrap"      => "pre-wrap",
        "break-spaces"  => "break-spaces",
        _ => return None,
    };
    Some(CssRule::new(class, &[("white-space", ws)]))
}

// ─── Word Break (Typography 7.17) ───────────────────────────────────────────

fn resolve_word_break(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("break-")?;
    let decls = match val {
        "normal" => vec![
            ("word-break".into(), "normal".into()),
            ("overflow-wrap".into(), "normal".into()),
        ],
        "words" => vec![("overflow-wrap".into(), "break-word".into())],
        "all"   => vec![("word-break".into(), "break-all".into())],
        "keep"  => vec![("word-break".into(), "keep-all".into())],
        _ => return None,
    };
    Some(CssRule::dynamic(class, decls))
}

// ─── Line Clamp (Typography 7.18) ───────────────────────────────────────────

fn resolve_line_clamp(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("line-clamp-")?;
    if val == "none" {
        return Some(CssRule::dynamic(
            class,
            vec![
                ("overflow".into(), "visible".into()),
                ("display".into(), "block".into()),
                ("-webkit-line-clamp".into(), "unset".into()),
                ("line-clamp".into(), "unset".into()),
            ],
        ));
    }
    let n: u32 = val.parse().ok()?;
    if (1..=6).contains(&n) {
        return Some(CssRule::dynamic(
            class,
            vec![
                ("overflow".into(), "hidden".into()),
                ("display".into(), "-webkit-box".into()),
                ("-webkit-box-orient".into(), "vertical".into()),
                ("-webkit-line-clamp".into(), n.to_string()),
                ("line-clamp".into(), n.to_string()),
            ],
        ));
    }
    None
}

// ─── List Style (Typography 7.19) ────────────────────────────────────────────

fn resolve_list_style(class: &str) -> Option<CssRule> {
    if let Some(val) = class.strip_prefix("list-") {
        if val == "image-none" {
            return Some(CssRule::new(class, &[("list-style-image", "none")]));
        }
        let (prop, css_val) = match val {
            "none"     => ("list-style-type", "none"),
            "disc"     => ("list-style-type", "disc"),
            "decimal"  => ("list-style-type", "decimal"),
            "inside"   => ("list-style-position", "inside"),
            "outside"  => ("list-style-position", "outside"),
            _ => return None,
        };
        return Some(CssRule::new(class, &[(prop, css_val)]));
    }
    None
}

// ─── Font Variant Numeric (Typography 7.20) ──────────────────────────────────

fn resolve_font_variant_numeric(class: &str) -> Option<CssRule> {
    let val = match class {
        "normal-nums"        => "normal",
        "ordinal"            => "ordinal",
        "slashed-zero"       => "slashed-zero",
        "lining-nums"        => "lining-nums",
        "oldstyle-nums"      => "oldstyle-nums",
        "proportional-nums"  => "proportional-nums",
        "tabular-nums"       => "tabular-nums",
        "diagonal-fractions"  => "diagonal-fractions",
        "stacked-fractions"   => "stacked-fractions",
        _ => return None,
    };
    Some(CssRule::new(class, &[("font-variant-numeric", val)]))
}

// ─── Border Width (9.1) ─────────────────────────────────────────────────────

fn border_width_value(val: &str) -> Option<&'static str> {
    match val {
        "0" => Some("0px"),
        "2" => Some("2px"),
        "4" => Some("4px"),
        "8" => Some("8px"),
        _ => None,
    }
}

fn resolve_border_width(class: &str) -> Option<CssRule> {
    // border, border-0, border-2, border-4, border-8
    if class == "border" {
        return Some(CssRule::new(class, &[("border-width", "1px")]));
    }
    if let Some(val) = class.strip_prefix("border-") {
        if let Some(w) = border_width_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("border-top-width".into(), w.into()),
                    ("border-right-width".into(), w.into()),
                    ("border-bottom-width".into(), w.into()),
                    ("border-left-width".into(), w.into()),
                ],
            ));
        }
    }

    // border-t, border-r, border-b, border-l (1px)
    let (props, val_str) = if let Some(v) = class.strip_prefix("border-t-") {
        (&["border-top-width"], v)
    } else if let Some(v) = class.strip_prefix("border-r-") {
        (&["border-right-width"], v)
    } else if let Some(v) = class.strip_prefix("border-b-") {
        (&["border-bottom-width"], v)
    } else if let Some(v) = class.strip_prefix("border-l-") {
        (&["border-left-width"], v)
    } else if let Some(v) = class.strip_prefix("border-s-") {
        (&["border-inline-start-width"], v)
    } else if let Some(v) = class.strip_prefix("border-e-") {
        (&["border-inline-end-width"], v)
    } else if let Some(v) = class.strip_prefix("border-bs-") {
        (&["border-block-start-width"], v)
    } else if let Some(v) = class.strip_prefix("border-be-") {
        (&["border-block-end-width"], v)
    } else if class == "border-t" || class == "border-r" || class == "border-b" || class == "border-l" {
        let prop = match class {
            "border-t" => "border-top-width",
            "border-r" => "border-right-width",
            "border-b" => "border-bottom-width",
            "border-l" => "border-left-width",
            _ => return None,
        };
        return Some(CssRule::new(class, &[(prop, "1px")]));
    } else if class == "border-x" {
        return Some(CssRule::dynamic(
            class,
            vec![
                ("border-left-width".into(), "1px".into()),
                ("border-right-width".into(), "1px".into()),
            ],
        ));
    } else if class == "border-y" {
        return Some(CssRule::dynamic(
            class,
            vec![
                ("border-top-width".into(), "1px".into()),
                ("border-bottom-width".into(), "1px".into()),
            ],
        ));
    } else if let Some(v) = class.strip_prefix("border-x-") {
        let w = border_width_value(v)?;
        return Some(CssRule::dynamic(
            class,
            vec![
                ("border-left-width".into(), w.into()),
                ("border-right-width".into(), w.into()),
            ],
        ));
    } else if let Some(v) = class.strip_prefix("border-y-") {
        let w = border_width_value(v)?;
        return Some(CssRule::dynamic(
            class,
            vec![
                ("border-top-width".into(), w.into()),
                ("border-bottom-width".into(), w.into()),
            ],
        ));
    } else {
        return None;
    };

    let w = border_width_value(val_str)?;
    let decls = match props {
        &["border-top-width"] => vec![("border-top-width".into(), w.into())],
        &["border-right-width"] => vec![("border-right-width".into(), w.into())],
        &["border-bottom-width"] => vec![("border-bottom-width".into(), w.into())],
        &["border-left-width"] => vec![("border-left-width".into(), w.into())],
        &["border-inline-start-width"] => vec![("border-inline-start-width".into(), w.into())],
        &["border-inline-end-width"] => vec![("border-inline-end-width".into(), w.into())],
        &["border-block-start-width"] => vec![("border-block-start-width".into(), w.into())],
        &["border-block-end-width"] => vec![("border-block-end-width".into(), w.into())],
        _ => return None,
    };
    Some(CssRule::dynamic(class, decls))
}

// ─── Border Color (9.2) ─────────────────────────────────────────────────────

fn resolve_border_color(class: &str) -> Option<CssRule> {
    // border-{color} (all sides)
    if let Some(val) = class.strip_prefix("border-") {
        if let Some(rule) = resolve_color_value(val, class, "border-color") {
            return Some(rule);
        }
    }
    // border-t-{color}, border-r-{color}, etc.
    let (prop, val) = if let Some(v) = class.strip_prefix("border-t-") {
        ("border-top-color", v)
    } else if let Some(v) = class.strip_prefix("border-r-") {
        ("border-right-color", v)
    } else if let Some(v) = class.strip_prefix("border-b-") {
        ("border-bottom-color", v)
    } else if let Some(v) = class.strip_prefix("border-l-") {
        ("border-left-color", v)
    } else {
        return None;
    };
    if let Some(color) = get_color_css_value(val) {
        return Some(CssRule::dynamic(class, vec![(prop.into(), color)]));
    }
    None
}

// ─── Border Style (9.3) ─────────────────────────────────────────────────────

fn resolve_border_style(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("border-")?;
    let style = match val {
        "solid"  => "solid",
        "dashed" => "dashed",
        "dotted" => "dotted",
        "double" => "double",
        "hidden" => "hidden",
        "none"   => "none",
        _ => return None,
    };
    Some(CssRule::new(class, &[("border-style", style)]))
}

// ─── Border Radius (9.4) ────────────────────────────────────────────────────

fn rounded_value(val: &str) -> Option<&'static str> {
    match val {
        "none" => Some("0"),
        "sm"   => Some("0.125rem"),
        "md"   => Some("0.375rem"),
        "lg"   => Some("0.5rem"),
        "xl"   => Some("0.75rem"),
        "2xl"  => Some("1rem"),
        "3xl"  => Some("1.5rem"),
        "full" => Some("9999px"),
        _ => None,
    }
}

fn resolve_border_radius(class: &str) -> Option<CssRule> {
    if class == "rounded" {
        return Some(CssRule::new(class, &[("border-radius", "0.25rem")]));
    }
    if class == "rounded-none" {
        return Some(CssRule::new(class, &[("border-radius", "0")]));
    }
    if let Some(val) = class.strip_prefix("rounded-") {
        if let Some(r) = rounded_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("border-top-left-radius".into(), r.into()),
                    ("border-top-right-radius".into(), r.into()),
                    ("border-bottom-right-radius".into(), r.into()),
                    ("border-bottom-left-radius".into(), r.into()),
                ],
            ));
        }
        // rounded-t-*, rounded-r-*, rounded-tl-*, etc.
        let decls: Vec<(String, String)> = if let Some(s) = val.strip_prefix("tl-") {
            vec![("border-top-left-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("tr-") {
            vec![("border-top-right-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("br-") {
            vec![("border-bottom-right-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("bl-") {
            vec![("border-bottom-left-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("t-") {
            let r = rounded_value(s)?;
            vec![
                ("border-top-left-radius".into(), r.into()),
                ("border-top-right-radius".into(), r.into()),
            ]
        } else if let Some(s) = val.strip_prefix("r-") {
            let r = rounded_value(s)?;
            vec![
                ("border-top-right-radius".into(), r.into()),
                ("border-bottom-right-radius".into(), r.into()),
            ]
        } else if let Some(s) = val.strip_prefix("b-") {
            let r = rounded_value(s)?;
            vec![
                ("border-bottom-right-radius".into(), r.into()),
                ("border-bottom-left-radius".into(), r.into()),
            ]
        } else if let Some(s) = val.strip_prefix("l-") {
            let r = rounded_value(s)?;
            vec![
                ("border-top-left-radius".into(), r.into()),
                ("border-bottom-left-radius".into(), r.into()),
            ]
        } else if let Some(s) = val.strip_prefix("ss-") {
            vec![("border-start-start-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("se-") {
            vec![("border-start-end-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("es-") {
            vec![("border-end-start-radius".into(), rounded_value(s)?.into())]
        } else if let Some(s) = val.strip_prefix("ee-") {
            vec![("border-end-end-radius".into(), rounded_value(s)?.into())]
        } else {
            return None;
        };
        return Some(CssRule::dynamic(class, decls));
    }
    None
}

// ─── Outline (9.5) ───────────────────────────────────────────────────────────

fn resolve_outline(class: &str) -> Option<CssRule> {
    if class == "outline-none" {
        return Some(CssRule::new(class, &[("outline", "none")]));
    }
    if class == "outline" {
        return Some(CssRule::new(class, &[("outline", "2px solid var(--tw-outline-color)")]));
    }
    if let Some(val) = class.strip_prefix("outline-") {
        let style = match val {
            ""        => Some("2px solid var(--tw-outline-color)"),
            "dashed"  => Some("2px dashed var(--tw-outline-color)"),
            "dotted"  => Some("2px dotted var(--tw-outline-color)"),
            "double"  => Some("2px double var(--tw-outline-color)"),
            "0"       => Some("2px solid transparent"),
            "1"       => Some("1px solid var(--tw-outline-color)"),
            "2"       => Some("2px solid var(--tw-outline-color)"),
            "4"       => Some("4px solid var(--tw-outline-color)"),
            "8"       => Some("8px solid var(--tw-outline-color)"),
            _ => None,
        };
        if let Some(s) = style {
            return Some(CssRule::new(class, &[("outline", s)]));
        }
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("--tw-outline-color".into(), color),
                    ("outline".into(), "2px solid var(--tw-outline-color)".into()),
                ],
            ));
        }
    }
    if let Some(val) = class.strip_prefix("outline-offset-") {
        let offset = match val {
            "0" => "0px",
            "1" => "1px",
            "2" => "2px",
            "4" => "4px",
            "8" => "8px",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("outline-offset", offset)]));
    }
    None
}

// ─── Ring (9.6) ─────────────────────────────────────────────────────────────

fn resolve_ring(class: &str) -> Option<CssRule> {
    if class == "ring-inset" {
        return Some(CssRule::new(class, &[("--tw-ring-inset", "inset")]));
    }
    if let Some(val) = class.strip_prefix("ring-offset-") {
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("--tw-ring-offset-color".into(), color),
                    ("--tw-ring-offset-width".into(), "0px".into()),
                ],
            ));
        }
        let offset = match val {
            "0" => "0px",
            "1" => "1px",
            "2" => "2px",
            "4" => "4px",
            "8" => "8px",
            _ => return None,
        };
        return Some(CssRule::new(class, &[("--tw-ring-offset-width", offset)]));
    }
    if let Some(val) = class.strip_prefix("ring-") {
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("--tw-ring-color".into(), color),
                    ("box-shadow".into(), "var(--tw-ring-inset) 0 0 0 calc(3px + var(--tw-ring-offset-width)) var(--tw-ring-color)".into()),
                ],
            ));
        }
    }
    let (ring_width, _) = if class == "ring" {
        ("3px", true)
    } else if let Some(val) = class.strip_prefix("ring-") {
        let w = match val {
            "0" => "0px",
            "1" => "1px",
            "2" => "2px",
            "4" => "4px",
            "8" => "8px",
            _ => return None,
        };
        (w, true)
    } else {
        return None;
    };
    let box_shadow = format!("var(--tw-ring-inset) 0 0 0 calc({} + var(--tw-ring-offset-width)) var(--tw-ring-color)", ring_width);
    Some(CssRule::dynamic(
        class,
        vec![
            ("--tw-ring-offset-width".into(), "0px".into()),
            ("--tw-ring-color".into(), "rgb(59 130 246 / 0.5)".into()),
            ("box-shadow".into(), box_shadow),
        ],
    ))
}

// ─── Divide (9.7) ────────────────────────────────────────────────────────────

fn resolve_divide(class: &str) -> Option<CssRule> {
    let (prop, val) = if let Some(v) = class.strip_prefix("divide-x-") {
        ("border-left-width", border_width_value(v)?)
    } else if let Some(v) = class.strip_prefix("divide-y-") {
        ("border-top-width", border_width_value(v)?)
    } else if class == "divide-x" {
        return Some(CssRule::with_compound_selector(
            format!(".{} > * + *", escape_selector(class)),
            vec![
                ("border-left-width".into(), "1px".into()),
                ("border-right-width".into(), "0px".into()),
            ],
        ));
    } else if class == "divide-y" {
        return Some(CssRule::with_compound_selector(
            format!(".{} > * + *", escape_selector(class)),
            vec![
                ("border-top-width".into(), "1px".into()),
                ("border-bottom-width".into(), "0px".into()),
            ],
        ));
    } else if class == "divide-x-reverse" {
        return Some(CssRule::new(class, &[("--tw-divide-x-reverse", "1")]));
    } else if class == "divide-y-reverse" {
        return Some(CssRule::new(class, &[("--tw-divide-y-reverse", "1")]));
    } else if let Some(val) = class.strip_prefix("divide-") {
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::new(class, &[("--tw-divide-color", &color)]));
        }
        let style = match val {
            "solid"  => Some("solid"),
            "dashed" => Some("dashed"),
            "dotted" => Some("dotted"),
            "double" => Some("double"),
            "none"   => Some("none"),
            _ => return None,
        };
        if let Some(s) = style {
            return Some(CssRule::new(class, &[("border-style", s)]));
        }
        return None;
    } else {
        return None;
    };
    let sel = format!(".{} > * + *", escape_selector(class));
    let decls = if prop == "border-left-width" {
        vec![
            ("border-left-width".into(), val.into()),
            ("border-right-width".into(), "0px".into()),
        ]
    } else {
        vec![
            ("border-top-width".into(), val.into()),
            ("border-bottom-width".into(), "0px".into()),
        ]
    };
    Some(CssRule::with_compound_selector(sel, decls))
}

// ─── Box Shadow (10.1) ───────────────────────────────────────────────────────

fn resolve_box_shadow(class: &str) -> Option<CssRule> {
    // shadow-{color} → --tw-shadow-color (use with shadow-sm/lg etc. for colored shadow)
    if let Some(val) = class.strip_prefix("shadow-") {
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![("--tw-shadow-color".into(), color)],
            ));
        }
    }
    let shadow = match class {
        "shadow-2xs"   => "0 1px rgb(0 0 0 / 0.05)",
        "shadow-xs"    => "0 1px 2px 0 rgb(0 0 0 / 0.05)",
        "shadow-sm"    => "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
        "shadow"       => "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
        "shadow-md"    => "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
        "shadow-lg"    => "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
        "shadow-xl"    => "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)",
        "shadow-2xl"   => "0 25px 50px -12px rgb(0 0 0 / 0.25)",
        "shadow-inner" => "inset 0 2px 4px 0 rgb(0 0 0 / 0.05)",
        "shadow-none"  => "0 0 #0000",
        _ => return None,
    };
    Some(CssRule::new(class, &[("box-shadow", shadow)]))
}

// ─── Opacity (10.2) ──────────────────────────────────────────────────────────

fn resolve_opacity(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("opacity-")?;
    let pct: u8 = val.parse().ok()?;
    if pct > 100 {
        return None;
    }
    let opacity = format!("{}", pct as f64 / 100.0);
    Some(CssRule::dynamic(class, vec![("opacity".into(), opacity)]))
}

// ─── Mix Blend Mode (10.3) ───────────────────────────────────────────────────

fn resolve_mix_blend_mode(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("mix-blend-")?;
    let mode = match val {
        "normal"        => "normal",
        "multiply"      => "multiply",
        "screen"        => "screen",
        "overlay"       => "overlay",
        "darken"        => "darken",
        "lighten"       => "lighten",
        "color-dodge"   => "color-dodge",
        "color-burn"    => "color-burn",
        "hard-light"    => "hard-light",
        "soft-light"    => "soft-light",
        "difference"    => "difference",
        "exclusion"     => "exclusion",
        "hue"           => "hue",
        "saturation"    => "saturation",
        "color"         => "color",
        "luminosity"    => "luminosity",
        "plus-darker"   => "plus-darker",
        "plus-lighter"  => "plus-lighter",
        _ => return None,
    };
    Some(CssRule::new(class, &[("mix-blend-mode", mode)]))
}

// ─── Background Blend Mode (10.4) ────────────────────────────────────────────

fn resolve_bg_blend_mode(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-blend-")?;
    let mode = match val {
        "normal"        => "normal",
        "multiply"      => "multiply",
        "screen"        => "screen",
        "overlay"       => "overlay",
        "darken"        => "darken",
        "lighten"       => "lighten",
        "color-dodge"   => "color-dodge",
        "color-burn"    => "color-burn",
        "hard-light"    => "hard-light",
        "soft-light"    => "soft-light",
        "difference"    => "difference",
        "exclusion"     => "exclusion",
        "hue"           => "hue",
        "saturation"    => "saturation",
        "color"         => "color",
        "luminosity"    => "luminosity",
        "plus-darker"   => "plus-darker",
        "plus-lighter"  => "plus-lighter",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-blend-mode", mode)]))
}

// ─── Filter (11.1) ───────────────────────────────────────────────────────────
//
// Each filter utility sets its --tw-* variable and the full filter chain so
// multiple filter classes on the same element combine correctly.

const FILTER_CHAIN: &str = "var(--tw-blur) var(--tw-brightness) var(--tw-contrast) var(--tw-grayscale) var(--tw-hue-rotate) var(--tw-invert) var(--tw-saturate) var(--tw-sepia) var(--tw-drop-shadow)";

fn resolve_filter(class: &str) -> Option<CssRule> {
    let (var, val) = if class == "blur" {
        ("--tw-blur", "blur(8px)".into())
    } else if let Some(v) = class.strip_prefix("blur-") {
        let blur = match v {
            "none" => "0",
            "sm"   => "4px",
            "md"   => "12px",
            "lg"   => "16px",
            "xl"   => "24px",
            "2xl"  => "40px",
            "3xl"  => "64px",
            _      => return None,
        };
        ("--tw-blur", format!("blur({})", blur))
    } else if let Some(v) = class.strip_prefix("brightness-") {
        let b = match v {
            "0"   => "0",
            "50"  => "0.5",
            "75"  => "0.75",
            "90"  => "0.9",
            "95"  => "0.95",
            "100" => "1",
            "105" => "1.05",
            "110" => "1.1",
            "125" => "1.25",
            "150" => "1.5",
            "200" => "2",
            _     => return None,
        };
        ("--tw-brightness", format!("brightness({})", b))
    } else if let Some(v) = class.strip_prefix("contrast-") {
        let c = match v {
            "0"   => "0",
            "50"  => "0.5",
            "75"  => "0.75",
            "100" => "1",
            "125" => "1.25",
            "150" => "1.5",
            "200" => "2",
            _     => return None,
        };
        ("--tw-contrast", format!("contrast({})", c))
    } else if class == "drop-shadow" {
        ("--tw-drop-shadow", "drop-shadow(0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1))".into())
    } else if let Some(v) = class.strip_prefix("drop-shadow-") {
        let ds = match v {
            "none" => "0 0 #0000",
            "sm"   => "0 1px 2px rgb(0 0 0 / 0.15)",
            "md"   => "0 3px 3px rgb(0 0 0 / 0.12)",
            "lg"   => "0 4px 4px rgb(0 0 0 / 0.15)",
            "xl"   => "0 9px 7px rgb(0 0 0 / 0.1)",
            "2xl"  => "0 25px 25px rgb(0 0 0 / 0.15)",
            _     => return None,
        };
        ("--tw-drop-shadow", format!("drop-shadow({})", ds))
    } else if class == "grayscale" {
        ("--tw-grayscale", "grayscale(1)".into())
    } else if class == "grayscale-0" {
        ("--tw-grayscale", "grayscale(0)".into())
    } else if let Some(v) = class.strip_prefix("hue-rotate-") {
        let deg = match v {
            "0"   => "0deg",
            "15"  => "15deg",
            "30"  => "30deg",
            "60"  => "60deg",
            "90"  => "90deg",
            "180" => "180deg",
            _     => return None,
        };
        ("--tw-hue-rotate", format!("hue-rotate({})", deg))
    } else if class == "invert" {
        ("--tw-invert", "invert(1)".into())
    } else if class == "invert-0" {
        ("--tw-invert", "invert(0)".into())
    } else if let Some(v) = class.strip_prefix("saturate-") {
        let s = match v {
            "0"   => "0",
            "50"  => "0.5",
            "100" => "1",
            "150" => "1.5",
            "200" => "2",
            _     => return None,
        };
        ("--tw-saturate", format!("saturate({})", s))
    } else if class == "sepia" {
        ("--tw-sepia", "sepia(1)".into())
    } else if class == "sepia-0" {
        ("--tw-sepia", "sepia(0)".into())
    } else {
        return None;
    };
    Some(CssRule::dynamic(
        class,
        vec![
            (var.into(), val),
            ("filter".into(), FILTER_CHAIN.into()),
        ],
    ))
}

// ─── Backdrop Filter (11.2) ───────────────────────────────────────────────────

const BACKDROP_FILTER_CHAIN: &str = "var(--tw-backdrop-blur) var(--tw-backdrop-brightness) var(--tw-backdrop-contrast) var(--tw-backdrop-grayscale) var(--tw-backdrop-hue-rotate) var(--tw-backdrop-invert) var(--tw-backdrop-opacity) var(--tw-backdrop-saturate) var(--tw-backdrop-sepia)";

fn resolve_backdrop_filter(class: &str) -> Option<CssRule> {
    let (var, val) = if class == "backdrop-blur" {
        ("--tw-backdrop-blur", "blur(8px)".into())
    } else if let Some(v) = class.strip_prefix("backdrop-blur-") {
        let blur = match v {
            "none" => "0",
            "sm"   => "4px",
            "md"   => "12px",
            "lg"   => "16px",
            "xl"   => "24px",
            "2xl"  => "40px",
            "3xl"  => "64px",
            _      => return None,
        };
        ("--tw-backdrop-blur", format!("blur({})", blur))
    } else if let Some(v) = class.strip_prefix("backdrop-brightness-") {
        let n: f64 = v.parse().ok()?;
        if !(0.0..=300.0).contains(&n) {
            return None;
        }
        ("--tw-backdrop-brightness", format!("brightness({})", n / 100.0))
    } else if let Some(v) = class.strip_prefix("backdrop-contrast-") {
        let n: f64 = v.parse().ok()?;
        if !(0.0..=300.0).contains(&n) {
            return None;
        }
        ("--tw-backdrop-contrast", format!("contrast({})", n / 100.0))
    } else if class == "backdrop-grayscale" {
        ("--tw-backdrop-grayscale", "grayscale(1)".into())
    } else if class == "backdrop-grayscale-0" {
        ("--tw-backdrop-grayscale", "grayscale(0)".into())
    } else if let Some(v) = class.strip_prefix("backdrop-hue-rotate-") {
        let deg: i32 = v.parse().ok()?;
        if !(0..=360).contains(&deg) {
            return None;
        }
        ("--tw-backdrop-hue-rotate", format!("hue-rotate({}deg)", deg))
    } else if class == "backdrop-invert" {
        ("--tw-backdrop-invert", "invert(1)".into())
    } else if class == "backdrop-invert-0" {
        ("--tw-backdrop-invert", "invert(0)".into())
    } else if let Some(v) = class.strip_prefix("backdrop-opacity-") {
        let pct: u8 = v.parse().ok()?;
        if pct > 100 {
            return None;
        }
        ("--tw-backdrop-opacity", format!("opacity({})", pct as f64 / 100.0))
    } else if let Some(v) = class.strip_prefix("backdrop-saturate-") {
        let n: f64 = v.parse().ok()?;
        if !(0.0..=300.0).contains(&n) {
            return None;
        }
        ("--tw-backdrop-saturate", format!("saturate({})", n / 100.0))
    } else if class == "backdrop-sepia" {
        ("--tw-backdrop-sepia", "sepia(1)".into())
    } else if class == "backdrop-sepia-0" {
        ("--tw-backdrop-sepia", "sepia(0)".into())
    } else {
        return None;
    };
    Some(CssRule::dynamic(
        class,
        vec![
            (var.into(), val),
            ("backdrop-filter".into(), BACKDROP_FILTER_CHAIN.into()),
            ("-webkit-backdrop-filter".into(), BACKDROP_FILTER_CHAIN.into()),
        ],
    ))
}

// ─── Transition (12.1–12.4) ───────────────────────────────────────────────────

const TRANSITION_DEFAULT: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform";
const TRANSITION_COLORS: &str = "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke";
const TRANSITION_TRANSFORM: &str = "transform, translate, scale, rotate";
const TRANSITION_TIMING: &str = "cubic-bezier(0.4, 0, 0.2, 1)";
const TRANSITION_DURATION_MS: u16 = 150;

fn resolve_transition(class: &str) -> Option<CssRule> {
    match class {
        "transition-none" => return Some(CssRule::new(class, &[("transition-property", "none")])),
        "transition-all" | "transition" | "transition-colors" | "transition-opacity"
        | "transition-shadow" | "transition-transform" => {}
        _ => return None,
    };
    let prop = match class {
        "transition-all"       => "all",
        "transition"           => TRANSITION_DEFAULT,
        "transition-colors"    => TRANSITION_COLORS,
        "transition-opacity"   => "opacity",
        "transition-shadow"    => "box-shadow",
        "transition-transform" => TRANSITION_TRANSFORM,
        _ => return None,
    };
    Some(CssRule::dynamic(
        class,
        vec![
            ("transition-property".into(), prop.into()),
            ("transition-timing-function".into(), TRANSITION_TIMING.into()),
            ("transition-duration".into(), format!("{}ms", TRANSITION_DURATION_MS)),
        ],
    ))
}

fn resolve_duration(class: &str) -> Option<CssRule> {
    let ms = class.strip_prefix("duration-")?;
    let val = match ms {
        "0"    => 0u16,
        "75"   => 75,
        "100"  => 100,
        "150"  => 150,
        "200"  => 200,
        "300"  => 300,
        "500"  => 500,
        "700"  => 700,
        "1000" => 1000,
        _      => return None,
    };
    Some(CssRule::new(class, &[("transition-duration", &format!("{}ms", val))]))
}

fn resolve_ease(class: &str) -> Option<CssRule> {
    let timing = match class {
        "ease-linear"   => "linear",
        "ease-in"       => "cubic-bezier(0.4, 0, 1, 1)",
        "ease-out"      => "cubic-bezier(0, 0, 0.2, 1)",
        "ease-in-out"   => "cubic-bezier(0.4, 0, 0.2, 1)",
        _ => return None,
    };
    Some(CssRule::new(class, &[("transition-timing-function", timing)]))
}

fn resolve_delay(class: &str) -> Option<CssRule> {
    let ms = class.strip_prefix("delay-")?;
    let val = match ms {
        "0"    => 0u16,
        "75"   => 75,
        "100"  => 100,
        "150"  => 150,
        "200"  => 200,
        "300"  => 300,
        "500"  => 500,
        "700"  => 700,
        "1000" => 1000,
        _      => return None,
    };
    Some(CssRule::new(class, &[("transition-delay", &format!("{}ms", val))]))
}

// ─── Animation (12.5) ───────────────────────────────────────────────────────

fn resolve_animation(class: &str) -> Option<CssRule> {
    match class {
        "animate-none" => Some(CssRule::new(class, &[("animation", "none")])),
        "animate-spin" => Some(
            CssRule::new(class, &[("animation", "spin 1s linear infinite")])
                .with_keyframes("@keyframes spin {\n  to { transform: rotate(360deg); }\n}"),
        ),
        "animate-ping" => Some(
            CssRule::new(class, &[("animation", "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite")])
                .with_keyframes("@keyframes ping {\n  75%, 100% { transform: scale(2); opacity: 0; }\n}"),
        ),
        "animate-pulse" => Some(
            CssRule::new(class, &[("animation", "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite")])
                .with_keyframes("@keyframes pulse {\n  50% { opacity: 0.5; }\n}"),
        ),
        "animate-bounce" => Some(
            CssRule::new(class, &[("animation", "bounce 1s infinite")])
                .with_keyframes("@keyframes bounce {\n  0%, 100% { transform: translateY(-25%); animation-timing-function: cubic-bezier(0.8, 0, 1, 1); }\n  50% { transform: none; animation-timing-function: cubic-bezier(0, 0, 0.2, 1); }\n}"),
        ),
        _ => None,
    }
}

// ─── Transforms (13) ──────────────────────────────────────────────────────────
//
// Scale, rotate, translate, skew use CSS variables so multiple transform
// utilities on the same element combine correctly.

const TRANSFORM_CHAIN: &str = "translate(var(--tw-translate-x, 0), var(--tw-translate-y, 0)) translateZ(var(--tw-translate-z, 0)) rotateX(var(--tw-rotate-x, 0)) rotateY(var(--tw-rotate-y, 0)) rotate(var(--tw-rotate, 0)) scale(var(--tw-scale-x, 1), var(--tw-scale-y, 1)) skewX(var(--tw-skew-x, 0)) skewY(var(--tw-skew-y, 0))";

fn transform_rule(class: &str, var: &str, val: String) -> CssRule {
    CssRule::dynamic(
        class,
        vec![
            (var.into(), val),
            ("transform".into(), TRANSFORM_CHAIN.into()),
        ],
    )
}

const SCALE_VALUES: &[&str] = &["0", "50", "75", "90", "95", "100", "105", "110", "125", "150"];

fn resolve_scale(class: &str) -> Option<CssRule> {
    let scale_val = |v: &str, neg: bool| -> Option<String> {
        if !SCALE_VALUES.contains(&v) {
            return None;
        }
        let n: f64 = v.parse().ok()?;
        let s = n / 100.0;
        Some(if neg { format!("{}", -s) } else { format!("{}", s) })
    };
    if let Some(v) = class.strip_prefix("-scale-x-") {
        let s = scale_val(v, true)?;
        return Some(transform_rule(class, "--tw-scale-x", s));
    }
    if let Some(v) = class.strip_prefix("scale-x-") {
        let s = scale_val(v, false)?;
        return Some(transform_rule(class, "--tw-scale-x", s));
    }
    if let Some(v) = class.strip_prefix("-scale-y-") {
        let s = scale_val(v, true)?;
        return Some(transform_rule(class, "--tw-scale-y", s));
    }
    if let Some(v) = class.strip_prefix("scale-y-") {
        let s = scale_val(v, false)?;
        return Some(transform_rule(class, "--tw-scale-y", s));
    }
    if let Some(v) = class.strip_prefix("-scale-") {
        let s = scale_val(v, true)?;
        return Some(CssRule::dynamic(
            class,
            vec![
                ("--tw-scale-x".into(), s.clone()),
                ("--tw-scale-y".into(), s),
                ("transform".into(), TRANSFORM_CHAIN.into()),
            ],
        ));
    }
    if let Some(v) = class.strip_prefix("scale-") {
        let s = scale_val(v, false)?;
        return Some(CssRule::dynamic(
            class,
            vec![
                ("--tw-scale-x".into(), s.clone()),
                ("--tw-scale-y".into(), s),
                ("transform".into(), TRANSFORM_CHAIN.into()),
            ],
        ));
    }
    None
}

fn resolve_rotate(class: &str) -> Option<CssRule> {
    let rot_val = |v: &str, neg: bool| -> Option<String> {
        let deg: i32 = v.parse().ok()?;
        Some(if neg {
            format!("{}deg", -deg)
        } else {
            format!("{}deg", deg)
        })
    };
    if let Some(v) = class.strip_prefix("-rotate-x-") {
        let r = rot_val(v, true)?;
        return Some(transform_rule(class, "--tw-rotate-x", r));
    }
    if let Some(v) = class.strip_prefix("rotate-x-") {
        let r = rot_val(v, false)?;
        return Some(transform_rule(class, "--tw-rotate-x", r));
    }
    if let Some(v) = class.strip_prefix("-rotate-y-") {
        let r = rot_val(v, true)?;
        return Some(transform_rule(class, "--tw-rotate-y", r));
    }
    if let Some(v) = class.strip_prefix("rotate-y-") {
        let r = rot_val(v, false)?;
        return Some(transform_rule(class, "--tw-rotate-y", r));
    }
    let valid_2d = ["0", "1", "2", "3", "6", "12", "45", "90", "180"];
    if let Some(v) = class.strip_prefix("-rotate-") {
        if valid_2d.contains(&v) {
            let r = rot_val(v, true)?;
            return Some(transform_rule(class, "--tw-rotate", r));
        }
    }
    if let Some(v) = class.strip_prefix("rotate-") {
        if valid_2d.contains(&v) {
            let r = rot_val(v, false)?;
            return Some(transform_rule(class, "--tw-rotate", r));
        }
    }
    None
}

fn resolve_translate(class: &str) -> Option<CssRule> {
    let (neg, axis, val) = if let Some(v) = class.strip_prefix("-translate-x-") {
        (true, "x", v)
    } else if let Some(v) = class.strip_prefix("translate-x-") {
        (false, "x", v)
    } else if let Some(v) = class.strip_prefix("-translate-y-") {
        (true, "y", v)
    } else if let Some(v) = class.strip_prefix("translate-y-") {
        (false, "y", v)
    } else if let Some(v) = class.strip_prefix("-translate-z-") {
        let sp = resolve_translate_val(v)?;
        let neg_sp = if sp.starts_with('-') { sp } else { format!("-{}", sp) };
        return Some(transform_rule(class, "--tw-translate-z", neg_sp));
    } else if let Some(v) = class.strip_prefix("translate-z-") {
        let sp = resolve_translate_val(v)?;
        return Some(transform_rule(class, "--tw-translate-z", sp));
    } else {
        return None;
    };
    let sp = resolve_translate_val(val)?;
    let final_val = if neg {
        if sp.starts_with('-') { sp } else { format!("-{}", sp) }
    } else {
        sp
    };
    let var = if axis == "x" { "--tw-translate-x" } else { "--tw-translate-y" };
    Some(transform_rule(class, var, final_val))
}

fn resolve_translate_val(v: &str) -> Option<String> {
    if v == "px" {
        return Some("1px".into());
    }
    if v == "full" {
        return Some("100%".into());
    }
    if v == "1/2" {
        return Some("50%".into());
    }
    if v == "1/3" {
        return Some("33.333333%".into());
    }
    if v == "2/3" {
        return Some("66.666667%".into());
    }
    if v == "1/4" {
        return Some("25%".into());
    }
    if v == "3/4" {
        return Some("75%".into());
    }
    if v == "1/6" {
        return Some("16.666667%".into());
    }
    if v == "5/6" {
        return Some("83.333333%".into());
    }
    spacing::spacing_value(v)
}

fn resolve_skew(class: &str) -> Option<CssRule> {
    let skew_val = |v: &str, neg: bool| -> Option<String> {
        let deg: i32 = v.parse().ok()?;
        Some(if neg {
            format!("{}deg", -deg)
        } else {
            format!("{}deg", deg)
        })
    };
    let valid = ["0", "1", "2", "3", "6", "12"];
    if let Some(v) = class.strip_prefix("-skew-x-") {
        if valid.contains(&v) {
            let s = skew_val(v, true)?;
            return Some(transform_rule(class, "--tw-skew-x", s));
        }
    }
    if let Some(v) = class.strip_prefix("skew-x-") {
        if valid.contains(&v) {
            let s = skew_val(v, false)?;
            return Some(transform_rule(class, "--tw-skew-x", s));
        }
    }
    if let Some(v) = class.strip_prefix("-skew-y-") {
        if valid.contains(&v) {
            let s = skew_val(v, true)?;
            return Some(transform_rule(class, "--tw-skew-y", s));
        }
    }
    if let Some(v) = class.strip_prefix("skew-y-") {
        if valid.contains(&v) {
            let s = skew_val(v, false)?;
            return Some(transform_rule(class, "--tw-skew-y", s));
        }
    }
    None
}

fn resolve_transform_origin(class: &str) -> Option<CssRule> {
    let origin = match class {
        "origin-center"      => "center",
        "origin-top"          => "top",
        "origin-top-right"    => "top right",
        "origin-right"        => "right",
        "origin-bottom-right" => "bottom right",
        "origin-bottom"       => "bottom",
        "origin-bottom-left"  => "bottom left",
        "origin-left"         => "left",
        "origin-top-left"     => "top left",
        _ => return None,
    };
    Some(CssRule::new(class, &[("transform-origin", origin)]))
}

fn resolve_perspective(class: &str) -> Option<CssRule> {
    let val = match class {
        "perspective-none"      => "none",
        "perspective-dramatic"  => "100px",
        "perspective-near"      => "300px",
        "perspective-normal"    => "500px",
        "perspective-midrange"  => "800px",
        "perspective-distant"   => "1200px",
        _ => return None,
    };
    Some(CssRule::new(class, &[("perspective", val)]))
}

// ─── Interactivity (14) ───────────────────────────────────────────────────────

fn resolve_cursor(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("cursor-")?;
    let css = match val {
        "auto" | "default" | "pointer" | "wait" | "text" | "move" | "help"
        | "not-allowed" | "none" | "context-menu" | "progress" | "cell"
        | "crosshair" | "vertical-text" | "alias" | "copy" | "no-drop"
        | "grab" | "grabbing" | "all-scroll" | "col-resize" | "row-resize"
        | "n-resize" | "e-resize" | "s-resize" | "w-resize"
        | "ne-resize" | "nw-resize" | "se-resize" | "sw-resize"
        | "ew-resize" | "ns-resize" | "nesw-resize" | "nwse-resize"
        | "zoom-in" | "zoom-out" => val,
        _ => return None,
    };
    Some(CssRule::new(class, &[("cursor", css)]))
}

fn resolve_pointer_events(class: &str) -> Option<CssRule> {
    let val = match class {
        "pointer-events-none" => "none",
        "pointer-events-auto" => "auto",
        _ => return None,
    };
    Some(CssRule::new(class, &[("pointer-events", val)]))
}

fn resolve_resize(class: &str) -> Option<CssRule> {
    let val = match class {
        "resize-none" => "none",
        "resize"      => "both",
        "resize-y"    => "y",
        "resize-x"    => "x",
        _ => return None,
    };
    Some(CssRule::new(class, &[("resize", val)]))
}

fn resolve_user_select(class: &str) -> Option<CssRule> {
    let val = match class {
        "select-none" => "none",
        "select-text"  => "text",
        "select-all"   => "all",
        "select-auto"  => "auto",
        _ => return None,
    };
    Some(CssRule::new(class, &[("user-select", val)]))
}

fn resolve_scroll(class: &str) -> Option<CssRule> {
    // scroll-auto, scroll-smooth
    if let Some(behavior) = class.strip_prefix("scroll-") {
        if behavior == "auto" || behavior == "smooth" {
            return Some(CssRule::new(class, &[("scroll-behavior", behavior)]));
        }
    }
    // scroll-m-{n}, scroll-p-{n} and directional variants
    let (props, val_str): (Vec<&str>, &str) = if let Some(v) = class.strip_prefix("scroll-m-") {
        (vec!["scroll-margin"], v)
    } else if let Some(v) = class.strip_prefix("scroll-mx-") {
        (vec!["scroll-margin-left", "scroll-margin-right"], v)
    } else if let Some(v) = class.strip_prefix("scroll-my-") {
        (vec!["scroll-margin-top", "scroll-margin-bottom"], v)
    } else if let Some(v) = class.strip_prefix("scroll-mt-") {
        (vec!["scroll-margin-top"], v)
    } else if let Some(v) = class.strip_prefix("scroll-mr-") {
        (vec!["scroll-margin-right"], v)
    } else if let Some(v) = class.strip_prefix("scroll-mb-") {
        (vec!["scroll-margin-bottom"], v)
    } else if let Some(v) = class.strip_prefix("scroll-ml-") {
        (vec!["scroll-margin-left"], v)
    } else if let Some(v) = class.strip_prefix("scroll-ms-") {
        (vec!["scroll-margin-inline-start"], v)
    } else if let Some(v) = class.strip_prefix("scroll-me-") {
        (vec!["scroll-margin-inline-end"], v)
    } else if let Some(v) = class.strip_prefix("scroll-p-") {
        (vec!["scroll-padding"], v)
    } else if let Some(v) = class.strip_prefix("scroll-px-") {
        (vec!["scroll-padding-left", "scroll-padding-right"], v)
    } else if let Some(v) = class.strip_prefix("scroll-py-") {
        (vec!["scroll-padding-top", "scroll-padding-bottom"], v)
    } else if let Some(v) = class.strip_prefix("scroll-pt-") {
        (vec!["scroll-padding-top"], v)
    } else if let Some(v) = class.strip_prefix("scroll-pr-") {
        (vec!["scroll-padding-right"], v)
    } else if let Some(v) = class.strip_prefix("scroll-pb-") {
        (vec!["scroll-padding-bottom"], v)
    } else if let Some(v) = class.strip_prefix("scroll-pl-") {
        (vec!["scroll-padding-left"], v)
    } else if let Some(v) = class.strip_prefix("scroll-ps-") {
        (vec!["scroll-padding-inline-start"], v)
    } else if let Some(v) = class.strip_prefix("scroll-pe-") {
        (vec!["scroll-padding-inline-end"], v)
    } else {
        return resolve_scroll_snap(class);
    };
    let css_val = resolve_spacing_or_arbitrary(val_str)?;
    let decls: Vec<(String, String)> = props
        .iter()
        .map(|p| (p.to_string(), css_val.clone()))
        .collect();
    Some(CssRule::dynamic(class, decls))
}

fn resolve_scroll_snap(class: &str) -> Option<CssRule> {
    match class {
        "snap-none" => Some(CssRule::new(class, &[("scroll-snap-type", "none")])),
        "snap-x" => Some(CssRule::dynamic(class, vec![
            ("--tw-scroll-snap-strictness".into(), "proximity".into()),
            ("scroll-snap-type".into(), "x var(--tw-scroll-snap-strictness)".into()),
        ])),
        "snap-y" => Some(CssRule::dynamic(class, vec![
            ("--tw-scroll-snap-strictness".into(), "proximity".into()),
            ("scroll-snap-type".into(), "y var(--tw-scroll-snap-strictness)".into()),
        ])),
        "snap-both" => Some(CssRule::dynamic(class, vec![
            ("--tw-scroll-snap-strictness".into(), "proximity".into()),
            ("scroll-snap-type".into(), "both var(--tw-scroll-snap-strictness)".into()),
        ])),
        "snap-mandatory" => Some(CssRule::new(class, &[("--tw-scroll-snap-strictness", "mandatory")])),
        "snap-proximity" => Some(CssRule::new(class, &[("--tw-scroll-snap-strictness", "proximity")])),
        "snap-start" => Some(CssRule::new(class, &[("scroll-snap-align", "start")])),
        "snap-end" => Some(CssRule::new(class, &[("scroll-snap-align", "end")])),
        "snap-center" => Some(CssRule::new(class, &[("scroll-snap-align", "center")])),
        "snap-align-none" => Some(CssRule::new(class, &[("scroll-snap-align", "none")])),
        "snap-normal" => Some(CssRule::new(class, &[("scroll-snap-stop", "normal")])),
        "snap-always" => Some(CssRule::new(class, &[("scroll-snap-stop", "always")])),
        _ => None,
    }
}

fn resolve_touch_action(class: &str) -> Option<CssRule> {
    let val = match class {
        "touch-auto"       => "auto",
        "touch-none"       => "none",
        "touch-pan-x"       => "pan-x",
        "touch-pan-left"    => "pan-left",
        "touch-pan-right"   => "pan-right",
        "touch-pan-y"       => "pan-y",
        "touch-pan-up"      => "pan-up",
        "touch-pan-down"    => "pan-down",
        "touch-pinch-zoom"  => "pinch-zoom",
        "touch-manipulation" => "manipulation",
        _ => return None,
    };
    Some(CssRule::new(class, &[("touch-action", val)]))
}

fn resolve_will_change(class: &str) -> Option<CssRule> {
    let val = match class {
        "will-change-auto"      => "auto",
        "will-change-scroll"    => "scroll-position",
        "will-change-contents"  => "contents",
        "will-change-transform" => "transform",
        _ => return None,
    };
    Some(CssRule::new(class, &[("will-change", val)]))
}

fn resolve_appearance(class: &str) -> Option<CssRule> {
    let val = match class {
        "appearance-none" => "none",
        "appearance-auto" => "auto",
        _ => return None,
    };
    Some(CssRule::new(class, &[("appearance", val)]))
}

fn resolve_caret_color(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("caret-")?;
    if val == "transparent" {
        return Some(CssRule::new(class, &[("caret-color", "transparent")]));
    }
    if val == "current" {
        return Some(CssRule::new(class, &[("caret-color", "currentColor")]));
    }
    if let Some(color) = get_color_css_value(val) {
        return Some(CssRule::new(class, &[("caret-color", &color)]));
    }
    None
}

fn resolve_accent_color(class: &str) -> Option<CssRule> {
    if class == "accent-auto" {
        return Some(CssRule::new(class, &[("accent-color", "auto")]));
    }
    let val = class.strip_prefix("accent-")?;
    if let Some(color) = get_color_css_value(val) {
        return Some(CssRule::new(class, &[("accent-color", &color)]));
    }
    None
}

// ─── Tables (16) ──────────────────────────────────────────────────────────────

fn resolve_table(class: &str) -> Option<CssRule> {
    match class {
        "border-collapse" => Some(CssRule::new(class, &[("border-collapse", "collapse")])),
        "border-separate" => Some(CssRule::new(class, &[("border-collapse", "separate")])),
        "table-auto" => Some(CssRule::new(class, &[("table-layout", "auto")])),
        "table-fixed" => Some(CssRule::new(class, &[("table-layout", "fixed")])),
        "caption-top" => Some(CssRule::new(class, &[("caption-side", "top")])),
        "caption-bottom" => Some(CssRule::new(class, &[("caption-side", "bottom")])),
        _ => resolve_border_spacing(class),
    }
}

fn resolve_border_spacing(class: &str) -> Option<CssRule> {
    // Check x/y first (border-spacing-x-2 would otherwise match border-spacing- with "x-2")
    if let Some(v) = class.strip_prefix("border-spacing-x-") {
        let sp = resolve_spacing_or_arbitrary(v)?;
        return Some(CssRule::new(class, &[("border-spacing", &format!("{} 0", sp))]));
    }
    if let Some(v) = class.strip_prefix("border-spacing-y-") {
        let sp = resolve_spacing_or_arbitrary(v)?;
        return Some(CssRule::new(class, &[("border-spacing", &format!("0 {}", sp))]));
    }
    if let Some(v) = class.strip_prefix("border-spacing-") {
        let sp = resolve_spacing_or_arbitrary(v)?;
        return Some(CssRule::new(class, &[("border-spacing", &sp)]));
    }
    None
}

// ─── Content (18.1) ───────────────────────────────────────────────────────────

fn resolve_content(class: &str) -> Option<CssRule> {
    if class == "content-none" {
        return Some(CssRule::new(class, &[("content", "none")]));
    }
    let val = class.strip_prefix("content-")?;
    if val.starts_with('[') && val.ends_with(']') {
        let inner = val[1..val.len() - 1].replace('_', " ");
        return Some(CssRule::new(class, &[("content", &inner)]));
    }
    None
}

// ─── Overscroll (18.2) ────────────────────────────────────────────────────────

fn resolve_overscroll(class: &str) -> Option<CssRule> {
    let (prop, val) = match class {
        "overscroll-auto"    => ("overscroll-behavior", "auto"),
        "overscroll-contain" => ("overscroll-behavior", "contain"),
        "overscroll-none"    => ("overscroll-behavior", "none"),
        "overscroll-x-auto"    => ("overscroll-behavior-x", "auto"),
        "overscroll-x-contain" => ("overscroll-behavior-x", "contain"),
        "overscroll-x-none"    => ("overscroll-behavior-x", "none"),
        "overscroll-y-auto"    => ("overscroll-behavior-y", "auto"),
        "overscroll-y-contain" => ("overscroll-behavior-y", "contain"),
        "overscroll-y-none"    => ("overscroll-behavior-y", "none"),
        _ => return None,
    };
    Some(CssRule::new(class, &[(prop, val)]))
}

// ─── Background Image / Gradient (8.2) ───────────────────────────────────────

fn resolve_bg_image(class: &str) -> Option<CssRule> {
    if class == "bg-none" {
        return Some(CssRule::new(class, &[("background-image", "none")]));
    }
    if let Some(val) = class.strip_prefix("bg-linear-to-") {
        let direction = match val {
            "t"  => "to top",
            "tr" => "to top right",
            "r"  => "to right",
            "br" => "to bottom right",
            "b"  => "to bottom",
            "bl" => "to bottom left",
            "l"  => "to left",
            "tl" => "to top left",
            _ => return None,
        };
        let gradient = format!("linear-gradient({}, var(--tw-gradient-stops))", direction);
        return Some(CssRule::dynamic(class, vec![("background-image".into(), gradient)]));
    }
    let gradient = match class {
        "bg-radial" => "radial-gradient(var(--tw-gradient-stops))",
        "bg-conic"  => "conic-gradient(var(--tw-gradient-stops))",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-image", gradient)]))
}

// ─── Background Size (8.3) ──────────────────────────────────────────────────

fn resolve_bg_size(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-")?;
    let size = match val {
        "auto"   => "auto",
        "cover"  => "cover",
        "contain" => "contain",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-size", size)]))
}

// ─── Background Position (8.4) ───────────────────────────────────────────────

fn resolve_bg_position(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-")?;
    let pos = match val {
        "center"      => "center",
        "top"         => "top",
        "bottom"      => "bottom",
        "left"        => "left",
        "right"       => "right",
        "left-top"    => "left top",
        "left-bottom" => "left bottom",
        "right-top"   => "right top",
        "right-bottom" => "right bottom",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-position", pos)]))
}

// ─── Background Repeat (8.5) ─────────────────────────────────────────────────

fn resolve_bg_repeat(class: &str) -> Option<CssRule> {
    if class == "bg-no-repeat" {
        return Some(CssRule::new(class, &[("background-repeat", "no-repeat")]));
    }
    let val = class.strip_prefix("bg-repeat")?;
    let repeat = match val {
        ""       => "repeat",
        "-x"     => "repeat-x",
        "-y"     => "repeat-y",
        "-round" => "round",
        "-space" => "space",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-repeat", repeat)]))
}

// ─── Background Attachment (8.6) ─────────────────────────────────────────────

fn resolve_bg_attachment(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-")?;
    let attachment = match val {
        "fixed"  => "fixed",
        "local"  => "local",
        "scroll" => "scroll",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-attachment", attachment)]))
}

// ─── Background Clip (8.7) ──────────────────────────────────────────────────

fn resolve_bg_clip(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-clip-")?;
    let clip = match val {
        "border"  => "border-box",
        "padding" => "padding-box",
        "content" => "content-box",
        "text"    => "text",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-clip", clip)]))
}

// ─── Background Origin (8.8) ────────────────────────────────────────────────

fn resolve_bg_origin(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-origin-")?;
    let origin = match val {
        "border"  => "border-box",
        "padding" => "padding-box",
        "content" => "content-box",
        _ => return None,
    };
    Some(CssRule::new(class, &[("background-origin", origin)]))
}

// ─── Gradient Stops (from-, via-, to-) ───────────────────────────────────────

fn resolve_gradient_stops(class: &str) -> Option<CssRule> {
    if let Some(val) = class.strip_prefix("from-") {
        // from-{color} or from-{n}%
        if let Some(pct) = val.strip_suffix('%') {
            if pct.parse::<u8>().ok().map_or(false, |n| n <= 100) {
                return Some(CssRule::new(class, &[("--tw-gradient-from-position", &format!("{}%", pct))]));
            }
        }
        if let Some(color) = get_color_css_value(val) {
            let transparent = color_to_transparent_rgb(&color);
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("--tw-gradient-from".into(), color),
                    ("--tw-gradient-to".into(), transparent),
                    ("--tw-gradient-stops".into(), "var(--tw-gradient-from), var(--tw-gradient-to)".into()),
                ],
            ));
        }
    }
    if let Some(val) = class.strip_prefix("via-") {
        if let Some(pct) = val.strip_suffix('%') {
            if pct.parse::<u8>().ok().map_or(false, |n| n <= 100) {
                return Some(CssRule::new(class, &[("--tw-gradient-via-position", &format!("{}%", pct))]));
            }
        }
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::dynamic(
                class,
                vec![
                    ("--tw-gradient-via".into(), color),
                    ("--tw-gradient-stops".into(), "var(--tw-gradient-from), var(--tw-gradient-via), var(--tw-gradient-to)".into()),
                ],
            ));
        }
    }
    if let Some(val) = class.strip_prefix("to-") {
        if let Some(pct) = val.strip_suffix('%') {
            if pct.parse::<u8>().ok().map_or(false, |n| n <= 100) {
                return Some(CssRule::new(class, &[("--tw-gradient-to-position", &format!("{}%", pct))]));
            }
        }
        if let Some(color) = get_color_css_value(val) {
            return Some(CssRule::new(class, &[("--tw-gradient-to", &color)]));
        }
    }
    None
}

/// Returns the CSS color value for a Tailwind color token (e.g. "blue-500" → "#3b82f6").
fn get_color_css_value(val: &str) -> Option<String> {
    if let Some(c) = match val {
        "transparent" => Some("transparent".into()),
        "current" => Some("currentColor".into()),
        "inherit" => Some("inherit".into()),
        "black" => Some("#000000".into()),
        "white" => Some("#ffffff".into()),
        _ => None,
    } {
        return Some(c);
    }
    if val.starts_with('[') && val.ends_with(']') {
        return Some(val[1..val.len() - 1].replace('_', " "));
    }
    let (color_part, opacity) = if let Some(slash) = val.rfind('/') {
        (&val[..slash], Some(&val[slash + 1..]))
    } else {
        (val, None)
    };
    let last_dash = color_part.rfind('-')?;
    let color_name = &color_part[..last_dash];
    let shade_str = &color_part[last_dash + 1..];
    let shade: u16 = shade_str.parse().ok()?;
    let hex = colors::lookup(color_name, shade)?;
    let css_val = if let Some(op_str) = opacity {
        let op_pct: f64 = op_str.parse().ok()?;
        let op = op_pct / 100.0;
        if let Some((r, g, b)) = colors::hex_to_rgb(hex) {
            format!("rgba({}, {}, {}, {})", r, g, b, op)
        } else {
            hex.to_string()
        }
    } else {
        hex.to_string()
    };
    Some(css_val)
}

/// Converts a color to its transparent RGB form for gradient --tw-gradient-to.
fn color_to_transparent_rgb(color: &str) -> String {
    if color.starts_with('#') && color.len() == 7 {
        if let Some((r, g, b)) = colors::hex_to_rgb(color) {
            return format!("rgb({} {} {} / 0)", r, g, b);
        }
    }
    if color.starts_with("rgba(") {
        if let Some(paren) = color.rfind(')') {
            let inner = &color[5..paren];
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() == 4 {
                return format!("rgba({}, {}, {}, 0)", parts[0], parts[1], parts[2]);
            }
        }
    }
    "transparent".into()
}

// ─── Colors ─────────────────────────────────────────────────────────────────

fn resolve_bg_color(class: &str) -> Option<CssRule> {
    let val = class.strip_prefix("bg-")?;
    resolve_color_value(val, class, "background-color")
}

fn resolve_text_color(class: &str) -> Option<CssRule> {
    // text-{color} sets the CSS `color` property.
    // Avoid matching text-xs/sm/base/etc. (font-size, Phase 2)
    let val = class.strip_prefix("text-")?;

    // Skip font-size classes so they don't get misinterpreted as colors
    match val {
        "xs" | "sm" | "base" | "lg" | "xl" | "2xl" | "3xl" | "4xl" | "5xl"
        | "6xl" | "7xl" | "8xl" | "9xl" => return None,
        // text-align (Phase 2)
        "left" | "center" | "right" | "justify" | "start" | "end" => return None,
        // text-wrap (Phase 2)
        "wrap" | "nowrap" | "balance" | "pretty" => return None,
        // text-overflow (Phase 2)
        "ellipsis" | "clip" => return None,
        _ => {}
    }

    resolve_color_value(val, class, "color")
}

/// Shared color resolution for both `bg-` and `text-` prefixes.
fn resolve_color_value(val: &str, class: &str, property: &str) -> Option<CssRule> {
    // Special flat colors
    match val {
        "transparent" => {
            return Some(CssRule::new(class, &[(property, "transparent")]));
        }
        "current" => {
            return Some(CssRule::new(class, &[(property, "currentColor")]));
        }
        "inherit" => {
            return Some(CssRule::new(class, &[(property, "inherit")]));
        }
        "black" => {
            return Some(CssRule::new(class, &[(property, "#000000")]));
        }
        "white" => {
            return Some(CssRule::new(class, &[(property, "#ffffff")]));
        }
        _ => {}
    }

    // Arbitrary: bg-[#ff0000] or bg-[rgb(255,0,0)]
    if val.starts_with('[') && val.ends_with(']') {
        let inner = val[1..val.len() - 1].replace('_', " ");
        return Some(CssRule::dynamic(
            class,
            vec![(property.to_string(), inner)],
        ));
    }

    // Opacity modifier: "blue-500/50" or "blue-500/75"
    let (color_part, opacity) = if let Some(slash) = val.rfind('/') {
        (&val[..slash], Some(&val[slash + 1..]))
    } else {
        (val, None)
    };

    // Parse "color-shade" e.g. "blue-500"
    let last_dash = color_part.rfind('-')?;
    let color_name = &color_part[..last_dash];
    let shade_str = &color_part[last_dash + 1..];
    let shade: u16 = shade_str.parse().ok()?;

    let hex = colors::lookup(color_name, shade)?;

    let css_val = if let Some(op_str) = opacity {
        let op_pct: f64 = op_str.parse().ok()?;
        let op = op_pct / 100.0;
        if let Some((r, g, b)) = colors::hex_to_rgb(hex) {
            format!("rgba({}, {}, {}, {})", r, g, b, op)
        } else {
            hex.to_string()
        }
    } else {
        hex.to_string()
    };

    Some(CssRule::dynamic(class, vec![(property.to_string(), css_val)]))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Resolves a value that can be a spacing scale token or an arbitrary `[value]`.
fn resolve_spacing_or_arbitrary(val: &str) -> Option<String> {
    if val.starts_with('[') && val.ends_with(']') {
        return Some(val[1..val.len() - 1].replace('_', " "));
    }
    spacing::spacing_value(val)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_classes() {
        let rule = resolve_class("flex").unwrap();
        assert_eq!(rule.selector, ".flex");
        assert_eq!(rule.declarations, vec![("display".into(), "flex".into())]);

        let rule = resolve_class("hidden").unwrap();
        assert_eq!(rule.declarations, vec![("display".into(), "none".into())]);
    }

    #[test]
    fn flex_direction() {
        let rule = resolve_class("flex-col").unwrap();
        assert_eq!(rule.declarations, vec![("flex-direction".into(), "column".into())]);
    }

    #[test]
    fn padding_scale() {
        let rule = resolve_class("p-4").unwrap();
        assert_eq!(rule.selector, ".p-4");
        assert_eq!(rule.declarations, vec![("padding".into(), "1rem".into())]);
    }

    #[test]
    fn padding_directional() {
        let rule = resolve_class("px-8").unwrap();
        assert!(rule.declarations.contains(&("padding-left".into(), "2rem".into())));
        assert!(rule.declarations.contains(&("padding-right".into(), "2rem".into())));
    }

    #[test]
    fn margin_auto() {
        let rule = resolve_class("mx-auto").unwrap();
        assert!(rule.declarations.contains(&("margin-left".into(), "auto".into())));
        assert!(rule.declarations.contains(&("margin-right".into(), "auto".into())));
    }

    #[test]
    fn negative_margin() {
        let rule = resolve_class("-m-4").unwrap();
        assert_eq!(rule.declarations, vec![("margin".into(), "-1rem".into())]);
    }

    #[test]
    fn width_fraction() {
        let rule = resolve_class("w-1/2").unwrap();
        assert_eq!(rule.selector, r".w-1\/2");
        assert_eq!(rule.declarations, vec![("width".into(), "50%".into())]);
    }

    #[test]
    fn width_full() {
        let rule = resolve_class("w-full").unwrap();
        assert_eq!(rule.declarations, vec![("width".into(), "100%".into())]);
    }

    #[test]
    fn bg_color() {
        let rule = resolve_class("bg-blue-500").unwrap();
        assert_eq!(rule.declarations, vec![("background-color".into(), "#3b82f6".into())]);
    }

    #[test]
    fn bg_color_with_opacity() {
        let rule = resolve_class("bg-blue-500/50").unwrap();
        let val = &rule.declarations[0].1;
        assert!(val.starts_with("rgba("), "expected rgba, got {}", val);
    }

    #[test]
    fn text_color() {
        let rule = resolve_class("text-white").unwrap();
        assert_eq!(rule.declarations, vec![("color".into(), "#ffffff".into())]);
    }

    #[test]
    fn gap() {
        let rule = resolve_class("gap-4").unwrap();
        assert_eq!(rule.declarations, vec![("gap".into(), "1rem".into())]);
    }

    #[test]
    fn space_x() {
        let rule = resolve_class("space-x-4").unwrap();
        assert!(rule.selector.contains("> * + *"));
        assert_eq!(rule.declarations, vec![("margin-left".into(), "1rem".into())]);
    }

    #[test]
    fn arbitrary_width() {
        let rule = resolve_class("w-[100px]").unwrap();
        assert_eq!(rule.declarations, vec![("width".into(), "100px".into())]);
    }

    #[test]
    fn size_shorthand() {
        let rule = resolve_class("size-8").unwrap();
        assert_eq!(rule.declarations.len(), 2);
    }

    #[test]
    fn unknown_class_returns_none() {
        assert!(resolve_class("nonexistent-class-xyz").is_none());
    }

    #[test]
    fn font_sans() {
        let rule = resolve_class("font-sans").unwrap();
        assert_eq!(rule.selector, ".font-sans");
        assert!(rule.declarations[0].1.contains("ui-sans-serif"));
    }

    #[test]
    fn font_serif() {
        let rule = resolve_class("font-serif").unwrap();
        assert!(rule.declarations[0].1.contains("ui-serif"));
    }

    #[test]
    fn font_mono() {
        let rule = resolve_class("font-mono").unwrap();
        assert!(rule.declarations[0].1.contains("ui-monospace"));
    }

    #[test]
    fn font_arbitrary_inter() {
        let rule = resolve_class("font-[Inter]").unwrap();
        assert_eq!(rule.declarations, vec![("font-family".into(), "Inter".into())]);
    }

    #[test]
    fn font_arbitrary_with_spaces() {
        let rule = resolve_class("font-[Open_Sans]").unwrap();
        assert_eq!(rule.declarations, vec![("font-family".into(), "\"Open Sans\"".into())]);
    }

    #[test]
    fn text_xs() {
        let rule = resolve_class("text-xs").unwrap();
        assert_eq!(rule.declarations, vec![
            ("font-size".into(), "0.75rem".into()),
            ("line-height".into(), "1rem".into()),
        ]);
    }

    #[test]
    fn text_base() {
        let rule = resolve_class("text-base").unwrap();
        assert_eq!(rule.declarations, vec![
            ("font-size".into(), "1rem".into()),
            ("line-height".into(), "1.5rem".into()),
        ]);
    }

    #[test]
    fn text_5xl() {
        let rule = resolve_class("text-5xl").unwrap();
        assert_eq!(rule.declarations, vec![
            ("font-size".into(), "3rem".into()),
            ("line-height".into(), "1".into()),
        ]);
    }

    #[test]
    fn text_arbitrary() {
        let rule = resolve_class("text-[14px]").unwrap();
        assert_eq!(rule.declarations, vec![("font-size".into(), "14px".into())]);
    }

    #[test]
    fn font_weight() {
        let rule = resolve_class("font-bold").unwrap();
        assert_eq!(rule.declarations, vec![("font-weight".into(), "700".into())]);
        let rule = resolve_class("font-thin").unwrap();
        assert_eq!(rule.declarations, vec![("font-weight".into(), "100".into())]);
    }

    #[test]
    fn font_style() {
        let rule = resolve_class("italic").unwrap();
        assert_eq!(rule.declarations, vec![("font-style".into(), "italic".into())]);
        let rule = resolve_class("not-italic").unwrap();
        assert_eq!(rule.declarations, vec![("font-style".into(), "normal".into())]);
    }

    #[test]
    fn font_smoothing() {
        let rule = resolve_class("antialiased").unwrap();
        assert!(rule.declarations.contains(&("-webkit-font-smoothing".into(), "antialiased".into())));
        assert!(rule.declarations.contains(&("-moz-osx-font-smoothing".into(), "grayscale".into())));
        let rule = resolve_class("subpixel-antialiased").unwrap();
        assert!(rule.declarations.contains(&("-webkit-font-smoothing".into(), "auto".into())));
    }

    #[test]
    fn letter_spacing() {
        let rule = resolve_class("tracking-tight").unwrap();
        assert_eq!(rule.declarations, vec![("letter-spacing".into(), "-0.025em".into())]);
        let rule = resolve_class("tracking-wide").unwrap();
        assert_eq!(rule.declarations, vec![("letter-spacing".into(), "0.025em".into())]);
    }

    #[test]
    fn line_height() {
        let rule = resolve_class("leading-none").unwrap();
        assert_eq!(rule.declarations, vec![("line-height".into(), "1".into())]);
        let rule = resolve_class("leading-4").unwrap();
        assert_eq!(rule.declarations, vec![("line-height".into(), "1rem".into())]);
    }

    #[test]
    fn text_align() {
        let rule = resolve_class("text-center").unwrap();
        assert_eq!(rule.declarations, vec![("text-align".into(), "center".into())]);
        let rule = resolve_class("text-start").unwrap();
        assert_eq!(rule.declarations, vec![("text-align".into(), "start".into())]);
    }

    #[test]
    fn text_decoration() {
        let rule = resolve_class("underline").unwrap();
        assert_eq!(rule.declarations, vec![("text-decoration-line".into(), "underline".into())]);
        let rule = resolve_class("truncate").unwrap();
        assert!(rule.declarations.iter().any(|(k, _)| k == "text-overflow"));
    }

    #[test]
    fn line_clamp() {
        let rule = resolve_class("line-clamp-2").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "-webkit-line-clamp" && v == "2"));
    }

    #[test]
    fn bg_utilities() {
        let rule = resolve_class("bg-none").unwrap();
        assert_eq!(rule.declarations, vec![("background-image".into(), "none".into())]);
        let rule = resolve_class("bg-cover").unwrap();
        assert_eq!(rule.declarations, vec![("background-size".into(), "cover".into())]);
        let rule = resolve_class("bg-center").unwrap();
        assert_eq!(rule.declarations, vec![("background-position".into(), "center".into())]);
        let rule = resolve_class("bg-linear-to-r").unwrap();
        assert!(rule.declarations[0].1.contains("linear-gradient"));
        let rule = resolve_class("from-blue-500").unwrap();
        assert!(rule.declarations.iter().any(|(k, _)| k == "--tw-gradient-from"));
    }

    #[test]
    fn border_utilities() {
        let rule = resolve_class("border").unwrap();
        assert_eq!(rule.declarations, vec![("border-width".into(), "1px".into())]);
        let rule = resolve_class("border-2").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "border-top-width" && v == "2px"));
        let rule = resolve_class("rounded-lg").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "border-top-left-radius" && v == "0.5rem"));
        let rule = resolve_class("ring-2").unwrap();
        assert!(rule.declarations.iter().any(|(k, _)| k == "box-shadow"));
    }

    #[test]
    fn effects_utilities() {
        let rule = resolve_class("shadow-md").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "box-shadow" && v.contains("4px")));
        let rule = resolve_class("opacity-50").unwrap();
        assert_eq!(rule.declarations, vec![("opacity".into(), "0.5".into())]);
        let rule = resolve_class("mix-blend-multiply").unwrap();
        assert_eq!(rule.declarations, vec![("mix-blend-mode".into(), "multiply".into())]);
    }

    #[test]
    fn filter_utilities() {
        let rule = resolve_class("blur-sm").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-blur" && v == "blur(4px)"));
        assert!(rule.declarations.iter().any(|(k, _)| k == "filter"));
        let rule = resolve_class("blur-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-blur" && v == "blur(0)"));
        let rule = resolve_class("brightness-50").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-brightness" && v == "brightness(0.5)"));
        let rule = resolve_class("grayscale").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-grayscale" && v == "grayscale(1)"));
        let rule = resolve_class("drop-shadow-lg").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-drop-shadow" && v.contains("drop-shadow")));
    }

    #[test]
    fn backdrop_filter_utilities() {
        let rule = resolve_class("backdrop-blur-md").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-backdrop-blur" && v == "blur(12px)"));
        assert!(rule.declarations.iter().any(|(k, _)| k == "backdrop-filter"));
        let rule = resolve_class("backdrop-opacity-50").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-backdrop-opacity" && v == "opacity(0.5)"));
    }

    #[test]
    fn transition_utilities() {
        let rule = resolve_class("transition-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "transition-property" && v == "none"));
        let rule = resolve_class("transition-colors").unwrap();
        assert!(rule.declarations.iter().any(|(k, _)| k == "transition-property"));
        assert!(rule.declarations.iter().any(|(k, _)| k == "transition-duration"));
        let rule = resolve_class("duration-300").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "transition-duration" && v == "300ms"));
        let rule = resolve_class("ease-in-out").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "transition-timing-function" && v.contains("cubic-bezier")));
        let rule = resolve_class("delay-150").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "transition-delay" && v == "150ms"));
    }

    #[test]
    fn animation_utilities() {
        let rule = resolve_class("animate-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "animation" && v == "none"));
        let rule = resolve_class("animate-spin").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "animation" && v.contains("spin")));
        assert!(rule.keyframes.as_ref().unwrap().contains("@keyframes spin"));
    }

    #[test]
    fn transform_utilities() {
        let rule = resolve_class("scale-50").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-scale-x" && v == "0.5"));
        assert!(rule.declarations.iter().any(|(k, _)| k == "transform"));
        let rule = resolve_class("rotate-45").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-rotate" && v == "45deg"));
        let rule = resolve_class("translate-x-4").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-translate-x" && v == "1rem"));
        let rule = resolve_class("translate-x-1/2").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-translate-x" && v == "50%"));
        let rule = resolve_class("skew-x-2").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "--tw-skew-x" && v == "2deg"));
        let rule = resolve_class("origin-center").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "transform-origin" && v == "center"));
        let rule = resolve_class("perspective-normal").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "perspective" && v == "500px"));
    }

    #[test]
    fn interactivity_utilities() {
        let rule = resolve_class("cursor-pointer").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "cursor" && v == "pointer"));
        let rule = resolve_class("pointer-events-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "pointer-events" && v == "none"));
        let rule = resolve_class("resize-x").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "resize" && v == "x"));
        let rule = resolve_class("select-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "user-select" && v == "none"));
        let rule = resolve_class("scroll-smooth").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "scroll-behavior" && v == "smooth"));
        let rule = resolve_class("scroll-m-4").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "scroll-margin" && v == "1rem"));
        let rule = resolve_class("snap-x").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "scroll-snap-type" && v.contains("x")));
        let rule = resolve_class("caret-blue-500").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "caret-color" && v.contains("#")));
        let rule = resolve_class("accent-auto").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "accent-color" && v == "auto"));
    }

    #[test]
    fn table_utilities() {
        let rule = resolve_class("border-collapse").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "border-collapse" && v == "collapse"));
        let rule = resolve_class("table-fixed").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "table-layout" && v == "fixed"));
        let rule = resolve_class("border-spacing-4").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "border-spacing" && v == "1rem"));
        let rule = resolve_class("border-spacing-x-2").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "border-spacing" && v.contains("0.5rem")));
        let rule = resolve_class("caption-bottom").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "caption-side" && v == "bottom"));
    }

    #[test]
    fn content_and_overscroll_utilities() {
        let rule = resolve_class("content-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "content" && v == "none"));
        let rule = resolve_class(r"content-['']").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "content" && v == "''"));
        let rule = resolve_class("overscroll-contain").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "overscroll-behavior" && v == "contain"));
        let rule = resolve_class("overscroll-x-none").unwrap();
        assert!(rule.declarations.iter().any(|(k, v)| k == "overscroll-behavior-x" && v == "none"));
    }

    #[test]
    fn variant_utilities() {
        let rule = resolve_class("hover:bg-blue-500").unwrap();
        assert!(rule.selector.contains(":hover"));
        assert!(rule.declarations.iter().any(|(k, v)| k == "background-color" && v == "#3b82f6"));
        let rule = resolve_class("sm:p-4").unwrap();
        assert!(rule.media_query.as_ref().unwrap().contains("min-width"));
        let rule = resolve_class("dark:text-white").unwrap();
        assert!(rule.media_query.as_ref().unwrap().contains("prefers-color-scheme"));
        let rule = resolve_class("before:content-['']").unwrap();
        assert!(rule.selector.contains("::before"));
        let rule = resolve_class("group-hover:opacity-50").unwrap();
        assert!(rule.selector.contains(".group"));
    }

    #[test]
    fn escape_selector_fraction() {
        assert_eq!(escape_selector("w-1/2"), r"w-1\/2");
    }

    #[test]
    fn escape_selector_decimal() {
        assert_eq!(escape_selector("p-0.5"), r"p-0\.5");
    }
}
