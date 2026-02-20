# Best practices

Conventions and patterns that keep NTML code maintainable, consistent, and performant across your NullTrace projects.

---

## File structure

Organize large projects with a consistent folder layout. Split UI into screens, reusable components, and themes.

```
/var/www/
  index.ntml             -- main entry
  dashboard.ntml
  settings.ntml
  components/
    navbar.ntml
    stat-card.ntml
    player-badge.ntml
  scripts/
    auth.lua
    dashboard.lua
    utils.lua
  themes/
    dark.yaml
    corp.yaml
```

Use kebab-case for filenames (`player-stats.ntml`, `hack-panel.ntml`). Keep component files in `components/` and scripts in `scripts/`.

---

## Naming conventions

Consistent naming prevents bugs and makes Lua scripts easier to write.

```ntml
<!-- Button actions: snake_case Lua function names -->
<Button action="hack_system" />
<Button action="send_message" />
<Button action="submit_login" />

<!-- Input names: snake_case (used in ui.get_value) -->
<Input name="username_input" />
<Input name="target_ip" />

<!-- Component IDs: kebab-case (used in ui.set_text etc) -->
<Text id="status-message" text="" />
<ProgressBar id="health-bar" value="100" max="100" />
<Container id="result-panel" />
```

---

## Layout patterns

Prefer `Flex`/`Grid` over deeply nested `Container`s. Use `gap` instead of margin for consistent spacing.

```ntml
<!-- Good: flat structure with gap -->
<Column gap="4" class="p-6">
  <Row gap="3" align="center">
    <Icon name="user" size="20" class="text-amber-400" />
    <Text text="Player" class="font-medium text-zinc-200" />
  </Row>
  <Text text="Stats below" class="text-zinc-400 text-sm" />
</Column>

<!-- Avoid: deeply nested containers for spacing -->
<Container>
  <Container class="p-2">
    <Container class="p-2">
      <Text text="Nested padding anti-pattern" />
    </Container>
  </Container>
</Container>
```

---

## Reusable components

Extract repeated UI patterns into component files. Import them with aliases in the full format head.

```ntml
<!-- components/stat-card.ntml -->
<Container class="p-4 bg-zinc-800 rounded-lg border border-zinc-700">
  <Column gap="2">
    <Text text="$props.label" class="text-xs text-zinc-400 uppercase tracking-wider" />
    <Text text="$props.value" class="text-2xl font-bold text-zinc-100 tabular-nums" />
    <ProgressBar value="$props.value" max="100" />
  </Column>
</Container>

<!-- dashboard.ntml -->
<head>
  <title>Dashboard</title>
  <import src="components/stat-card.ntml" as="StatCard" />
</head>
<body>
  <Row gap="4" wrap="true">
    <StatCard label="Health" value="85" />
    <StatCard label="Energy" value="60" />
  </Row>
</body>
```

---

## Scripting patterns

Keep Lua scripts focused. One script per feature, not one massive script. Always check `ok` before using response data.

```lua
-- Good: always check res.ok
function loadData()
  ui.set_disabled("refresh-btn", true)
  ui.set_text("status", "Loading...")
  local res = http.get("/api/data")
  if res.ok then
    ui.set_text("value", tostring(res.data.count))
    ui.set_text("status", "")
  else
    ui.set_text("status", "Error: " .. (res.error or "unknown"))
  end
  ui.set_disabled("refresh-btn", false)
end
```

---

## Anti-patterns to avoid

```ntml
<!-- Don't: hardcode colors as hex when Tailwind classes exist -->
<Text text="Bad" style="color: #f59e0b" />
<Text text="Good" class="text-amber-400" />

<!-- Don't: use id on non-manipulated components -->
<Text id="static-label" text="Username:" />  <!-- unnecessary -->
<Input id="username" name="username" />       <!-- id not needed for get_value -->

<!-- Don't: mix style props and Tailwind for the same property -->
<Container class="p-4" style="padding: 8" />  <!-- ambiguous -->

<!-- Don't: nest more than 4 levels deep without extracting a component -->
```

---

## Themes

Define theme variables in a YAML theme file. Reference them with `$theme.*` in props. Theme files let players skin your UI.

```yaml
# themes/dark.yaml
colors:
  primary: "#f59e0b"
  background: "#09090b"
  surface: "#18181b"
  border: "#3f3f46"
spacing:
  sm: 4
  md: 8
  lg: 16
```
