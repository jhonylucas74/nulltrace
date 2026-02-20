# Divider / Spacer

**Display**

Divider renders a horizontal or vertical separator line. Spacer adds a fixed or flexible amount of empty space between siblings.

---

## Divider props

```ntml
orientation  horizontal|vertical  (default: horizontal)
class        string              - Tailwind utility classes
```

---

## Divider examples

```ntml
<!-- Default horizontal divider -->
<Column gap="4">
  <Text text="Section A" class="text-zinc-300" />
  <Divider />
  <Text text="Section B" class="text-zinc-300" />
</Column>

<!-- Styled divider -->
<Divider class="border-zinc-700 my-6" />

<!-- Vertical divider in a Row -->
<Row gap="4" align="stretch">
  <Column gap="2">
    <Text text="Left panel" class="text-zinc-400" />
  </Column>
  <Divider orientation="vertical" class="border-zinc-700" />
  <Column gap="2">
    <Text text="Right panel" class="text-zinc-400" />
  </Column>
</Row>

<!-- With label -->
<Row gap="3" align="center">
  <Divider class="flex-1" />
  <Text text="or" class="text-xs text-zinc-500" />
  <Divider class="flex-1" />
</Row>
```

---

## Spacer props

```ntml
size   number|"auto"   - fixed size in pixels, or "auto" to fill remaining space
class  string          - Tailwind utility classes
```

---

## Spacer examples

```ntml
<!-- Push content to bottom of a Column -->
<Column class="min-h-screen p-6">
  <Text text="Top content" class="text-zinc-300" />
  <Spacer size="auto" />
  <Text text="Pinned to bottom" class="text-sm text-zinc-500" />
</Column>

<!-- Push button to right in a Row -->
<Row>
  <Text text="Label" class="font-medium text-zinc-200" />
  <Spacer size="auto" />
  <Button variant="secondary"><Text text="Edit" /></Button>
</Row>

<!-- Fixed gap -->
<Column>
  <Text text="Section A" />
  <Spacer size="32" />
  <Text text="Section B (32px below)" />
</Column>
```
