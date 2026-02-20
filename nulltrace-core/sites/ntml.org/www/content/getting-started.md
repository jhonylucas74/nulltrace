# Getting started

NTML is a declarative XML language that runs inside NullTrace. No installation needed - just create an `.ntml` file, serve it with `httpd`, and open it in the in-game browser.

---

## Step 1 - Your first file

Create a file called `index.ntml` in your VM's `/var/www` directory. The simplest NTML document is a single root element (classic format).

```ntml
<Container class="p-8 bg-zinc-900 min-h-screen">
  <Text text="Hello, NullTrace!" class="text-2xl font-bold text-zinc-100" />
</Container>
```

Open the in-game browser and navigate to your VM's hostname. You should see the text rendered.

---

## Step 2 - Add layout and components

Nest components inside layout primitives like `Column`, `Row`, and `Container` to build your UI.

```ntml
<Column gap="6" class="p-8 bg-zinc-900 min-h-screen">
  <Row gap="3" align="center">
    <Icon name="terminal" size="24" class="text-amber-400" />
    <Text text="System Monitor" class="text-xl font-semibold text-zinc-100" />
  </Row>
  <Container class="p-4 bg-zinc-800 rounded-lg border border-zinc-700">
    <Column gap="3">
      <Text text="CPU" class="text-xs text-zinc-400 uppercase tracking-wider" />
      <ProgressBar value="72" max="100" variant="success" class="h-2" />
    </Column>
  </Container>
</Column>
```

---

## Step 3 - Apply styles with Tailwind

Use the `class` attribute with Tailwind utility classes on any component. Tailwind is always available - no import needed.

```ntml
<Row gap="3" wrap="true">
  <Container class="px-4 py-2 rounded-full bg-amber-500/15 border border-amber-500/30">
    <Text text="Online" class="text-sm text-amber-400 font-medium" />
  </Container>
  <Container class="px-4 py-2 rounded-full bg-zinc-800 border border-zinc-700">
    <Text text="Level 12" class="text-sm text-zinc-300" />
  </Container>
</Row>
```

---

## Step 4 - Add interactivity with Lua

Use the full format (`head` + `body`) to attach Lua scripts. Button `action` attributes call Lua functions by name.

```ntml
<head>
  <title>Counter</title>
  <script src="counter.lua" />
</head>
<body>
  <Column gap="6" align="center" class="p-12">
    <Text id="count" text="0" class="text-6xl font-bold text-zinc-100 tabular-nums" />
    <Row gap="3">
      <Button action="decrement" variant="secondary">
        <Text text="-" class="text-lg font-bold px-2" />
      </Button>
      <Button action="increment" variant="primary">
        <Text text="+" class="text-lg font-bold px-2" />
      </Button>
    </Row>
  </Column>
</body>
```

```lua
-- counter.lua
local count = 0

function increment()
  count = count + 1
  ui.set_text("count", tostring(count))
end

function decrement()
  count = count - 1
  ui.set_text("count", tostring(count))
end
```

The script maintains state between button clicks. `ui.set_text` updates the `Text` component with `id="count"`.

---

## Full format with fonts and imports

A real-world NTML document with all head elements. Use `font` to load custom fonts, `import` to reuse component files.

```ntml
<head>
  <title>Player Dashboard</title>
  <font family="JetBrains Mono" weights="400,700" />
  <script src="scripts/dashboard.lua" />
  <import src="components/stat-card.ntml" as="StatCard" />
  <import src="components/navbar.ntml" as="Navbar" />
</head>
<body>
  <Column class="min-h-screen bg-zinc-950">
    <Navbar active="dashboard" />
    <Column gap="6" class="p-8 max-w-4xl mx-auto w-full">
      <Heading level="1" text="Dashboard" class="text-2xl font-bold text-zinc-100" />
      <Row gap="4" wrap="true">
        <StatCard label="Health" value="85" max="100" variant="success" />
        <StatCard label="Energy" value="60" max="100" variant="default" />
      </Row>
    </Column>
  </Column>
</body>
```
