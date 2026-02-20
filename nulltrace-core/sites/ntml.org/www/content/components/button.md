# Button

**Interactive**

A clickable button that calls a Lua function by name. Supports four visual variants and can be disabled via Lua. Children define the button content.

---

## Props

```ntml
action   string              - Lua function name to call on click
variant  primary|secondary|danger|ghost  (default: primary)
id       string              - Used with ui.set_disabled(id, bool)
class    string              - Tailwind utility classes
children any components      - Button content (usually Text or Row)
```

---

## Variants

```ntml
<!-- primary: filled amber (default) -->
<Button action="submit" variant="primary">
  <Text text="Submit" class="font-medium" />
</Button>

<!-- secondary: outlined / subdued -->
<Button action="back" variant="secondary">
  <Text text="Back" />
</Button>

<!-- danger: red, for destructive actions -->
<Button action="deleteItem" variant="danger">
  <Text text="Delete" class="font-medium" />
</Button>

<!-- ghost: transparent, minimal styling -->
<Button action="cancel" variant="ghost">
  <Text text="Cancel" class="text-zinc-400" />
</Button>
```

---

## With icon

```ntml
<Button action="sendMessage" variant="primary">
  <Row gap="2" align="center">
    <Icon name="send" size="16" />
    <Text text="Send" class="font-medium" />
  </Row>
</Button>

<Button action="refreshData" variant="secondary">
  <Row gap="2" align="center">
    <Icon name="refresh-cw" size="16" />
    <Text text="Refresh" />
  </Row>
</Button>
```

---

## Passing parameters (data-* attributes)

Use `data-*` attributes to pass values to your handler. The handler receives a context object with `eventData` containing these attributes.

```ntml
<Button action="deleteItem" data-item-id="42" data-item-name="Old File">
  <Text text="Delete" class="font-medium" />
</Button>
```

```lua
function deleteItem(ctx)
  local id = ctx.eventData["item-id"]      -- "42"
  local name = ctx.eventData["item-name"]  -- "Old File"
  -- ctx.formValues has form field values
  -- ctx.targetId is the button's id if set
  -- ...
end
```

---

## Disable from Lua

Give the button an id and use ui.set_disabled to prevent double-clicks during async operations.

```ntml
<Button id="submit-btn" action="submitForm" variant="primary">
  <Text text="Submit" class="font-medium" />
</Button>
```

```lua
function submitForm()
  ui.set_disabled("submit-btn", true)
  local res = http.post("/api/submit", { data = "..." })
  ui.set_disabled("submit-btn", false)
  if res.ok then
    ui.set_text("status", "Submitted!")
  end
end
```

---

## Button groups

```ntml
<Row gap="3" justify="end">
  <Button action="closePanel" variant="ghost">
    <Text text="Cancel" class="text-zinc-400" />
  </Button>
  <Button action="confirmDelete" variant="danger">
    <Text text="Delete" class="font-medium" />
  </Button>
</Row>
```
