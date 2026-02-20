# Styling

Every NTML component accepts a `class` attribute for Tailwind utility classes, plus individual style props. Tailwind is always available - no imports needed.

---

## Tailwind (class attribute)

Set the `class` attribute to any space-separated list of Tailwind v3 utility classes. The renderer handles class resolution and dark mode.

```ntml
<Container class="p-6 bg-zinc-800 rounded-xl border border-zinc-700 shadow-lg">
  <Text text="Styled with Tailwind" class="text-lg font-semibold text-amber-400" />
</Container>
```

See the [Tailwind reference](/tailwind-reference) page for common patterns.

---

## Spacing

Control padding, margin, and gap with Tailwind or style props. `gap` on layout components controls spacing between children.

```ntml
<!-- Tailwind spacing -->
<Column gap="6" class="p-8 m-4">
  <Text text="8 units padding, 4 units margin" />
</Column>

<!-- Style props -->
<Container style="padding: 24; marginHorizontal: 16">
  <Text text="Same result" />
</Container>

<!-- Per-side control -->
<Container class="pt-4 pb-8 px-6">
  <Text text="Asymmetric padding" />
</Container>
```

---

## Typography

Text sizing, weight, family, alignment, and decoration.

```ntml
<Column gap="3">
  <Text text="Heading XL" class="text-4xl font-bold tracking-tight text-zinc-100" />
  <Text text="Body text" class="text-base text-zinc-400 leading-relaxed" />
  <Text text="Small mono" class="text-sm font-mono text-zinc-500" />
  <Text text="UPPERCASE LABEL" class="text-xs font-semibold uppercase tracking-widest text-zinc-500" />
  <Text text="Accent" class="text-lg font-semibold text-amber-400" />
</Column>
```

---

## Colors

Use Tailwind color utilities for text, background, and border. Opacity modifier (`/50`, `/30`, `/15`) creates transparent variants.

```ntml
<!-- Backgrounds -->
<Container class="bg-zinc-900" />     <!-- solid dark -->
<Container class="bg-zinc-800/50" />  <!-- 50% transparent -->
<Container class="bg-amber-500/15" /> <!-- subtle accent tint -->

<!-- Text colors -->
<Text text="Primary" class="text-zinc-100" />
<Text text="Muted" class="text-zinc-400" />
<Text text="Accent" class="text-amber-400" />
<Text text="Danger" class="text-red-400" />
<Text text="Success" class="text-green-400" />

<!-- Borders -->
<Container class="border border-zinc-700" />
<Container class="border border-amber-500/30" />
```

---

## Borders and radius

```ntml
<Container class="rounded-md" />    <!-- 6px -->
<Container class="rounded-lg" />    <!-- 8px -->
<Container class="rounded-xl" />    <!-- 12px -->
<Container class="rounded-2xl" />   <!-- 16px -->
<Container class="rounded-full" />  <!-- pill shape -->

<Container class="border border-zinc-700 rounded-lg" />
<Container class="border-2 border-amber-500 rounded-xl" />
```

---

## Style props reference

All components accept these style props as XML attributes in addition to the `class` attribute.

```
-- Dimensions (number or "auto")
width, height, minWidth, maxWidth, minHeight, maxHeight

-- Spacing (number)
padding, paddingVertical, paddingHorizontal
paddingTop, paddingRight, paddingBottom, paddingLeft
margin, marginVertical, marginHorizontal
marginTop, marginRight, marginBottom, marginLeft

-- Colors (hex or named)
color, backgroundColor, borderColor
opacity  (0.0 - 1.0)

-- Typography
fontSize, fontWeight, fontFamily
textAlign, textTransform, letterSpacing, lineHeight, textDecoration

-- Borders
borderWidth, borderStyle, borderRadius

-- Shadow: "small" | "medium" | "large"

-- Position
position, top, right, bottom, left, zIndex

-- Flex
flex, alignSelf

-- Display
display, overflow, cursor
```
