# Lua API

Scripts declared in head run in a sandboxed Lua environment. Only the `ui`, `http`, and `storage` modules are available. Button `action` attributes call Lua functions by name when clicked.

---

## ui - DOM manipulation

Read and modify component state from Lua scripts. Components must have an `id` attribute to be targeted by `ui` functions.

```lua
ui.get_value(name)        -- value of Input/Checkbox/Select by name attr
ui.set_text(id, text)     -- set Text content by id
ui.set_visible(id, bool)  -- show or hide a component by id
ui.set_value(id, val)     -- set ProgressBar value by id
ui.set_disabled(id, bool) -- enable/disable Button or Input by id
```

---

## http - HTTP requests

Make HTTP requests to in-game servers. Requests are synchronous and have a 200ms timeout.

```lua
local res = http.get("http://api.corp.local/status")
-- res.ok      boolean
-- res.status  number (e.g. 200)
-- res.data    parsed table (if JSON) or string
-- res.error   string or nil

http.post(url, body)    -- body: table or string
http.put(url, body)
http.patch(url, body)
http.delete(url)
```

---

## storage - Persistent storage

Save and load data that persists between page visits. Each origin gets its own isolated storage.

```lua
storage.set("key", "value")  -- store a string value
storage.get("key")           -- returns string or nil
storage.remove("key")        -- delete a key
storage.keys()               -- returns table of all keys
storage.clear()              -- delete all keys for this origin
```

---

## Button actions

Set `action="functionName"` on a Button. The runtime calls that Lua function when clicked. Handlers can accept an optional context parameter (React-style).

### Handler parameters (ctx)

Handlers receive a context object as the first argument. You can use it or ignore it (backward compatible).

| Field | Type | Description |
|-------|------|-------------|
| `ctx.eventData` | table | data-* attributes from the element (e.g. `data-item-id` -> `ctx.eventData["item-id"]`) |
| `ctx.formValues` | table | Form field values (Input/Checkbox/Select by name) |
| `ctx.targetId` | string | id of the clicked element, or nil |

```lua
-- Without parameters (still works)
function doLogin()
  local user = ui.get_value("username")
  local pass = ui.get_value("password")
  -- ...
end

-- With context parameter (React-style)
function deleteItem(ctx)
  local itemId = ctx.eventData["item-id"]
  local form = ctx.formValues
  -- ctx.targetId is the button's id if set
end
```

### Example: no parameters

```lua
function doLogin()
  local user = ui.get_value("username")
  local pass = ui.get_value("password")
  local res = http.post("/api/login", {
    username = user,
    password = pass
  })
  if res.ok then
    ui.set_text("msg", "Welcome, " .. user .. "!")
    ui.set_visible("form", false)
  else
    ui.set_text("msg", res.error or "Login failed")
  end
end
```

Input, Checkbox, Radio, and Select `onchange` handlers receive the same context object when the event fires.

---

## Accessing component values

Use the `name` attribute on form elements, and `id` on components you want to manipulate.

```ntml
<Column gap="4" id="form">
  <Input name="username" placeholder="Username" />
  <Input name="password" type="password" placeholder="Password" />
  <Button action="doLogin" variant="primary">
    <Text text="Sign in" />
  </Button>
  <Text id="msg" text="" class="text-sm text-red-400" />
</Column>
```

---

## Sandbox limits

- 200ms timeout per handler
- 500 lines per script
- 5 scripts per document
- Blocked globals: `io`, `os`, `file`, `require`, `dofile`, `loadfile`
