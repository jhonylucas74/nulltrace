# NullTrace UI Markup Language (NTML)

## Overview

NullTrace UI Markup Language (NTML) is a secure, XML-based UI description language designed specifically for the NullTrace game. Unlike HTML, which poses security risks and potential exploits, NTML provides a controlled, sandboxed environment for creating user interfaces.

**Key Benefits:**
- **Security First**: No script injection, no XSS vulnerabilities
- **Type-Safe**: Strongly typed components and properties
- **Designer-Friendly**: Inspired by Figma's auto-layout system
- **Performance**: Compiled and validated server-side
- **Familiar**: HTML-like XML syntax with CSS-like styling

---

## Core Concepts

### Components (Tags)

Components are the building blocks of NTML interfaces. Each component is an XML element where the tag name is the component type, and its properties are XML attributes. Children are nested elements — no `children:` key needed.

**Syntax:**
```xml
<ComponentName property1="value" style="key:value" class="tailwind-class">
  <ChildComponent property="value" />
</ComponentName>
```

### Styling

Styles follow CSS conventions but are validated and sanitized. Use the `style` attribute with `key:value; key2:value2` syntax. You can also pass **CSS utility classes** (e.g. Tailwind) via the `class` attribute; the renderer outputs a `class` attribute so any CSS loaded in the page can apply.

### Layout System

NTML uses modern layout primitives (Flex, Grid) as first-class components, eliminating the need for generic divs with complex styling.

---

## Component Reference

### Layout Components

#### `Container`
A basic rectangular container for grouping elements.

**Properties:**
- `style`: Style string (see Style Reference)
- `class`: CSS class names
- children: Component elements

**Example:**
```xml
<Container style="padding:16; backgroundColor:#1a1a1a; borderRadius:8">
  <Text text="Hello World" />
</Container>
```

---

#### `Flex`
A flexible box layout container with automatic spacing and alignment.

**Properties:**
- `direction`: `row` | `column` (default: `row`)
- `justify`: `start` | `center` | `end` | `spaceBetween` | `spaceAround` | `spaceEvenly` (default: `start`)
- `align`: `start` | `center` | `end` | `stretch` (default: `stretch`)
- `gap`: Number (spacing between children in pixels)
- `wrap`: Boolean string (default: `false`)
- `style`, `class`: Optional

**Example:**
```xml
<Flex direction="column" gap="12" align="center" style="padding:20">
  <Text text="Item 1" />
  <Text text="Item 2" />
</Flex>
```

---

#### `Grid`
A grid layout container for structured layouts.

**Properties:**
- `columns`: Number or space-separated size definitions (e.g., `"1fr 2fr 1fr"`)
- `rows`: Number or space-separated size definitions
- `gap`: Number
- `style`, `class`: Optional

**Example:**
```xml
<Grid columns="3" gap="16" style="padding:20">
  <Text text="Col 1" />
  <Text text="Col 2" />
  <Text text="Col 3" />
</Grid>
```

---

#### `Stack`
Layers children on top of each other (z-index stacking).

**Properties:**
- `alignment`: `topLeft` | `topCenter` | `topRight` | `centerLeft` | `center` | `centerRight` | `bottomLeft` | `bottomCenter` | `bottomRight` (default: `topLeft`)
- `style`, `class`: Optional

**Example:**
```xml
<Stack alignment="center">
  <Image src="background.png" />
  <Text text="Overlay Text" style="color:#ffffff; fontSize:24" />
</Stack>
```

---

#### `Row`
Shorthand for `Flex` with `direction: row`.

**Properties:** Same as `Flex` (direction is automatically set to `row`)

---

#### `Column`
Shorthand for `Flex` with `direction: column`.

**Properties:** Same as `Flex` (direction is automatically set to `column`)

---

### Content Components

#### `Text`
Displays text content.

**Properties:**
- `text`: String (required)
- `style`, `class`: Optional (supports typography styles)

**Example:**
```xml
<Text text="Player Name" style="fontSize:18; fontWeight:600; color:#00ff00" />
```

---

#### `Image`
Displays an image from approved game assets.

