//! Variant modifiers (responsive, hover, focus, dark, etc.)
//!
//! Parses variant prefixes from class names and applies the appropriate
//! selector or media query transformation.

use super::registry::CssRule;

/// Variant prefixes that require the longest match first (e.g. group-hover before hover).
const PSEUDO_CLASS_VARIANTS: &[(&str, &str)] = &[
    ("group-hover:", ":hover"),
    ("group-focus:", ":focus"),
    ("group-active:", ":active"),
    ("group-disabled:", ":disabled"),
    ("group-focus-within:", ":focus-within"),
    ("peer-hover:", ":hover"),
    ("peer-focus:", ":focus"),
    ("peer-checked:", ":checked"),
    ("peer-disabled:", ":disabled"),
    ("peer-focus-within:", ":focus-within"),
    ("focus-within:", ":focus-within"),
    ("focus-visible:", ":focus-visible"),
    ("hover:", ":hover"),
    ("focus:", ":focus"),
    ("active:", ":active"),
    ("visited:", ":visited"),
    ("disabled:", ":disabled"),
    ("enabled:", ":enabled"),
    ("checked:", ":checked"),
    ("indeterminate:", ":indeterminate"),
    ("required:", ":required"),
    ("valid:", ":valid"),
    ("invalid:", ":invalid"),
    ("placeholder:", "::placeholder"),
    ("first:", ":first-child"),
    ("last:", ":last-child"),
    ("only:", ":only-child"),
    ("odd:", ":nth-child(odd)"),
    ("even:", ":nth-child(even)"),
    ("empty:", ":empty"),
];

const PSEUDO_ELEMENT_VARIANTS: &[(&str, &str)] = &[
    ("before:", "::before"),
    ("after:", "::after"),
    ("selection:", "::selection"),
    ("first-line:", "::first-line"),
    ("first-letter:", "::first-letter"),
    ("marker:", "::marker"),
];

/// Breakpoints: (min-width) for sm:, md:, etc.
const BREAKPOINTS: &[(&str, &str)] = &[
    ("2xl:", "96rem"),   // 1536px
    ("xl:", "80rem"),    // 1280px
    ("lg:", "64rem"),    // 1024px
    ("md:", "48rem"),    // 768px
    ("sm:", "40rem"),    // 640px
];

/// Max-width breakpoints for max-sm:, max-md:, etc.
const MAX_BREAKPOINTS: &[(&str, &str)] = &[
    ("max-2xl:", "89.9375rem"),  // 1535px
    ("max-xl:", "79.9375rem"),   // 1279px
    ("max-lg:", "63.9375rem"),   // 1023px
    ("max-md:", "47.9375rem"),   // 767px
    ("max-sm:", "39.9375rem"),   // 639px
];

/// Container query breakpoints (@sm:, @md:, etc.)
const CONTAINER_BREAKPOINTS: &[(&str, &str)] = &[
    ("@7xl:", "80rem"),
    ("@6xl:", "72rem"),
    ("@5xl:", "64rem"),
    ("@4xl:", "56rem"),
    ("@3xl:", "48rem"),
    ("@2xl:", "42rem"),
    ("@xl:", "36rem"),
    ("@lg:", "32rem"),
    ("@md:", "28rem"),
    ("@sm:", "24rem"),
];

/// Parses a variant prefix from the class and returns (variant_prefix, base_class).
/// Checks longest prefixes first.
pub fn parse_variant(class: &str) -> Option<(&'static str, &str)> {
    if !class.contains(':') {
        return None;
    }
    // Group/peer need special handling: selector is .group:hover .group-hover\:bg-blue-500
    for (prefix, _) in PSEUDO_CLASS_VARIANTS {
        if class.starts_with(prefix) {
            let base = &class[prefix.len()..];
            if !base.is_empty() {
                return Some((prefix, base));
            }
        }
    }
    for (prefix, _) in PSEUDO_ELEMENT_VARIANTS {
        if class.starts_with(prefix) {
            let base = &class[prefix.len()..];
            if !base.is_empty() {
                return Some((prefix, base));
            }
        }
    }
    for (prefix, _) in MAX_BREAKPOINTS {
        if class.starts_with(prefix) {
            let base = &class[prefix.len()..];
            if !base.is_empty() {
                return Some((prefix, base));
            }
        }
    }
    for (prefix, _) in BREAKPOINTS {
        if class.starts_with(prefix) {
            let base = &class[prefix.len()..];
            if !base.is_empty() {
                return Some((prefix, base));
            }
        }
    }
    for (prefix, _) in CONTAINER_BREAKPOINTS {
        if class.starts_with(prefix) {
            let base = &class[prefix.len()..];
            if !base.is_empty() {
                return Some((prefix, base));
            }
        }
    }
    // Media query variants
    if class.starts_with("dark:") {
        let base = &class[5..];
        if !base.is_empty() {
            return Some(("dark:", base));
        }
    }
    if class.starts_with("motion-safe:") {
        let base = &class[11..];
        if !base.is_empty() {
            return Some(("motion-safe:", base));
        }
    }
    if class.starts_with("motion-reduce:") {
        let base = &class[12..];
        if !base.is_empty() {
            return Some(("motion-reduce:", base));
        }
    }
    if class.starts_with("print:") {
        let base = &class[6..];
        if !base.is_empty() {
            return Some(("print:", base));
        }
    }
    None
}

