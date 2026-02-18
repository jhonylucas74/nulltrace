# NullTrace UI Markup Language (NTML)

## Overview

NullTrace UI Markup Language (NTML) is a secure, YAML-based UI description language designed specifically for the NullTrace game. Unlike HTML, which poses security risks and potential exploits, NTML provides a controlled, sandboxed environment for creating user interfaces.

**Key Benefits:**
- **Security First**: No script injection, no XSS vulnerabilities
- **Type-Safe**: Strongly typed components and properties
- **Designer-Friendly**: Inspired by Figma's auto-layout system
- **Performance**: Compiled and validated server-side
- **Familiar**: CSS-like styling with modern layout primitives

---

## Core Concepts

### Components (Tags)

Components are the building blocks of NTML interfaces. Each component is represented as a YAML object where the component name is the key, and its properties (including `style` and `children`) are nested within.

**Syntax:**
```yaml
ComponentName:
  property1: value
  style:
    # styles here
  children:
    - ChildComponent:
        property: value
```

### Styling

Styles follow CSS conventions but are validated and sanitized. All style properties are optional and have sensible defaults.

### Layout System

NTML uses modern layout primitives (Flex, Grid) as first-class components, eliminating the need for generic divs with complex styling.

---

## Component Reference

### Layout Components

#### `Container`
A basic rectangular container for grouping elements.

**Properties:**
- `style`: Style object (see Style Reference)
- `children`: Array of child components

**Example:**
```yaml
Container:
  style:
    padding: 16
    backgroundColor: "#1a1a1a"
    borderRadius: 8
  children:
    - Text:
        text: "Hello World"
```

---

#### `Flex`
A flexible box layout container with automatic spacing and alignment.

**Properties:**
- `direction`: `row` | `column` (default: `row`)
- `justify`: `start` | `center` | `end` | `spaceBetween` | `spaceAround` | `spaceEvenly` (default: `start`)
- `align`: `start` | `center` | `end` | `stretch` (default: `stretch`)
- `gap`: Number (spacing between children in pixels)
- `wrap`: Boolean (default: `false`)
- `style`: Style object
- `children`: Array of child components

**Example:**
```yaml
Flex:
  direction: column
  gap: 12
  align: center
  style:
    padding: 20
  children:
    - Text:
        text: "Item 1"
    - Text:
        text: "Item 2"
```

---

#### `Grid`
A grid layout container for structured layouts.

**Properties:**
- `columns`: Number or Array of size definitions (e.g., `[100, "1fr", "2fr"]`)
- `rows`: Number or Array of size definitions
- `gap`: Number or Object `{row: 10, column: 10}`
- `style`: Style object
- `children`: Array of child components

**Example:**
```yaml
Grid:
  columns: [200, "1fr", "1fr"]
  gap: 16
  style:
    padding: 20
  children:
    - Text:
        text: "Sidebar"
    - Text:
        text: "Content"
    - Text:
        text: "Right Panel"
```

---

#### `Stack`
Layers children on top of each other (z-index stacking).

**Properties:**
- `alignment`: `topLeft` | `topCenter` | `topRight` | `centerLeft` | `center` | `centerRight` | `bottomLeft` | `bottomCenter` | `bottomRight` (default: `topLeft`)
- `style`: Style object
- `children`: Array of child components

**Example:**
```yaml
Stack:
  alignment: center
  children:
    - Image:
        src: "background.png"
    - Text:
        text: "Overlay Text"
        style:
          color: "#ffffff"
          fontSize: 24
```

---

#### `Row`
Shorthand for `Flex` with `direction: row`.

**Properties:**
Same as `Flex` (direction is automatically set to `row`)

---

#### `Column`
Shorthand for `Flex` with `direction: column`.

**Properties:**
Same as `Flex` (direction is automatically set to `column`)

---

### Content Components

#### `Text`
Displays text content.

**Properties:**
- `text`: String (required)
- `style`: Style object (supports typography styles)

**Example:**
```yaml
Text:
  text: "Player Name"
  style:
    fontSize: 18
    fontWeight: 600
    color: "#00ff00"
```

---

#### `Image`
Displays an image from approved game assets.

**Properties:**
- `src`: String (asset path, validated against whitelist)
- `alt`: String (accessibility text)
- `fit`: `cover` | `contain` | `fill` | `none` | `scaleDown` (default: `contain`)
- `style`: Style object

