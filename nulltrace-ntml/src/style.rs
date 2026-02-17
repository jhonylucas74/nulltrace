use serde::{Deserialize, Serialize};

/// A dimension value that can be either a number (pixels) or "auto"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dimension {
    Pixels(f64),
    Auto,
}

/// Shadow offset configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowOffset {
    pub x: f64,
    pub y: f64,
}

/// Shadow configuration (custom or preset)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Shadow {
    Preset(ShadowPreset),
    Custom {
        #[serde(rename = "shadowColor")]
        color: String,
        #[serde(rename = "shadowOffset")]
        offset: ShadowOffset,
        #[serde(rename = "shadowBlur")]
        blur: f64,
        #[serde(rename = "shadowOpacity")]
        opacity: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShadowPreset {
    Small,
    Medium,
    Large,
}

/// Complete style properties for NTML components
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Style {
    // Dimension properties
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,

    // Padding properties
    pub padding: Option<f64>,
    pub padding_vertical: Option<f64>,
    pub padding_horizontal: Option<f64>,
    pub padding_top: Option<f64>,
    pub padding_right: Option<f64>,
    pub padding_bottom: Option<f64>,
    pub padding_left: Option<f64>,

    // Margin properties
    pub margin: Option<f64>,
    pub margin_vertical: Option<f64>,
    pub margin_horizontal: Option<f64>,
    pub margin_top: Option<f64>,
    pub margin_right: Option<f64>,
    pub margin_bottom: Option<f64>,
    pub margin_left: Option<f64>,

    // Color properties
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub opacity: Option<f64>,

    // Typography properties
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub font_family: Option<FontFamily>,
    pub text_align: Option<TextAlign>,
    pub text_transform: Option<TextTransform>,
    pub letter_spacing: Option<f64>,
    pub line_height: Option<f64>,
    pub text_decoration: Option<TextDecoration>,

    // Border properties
    pub border_width: Option<f64>,
    pub border_top_width: Option<f64>,
    pub border_right_width: Option<f64>,
    pub border_bottom_width: Option<f64>,
    pub border_left_width: Option<f64>,
    pub border_style: Option<BorderStyle>,
    pub border_radius: Option<f64>,
    pub border_top_left_radius: Option<f64>,
    pub border_top_right_radius: Option<f64>,
    pub border_bottom_left_radius: Option<f64>,
    pub border_bottom_right_radius: Option<f64>,

    // Shadow properties
    pub shadow: Option<Shadow>,

    // Position properties
    pub position: Option<Position>,
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
    pub z_index: Option<i32>,

    // Flex item properties
    pub flex: Option<f64>,
    pub align_self: Option<Alignment>,

    // Display properties
    pub display: Option<Display>,
    pub overflow: Option<Overflow>,

    // Cursor property
    pub cursor: Option<Cursor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FontWeight {
    Number(u16),
    Named(FontWeightNamed),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeightNamed {
    Normal,
    Bold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontFamily {
    Sans,
    Serif,
    Monospace,
    Game,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecoration {
    None,
    Underline,
    #[serde(rename = "line-through")]
    LineThrough,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    Flex,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cursor {
    Default,
    Pointer,
    NotAllowed,
    Text,
}
