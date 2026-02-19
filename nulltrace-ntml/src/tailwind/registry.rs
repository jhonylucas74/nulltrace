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
        }
    }

    fn dynamic(class: &str, props: Vec<(String, String)>) -> Self {
        Self {
            selector: format!(".{}", escape_selector(class)),
            declarations: props,
            media_query: None,
        }
    }

    fn with_compound_selector(selector: String, props: Vec<(String, String)>) -> Self {
        Self {
            selector,
            declarations: props,
            media_query: None,
        }
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
pub fn resolve_class(class: &str) -> Option<CssRule> {
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
    fn escape_selector_fraction() {
        assert_eq!(escape_selector("w-1/2"), r"w-1\/2");
    }

    #[test]
    fn escape_selector_decimal() {
        assert_eq!(escape_selector("p-0.5"), r"p-0\.5");
    }
}
