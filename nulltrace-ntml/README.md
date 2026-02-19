# NullTrace Markup Language (NTML)

**NTML** is an XML-based markup language for creating secure and validated user interfaces in the NullTrace game. It allows players to create custom UIs for their in-game programs, with strict validation to prevent exploits.

## 🎯 Features

- ✅ **Robust Parser and Validator**: Parses and validates NTML with clear error messages
- ✅ **Type-Safe**: All component structures are strongly typed
- ✅ **Rich Component System**: Container, Flex, Grid, Text, Button, Input, Code, Markdown, List, Heading, Table, Blockquote, Pre, Details, and more
- ✅ **CSS-like Style System**: Full support for style properties and optional `class` attribute (e.g. Tailwind)
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

let ntml = r#"<Container style="padding:16; backgroundColor:#1a1a1a">
  <Text text="Hello NullTrace!" />
</Container>"#;

match parse_ntml(ntml) {
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

```xml
<Container style="padding:16; backgroundColor:#1a1a1a">
  <Text text="Content" />
</Container>
```

#### Flex
Flexible layout with control over direction, alignment, and spacing.

```xml
<Flex direction="column" justify="center" align="center" gap="12" wrap="false">
  <Text text="Item 1" />
</Flex>
```

#### Grid
Grid layout with column and row definitions.

```xml
<Grid columns="3" gap="8">
  <Badge text="Item" />
</Grid>
```

#### Row / Column
Shortcuts for Flex with predefined direction.

```xml
<Row gap="8" align="center">
  <Text text="Left" />
  <Text text="Right" />
</Row>

<Column gap="12" justify="spaceBetween">
  <Text text="Top" />
  <Text text="Bottom" />
</Column>
```

#### Stack
Stacks elements on top of each other (z-index).

```xml
<Stack alignment="center">
  <Image src="background.png" />
  <Text text="Overlay text" />
</Stack>
```

### Content Components

#### Text
Displays text.

```xml
<Text text="Hello World" style="fontSize:24; fontWeight:bold; color:#00ff00" />
```

#### Image
Displays an image.

```xml
<Image src="player-avatar.png" alt="Player avatar" fit="cover" style="width:100; height:100; borderRadius:50" />
```

#### Icon
Displays an icon.

```xml
<Icon name="heart" size="24" style="color:red" />
```

### Interactive Components

#### Button
Clickable button with action handler.

```xml
<Button action="hack_system" variant="primary" disabled="false">
  <Text text="HACK" />
</Button>
```

#### Input
Text input field.

```xml
<Input name="password" placeholder="Enter password" type="password" maxLength="50" />
```

#### Checkbox
Checkbox input.

```xml
<Checkbox name="agree" label="I agree to the terms" checked="false" />
```

#### Radio
Radio button.

```xml
<Radio name="difficulty" value="hard" label="Hard Mode" checked="true" />
```

#### Select
Dropdown menu.

```xml
<Select name="target" value="db1">
  <option label="Database Server" value="db1" />
  <option label="Web Server" value="web1" />
</Select>
```

### Display Components

#### ProgressBar
Progress bar (health, mana, etc).

```xml
<ProgressBar value="75" max="100" variant="danger" showLabel="true" />
```

#### Badge
Small badge or label.

```xml
<Badge text="Level 42" variant="primary" />
```

#### Divider
Horizontal or vertical divider line.

```xml
<Divider orientation="horizontal" />
```

#### Spacer
Flexible empty space.

```xml
<Spacer size="16" />
```

### Document & code components

#### Code
Inline or block code with optional `language` for syntax highlighting.

```xml
<Code language="lua" block="true">local x = 1</Code>
```

Or via attribute:

```xml
<Code text="local x = 1" language="lua" block="true" />
```

#### Markdown
Renders markdown content as HTML (headings, lists, tables, etc.).

```xml
<Markdown content="## Hello&#10;- item 1" />
```

#### List / ListItem
Ordered or unordered list.

```xml
<List ordered="false">
  <ListItem>
    <Text text="Item 1" />
  </ListItem>
</List>
```

#### Heading
Semantic h1, h2, h3.

```xml
<Heading level="1" text="Page Title" />
```

#### Table
Data table with headers and rows.

```xml
<Table headers="Name,Score" rows="Alice,100|Bob,85" />
```

#### Blockquote
Quoted block.

```xml
<Blockquote>
  <Text text="Quote text" />
</Blockquote>
```

#### Pre
Preformatted text.

```xml
<Pre text="  preformatted" />
```

#### Details
Collapsible section.

```xml
<Details summary="Expand">
  <Text text="Hidden content" />
</Details>
```

## 🎨 Style System

NTML supports CSS-like style properties via the `style` attribute using `key:value; key2:value2` syntax.

### Dimensions
```
style="width:200; height:auto; minWidth:100; maxWidth:500"
```

### Padding/Margin
```
style="padding:16; paddingHorizontal:20; paddingVertical:10; paddingTop:8"
style="margin:12; marginLeft:16"
```

### Colors
```
style="color:#00ff00"           /* hex */
style="backgroundColor:red"    /* named color */
style="borderColor:#ff0000"
style="opacity:0.8"             /* 0.0 to 1.0 */
```

Supported named colors: `red`, `blue`, `green`, `white`, `black`, `transparent`, `yellow`, `orange`, `purple`, `pink`, `gray`, `grey`

### Typography
```
style="fontSize:16; fontWeight:bold; fontFamily:monospace; textAlign:center"
style="textTransform:uppercase; letterSpacing:1.2; lineHeight:1.5; textDecoration:underline"
```

### Borders
```
style="borderWidth:2; borderColor:#00ff00; borderStyle:solid; borderRadius:8"
style="borderTopLeftRadius:4"
```

### Shadows
```
style="shadow:medium"      /* small | medium | large */
```

### Positioning
```
style="position:absolute; top:10; left:20; zIndex:100"
```

### Flex Item
```
style="flex:1; alignSelf:center"
```

### Display
```
style="display:flex; overflow:auto; cursor:pointer"
```

### CSS classes (Tailwind)
```xml
<Container class="p-4 bg-gray-100 rounded">
  <Text text="Hello" />
</Container>
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

let ntml = r#"<Text text="Themed text" style="color:$theme.colors.primary" />"#;

let component = parse_with_theme(ntml, theme)?;
```

Theme categories:
- `$theme.colors.<key>` - Colors
- `$theme.spacing.<key>` - Spacing values
- `$theme.borderRadius.<key>` - Border radius values
- `$theme.typography.<key>` - Font sizes

## 📄 Document Formats

### Classic format
Single root element — no `head` or `body` needed:

```xml
<Container style="padding:16; backgroundColor:#1a1a1a">
  <Text text="Hello!" />
</Container>
```

### Full format (head + body)
Use `head` for metadata, fonts, scripts, and component imports:

```xml
<head>
  <title>My App</title>
  <description>Dashboard</description>
  <font family="Roboto Mono" weights="400,700" />
  <script src="scripts/main.lua" />
  <import src="components/nav.ntml" as="Nav" />
</head>
<body>
  <Column>
    <Nav title="My App" />
    <Text text="Content" />
  </Column>
</body>
```

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
│   ├── parser.rs        # XML → Components parser
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
let component: Component = parse_ntml(ntml_str)?;

// Parsing with theme
let theme = Theme::default();
let component = parse_with_theme(ntml_str, theme)?;
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

const ntml = `<Text text="Hello" />`;

const component = await invoke('parse_ntml', { ntml });
```

### In the Server (Rust)

```rust
use nulltrace_ntml::parse_ntml;

fn handle_ui_upload(ntml: &str) -> Result<(), NtmlError> {
    let component = parse_ntml(ntml)?;
    // Save and use the component
    Ok(())
}
```

## 📄 License

This project is part of the NullTrace game.

## 🎮 About NullTrace

NullTrace is a competitive hacking game where players can create their own programs and custom interfaces using NTML.
