use serde::{Deserialize, Serialize};

/// Head section of an NTML v0.2.0 document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Head {
    /// Page title — required when head is present
    pub title: String,
    /// Optional page description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional author name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional tags for indexing (max 10, lowercase, no spaces)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Fonts to import from Google Fonts (max 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fonts: Option<Vec<FontImport>>,
    /// Lua scripts to load (max 5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<Vec<ScriptImport>>,
    /// External NTML component files to import (max 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<ComponentImport>>,
}

/// A Google Fonts import declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontImport {
    /// Font family name (e.g., "Roboto Mono")
    pub family: String,
    /// Font weights to load (e.g., [400, 700])
    pub weights: Vec<u16>,
}

/// A Lua script import declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptImport {
    /// Path to the Lua script file (must end with .lua)
    pub src: String,
}

/// An external NTML component import declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentImport {
    /// Path to the component .ntml file
    pub src: String,
    /// PascalCase alias to use in the document body
    #[serde(rename = "as")]
    pub alias: String,
}

impl Head {
    /// Returns all declared font family names
    pub fn font_families(&self) -> Vec<String> {
        self.fonts
            .as_ref()
            .map(|fonts| fonts.iter().map(|f| f.family.clone()).collect())
            .unwrap_or_default()
    }
}