**Example:**
```yaml
Image:
  src: "icons/player-avatar.png"
  alt: "Player Avatar"
  fit: cover
  style:
    width: 64
    height: 64
    borderRadius: 32
```

---

#### `Icon`
Displays an icon from the game's icon set.

**Properties:**
- `name`: String (icon identifier)
- `size`: Number (default: 24)
- `style`: Style object (color applies to icon)

**Example:**
```yaml
Icon:
  name: "shield"
  size: 32
  style:
    color: "#4a90e2"
```

---

### Interactive Components

#### `Link`
A hyperlink that navigates the Browser. Behaves like an `<a>` tag.

**Properties:**
- `href`: String (required) — URL or path (e.g. `/about`, `robot.txt`, `http://example.com`)
- `target`: `same` | `new` (default: `same`) — open in current tab or new tab
- `style`: Style object
- `children`: Array of components (typically Text) — link content; if omitted, shows `href`

**Example:**
```yaml
Link:
  href: "/about"
  children:
    - Text:
        text: "About"
        style:
          color: "#6b4cdf"

Link:
  href: "http://example.com"
  target: new
  children:
    - Text:
        text: "External (new tab)"
```

---

#### `Button`
A clickable button component.

**Properties:**
- `action`: String (action identifier sent to game server)
- `variant`: `primary` | `secondary` | `danger` | `ghost` (default: `primary`)
- `disabled`: Boolean (default: `false`)
- `style`: Style object
- `children`: Array of child components (typically Text or Icon)

**Example:**
```yaml
Button:
  action: "attack_target"
  variant: primary
  style:
    padding: 12
    borderRadius: 6
  children:
    - Text:
        text: "Attack"
        style:
          fontWeight: 600
```

---

#### `Input`
Text input field.

**Properties:**
- `name`: String (field identifier)
- `placeholder`: String
- `value`: String (initial value)
- `type`: `text` | `password` | `number` (default: `text`)
- `maxLength`: Number
- `disabled`: Boolean (default: `false`)
- `style`: Style object

**Example:**
```yaml
Input:
  name: "username"
  placeholder: "Enter username"
  maxLength: 20
  style:
    padding: 10
    borderRadius: 4
    borderWidth: 1
    borderColor: "#333333"
```

---

#### `Checkbox`
A checkbox input.

**Properties:**
- `name`: String (field identifier)
- `label`: String
- `checked`: Boolean (default: `false`)
- `disabled`: Boolean (default: `false`)
- `style`: Style object

**Example:**
```yaml
Checkbox:
  name: "agree_terms"
  label: "I agree to the terms"
  checked: false
```

---

#### `Radio`
A radio button input.

**Properties:**
- `name`: String (group identifier)
- `value`: String (option value)
- `label`: String
- `checked`: Boolean (default: `false`)
- `disabled`: Boolean (default: `false`)
- `style`: Style object

**Example:**
```yaml
Radio:
  name: "difficulty"
  value: "hard"
  label: "Hard Mode"
```

---

#### `Select`
A dropdown select component.

**Properties:**
- `name`: String (field identifier)
- `options`: Array of `{label: String, value: String}`
- `value`: String (selected value)
- `disabled`: Boolean (default: `false`)
- `style`: Style object

**Example:**
```yaml
Select:
  name: "weapon"
  value: "sword"
  options:
    - label: "Sword"
      value: "sword"
    - label: "Axe"
      value: "axe"
    - label: "Bow"
      value: "bow"
  style:
    padding: 8
    borderRadius: 4
```

---

### Specialized Components

#### `ProgressBar`
Displays progress/health/mana bars.

**Properties:**
- `value`: Number (0-100)
- `max`: Number (default: 100)
- `variant`: `default` | `success` | `warning` | `danger` (default: `default`)
- `showLabel`: Boolean (default: `false`)
- `style`: Style object

**Example:**
```yaml
ProgressBar:
  value: 75
  max: 100
  variant: success
  showLabel: true
  style:
    height: 20
    borderRadius: 10
```

---

#### `Badge`
Displays a small badge or label.

**Properties:**
- `text`: String
- `variant`: `default` | `primary` | `success` | `warning` | `danger` (default: `default`)
- `style`: Style object

