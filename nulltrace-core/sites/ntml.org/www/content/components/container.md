# Container

**Layout**

A generic block-level wrapper element. Renders as a div with no default layout behavior. Use it to group elements, apply styling, or provide a target for Lua ui.set_visible.

---

## Props

```ntml
class    string    - Tailwind utility classes
id       string    - Used with ui.set_visible(id, bool)

-- Plus all style props (width, height, padding, margin,
-- backgroundColor, borderColor, borderRadius, etc.)
```

---

## Basic usage

```ntml
<Container class="p-6 bg-zinc-800 rounded-xl border border-zinc-700">
  <Text text="Content inside a Container" class="text-zinc-200" />
</Container>
```

---

## As a card

```ntml
<Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700 shadow-lg max-w-sm">
  <Column gap="3">
    <Text text="Card title" class="text-base font-semibold text-zinc-100" />
    <Text text="Card description goes here." class="text-sm text-zinc-400" />
  </Column>
</Container>
```

---

## As a Lua target

Give a Container an id to show or hide it from Lua with ui.set_visible.

```ntml
<Container id="error-panel" class="p-4 bg-red-900/20 border border-red-500/30 rounded-lg hidden">
  <Text id="error-text" text="" class="text-sm text-red-400" />
</Container>
```

```lua
-- In a Lua script
ui.set_text("error-text", "Something went wrong.")
ui.set_visible("error-panel", true)
```

---

## Notes

Container has no inherent layout. To lay out children, use Column, Row, Flex, or Grid instead. Container is the equivalent of a plain div.
