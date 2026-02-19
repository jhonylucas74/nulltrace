//! # NullTrace Tailwind Engine — Phase 1
//!
//! A Tailwind CSS-compatible utility class generator implemented in Rust.
//! Scans HTML for `class="..."` attributes, resolves each recognised class
//! to its CSS rule, and returns a CSS string ready to inject into `<style>`.
//!
//! ## Covered in Phase 1
//! - **Display** — block, inline-block, inline, flex, inline-flex, grid, hidden, …
//! - **Flexbox** — flex-row/col, flex-wrap, grow, shrink, order, basis
//! - **Alignment & Gap** — justify-*, items-*, self-*, content-*, place-*, gap-*, space-x/y-*
//! - **Spacing** — p-*, px-*, py-*, pt/r/b/l-*, m-*, mx-*, my-*, mt/r/b/l-*, negative margins
//! - **Sizing** — w-*, h-*, min-w-*, max-w-*, min-h-*, max-h-*, size-*
//! - **Colors** — bg-{color}-{shade}, text-{color}-{shade}, opacity modifiers (/50)
//!
//! ## Usage
//! ```ignore
//! let css = nulltrace_ntml::tailwind::generate_css(&rendered_html);
//! // inject `css` into <style> tag in the Browser component
//! ```

pub mod colors;
pub mod parser;
pub mod registry;
pub mod spacing;

pub use registry::CssRule;

/// Scan `html` for class attributes, resolve every recognised Tailwind utility
/// class, and return the resulting CSS string.
///
/// Includes a minimal base reset (`box-sizing: border-box`) as preamble.
pub fn generate_css(html: &str) -> String {
    let classes = parser::extract_classes(html);
    let refs: Vec<&str> = classes.iter().map(String::as_str).collect();
    generate_css_for_classes(&refs)
}

/// Resolve an explicit slice of class names to CSS.
///
/// Deduplicates classes and skips any that are not recognised.
pub fn generate_css_for_classes(classes: &[&str]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut rules: Vec<CssRule> = Vec::new();

    for &class in classes {
        if seen.insert(class) {
            if let Some(rule) = registry::resolve_class(class) {
                rules.push(rule);
            }
        }
    }

    render_css(&rules)
}

fn render_css(rules: &[CssRule]) -> String {
    if rules.is_empty() {
        return String::new();
    }

    let mut css = String::from(
        "/* NullTrace Tailwind Engine — Phase 1 */\n\
         *, *::before, *::after { box-sizing: border-box; }\n\n",
    );

    for rule in rules {
        match &rule.media_query {
            None => {
                css.push_str(&rule.selector);
                css.push_str(" {");
                for (prop, val) in &rule.declarations {
                    css.push(' ');
                    css.push_str(prop);
                    css.push_str(": ");
                    css.push_str(val);
                    css.push(';');
                }
                css.push_str(" }\n");
            }
            Some(mq) => {
                css.push_str(mq);
                css.push_str(" {\n  ");
                css.push_str(&rule.selector);
                css.push_str(" {");
                for (prop, val) in &rule.declarations {
                    css.push(' ');
                    css.push_str(prop);
                    css.push_str(": ");
                    css.push_str(val);
                    css.push(';');
                }
                css.push_str(" }\n}\n");
            }
        }
    }

    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pipeline() {
        let html = r#"<div class="flex flex-col p-4 bg-blue-500 text-white w-full"></div>"#;
        let css = generate_css(html);

        assert!(css.contains(".flex { display: flex; }"));
        assert!(css.contains(".flex-col { flex-direction: column; }"));
        assert!(css.contains(".p-4 { padding: 1rem; }"));
        assert!(css.contains(".bg-blue-500 { background-color: #3b82f6; }"));
        assert!(css.contains(".text-white { color: #ffffff; }"));
        assert!(css.contains(".w-full { width: 100%; }"));
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(generate_css("<div></div>"), "");
    }

    #[test]
    fn deduplication() {
        let html = r#"<div class="flex p-4"><span class="flex p-8"></span></div>"#;
        let css = generate_css(html);
        // "flex" appears once in CSS
        assert_eq!(css.matches(".flex {").count(), 1);
    }
}
