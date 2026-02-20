# Image / Icon

**Content**

Image renders a picture from a URL or relative path. Icon renders a Lucide icon by name with configurable size and color.

---

## Image props

```ntml
src    string   (required) - URL or relative path
alt    string              - accessible alt text
fit    cover|contain|fill|none|scaleDown  (default: cover)
class  string              - Tailwind utility classes
```

---

## Image examples

```ntml
<!-- Basic image -->
<Image src="/assets/banner.png" alt="Banner" class="w-full rounded-lg" />

<!-- Circular avatar with cover fit -->
<Image src="/assets/avatar.png" alt="Player avatar"
       fit="cover" class="w-16 h-16 rounded-full" />

<!-- Image in a card -->
<Container class="rounded-xl overflow-hidden border border-zinc-700">
  <Image src="/assets/screenshot.png" alt="Screenshot" fit="contain" class="w-full h-48" />
  <Column gap="2" class="p-4">
    <Text text="Caption text" class="text-sm text-zinc-400" />
  </Column>
</Container>
```

---

## Icon props

```ntml
name   string   (required) - Lucide icon name (kebab-case)
size   number              - width and height in pixels (default: 24)
class  string              - Tailwind utility classes (use text-* for color)
```

---

## Icon examples

```ntml
<!-- Color via text utility -->
<Icon name="terminal" size="20" class="text-amber-400" />
<Icon name="shield-check" size="20" class="text-green-400" />
<Icon name="alert-triangle" size="20" class="text-red-400" />
<Icon name="info" size="20" class="text-blue-400" />

<!-- Large decorative icon -->
<Icon name="cpu" size="64" class="text-zinc-700" />

<!-- Icon with label -->
<Row gap="2" align="center">
  <Icon name="wifi" size="16" class="text-zinc-400" />
  <Text text="Connected" class="text-sm text-zinc-400" />
</Row>

<!-- Common icons: terminal, server, network, folder, file,
     user, settings, activity, zap, shield, lock, key,
     search, send, download, upload, refresh-cw, x, check,
     chevron-right, chevron-down, arrow-up, arrow-down -->
```
