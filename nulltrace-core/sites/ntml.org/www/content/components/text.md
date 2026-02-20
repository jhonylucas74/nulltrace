# Text / Heading

**Content**

Text renders a styled text node. Heading renders semantic h1-h3 elements with appropriate default sizing. Both support Tailwind classes for full typography control.

---

## Text props

```ntml
text    string   (required) - the text content to display
id      string              - Lua target for ui.set_text(id, newText)
class   string              - Tailwind utility classes
```

---

## Heading props

```ntml
level   1|2|3    (required) - heading level (h1, h2, h3)
text    string   (required) - heading text
class   string              - Tailwind utility classes
```

---

## Typography scale

```ntml
<Heading level="1" text="Page Title" class="text-3xl font-bold text-zinc-100 tracking-tight" />
<Heading level="2" text="Section Title" class="text-xl font-semibold text-zinc-100" />
<Heading level="3" text="Subsection" class="text-base font-semibold text-zinc-200" />
<Text text="Body text" class="text-base text-zinc-400 leading-relaxed" />
<Text text="Small text" class="text-sm text-zinc-400" />
<Text text="Caption / label" class="text-xs text-zinc-500 uppercase tracking-wider" />
<Text text="Monospace" class="text-sm font-mono text-zinc-300" />
```

---

## Lua - dynamic text

Give Text an id to update its content from Lua with ui.set_text.

```ntml
<Text id="status-label" text="Ready" class="text-sm text-zinc-400" />
<Text id="score-counter" text="0" class="text-4xl font-bold tabular-nums text-zinc-100" />
```

```lua
ui.set_text("status-label", "Uploading...")
ui.set_text("score-counter", tostring(score))
```
