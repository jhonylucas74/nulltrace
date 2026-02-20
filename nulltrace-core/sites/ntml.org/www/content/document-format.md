# Document format

NTML supports two formats: classic (single root element) and full (head + body) for metadata, fonts, scripts, and imports.

---

## Classic format

One root element at the document root. No head or body. Best for simple pages and reusable component files.

```ntml
<Container class="p-6 bg-zinc-800 rounded-lg">
  <Text text="Hello, NullTrace!" class="text-lg text-zinc-100" />
</Container>
```

The renderer treats the single root element as the entire document body.

---

## Full format (head + body)

When head is present, body is required. Use the full format when you need metadata, fonts, scripts, or imported components.

```ntml
<head>
  <title>My App</title>
  <description>Dashboard</description>
  <font family="Roboto Mono" weights="400,700" />
  <script src="scripts/main.lua" />
  <import src="components/nav.ntml" as="Nav" />
</head>
<body>
  <Column>
    <Nav title="My App" />
    <Text text="Content" />
  </Column>
</body>
```

---

## Head elements

- **title** (required) - Page title shown in the browser tab.
- **description** - Short description of the document.
- **author** - Author name or ID.
- **tags** - Space-separated tags for search indexing (max 10, lowercase, no spaces in each tag).

```ntml
<head>
  <title>Player Dashboard</title>
  <description>Real-time player stats and inventory</description>
  <author>system</author>
  <tags>dashboard stats inventory</tags>
</head>
```

---

## Fonts

Import custom fonts with the `font` element. Use the family name in Tailwind (`font-[family]`) or the `fontFamily` style prop.

```ntml
<head>
  <title>Styled App</title>
  <font family="JetBrains Mono" weights="400,700" />
  <font family="Inter" weights="300,400,500,600" />
</head>
<body>
  <Text text="Monospace text" class="font-[JetBrains_Mono]" />
</body>
```

---

## Scripts

Link Lua scripts with `script` elements. Scripts run in a sandboxed environment. Multiple scripts are loaded in order.

```ntml
<head>
  <title>Interactive App</title>
  <script src="scripts/auth.lua" />
  <script src="scripts/ui.lua" />
</head>
<body>
  <Button action="doLogin" variant="primary">
    <Text text="Login" />
  </Button>
</body>
```

Max 5 scripts per document. Each script has a 500-line limit.

---

## Component imports

Import reusable NTML component files with `import` elements. Use the alias as a tag in the body.

```ntml
<head>
  <title>Main Menu</title>
  <import src="components/navbar.ntml" as="Navbar" />
  <import src="components/footer.ntml" as="Footer" />
</head>
<body>
  <Column class="min-h-screen">
    <Navbar active="home" />
    <Container class="flex-1 p-6">
      <Text text="Welcome" class="text-2xl font-bold text-zinc-100" />
    </Container>
    <Footer />
  </Column>
</body>
```

Component files must use the classic format (single root element) and declare their props at the top.