**Properties:**
- `src`: String (asset path, validated against whitelist) — required
- `alt`: String (accessibility text)
- `fit`: `cover` | `contain` | `fill` | `none` | `scaleDown` (default: `contain`)
- `style`, `class`: Optional

**Example:**
```xml
<Image src="icons/player-avatar.png" alt="Player Avatar" fit="cover"
  style="width:64; height:64; borderRadius:32" />
```

---

#### `Icon`
Displays an icon from the game's icon set.

**Properties:**
- `name`: String (icon identifier) — required
- `size`: Number (default: 24)
- `style`, `class`: Optional (color applies to icon)

**Example:**
```xml
<Icon name="shield" size="32" style="color:#4a90e2" />
```

---

### Interactive Components

#### `Link`
A hyperlink that navigates the Browser. Behaves like an `<a>` tag.

**Properties:**
- `href`: String (required) — URL or path
- `target`: `same` | `new` (default: `same`) — open in current tab or new tab
- `style`, `class`: Optional
- children: Component elements (typically Text) — link content

**Example:**
```xml
<Link href="/about">
  <Text text="About" style="color:#6b4cdf" />
</Link>

<Link href="http://example.com" target="new">
  <Text text="External (new tab)" />
</Link>
```

---

#### `Button`
A clickable button component.

**Properties:**
- `action`: String (action identifier or Lua function name) — required
- `variant`: `primary` | `secondary` | `danger` | `ghost` (default: `primary`)
- `disabled`: Boolean string (default: `false`)
- `style`, `class`: Optional
- children: Component elements (typically Text or Icon)

**Example:**
```xml
<Button action="attack_target" variant="primary"
  style="padding:12; borderRadius:6">
  <Text text="Attack" style="fontWeight:600" />
</Button>
```

---

#### `Input`
Text input field.

**Properties:**
- `name`: String (field identifier) — required
- `placeholder`: String
- `value`: String (initial value)
- `type`: `text` | `password` | `number` (default: `text`)
- `maxLength`: Number
- `disabled`: Boolean string (default: `false`)
- `style`, `class`: Optional

**Example:**
```xml
<Input name="username" placeholder="Enter username" maxLength="20"
  style="padding:10; borderRadius:4; borderWidth:1; borderColor:#333333" />
```

---

#### `Checkbox`
A checkbox input.

**Properties:**
- `name`: String (field identifier) — required
- `label`: String
- `checked`: Boolean string (default: `false`)
- `disabled`: Boolean string (default: `false`)
- `style`, `class`: Optional

**Example:**
```xml
<Checkbox name="agree_terms" label="I agree to the terms" checked="false" />
```

---

#### `Radio`
A radio button input.

**Properties:**
- `name`: String (group identifier) — required
- `value`: String (option value) — required
- `label`: String
- `checked`: Boolean string (default: `false`)
- `disabled`: Boolean string (default: `false`)
- `style`, `class`: Optional

**Example:**
```xml
<Radio name="difficulty" value="hard" label="Hard Mode" />
```

---

#### `Select`
A dropdown select component.

**Properties:**
- `name`: String (field identifier) — required
- `value`: String (selected value)
- `disabled`: Boolean string (default: `false`)
- `style`, `class`: Optional
- children: `<option label="..." value="..." />` elements

**Example:**
```xml
<Select name="weapon" value="sword" style="padding:8; borderRadius:4">
  <option label="Sword" value="sword" />
  <option label="Axe" value="axe" />
  <option label="Bow" value="bow" />
</Select>
```

---

### Specialized Components

#### `ProgressBar`
Displays progress/health/mana bars.

**Properties:**
- `value`: Number (0-100) — required
- `max`: Number (default: 100)
- `variant`: `default` | `success` | `warning` | `danger` (default: `default`)
- `showLabel`: Boolean string (default: `false`)
- `style`, `class`: Optional

**Example:**
```xml
<ProgressBar value="75" max="100" variant="success" showLabel="true"
  style="height:20; borderRadius:10" />
```

---

#### `Badge`
Displays a small badge or label.

