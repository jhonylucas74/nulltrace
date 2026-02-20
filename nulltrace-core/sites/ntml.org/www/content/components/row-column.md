# Row / Column

**Layout**

Shorthand layout components. Row is a horizontal flex container (flex-direction: row). Column is a vertical flex container (flex-direction: column). Both support gap, alignment, and wrapping.

---

## Props

```ntml
gap      number             - space between children (multiples of 4px)
align    start|center|end|stretch|baseline  - cross-axis alignment
justify  start|center|end|spaceBetween|spaceAround  - main-axis alignment
wrap     true|false         - allow children to wrap (Row only)
class    string             - Tailwind utility classes
id       string             - Lua target id
```

---

## Row - horizontal layout

```ntml
<!-- Basic row -->
<Row gap="4">
  <Text text="Left" />
  <Text text="Middle" />
  <Text text="Right" />
</Row>

<!-- Space between -->
<Row gap="3" justify="spaceBetween" align="center">
  <Text text="Label" class="text-zinc-200 font-medium" />
  <Badge text="Active" variant="success" />
</Row>

<!-- Row with wrap (responsive) -->
<Row gap="3" wrap="true">
  <Container class="p-3 bg-zinc-800 rounded-lg min-w-[120px]">
    <Text text="Card 1" />
  </Container>
  <Container class="p-3 bg-zinc-800 rounded-lg min-w-[120px]">
    <Text text="Card 2" />
  </Container>
</Row>
```

---

## Column - vertical layout

```ntml
<!-- Basic column -->
<Column gap="4" class="p-6">
  <Heading level="2" text="Section" class="text-xl font-semibold text-zinc-100" />
  <Text text="Description text below the heading." class="text-zinc-400" />
  <Button action="doAction" variant="primary">
    <Text text="Continue" />
  </Button>
</Column>

<!-- Centered column -->
<Column gap="6" align="center" class="min-h-screen justify-center">
  <Icon name="check-circle" size="48" class="text-green-400" />
  <Text text="Success!" class="text-2xl font-bold text-zinc-100" />
</Column>
```

---

## Nesting

Row and Column compose naturally. Each establishes its own flex context.

```ntml
<Column gap="0" class="min-h-screen bg-zinc-950">
  <!-- Header -->
  <Row gap="4" align="center" justify="spaceBetween" class="px-6 py-4 border-b border-zinc-800">
    <Text text="App Name" class="font-semibold text-zinc-100" />
    <Row gap="2">
      <Link href="/help"><Text text="Help" class="text-sm text-zinc-400" /></Link>
    </Row>
  </Row>
  <!-- Body -->
  <Row gap="0" class="flex-1">
    <Column gap="2" class="w-48 border-r border-zinc-800 p-4">
      <Text text="Sidebar" class="text-sm text-zinc-400" />
    </Column>
    <Column gap="4" class="flex-1 p-8">
      <Text text="Main content" class="text-zinc-200" />
    </Column>
  </Row>
</Column>
```
