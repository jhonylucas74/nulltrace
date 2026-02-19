use std::collections::HashSet;

/// Extracts all unique CSS class tokens from an HTML string.
///
/// Scans for `class="..."` and `class='...'` attributes and returns
/// each whitespace-separated token exactly once, in order of first appearance.
pub fn extract_classes(html: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut classes = Vec::new();

    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Ensure we never slice at a byte inside a multi-byte UTF-8 character
        while i < len && !html.is_char_boundary(i) {
            i += 1;
        }
        if i >= len {
            break;
        }
        // Look for the literal sequence "class"
        if html[i..].starts_with("class") {
            let after = i + 5;
            // Skip optional whitespace, then expect '='
            let mut j = after;
            while j < len && bytes[j] == b' ' {
                j += 1;
            }
            if j < len && bytes[j] == b'=' {
                j += 1;
                // Skip optional whitespace
                while j < len && bytes[j] == b' ' {
                    j += 1;
                }
                // Expect opening quote
                if j < len && (bytes[j] == b'"' || bytes[j] == b'\'') {
                    let quote = bytes[j];
                    j += 1;
                    let start = j;
                    // Find closing quote
                    while j < len && bytes[j] != quote {
                        j += 1;
                    }
                    let value = &html[start..j];
                    for token in value.split_whitespace() {
                        if seen.insert(token.to_string()) {
                            classes.push(token.to_string());
                        }
                    }
                    i = j + 1;
                    continue;
                }
            }
        }
        i += 1;
    }

    classes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_double_quoted() {
        let html = r#"<div class="flex p-4 text-white"></div>"#;
        assert_eq!(extract_classes(html), vec!["flex", "p-4", "text-white"]);
    }

    #[test]
    fn deduplicates_across_elements() {
        let html = r#"<div class="flex p-4"><span class="flex text-sm"></span></div>"#;
        let classes = extract_classes(html);
        assert_eq!(classes, vec!["flex", "p-4", "text-sm"]);
    }

    #[test]
    fn handles_single_quoted() {
        let html = "<div class='bg-blue-500 rounded'></div>";
        assert_eq!(extract_classes(html), vec!["bg-blue-500", "rounded"]);
    }

    #[test]
    fn returns_empty_for_no_classes() {
        let html = "<div id='foo'></div>";
        assert!(extract_classes(html).is_empty());
    }

    #[test]
    fn handles_utf8_multibyte_in_html() {
        // Em dash (—) is 3 bytes in UTF-8; parser must not slice at byte boundaries inside it
        let html = r#"<div class="flex">Hello — world</div>"#;
        assert_eq!(extract_classes(html), vec!["flex"]);
    }
}