**Properties:**
- `text`: String — required
- `variant`: `default` | `primary` | `success` | `warning` | `danger` (default: `default`)
- `style`, `class`: Optional

**Example:**
```xml
<Badge text="NEW" variant="primary" style="fontSize:12; padding:4" />
```

---

#### `Divider`
A horizontal or vertical divider line.

**Properties:**
- `orientation`: `horizontal` | `vertical` (default: `horizontal`)
- `style`, `class`: Optional (backgroundColor sets divider color)

**Example:**
```xml
<Divider orientation="horizontal"
  style="backgroundColor:#333333; height:1; marginVertical:16" />
```

---

#### `Spacer`
Flexible empty space for layout.

**Properties:**
- `size`: Number (fixed size in pixels) or `"auto"` (fills available space) — required

**Example:**
```xml
<Flex direction="row">
  <Text text="Left" />
  <Spacer size="auto" />
  <Text text="Right" />
</Flex>
```

---

### Document & list components

#### `Code`
Inline or block code, with optional syntax highlighting. Content via `text` attribute or element body (supports CDATA).

**Properties:**
- `text`: String — code content (optional if provided as element body)
- `language`: String (optional) — e.g. `lua`, `python` (for highlighting)
- `block`: Boolean string (optional, default: false) — if true, rendered as `<pre><code>`
- `id`, `style`, `class`: Optional

**Example:**
```xml
<Code text="local x = 1" language="lua" block="true" style="fontFamily:monospace" />
```

#### `Markdown`
Renders markdown content as HTML (headings, lists, tables, etc.). Content is parsed and sanitized.

**Properties:**
- `content`: String (required) — markdown source
- `id`, `style`, `class`: Optional

**Example:**
```xml
<Markdown content="## Title&#10;- Item 1&#10;- Item 2" />
```

#### `List` and `ListItem`
Ordered or unordered list.

**List properties:**
- `ordered`: Boolean string (default: false) — false = `<ul>`, true = `<ol>`
- `id`, `style`, `class`: Optional
- children: `ListItem` elements

**ListItem properties:**
- `id`, `style`, `class`: Optional
- children: Component elements (e.g. Text, Link)

**Example:**
```xml
<List ordered="false">
  <ListItem>
    <Text text="First" />
  </ListItem>
  <ListItem>
    <Text text="Second" />
  </ListItem>
</List>
```

#### `Heading`
Semantic heading (h1, h2, h3).

**Properties:**
- `level`: `1` | `2` | `3` (required)
- `text`: String (required)
- `id`, `style`, `class`: Optional

**Example:**
```xml
<Heading level="1" text="Page Title" />
```

#### `Table`
Data table with header row and body rows.

**Properties:**
- `headers`: Comma-separated string of column headers (required)
- `rows`: `|`-separated rows, each row is comma-separated cells (required)
- `id`, `style`, `class`: Optional

**Example:**
```xml
<Table headers="Name,Score" rows="Alice,100|Bob,85" />
```

#### `Blockquote`
Quoted block of content.

**Properties:**
- `id`, `style`, `class`: Optional
- children: Component elements

**Example:**
```xml
<Blockquote>
  <Text text="To be or not to be." />
</Blockquote>
```

#### `Pre`
Preformatted text (no syntax highlighting).

**Properties:**
- `text`: String (required)
- `id`, `style`, `class`: Optional

**Example:**
```xml
<Pre text="  indented&#10;  lines" />
```

#### `Details`
Collapsible section with a summary.

**Properties:**
- `summary`: String (required) — visible toggle label
- `open`: Boolean string (optional, default: false) — whether expanded by default
- `id`, `style`, `class`: Optional
- children: Component elements (expandable body)

**Example:**
```xml
<Details summary="Click to expand" open="false">
  <Text text="Hidden content here." />
</Details>
```

---

## Style Reference

### Dimension Properties

```
style="width:100; height:100; minWidth:50; maxWidth:500; minHeight:50; maxHeight:500"
```

### Spacing Properties

