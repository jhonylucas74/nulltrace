# Flex

**Layout**

A full-featured flexbox container. Row and Column are shorthand for Flex with a fixed direction. Use Flex directly when you need to switch direction dynamically or set all flex properties explicitly.

---

## Props

```ntml
direction  row|column|row-reverse|column-reverse   (default: row)
gap        number                 - space between children
align      start|center|end|stretch|baseline        - cross-axis
justify    start|center|end|spaceBetween|spaceAround - main-axis
wrap       true|false             - flex-wrap
class      string                 - Tailwind utility classes
id         string                 - Lua target id
```

---

## Row direction (default)

```ntml
<Flex direction="row" gap="4" align="center" justify="spaceBetween">
  <Text text="Left side" class="font-medium text-zinc-200" />
  <Row gap="2">
    <Button variant="ghost"><Text text="Cancel" /></Button>
    <Button variant="primary"><Text text="Save" /></Button>
  </Row>
</Flex>
```

---

## Column direction

```ntml
<Flex direction="column" gap="6" class="p-8 min-h-screen">
  <Text text="Top" class="text-zinc-400" />
  <Spacer size="auto" />
  <Text text="Bottom (pushed down by Spacer)" class="text-zinc-400" />
</Flex>
```

---

## Wrapping grid pattern

Use wrap=true with min-width on children to create a responsive wrapping layout.

```ntml
<Flex direction="row" gap="4" wrap="true">
  <Container class="p-4 bg-zinc-800 rounded-lg min-w-[200px] flex-1">
    <Text text="Card A" />
  </Container>
  <Container class="p-4 bg-zinc-800 rounded-lg min-w-[200px] flex-1">
    <Text text="Card B" />
  </Container>
  <Container class="p-4 bg-zinc-800 rounded-lg min-w-[200px] flex-1">
    <Text text="Card C" />
  </Container>
</Flex>
```
