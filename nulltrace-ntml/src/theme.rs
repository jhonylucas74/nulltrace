use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme configuration with reusable design tokens
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "borderRadius")]
    pub border_radius: Option<HashMap<String, f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typography: Option<HashMap<String, f64>>,
}

impl Theme {
    /// Create a new empty theme
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a theme variable reference (e.g., "$theme.colors.primary")
    pub fn resolve(&self, reference: &str) -> Option<String> {
        if !reference.starts_with("$theme.") {
            return None;
        }

        let parts: Vec<&str> = reference.trim_start_matches("$theme.").split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        let category = parts[0];
        let key = parts[1];

        match category {
            "colors" => self
                .colors
                .as_ref()
                .and_then(|c| c.get(key))
                .map(|v| v.clone()),
            "spacing" => self
                .spacing
                .as_ref()
                .and_then(|s| s.get(key))
                .map(|v| v.to_string()),
            "borderRadius" => self
                .border_radius
                .as_ref()
                .and_then(|b| b.get(key))
                .map(|v| v.to_string()),
            "typography" => self
                .typography
                .as_ref()
                .and_then(|t| t.get(key))
                .map(|v| v.to_string()),
            _ => None,
        }
    }

    /// Check if a string is a theme variable reference
    pub fn is_theme_reference(value: &str) -> bool {
        value.starts_with("$theme.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_resolve_colors() {
        let mut theme = Theme::new();
        let mut colors = HashMap::new();
        colors.insert("primary".to_string(), "#4a90e2".to_string());
        colors.insert("danger".to_string(), "#ff6b6b".to_string());
        theme.colors = Some(colors);

        assert_eq!(
            theme.resolve("$theme.colors.primary"),
            Some("#4a90e2".to_string())
        );
        assert_eq!(
            theme.resolve("$theme.colors.danger"),
            Some("#ff6b6b".to_string())
        );
        assert_eq!(theme.resolve("$theme.colors.unknown"), None);
    }

    #[test]
    fn test_theme_resolve_spacing() {
        let mut theme = Theme::new();
        let mut spacing = HashMap::new();
        spacing.insert("small".to_string(), 8.0);
        spacing.insert("medium".to_string(), 16.0);
        theme.spacing = Some(spacing);

        assert_eq!(
            theme.resolve("$theme.spacing.small"),
            Some("8".to_string())
        );
        assert_eq!(
            theme.resolve("$theme.spacing.medium"),
            Some("16".to_string())
        );
    }

    #[test]
    fn test_is_theme_reference() {
        assert!(Theme::is_theme_reference("$theme.colors.primary"));
        assert!(Theme::is_theme_reference("$theme.spacing.medium"));
        assert!(!Theme::is_theme_reference("#4a90e2"));
        assert!(!Theme::is_theme_reference("16"));
    }
}