```
/* Padding (inner spacing) */
style="padding:16"
style="paddingVertical:12; paddingHorizontal:20"
style="paddingTop:8; paddingRight:8; paddingBottom:8; paddingLeft:8"

/* Margin (outer spacing) */
style="margin:16"
style="marginVertical:12; marginHorizontal:20"
style="marginTop:8; marginRight:8; marginBottom:8; marginLeft:8"
```

### Color Properties

```
style="color:#00ff00"              /* Text color (hex or named) */
style="backgroundColor:#1a1a1a"   /* Background color */
style="borderColor:#333333"       /* Border color */
style="opacity:0.8"               /* 0.0 to 1.0 */

/* Named colors supported */
style="color:red"
style="color:blue"
style="color:green"
style="color:white"
style="color:black"
style="color:transparent"
```

### Typography Properties

```
style="fontSize:16"
style="fontWeight:400"             /* 100-900 or "normal", "bold" */
style="fontFamily:monospace"       /* "sans", "serif", "monospace", "game" */
style="textAlign:center"          /* "left", "center", "right", "justify" */
style="textTransform:uppercase"   /* "none", "uppercase", "lowercase", "capitalize" */
style="letterSpacing:1"
style="lineHeight:1.5"
style="textDecoration:underline"  /* "none", "underline", "line-through" */
```

### Border Properties

```
style="borderWidth:1"
style="borderTopWidth:1; borderRightWidth:1; borderBottomWidth:1; borderLeftWidth:1"
style="borderStyle:solid"         /* "solid", "dashed", "dotted" */
style="borderRadius:8"
style="borderTopLeftRadius:4; borderTopRightRadius:4"
style="borderBottomLeftRadius:4; borderBottomRightRadius:4"
```

### Shadow Properties

```
style="shadow:small"              /* Preset: "small", "medium", "large" */
/* Or custom shadow */
style="shadowColor:#000000; shadowBlur:4; shadowOpacity:0.25"
```

### Position Properties

```
style="position:absolute"         /* "relative", "absolute" */
style="top:0; right:0; bottom:0; left:0"
style="zIndex:10"
```

### Flex Item Properties

```
style="flex:1"
style="alignSelf:center"
```

### Display Properties

```
style="display:flex"              /* "flex", "none" */
style="overflow:hidden"           /* "visible", "hidden", "scroll", "auto" */
style="cursor:pointer"            /* "default", "pointer", "not-allowed", "text" */
```

### CSS Classes (Tailwind / utility classes)

```xml
<Container class="p-4 bg-gray-100 rounded-lg">
  <Text text="Hello" class="text-white font-bold" />
</Container>
```

When set, the rendered HTML element receives a `class` attribute with the given string (sanitized). Use this for Tailwind or other utility-class frameworks. The document must include the corresponding CSS for classes to take effect.

---

## Document Formats

### Classic Format

A single root element — no `head` or `body` needed:

```xml
<Container style="backgroundColor:#1a1a1a; borderRadius:12; padding:20">
  <Text text="Hello!" style="color:#00ff00" />
</Container>
```

### Full Format (head + body)

For pages with metadata, fonts, scripts, or imported components:

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

---

## Complete Examples

### Player Status Card

