# Grid / Stack

**Layout**

Grid arranges children in a fixed-column CSS grid. Stack layers children on top of each other (position: relative with absolute children) for overlay patterns.

---

## Grid props

```ntml
columns  number   - number of columns (required)
rows     number   - number of rows (optional, auto by default)
gap      number   - gap between cells
class    string   - Tailwind utility classes
```

---

## Grid examples

```ntml
<!-- 3-column metric grid -->
<Grid columns="3" gap="4">
  <Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700">
    <Column gap="1">
      <Text text="VMs" class="text-xs text-zinc-400 uppercase tracking-wider" />
      <Text text="12" class="text-3xl font-bold text-zinc-100 tabular-nums" />
    </Column>
  </Container>
  <Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700">
    <Column gap="1">
      <Text text="Connections" class="text-xs text-zinc-400 uppercase tracking-wider" />
      <Text text="847" class="text-3xl font-bold text-zinc-100 tabular-nums" />
    </Column>
  </Container>
  <Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700">
    <Column gap="1">
      <Text text="Alerts" class="text-xs text-zinc-400 uppercase tracking-wider" />
      <Text text="3" class="text-3xl font-bold text-amber-400 tabular-nums" />
    </Column>
  </Container>
</Grid>

<!-- 2-column form layout -->
<Grid columns="2" gap="4">
  <Column gap="1">
    <Text text="First name" class="text-sm text-zinc-400" />
    <Input name="first_name" class="w-full p-2.5 rounded-lg bg-zinc-800 border border-zinc-700 text-zinc-200 text-sm" />
  </Column>
  <Column gap="1">
    <Text text="Last name" class="text-sm text-zinc-400" />
    <Input name="last_name" class="w-full p-2.5 rounded-lg bg-zinc-800 border border-zinc-700 text-zinc-200 text-sm" />
  </Column>
</Grid>
```

---

## Stack - overlay layers

Stack renders children as positioned layers. Use z-index classes to control order. Good for badges over images or overlay UI.

```ntml
<Stack class="w-24 h-24">
  <!-- Base layer: avatar placeholder -->
  <Container class="w-24 h-24 rounded-full bg-zinc-700 flex items-center justify-center">
    <Icon name="user" size="32" class="text-zinc-400" />
  </Container>
  <!-- Top layer: online indicator badge -->
  <Container class="absolute bottom-1 right-1 w-4 h-4 rounded-full bg-green-400 border-2 border-zinc-950" />
</Stack>
```
