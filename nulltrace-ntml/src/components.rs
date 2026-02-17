use serde::{Deserialize, Serialize};
use crate::style::Style;

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
}

/// Container component - basic rectangular container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Container {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
}

/// Flex component - flexible box layout
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flex {
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
    pub columns: GridSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<GridSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<GridGap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
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
    pub alignment: Option<StackAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
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
}

/// Column component - shorthand for Flex with direction: column
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
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
}

/// Text component - displays text content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

/// Image component - displays an image
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<ImageFit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
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
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

/// Button component - clickable button
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Button {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ButtonVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Component>>,
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
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

/// Radio component - radio button input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Radio {
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
}

/// Select component - dropdown select
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Select {
    pub name: String,
    pub options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

/// ProgressBar component - displays progress/health/mana bars
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressBar {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ProgressBarVariant>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "showLabel")]
    pub show_label: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
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
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<BadgeVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
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
    pub orientation: Option<DividerOrientation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

/// Spacer component - flexible empty space
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spacer {
    pub size: SpacerSize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpacerSize {
    Fixed(f64),
    Auto(String), // "auto"
}