```xml
<Container style="backgroundColor:#1a1a2e; borderRadius:12; padding:20; shadow:medium">
  <!-- Header with avatar and name -->
  <Flex direction="row" gap="12" align="center" style="marginBottom:16">
    <Image src="avatars/player_01.png" alt="Player Avatar" fit="cover"
      style="width:64; height:64; borderRadius:32; borderWidth:2; borderColor:#00ff00" />

    <Column gap="4">
      <Text text="PlayerName" style="fontSize:20; fontWeight:700; color:#ffffff" />
      <Flex direction="row" gap="8">
        <Badge text="Level 42" variant="primary" />
        <Badge text="Elite" variant="success" />
      </Flex>
    </Column>
  </Flex>

  <!-- Stats -->
  <Column gap="12">
    <!-- Health bar -->
    <Column gap="4">
      <Flex direction="row" justify="spaceBetween">
        <Text text="Health" style="fontSize:14; color:#aaaaaa" />
        <Text text="850 / 1000" style="fontSize:14; fontWeight:600; color:#ff6b6b" />
      </Flex>
      <ProgressBar value="85" variant="danger" style="height:12; borderRadius:6" />
    </Column>

    <!-- Mana bar -->
    <Column gap="4">
      <Flex direction="row" justify="spaceBetween">
        <Text text="Mana" style="fontSize:14; color:#aaaaaa" />
        <Text text="420 / 500" style="fontSize:14; fontWeight:600; color:#4a90e2" />
      </Flex>
      <ProgressBar value="84" variant="primary" style="height:12; borderRadius:6" />
    </Column>
  </Column>

  <!-- Action buttons -->
  <Flex direction="row" gap="8" style="marginTop:16">
    <Button action="attack" variant="danger"
      style="flex:1; padding:12; borderRadius:8">
      <Text text="Attack" style="fontSize:16; fontWeight:600; color:#ffffff" />
    </Button>

    <Button action="defend" variant="primary"
      style="flex:1; padding:12; borderRadius:8">
      <Text text="Defend" style="fontSize:16; fontWeight:600; color:#ffffff" />
    </Button>
  </Flex>
</Container>
```

---

### Inventory Grid

```xml
<Container style="backgroundColor:#16213e; borderRadius:8; padding:16">
  <!-- Header -->
  <Flex direction="row" justify="spaceBetween" align="center"
    style="marginBottom:16">
    <Text text="Inventory" style="fontSize:20; fontWeight:700; color:#ffffff" />
    <Text text="12 / 50" style="fontSize:14; color:#aaaaaa" />
  </Flex>

  <!-- Item grid -->
  <Grid columns="5" gap="8">
    <!-- Item slot 1 -->
    <Stack alignment="center"
      style="backgroundColor:#0f3460; borderRadius:6; borderWidth:1; borderColor:#1a4d7a; padding:8; cursor:pointer">
      <Image src="items/sword_001.png" alt="Iron Sword" fit="contain"
        style="width:48; height:48" />
      <Container style="position:absolute; bottom:4; right:4">
        <Text text="x3" style="fontSize:12; fontWeight:700; color:#ffffff; backgroundColor:#000000; padding:2; borderRadius:3" />
      </Container>
    </Stack>

    <!-- Item slot 2 (empty) -->
    <Container
      style="backgroundColor:#0f3460; borderRadius:6; borderWidth:1; borderColor:#1a4d7a; padding:8; height:64; cursor:pointer" />
  </Grid>
</Container>
```

---

### Login Form

```xml
<Container style="backgroundColor:#ffffff; borderRadius:16; padding:32; shadow:large; maxWidth:400">
  <!-- Logo -->
  <Container style="marginBottom:24">
    <Text text="NullTrace" style="fontSize:32; fontWeight:800; color:#1a1a1a; textAlign:center" />
  </Container>

  <!-- Form fields -->
  <Column gap="16">
    <!-- Username -->
    <Column gap="6">
      <Text text="Username" style="fontSize:14; fontWeight:600; color:#333333" />
      <Input name="username" placeholder="Enter your username"
        style="padding:12; borderRadius:8; borderWidth:1; borderColor:#dddddd; fontSize:16" />
    </Column>

    <!-- Password -->
    <Column gap="6">
      <Text text="Password" style="fontSize:14; fontWeight:600; color:#333333" />
      <Input name="password" type="password" placeholder="Enter your password"
        style="padding:12; borderRadius:8; borderWidth:1; borderColor:#dddddd; fontSize:16" />
    </Column>

    <!-- Remember me -->
    <Checkbox name="remember" label="Remember me" style="marginTop:8" />

    <!-- Submit button -->
    <Button action="login" variant="primary"
      style="padding:14; borderRadius:8; backgroundColor:#4a90e2; marginTop:8">
      <Text text="Sign In" style="fontSize:16; fontWeight:700; color:#ffffff; textAlign:center" />
    </Button>
  </Column>
</Container>
```

---

## Security & Validation

### Asset Whitelisting
All `src` attributes in `Image` components are validated against a server-side whitelist. Only approved game assets can be loaded.

