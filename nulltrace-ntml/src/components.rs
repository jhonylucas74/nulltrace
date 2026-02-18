use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::style::Style;

pub type DataAttributes = HashMap<String, String>;

/// Root component type - can be any NTML component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Component {
    Container(Container),
    Flex(Flex),
    Grid(Grid),
    Stack(Stack),
    Row(Row),
    Column(Column),
    Text(Text),
    Image(Image),
    Icon(Icon),
    Button(Button),
    Input(Input),
    Checkbox(Checkbox),
    Radio(Radio),
    Select(Select),
    ProgressBar(ProgressBar),
    Badge(Badge),
    Divider(Divider),
    Spacer(Spacer),
    Link(Link),
    /// An instance of an imported component declared in head.imports (v0.2.0)
    ImportedComponent(ImportedComponentInstance),
}

/// An instance of an imported NTML component
///
/// Used when the body references a component alias declared in head.imports.
/// The props are raw values to be resolved by the runtime against the component's prop definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedComponentInstance {
    /// Optional id for Lua integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The component alias (PascalCase, e.g., "NavBar")
    pub name: String,
    /// Props passed to the component (key = prop name, value = raw YAML value)
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub props: HashMap<String, serde_yaml::Value>,
}

/// Container component - basic rectangular container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Container {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Flex component - flexible box layout
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flex {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<FlexDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<JustifyContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<AlignItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

/// Grid component - grid layout
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub columns: GridSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<GridSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<GridGap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GridSize {
    Count(usize),
    Definitions(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GridGap {
    Single(f64),
    Separate { row: f64, column: f64 },
}

/// Stack component - layers children on top of each other
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stack {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<StackAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StackAlignment {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Row component - shorthand for Flex with direction: row
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<JustifyContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<AlignItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Column component - shorthand for Flex with direction: column
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<JustifyContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<AlignItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Text component - displays text content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Image component - displays an image
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<ImageFit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageFit {
    Cover,
    Contain,
    Fill,
    None,
    ScaleDown,
}

/// Icon component - displays an icon
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icon {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Link component - hyperlink that navigates the Browser or opens in new tab
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<LinkTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkTarget {
    Same,
    New,
}

/// Button component - clickable button
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Button {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ButtonVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

/// Input component - text input field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub input_type: Option<InputType>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Text,
    Password,
    Number,
}

/// Checkbox component - checkbox input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkbox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Radio component - radio button input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Radio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

/// Select component - dropdown select
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Select {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// ProgressBar component - displays progress/health/mana bars
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressBar {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ProgressBarVariant>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "showLabel")]
    pub show_label: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressBarVariant {
    Default,
    Success,
    Warning,
    Danger,
}

/// Badge component - displays a small badge or label
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Badge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<BadgeVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BadgeVariant {
    Default,
    Primary,
    Success,
    Warning,
    Danger,
}

/// Divider component - horizontal or vertical divider line
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<DividerOrientation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

/// Spacer component - flexible empty space
/// Note: Spacer does not have an `id` field per the v0.2.0 spec
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spacer {
    pub size: SpacerSize,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub data: DataAttributes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpacerSize {
    Fixed(f64),
    Auto(String), // "auto"
}
