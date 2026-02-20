# ProgressBar / Badge

**Display**

ProgressBar shows a filled bar to indicate completion or quantity. Badge is a small status label. Both support semantic color variants.

---

## ProgressBar props

```ntml
value       number   (required) - current value
max         number              - maximum value (default: 100)
variant     default|success|warning|danger  - color scheme
showLabel   true|false          - show value label (default: false)
id          string              - ui.set_value(id, newValue)
class       string              - Tailwind utility classes
```

---

## ProgressBar examples

```ntml
<!-- Health / energy bars -->
<Column gap="3">
  <Row gap="3" align="center">
    <Text text="HP" class="text-xs text-zinc-400 w-6 font-mono" />
    <ProgressBar value="85" max="100" variant="success" class="flex-1 h-2 rounded-full" />
    <Text text="85%" class="text-xs text-zinc-500 w-8 text-right font-mono" />
  </Row>
  <Row gap="3" align="center">
    <Text text="EN" class="text-xs text-zinc-400 w-6 font-mono" />
    <ProgressBar value="40" max="100" variant="warning" class="flex-1 h-2 rounded-full" />
    <Text text="40%" class="text-xs text-zinc-500 w-8 text-right font-mono" />
  </Row>
  <Row gap="3" align="center">
    <Text text="SH" class="text-xs text-zinc-400 w-6 font-mono" />
    <ProgressBar value="10" max="100" variant="danger" class="flex-1 h-2 rounded-full" />
    <Text text="10%" class="text-xs text-zinc-500 w-8 text-right font-mono" />
  </Row>
</Column>

<!-- Upload / progress indicator -->
<Column gap="2">
  <Row gap="3" align="center" justify="spaceBetween">
    <Text text="Uploading..." class="text-sm text-zinc-400" />
    <Text id="upload-pct" text="0%" class="text-sm text-zinc-400 font-mono" />
  </Row>
  <ProgressBar id="upload-bar" value="0" max="100" variant="default" class="h-1.5" />
</Column>
```

---

## Update from Lua

```lua
-- Update progress bar value
ui.set_value("upload-bar", 75)
ui.set_text("upload-pct", "75%")
```

---

## Badge props

```ntml
text     string   (required) - badge label
variant  default|primary|success|warning|danger  - color scheme
class    string              - Tailwind utility classes
```

---

## Badge examples

```ntml
<!-- Status badges -->
<Row gap="2" wrap="true">
  <Badge text="Online" variant="success" />
  <Badge text="Idle" variant="default" />
  <Badge text="Warning" variant="warning" />
  <Badge text="Critical" variant="danger" />
  <Badge text="Primary" variant="primary" />
</Row>

<!-- Notification counter -->
<Row gap="2" align="center">
  <Icon name="bell" size="20" class="text-zinc-400" />
  <Badge text="3" variant="danger" />
</Row>

<!-- In a list row -->
<Row gap="3" align="center" justify="spaceBetween">
  <Text text="api-server" class="text-sm font-mono text-zinc-300" />
  <Badge text="Vulnerable" variant="warning" />
</Row>
```