### Action Validation
Button `action` properties are validated and rate-limited server-side. Unknown actions are rejected.

### Input Sanitization
All user input from `Input`, `Checkbox`, `Radio`, and `Select` components is sanitized before being sent to the game server.

### Style Bounds
- Numeric values are clamped to reasonable ranges
- Color values must be valid hex codes or named colors
- URLs and external references are not allowed in any property

### Component Nesting Limits
Maximum nesting depth: 20 levels (prevents stack overflow attacks)

---

## Performance Guidelines

### Best Practices
1. **Use specific layout components**: Prefer `Flex` and `Grid` over nested `Container` elements
2. **Minimize deep nesting**: Keep component trees shallow when possible
3. **Reuse styles**: Common styles should be defined in theme presets
4. **Optimize images**: Use appropriately sized assets
5. **Limit dynamic content**: Avoid creating large lists with hundreds of items

### Theme System
Define reusable theme tokens for consistency via the Rust API:

```rust
use nulltrace_ntml::{parse_with_theme, Theme};
use std::collections::HashMap;

let mut theme = Theme::new();
let mut colors = HashMap::new();
colors.insert("primary".to_string(), "#4a90e2".to_string());
colors.insert("danger".to_string(), "#ff6b6b".to_string());
theme.colors = Some(colors);

// Reference in NTML
// <Button style="backgroundColor:$theme.colors.primary">
```

---

## Browser NTML Rendering

The in-app **Browser** renders NTML pages served by VMs (e.g. ntml.org) by converting NTML to safe HTML. The renderer supports all components and style properties defined in this document.

### Supported Features

- **All layout components:** Container, Flex, Row, Column, Grid, Stack
- **All content components:** Text, Image, Icon, Code, Markdown, Pre
- **List components:** List, ListItem
- **Document components:** Heading, Table, Blockquote, Details (collapsible with summary)
- **All interactive components:** Button, Link, Input, Checkbox, Radio, Select
- **All display components:** ProgressBar, Badge, Divider, Spacer
- **Full style support:** All properties in the Style Reference (dimensions, spacing, typography, borders, shadow, position, flex, display, overflow, cursor) plus **`class`** for CSS utility classes (e.g. Tailwind)
- **Layout props:** Flex/Row/Column `justify`, `align`, `wrap`; Grid `columns` and `rows`; Stack `alignment`; Divider `orientation`; Spacer `size="auto"`

### Image Resolution

When NTML is rendered in the Browser, Image `src` paths are resolved against the page base URL. For example, a page at `http://ntml.org/about` with `<Image src="img/logo.png" />` will load `http://ntml.org/img/logo.png`. The `fit` property maps to CSS `object-fit` (cover, contain, fill, none, scale-down).

### Lua Scripts and Patches

Pages with `<script>` elements in `head` can run Lua in a sandbox. Button `action` values without `:` call Lua functions. The `ui` API (`set_text`, `set_visible`, `set_value`, `set_disabled`) applies patches that update the rendered HTML on the next frame.

---

## FAQ

**Q: Why XML instead of YAML?**
A: XML/HTML-like syntax is more familiar to most developers, makes nesting naturally explicit through tag structure, and integrates well with editors and tooling.

**Q: Can I use custom fonts?**
A: Yes — declare them in `head` via `<font family="FontName" weights="400,700" />`. They are loaded from Google Fonts.

**Q: How do I handle dynamic data?**
A: Use Lua scripts declared in `head`. The `ui` API lets scripts update component text, visibility, and values at runtime.

**Q: Can I create custom components?**
A: Yes — create `.ntml` component files with a `<props>` section and `<body>`, then import them via `<import src="..." as="MyComponent" />` in the page `head`.

**Q: Is animation supported?**
A: Animation is handled by the game engine, not NTML. You can define states, and the engine will interpolate between them.

---

## Additional Resources

- [Game Modding Guide](./modding-guide.md)
- [UI Design Guidelines](./ui-design-guidelines.md)
- [Theme Customization](./theme-customization.md)
- [Accessibility Best Practices](./accessibility.md)
