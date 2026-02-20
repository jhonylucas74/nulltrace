# Components

Components are the building blocks of NTML interfaces. Each is an XML element - the component name is the tag, props are attributes, and children are nested elements.

---

## Layout

Layout components control how children are arranged in space. All support the class attribute for Tailwind styling.

- [Container](/components/container) - Generic wrapper
- [Row / Column](/components/row-column) - Flex shorthand
- [Flex](/components/flex) - Flexbox layout
- [Grid / Stack](/components/grid) - Grid and layers

```ntml
<Column gap="4" class="p-6 bg-zinc-900">
  <Row gap="3" align="center" justify="spaceBetween">
    <Text text="Title" class="font-semibold text-zinc-100" />
    <Badge text="Active" variant="success" />
  </Row>
  <Grid columns="3" gap="4">
    <Container class="p-4 bg-zinc-800 rounded-lg">
      <Text text="Card 1" />
    </Container>
    <Container class="p-4 bg-zinc-800 rounded-lg">
      <Text text="Card 2" />
    </Container>
    <Container class="p-4 bg-zinc-800 rounded-lg">
      <Text text="Card 3" />
    </Container>
  </Grid>
</Column>
```

---

## Content

Display text, images, and icons. All content components render inline within their parent layout.

- [Text / Heading](/components/text) - Typography
- [Image / Icon](/components/image) - Media components

```ntml
<Column gap="2">
  <Heading level="1" text="Dashboard" class="text-3xl font-bold text-zinc-100" />
  <Heading level="2" text="Statistics" class="text-xl font-semibold text-zinc-200" />
  <Row gap="2" align="center">
    <Icon name="activity" size="16" class="text-amber-400" />
    <Text text="All systems operational" class="text-sm text-zinc-400" />
  </Row>
</Column>
```

---

## Interactive

Form elements and controls. Use name for Lua ui.get_value() and id for ui.set_disabled/set_text.

- [Button](/components/button) - Actions, variants
- [Input · Checkbox · Radio · Select](/components/input) - Form controls
- [Link](/components/link) - Navigation

```ntml
<Column gap="3" class="max-w-sm">
  <Input name="username" placeholder="Username" class="..." />
  <Input name="password" type="password" placeholder="Password" class="..." />
  <Select name="role" value="viewer">
    <option label="Viewer" value="viewer" />
    <option label="Editor" value="editor" />
    <option label="Admin" value="admin" />
  </Select>
  <Row gap="2">
    <Checkbox name="remember" label="Remember me" />
  </Row>
  <Button action="doLogin" variant="primary">
    <Text text="Sign in" class="font-medium" />
  </Button>
</Column>
```

---

## Display

Status indicators, progress, dividers, and spacing utilities.

- [ProgressBar · Badge](/components/progress) - Status indicators
- [Divider · Spacer](/components/layout-utils) - Spacing utils

```ntml
<Column gap="3">
  <Row gap="2" align="center">
    <Text text="Health" class="text-sm text-zinc-400 w-16" />
    <ProgressBar value="72" max="100" variant="success" class="flex-1 h-2" />
    <Badge text="72%" variant="success" />
  </Row>
  <Divider />
  <Row gap="2" align="center">
    <Text text="Shield" class="text-sm text-zinc-400 w-16" />
    <ProgressBar value="30" max="100" variant="danger" class="flex-1 h-2" />
    <Badge text="30%" variant="danger" />
  </Row>
</Column>
```

---

## Document & code

Rich content rendering for documentation, code, lists, tables, and more.

- [Code · Markdown · List · Table · Blockquote · Pre · Details](/components/content) - Rich content and document components

```ntml
<Column gap="6">
  <Code language="lua" block="true"><![CDATA[
    local x = 42
    print(x)
  ]]></Code>
  <Details summary="Advanced options">
    <Column gap="2" class="mt-2">
      <Text text="These are hidden by default." class="text-sm text-zinc-400" />
    </Column>
  </Details>
  <List>
    <ListItem text="First item" />
    <ListItem text="Second item" />
  </List>
</Column>
```