**Example:**
```yaml
Badge:
  text: "NEW"
  variant: primary
  style:
    fontSize: 12
    padding: 4
```

---

#### `Divider`
A horizontal or vertical divider line.

**Properties:**
- `orientation`: `horizontal` | `vertical` (default: `horizontal`)
- `style`: Style object (backgroundColor sets divider color)

**Example:**
```yaml
Divider:
  orientation: horizontal
  style:
    backgroundColor: "#333333"
    height: 1
    marginVertical: 16
```

---

#### `Spacer`
Flexible empty space for layout.

**Properties:**
- `size`: Number (fixed size in pixels) or `"auto"` (fills available space)

**Example:**
```yaml
Flex:
  direction: row
  children:
    - Text:
        text: "Left"
    - Spacer:
        size: auto
    - Text:
        text: "Right"
```

---

## Style Reference

### Dimension Properties

```yaml
width: 100              # Number (px) or "auto"
height: 100
minWidth: 50
maxWidth: 500
minHeight: 50
maxHeight: 500
```

### Spacing Properties

```yaml
# Padding (inner spacing)
padding: 16                    # All sides
paddingVertical: 12           # Top and bottom
paddingHorizontal: 20         # Left and right
paddingTop: 8
paddingRight: 8
paddingBottom: 8
paddingLeft: 8

# Margin (outer spacing)
margin: 16
marginVertical: 12
marginHorizontal: 20
marginTop: 8
marginRight: 8
marginBottom: 8
marginLeft: 8
```

### Color Properties

```yaml
color: "#00ff00"              # Text color (hex or named)
backgroundColor: "#1a1a1a"    # Background color
borderColor: "#333333"        # Border color
opacity: 0.8                  # 0.0 to 1.0

# Named colors supported
color: "red"
color: "blue"
color: "green"
color: "white"
color: "black"
color: "transparent"
```

### Typography Properties

```yaml
fontSize: 16                  # Number (px)
fontWeight: 400              # 100-900 or "normal", "bold"
fontFamily: "monospace"      # "sans", "serif", "monospace", "game"
textAlign: "center"          # "left", "center", "right", "justify"
textTransform: "uppercase"   # "none", "uppercase", "lowercase", "capitalize"
letterSpacing: 1             # Number (px)
lineHeight: 1.5              # Number (multiplier)
textDecoration: "underline"  # "none", "underline", "line-through"
```

### Border Properties

```yaml
borderWidth: 1                # All sides
borderTopWidth: 1
borderRightWidth: 1
borderBottomWidth: 1
borderLeftWidth: 1

borderStyle: "solid"          # "solid", "dashed", "dotted"
borderRadius: 8               # All corners
borderTopLeftRadius: 4
borderTopRightRadius: 4
borderBottomLeftRadius: 4
borderBottomRightRadius: 4
```

### Shadow Properties

```yaml
shadow: "small"               # Preset: "small", "medium", "large"
# Or custom shadow
shadowColor: "#000000"
shadowOffset: {x: 0, y: 2}
shadowBlur: 4
shadowOpacity: 0.25
```

### Position Properties

```yaml
position: "absolute"          # "relative", "absolute"
top: 0
right: 0
bottom: 0
left: 0
zIndex: 10                    # Stacking order
```

### Flex Item Properties

```yaml
flex: 1                       # Flex grow factor
alignSelf: "center"          # Override parent alignment
```

### Display Properties

```yaml
display: "flex"               # "flex", "none"
overflow: "hidden"           # "visible", "hidden", "scroll", "auto"
```

### Cursor Properties

```yaml
cursor: "pointer"            # "default", "pointer", "not-allowed", "text"
```

---

## Complete Examples

### Player Status Card

