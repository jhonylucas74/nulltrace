# NullTrace Markup Language (NTML)

**NTML** is a YAML-based markup language for creating secure and validated user interfaces in the NullTrace game. It allows players to create custom UIs for their in-game programs, with strict validation to prevent exploits.

## 🎯 Features

- ✅ **Robust Parser and Validator**: Parses and validates NTML with clear error messages
- ✅ **Type-Safe**: All component structures are strongly typed
- ✅ **Rich Component System**: Container, Flex, Grid, Text, Button, Input, Code, Markdown, List, Heading, Table, Blockquote, Pre, Details, and more
- ✅ **CSS-like Style System**: Full support for style properties and optional `style.classes` (e.g. Tailwind)
- ✅ **Theme System**: Reusable theme variables
- ✅ **Security Validation**: Prevents injections and exploits
- ✅ **CLI Tool**: `ntml-validate` for file validation

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
nulltrace-ntml = { path = "../nulltrace-ntml" }
```

Or install the CLI tool:

```bash
cd nulltrace-ntml
cargo install --path .
```

## 🚀 Quick Start

### As a Library

```rust
use nulltrace_ntml::parse_ntml;

let yaml = r#"
Container:
  style:
    padding: 16
    backgroundColor: "#1a1a1a"
  children:
    - Text:
        text: "Hello NullTrace!"
"#;

match parse_ntml(yaml) {
    Ok(component) => println!("✓ Valid NTML!"),
    Err(e) => eprintln!("✗ Error: {}", e),
}
```

### As CLI

```bash
# Validate a file
ntml-validate ui.ntml

