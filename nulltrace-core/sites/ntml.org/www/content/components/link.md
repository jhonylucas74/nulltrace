# Link

**Interactive**

A clickable anchor element for navigation. Can wrap any component tree - Text, Row, Container, etc. Supports internal (same-origin) and external (new tab) navigation.

---

## Props

```ntml
href     string   (required) - destination URL or path
target   same|new            - open in same tab (default) or new tab
class    string              - Tailwind utility classes
children any components      - clickable content
```

---

## Text link

```ntml
<!-- Basic navigation link -->
<Link href="/dashboard">
  <Text text="Dashboard" class="text-sm text-amber-400 hover:text-amber-300 transition-colors duration-200" />
</Link>

<!-- With arrow -->
<Link href="/examples">
  <Row gap="1" align="center">
    <Text text="View examples" class="text-sm font-medium text-amber-400 hover:text-amber-300 transition-colors duration-200" />
    <Icon name="arrow-right" size="16" class="text-amber-400" />
  </Row>
</Link>

<!-- External link -->
<Link href="http://docs.nulltrace.io" target="new">
  <Row gap="1" align="center"><Text text="External docs" class="text-sm text-zinc-400 hover:text-amber-400 transition-colors duration-200" /><Icon name="arrow-right" size="14" class="text-zinc-400" /></Row>
</Link>
```

---

## Card link

Wrap an entire card to make it clickable.

```ntml
<Link href="/components/button">
  <Container class="rounded-lg px-4 py-3 bg-zinc-800/60 border border-zinc-700/80
                    hover:border-amber-500/30 hover:bg-zinc-800 transition-colors duration-200">
    <Column gap="0">
      <Text text="Button" class="text-sm font-medium text-zinc-200" />
      <Text text="Variants, actions, disable" class="text-xs text-zinc-500 mt-1" />
    </Column>
  </Container>
</Link>
```

---

## Nav item with active state

```ntml
<!-- Active nav item (current page) -->
<Link href="/dashboard">
  <Row gap="2" align="center" class="px-3 py-2 rounded-lg bg-amber-500/10 border border-amber-500/20">
    <Icon name="layout-dashboard" size="16" class="text-amber-400" />
    <Text text="Dashboard" class="text-sm font-medium text-amber-400" />
  </Row>
</Link>

<!-- Inactive nav item -->
<Link href="/settings">
  <Row gap="2" align="center" class="px-3 py-2 rounded-lg hover:bg-zinc-800 transition-colors duration-200">
    <Icon name="settings" size="16" class="text-zinc-400" />
    <Text text="Settings" class="text-sm text-zinc-400 hover:text-zinc-200" />
  </Row>
</Link>
```