```yaml
Container:
  style:
    backgroundColor: "#1a1a2e"
    borderRadius: 12
    padding: 20
    shadow: "medium"
  children:
    # Header with avatar and name
    - Flex:
        direction: row
        gap: 12
        align: center
        style:
          marginBottom: 16
        children:
          - Image:
              src: "avatars/player_01.png"
              alt: "Player Avatar"
              fit: cover
              style:
                width: 64
                height: 64
                borderRadius: 32
                borderWidth: 2
                borderColor: "#00ff00"

          - Column:
              gap: 4
              children:
                - Text:
                    text: "PlayerName"
                    style:
                      fontSize: 20
                      fontWeight: 700
                      color: "#ffffff"

                - Flex:
                    direction: row
                    gap: 8
                    children:
                      - Badge:
                          text: "Level 42"
                          variant: primary
                      - Badge:
                          text: "Elite"
                          variant: success

    # Stats
    - Column:
        gap: 12
        children:
          # Health bar
          - Column:
              gap: 4
              children:
                - Flex:
                    direction: row
                    justify: spaceBetween
                    children:
                      - Text:
                          text: "Health"
                          style:
                            fontSize: 14
                            color: "#aaaaaa"
                      - Text:
                          text: "850 / 1000"
                          style:
                            fontSize: 14
                            fontWeight: 600
                            color: "#ff6b6b"

                - ProgressBar:
                    value: 85
                    variant: danger
                    style:
                      height: 12
                      borderRadius: 6

          # Mana bar
          - Column:
              gap: 4
              children:
                - Flex:
                    direction: row
                    justify: spaceBetween
                    children:
                      - Text:
                          text: "Mana"
                          style:
                            fontSize: 14
                            color: "#aaaaaa"
                      - Text:
                          text: "420 / 500"
                          style:
                            fontSize: 14
                            fontWeight: 600
                            color: "#4a90e2"

                - ProgressBar:
                    value: 84
                    variant: primary
                    style:
                      height: 12
                      borderRadius: 6

    # Action buttons
    - Flex:
        direction: row
        gap: 8
        style:
          marginTop: 16
        children:
          - Button:
              action: "attack"
              variant: danger
              style:
                flex: 1
                padding: 12
                borderRadius: 8
              children:
                - Text:
                    text: "Attack"
                    style:
                      fontSize: 16
                      fontWeight: 600
                      color: "#ffffff"

          - Button:
              action: "defend"
              variant: primary
              style:
                flex: 1
                padding: 12
                borderRadius: 8
              children:
                - Text:
                    text: "Defend"
                    style:
                      fontSize: 16
                      fontWeight: 600
                      color: "#ffffff"
```

---

### Inventory Grid

```yaml
Container:
  style:
    backgroundColor: "#16213e"
    borderRadius: 8
    padding: 16
  children:
    # Header
    - Flex:
        direction: row
        justify: spaceBetween
        align: center
        style:
          marginBottom: 16
        children:
          - Text:
              text: "Inventory"
              style:
                fontSize: 20
                fontWeight: 700
                color: "#ffffff"

          - Text:
              text: "12 / 50"
              style:
                fontSize: 14
                color: "#aaaaaa"

    # Item grid
    - Grid:
        columns: 5
        gap: 8
        children:
          # Item slot 1
          - Stack:
              alignment: center
              style:
                backgroundColor: "#0f3460"
                borderRadius: 6
                borderWidth: 1
                borderColor: "#1a4d7a"
                padding: 8
                cursor: "pointer"
              children:
                - Image:
                    src: "items/sword_001.png"
                    alt: "Iron Sword"
                    fit: contain
                    style:
                      width: 48
                      height: 48

                - Container:
                    style:
                      position: "absolute"
                      bottom: 4
                      right: 4
                    children:
                      - Text:
                          text: "x3"
                          style:
                            fontSize: 12
                            fontWeight: 700
                            color: "#ffffff"
                            backgroundColor: "#000000"
                            padding: 2
                            borderRadius: 3

          # Item slot 2 (empty)
          - Container:
              style:
                backgroundColor: "#0f3460"
                borderRadius: 6
                borderWidth: 1
                borderColor: "#1a4d7a"
                padding: 8
                height: 64
                cursor: "pointer"

          # ... more slots
```

---

### Login Form