/// Applies the variant transformation to a rule. Replaces the base selector with
/// the variant-prefixed class and adds pseudo-class/element or wraps in media query.
/// Preserves compound selectors (e.g. .space-x-4 > * + *).
pub fn apply_variant(rule: CssRule, variant: &str, full_class: &str) -> CssRule {
    let escaped = escape_selector(full_class);
    let base_selector = &rule.selector;
    let compound_suffix = base_selector.find(' ').map(|i| &base_selector[i..]);

    if let Some(pseudo) = pseudo_class_for(variant) {
        let sel = if variant.starts_with("group-") {
            let base = format!(".group{} .{}", pseudo, escaped);
            compound_suffix.map_or(base.clone(), |s| base + s)
        } else if variant.starts_with("peer-") {
            let base = format!(".peer{} ~ .{}", pseudo, escaped);
            compound_suffix.map_or(base.clone(), |s| base + s)
        } else {
            let main = format!(".{}{}", escaped, pseudo);
            compound_suffix.map_or(main.clone(), |s| main + s)
        };
        return CssRule {
            selector: sel,
            declarations: rule.declarations,
            media_query: rule.media_query,
            keyframes: rule.keyframes,
        };
    }

    if let Some(pseudo) = pseudo_element_for(variant) {
        let main = format!(".{}{}", escaped, pseudo);
        let sel = compound_suffix.map_or(main.clone(), |s| main + s);
        return CssRule {
            selector: sel,
            declarations: rule.declarations,
            media_query: rule.media_query,
            keyframes: rule.keyframes,
        };
    }

    if let Some(mq) = media_query_for(variant) {
        let base = format!(".{}", escaped);
        let sel = compound_suffix.map_or(base.clone(), |s| base + s);
        return CssRule {
            selector: sel,
            declarations: rule.declarations,
            media_query: Some(mq.into()),
            keyframes: rule.keyframes,
        };
    }

    if let Some(cq) = container_query_for(variant) {
        let base = format!(".{}", escaped);
        let sel = compound_suffix.map_or(base.clone(), |s| base + s);
        return CssRule {
            selector: sel,
            declarations: rule.declarations,
            media_query: Some(cq.into()),
            keyframes: rule.keyframes,
        };
    }

    rule
}

fn pseudo_class_for(variant: &str) -> Option<&'static str> {
    PSEUDO_CLASS_VARIANTS
        .iter()
        .find(|(p, _)| *p == variant)
        .map(|(_, s)| *s)
}

fn pseudo_element_for(variant: &str) -> Option<&'static str> {
    PSEUDO_ELEMENT_VARIANTS
        .iter()
        .find(|(p, _)| *p == variant)
        .map(|(_, s)| *s)
}

fn media_query_for(variant: &str) -> Option<String> {
    for (prefix, width) in BREAKPOINTS {
        if variant == *prefix {
            return Some(format!("@media (min-width: {})", width));
        }
    }
    for (prefix, width) in MAX_BREAKPOINTS {
        if variant == *prefix {
            return Some(format!("@media (max-width: {})", width));
        }
    }
    match variant {
        "dark:" => Some("@media (prefers-color-scheme: dark)".into()),
        "motion-safe:" => Some("@media (prefers-reduced-motion: no-preference)".into()),
        "motion-reduce:" => Some("@media (prefers-reduced-motion: reduce)".into()),
        "print:" => Some("@media print".into()),
        _ => None,
    }
}

fn container_query_for(variant: &str) -> Option<String> {
    for (prefix, width) in CONTAINER_BREAKPOINTS {
        if variant == *prefix {
            return Some(format!("@container (min-width: {})", width));
        }
    }
    None
}

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