# Validate multiple files
ntml-validate ui/*.ntml

# See detailed errors
ntml-validate examples/invalid-color.ntml
```

## 📚 Available Components

### Layout Components

#### Container
Basic rectangular container for grouping elements.

```yaml
Container:
  style:
    padding: 16
    backgroundColor: "#1a1a1a"
  children:
    - Text:
        text: "Content"
```

#### Flex
Flexible layout with control over direction, alignment, and spacing.

```yaml
Flex:
  direction: column  # row | column
  justify: center    # start | center | end | spaceBetween | spaceAround | spaceEvenly
  align: center      # start | center | end | stretch
  gap: 12
  wrap: false
  children:
    - Text:
        text: "Item 1"
```

#### Grid
Grid layout with column and row definitions.

```yaml
Grid:
  columns: 3  # or ["1fr", "2fr", "1fr"]
  gap: 8      # or { row: 8, column: 16 }
  children:
    - Badge:
        text: "Item"
```

#### Row / Column
Shortcuts for Flex with predefined direction.

```yaml
Row:
  gap: 8
  align: center
  children: [...]

Column:
  gap: 12
  justify: spaceBetween
  children: [...]
```

#### Stack
Stacks elements on top of each other (z-index).

```yaml
Stack:
  alignment: center  # topLeft | topCenter | center | bottomRight | etc
  children:
    - Image:
        src: "background.png"
    - Text:
        text: "Overlay text"
```

### Content Components

#### Text
Displays text.

```yaml
Text:
  text: "Hello World"
  style:
    fontSize: 24
    fontWeight: bold
    color: "#00ff00"
```

#### Image
Displays an image.

```yaml
Image:
  src: "player-avatar.png"
  alt: "Player avatar"
  fit: cover  # cover | contain | fill | none | scaleDown
  style:
    width: 100
    height: 100
    borderRadius: 50
```

#### Icon
Displays an icon.

```yaml
Icon:
  name: "heart"
  size: 24
  style:
    color: red
```

### Interactive Components

#### Button
Clickable button with action handler.

```yaml
Button:
  action: "hack_system"
  variant: primary  # primary | secondary | danger | ghost
  disabled: false
  children:
    - Text:
        text: "HACK"
```

#### Input
Text input field.

```yaml
Input:
  name: "password"
  placeholder: "Enter password"
  type: password  # text | password | number
  maxLength: 50
```

#### Checkbox
Checkbox input.

```yaml
Checkbox:
  name: "agree"
  label: "I agree to the terms"
  checked: false
```

#### Radio
Radio button.

```yaml
Radio:
  name: "difficulty"
  value: "hard"
  label: "Hard Mode"
  checked: true
```

#### Select
Dropdown menu.

```yaml
Select:
  name: "target"
  options:
    - label: "Database Server"
      value: "db1"
    - label: "Web Server"
      value: "web1"
  value: "db1"
```

### Display Components

#### ProgressBar
Progress bar (health, mana, etc).

```yaml
ProgressBar:
  value: 75
  max: 100
  variant: danger  # default | success | warning | danger
  showLabel: true
```

#### Badge
Small badge or label.

```yaml
Badge:
  text: "Level 42"
  variant: primary  # default | primary | success | warning | danger
```

#### Divider
Horizontal or vertical divider line.

```yaml
Divider:
  orientation: horizontal  # horizontal | vertical
```

#### Spacer
Flexible empty space.

```yaml
Spacer:
  size: 16  # or "auto"
```

### Document & code components

#### Code
Inline or block code with optional `language` for syntax highlighting.

```yaml
Code:
  text: "local x = 1"
  language: lua
  block: true
```

#### Markdown
Renders markdown content as HTML (headings, lists, tables, etc.).

```yaml
Markdown:
  content: |
    ## Hello
    - item 1
```

#### List / ListItem
Ordered or unordered list.

```yaml
List:
  ordered: false
  children:
    - ListItem:
        children:
          - Text:
              text: "Item 1"
```

#### Heading
Semantic h1, h2, h3.

```yaml
Heading:
  level: 1
  text: "Page Title"
```

#### Table
Data table with headers and rows.

```yaml
Table:
  headers: [Name, Score]
  rows:
    - [Alice, "100"]
    - [Bob, "85"]
```

#### Blockquote
Quoted block.

```yaml
Blockquote:
  children:
    - Text:
        text: "Quote text"
```

#### Pre
Preformatted text.

```yaml
Pre:
  text: "  preformatted"
```

#### Details
Collapsible section.

```yaml
Details:
  summary: "Expand"
  children:
    - Text:
        text: "Hidden content"
```

## 🎨 Style System

NTML supports CSS-like style properties:

### Dimensions
```yaml
style:
  width: 200
  height: auto
  minWidth: 100
  maxWidth: 500
```

### Padding/Margin
```yaml
style:
  padding: 16
  paddingHorizontal: 20
  paddingVertical: 10
  paddingTop: 8

  margin: 12
  marginLeft: 16
```

### Colors
```yaml
style:
  color: "#00ff00"           # hex
  backgroundColor: red        # named color
  borderColor: "#ff0000"
  opacity: 0.8                # 0.0 to 1.0
```

Supported named colors: `red`, `blue`, `green`, `white`, `black`, `transparent`, `yellow`, `orange`, `purple`, `pink`, `gray`, `grey`

### Typography
```yaml
style:
  fontSize: 16
  fontWeight: bold            # or 100-900
  fontFamily: monospace       # sans | serif | monospace | game
  textAlign: center           # left | center | right | justify
  textTransform: uppercase    # none | uppercase | lowercase | capitalize
  letterSpacing: 1.2
  lineHeight: 1.5
  textDecoration: underline   # none | underline | line-through
```

### Borders
```yaml
style:
  borderWidth: 2
  borderColor: "#00ff00"
  borderStyle: solid          # solid | dashed | dotted
  borderRadius: 8
  borderTopLeftRadius: 4
```

### Shadows
```yaml
style:
  shadow: medium              # small | medium | large
  # or custom:
  shadow:
    shadowColor: "#000000"
    shadowOffset:
      x: 2
      y: 2
    shadowBlur: 4
    shadowOpacity: 0.5
```

### Positioning
```yaml
style:
  position: absolute          # relative | absolute
  top: 10
  left: 20
  zIndex: 100
```

### Flex Item
```yaml
style:
  flex: 1
  alignSelf: center           # start | center | end | stretch
```

### Display
```yaml
style:
  display: flex               # flex | none
  overflow: auto              # visible | hidden | scroll | auto
  cursor: pointer             # default | pointer | not-allowed | text
```

### CSS classes (Tailwind)
```yaml
style:
  classes: "p-4 bg-gray-100 rounded"   # Optional; space-separated class names
```
When set, the rendered HTML gets a `class` attribute (sanitized). Include Tailwind or your CSS so classes apply.

## 🎨 Theme System

Define reusable theme variables:

```rust
use nulltrace_ntml::{parse_with_theme, Theme};
use std::collections::HashMap;

let mut theme = Theme::new();

let mut colors = HashMap::new();
colors.insert("primary".to_string(), "#4a90e2".to_string());
colors.insert("danger".to_string(), "#ff6b6b".to_string());
theme.colors = Some(colors);

let yaml = r#"
Text:
  text: "Themed text"
  style:
    color: "$theme.colors.primary"
"#;

let component = parse_with_theme(yaml, theme)?;
```

Theme categories:
- `$theme.colors.<key>` - Colors
- `$theme.spacing.<key>` - Spacing values
- `$theme.borderRadius.<key>` - Border radius values
- `$theme.typography.<key>` - Font sizes

## ⚠️ Validation and Security

NTML automatically validates:

- ✅ Only known components
- ✅ Required properties present
- ✅ Correct value types
- ✅ Valid colors (hex or named)
- ✅ Value ranges (opacity 0-1, etc)
- ✅ Maximum nesting depth (20 levels)
- ✅ Valid font weight (100-900 in increments of 100)
- ✅ Valid enums for all properties

### Error Examples

```bash
$ ntml-validate examples/invalid-color.ntml
✗ examples/invalid-color.ntml has errors:
  Invalid color value 'not-a-valid-color':
    must be a valid hex color (e.g., #ff0000) or named color (...)
```

```bash
$ ntml-validate examples/invalid-missing-property.ntml
✗ examples/invalid-missing-property.ntml has errors:
  Missing required property 'text' for component 'Text'
```

## 🧪 Tests

Run tests:

```bash
cargo test
```

Test the validator with examples:

```bash
cargo run --bin ntml-validate examples/*.ntml
```

## 📁 Project Structure

```
nulltrace-ntml/
├── src/
│   ├── lib.rs           # Main API
│   ├── components.rs    # Component definitions
│   ├── error.rs         # Error types
│   ├── parser.rs        # YAML → Components parser
│   ├── validator.rs     # Component validation
│   ├── style.rs         # Style system
│   ├── theme.rs         # Theme system
│   └── bin/
│       └── ntml-validate.rs  # Validation CLI
├── examples/            # .ntml example files
├── Cargo.toml
└── README.md
```

## 🔧 Rust API

### Parsing

```rust
use nulltrace_ntml::{parse_ntml, parse_with_theme, Component, Theme};

// Simple parsing
let component: Component = parse_ntml(yaml_str)?;

// Parsing with theme
let theme = Theme::default();
let component = parse_with_theme(yaml_str, theme)?;
```

### Exported Types

```rust
pub use components::Component;
pub use error::{NtmlError, NtmlResult};
pub use parser::parse_ntml;
pub use style::Style;
pub use theme::Theme;
```

## 🤝 NullTrace Integration

This crate is shared between:
- **nulltrace-client**: Tauri/React frontend
- **nulltrace-core**: Rust backend

Both can use the same library for consistent parsing and validation.

### In the Client (React/TypeScript)

Via Tauri bindings:

```typescript
import { invoke } from '@tauri-apps/api/tauri';

const ntmlYaml = `
Text:
  text: "Hello"
`;

const component = await invoke('parse_ntml', { yaml: ntmlYaml });
```

### In the Server (Rust)

```rust
use nulltrace_ntml::parse_ntml;

fn handle_ui_upload(yaml: &str) -> Result<(), NtmlError> {
    let component = parse_ntml(yaml)?;
    // Save and use the component
    Ok(())
}
```

## 📄 License

This project is part of the NullTrace game.

## 🎮 About NullTrace

NullTrace is a competitive hacking game where players can create their own programs and custom interfaces using NTML.