```yaml
Container:
  style:
    backgroundColor: "#ffffff"
    borderRadius: 16
    padding: 32
    shadow: "large"
    maxWidth: 400
  children:
    # Logo
    - Container:
        style:
          marginBottom: 24
        children:
          - Text:
              text: "NullTrace"
              style:
                fontSize: 32
                fontWeight: 800
                color: "#1a1a1a"
                textAlign: "center"

    # Form fields
    - Column:
        gap: 16
        children:
          # Username
          - Column:
              gap: 6
              children:
                - Text:
                    text: "Username"
                    style:
                      fontSize: 14
                      fontWeight: 600
                      color: "#333333"

                - Input:
                    name: "username"
                    placeholder: "Enter your username"
                    style:
                      padding: 12
                      borderRadius: 8
                      borderWidth: 1
                      borderColor: "#dddddd"
                      fontSize: 16

          # Password
          - Column:
              gap: 6
              children:
                - Text:
                    text: "Password"
                    style:
                      fontSize: 14
                      fontWeight: 600
                      color: "#333333"

                - Input:
                    name: "password"
                    type: "password"
                    placeholder: "Enter your password"
                    style:
                      padding: 12
                      borderRadius: 8
                      borderWidth: 1
                      borderColor: "#dddddd"
                      fontSize: 16

          # Remember me
          - Checkbox:
              name: "remember"
              label: "Remember me"
              style:
                marginTop: 8

          # Submit button
          - Button:
              action: "login"
              variant: primary
              style:
                padding: 14
                borderRadius: 8
                backgroundColor: "#4a90e2"
                marginTop: 8
              children:
                - Text:
                    text: "Sign In"
                    style:
                      fontSize: 16
                      fontWeight: 700
                      color: "#ffffff"
                      textAlign: "center"
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
Define reusable theme tokens for consistency:

```yaml
theme:
  colors:
    primary: "#4a90e2"
    danger: "#ff6b6b"
    success: "#51cf66"
    background: "#1a1a1a"
    text: "#ffffff"

  spacing:
    small: 8
    medium: 16
    large: 24

  borderRadius:
    small: 4
    medium: 8
    large: 16

# Use in components
Button:
  style:
    backgroundColor: "$theme.colors.primary"
    padding: "$theme.spacing.medium"
    borderRadius: "$theme.borderRadius.medium"
```

---

## Browser NTML Rendering

The in-app **Browser** renders NTML pages served by VMs (e.g. ntml.org) by converting NTML to safe HTML. The renderer supports all components and style properties defined in this document.

### Supported Features

- **All layout components:** Container, Flex, Row, Column, Grid, Stack
- **All content components:** Text, Image, Icon
- **All interactive components:** Button, Link, Input, Checkbox, Radio, Select
- **All display components:** ProgressBar, Badge, Divider, Spacer
- **Full style support:** All properties in the Style Reference (dimensions, spacing, typography, borders, shadow, position, flex, display, overflow, cursor)
- **Layout props:** Flex/Row/Column `justify`, `align`, `wrap`; Grid `columns` and `rows`; Stack `alignment`; Divider `orientation`; Spacer `size: "auto"`

### Image Resolution

When NTML is rendered in the Browser, Image `src` paths are resolved against the page base URL. For example, a page at `http://ntml.org/about` with `Image: src: "img/logo.png"` will load `http://ntml.org/img/logo.png`. The `fit` property maps to CSS `object-fit` (cover, contain, fill, none, scale-down).

### Lua Scripts and Patches

Pages with `head.scripts` can run Lua in a sandbox. Button `action` values without `:` call Lua functions. The `ui` API (`set_text`, `set_visible`, `set_value`, `set_disabled`) applies patches that update the rendered HTML on the next frame.

---

## Migration & Updates

This specification is versioned. Current version: **1.0.0**

Breaking changes will increment major version. Always specify version in your NTML files:

```yaml
version: "1.0.0"
root:
  Container:
    children:
      # ...
```

---

## FAQ

**Q: Why YAML instead of JSON?**
A: YAML is more human-readable, supports comments, and has less syntax overhead for nested structures.

**Q: Can I use custom fonts?**
A: Only fonts approved and bundled with the game. Custom font loading is disabled for security.

**Q: How do I handle dynamic data?**
A: NTML is a markup language. Data binding is handled by the game engine using template variables (e.g., `{{playerName}}`).

**Q: Can I create custom components?**
A: Not directly. You can request new components through the game's modding API, which will be validated and approved.

**Q: Is animation supported?**
A: Animation is handled by the game engine, not NTML. You can define states, and the engine will interpolate between them.

---

## Additional Resources

- [Game Modding Guide](./modding-guide.md)
- [UI Design Guidelines](./ui-design-guidelines.md)
- [Theme Customization](./theme-customization.md)
- [Accessibility Best Practices](./accessibility.md)
