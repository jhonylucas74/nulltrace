# Input · Checkbox · Radio · Select

**Interactive**

Form components for collecting user input. Use the name attribute to read values from Lua with ui.get_value(name). Use id with ui.set_disabled to control state.

---

## Input

```ntml
name         string   (required) - used with ui.get_value(name)
placeholder  string              - placeholder text
type         text|password|number  (default: text)
value        string              - default value
id           string              - ui.set_disabled(id, bool)
class        string              - Tailwind utility classes
```

```ntml
<Input name="username" placeholder="Username"
       class="w-full p-2.5 rounded-lg bg-zinc-900 border border-zinc-600 text-zinc-200 text-sm" />

<Input name="password" type="password" placeholder="Password"
       class="w-full p-2.5 rounded-lg bg-zinc-900 border border-zinc-600 text-zinc-200 text-sm" />

<Input name="target_ip" placeholder="192.168.x.x" value="10.0.1.1"
       class="w-full p-2 rounded bg-zinc-800 border border-zinc-700 text-sm font-mono text-zinc-300" />
```

---

## Checkbox

```ntml
name     string    - ui.get_value returns "true" or "false"
label    string    - text shown next to the checkbox
checked  true|false - initial state
class    string    - Tailwind utility classes
```

```ntml
<Checkbox name="agree_terms" label="I agree to the terms" />
<Checkbox name="remember_me" label="Remember me" checked="true" />
<Checkbox name="notify_email" label="Email notifications" />
```

---

## Radio

Group multiple Radio buttons with the same name. ui.get_value returns the value of the selected radio.

```ntml
name     string    - group identifier
value    string    - this option's value
label    string    - text shown next to the radio

<Column gap="2">
  <Radio name="role" value="viewer" label="Viewer" />
  <Radio name="role" value="editor" label="Editor" />
  <Radio name="role" value="admin" label="Admin" />
</Column>
```

---

## Select

Dropdown with Option children. ui.get_value returns the selected option's value.

```ntml
name     string    - ui.get_value returns selected option value
value    string    - default selected option value
class    string    - Tailwind utility classes
children <option label="..." value="..." />  - at least one option
```

```ntml
<Select name="theme" value="Dark"
        class="w-full p-2.5 rounded-lg bg-zinc-900 border border-zinc-600 text-zinc-200 text-sm">
  <option label="Dark" value="Dark" />
  <option label="Light" value="Light" />
  <option label="System" value="System" />
</Select>

<Select name="priority" value="Medium"
        class="p-2 rounded bg-zinc-800 border border-zinc-700 text-sm text-zinc-300">
  <option label="Low" value="Low" />
  <option label="Medium" value="Medium" />
  <option label="High" value="High" />
  <option label="Critical" value="Critical" />
</Select>
```

---

## Reading values in Lua

```lua
function onSubmit()
  local username = ui.get_value("username")       -- string
  local password = ui.get_value("password")       -- string
  local remember  = ui.get_value("remember_me")   -- "true" or "false"
  local role      = ui.get_value("role")          -- "viewer"/"editor"/"admin"
  local theme     = ui.get_value("theme")         -- "Dark"/"Light"/"System"

  if remember == "true" then
    -- store session...
  end
end
```
