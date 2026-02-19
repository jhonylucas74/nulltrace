# NullTrace Tailwind CSS — Plano de Implementação

Engine CSS utilitária inspirada no Tailwind CSS, implementada em Rust, gerando CSS injetado no componente `Browser` do jogo NullTrace.

## Arquitetura

```
nulltrace-ntml/src/
  tailwind/
    mod.rs          ← entry point: generate_css(html) / generate_css_for_classes(classes)
    parser.rs       ← extrai classes do HTML/NTML (sem regex, scan manual)
    registry.rs     ← resolve_class(class) → Option<CssRule>
    colors.rs       ← paleta completa Tailwind v3 (22 famílias × 11 shades + hex_to_rgb)
    spacing.rs      ← escala de espaçamento (0, px, 0.5, 1 … 96)
    [variants.rs]   ← (Fase 7) hover:, focus:, dark:, responsive sm:/md:/lg:/xl:
```

### API pública

```rust
// Gera CSS a partir de HTML (scan automático de class="...")
pub fn generate_css(html: &str) -> String

// Gera CSS a partir de lista explícita de classes
pub fn generate_css_for_classes(classes: &[&str]) -> String
```

### Princípio de geração

1. Fazer scan das classes usadas no documento NTML/HTML
2. Para cada classe reconhecida, emitir a regra CSS correspondente
3. Injetar o CSS resultante em `<style>` no `<head>` antes de renderizar no Browser

---

## Convenções de escala

### Escala de espaçamento (`--spacing`)
Base: `1 unit = 0.25rem = 4px`

| Token | rem    | px  |
|-------|--------|-----|
| 0     | 0      | 0   |
| px    | —      | 1px |
| 0.5   | 0.125  | 2   |
| 1     | 0.25   | 4   |
| 1.5   | 0.375  | 6   |
| 2     | 0.5    | 8   |
| 2.5   | 0.625  | 10  |
| 3     | 0.75   | 12  |
| 3.5   | 0.875  | 14  |
| 4     | 1      | 16  |
| 5     | 1.25   | 20  |
| 6     | 1.5    | 24  |
| 7     | 1.75   | 28  |
| 8     | 2      | 32  |
| 9     | 2.25   | 36  |
| 10    | 2.5    | 40  |
| 11    | 2.75   | 44  |
| 12    | 3      | 48  |
| 14    | 3.5    | 56  |
| 16    | 4      | 64  |
| 20    | 5      | 80  |
| 24    | 6      | 96  |
| 28    | 7      | 112 |
| 32    | 8      | 128 |
| 36    | 9      | 144 |
| 40    | 10     | 160 |
| 44    | 11     | 176 |
| 48    | 12     | 192 |
| 52    | 13     | 208 |
| 56    | 14     | 224 |
| 60    | 15     | 240 |
| 64    | 16     | 256 |
| 72    | 18     | 288 |
| 80    | 20     | 320 |
| 96    | 24     | 384 |

### Breakpoints responsivos
| Prefixo | min-width |
|---------|-----------|
| `sm:`   | 640px     |
| `md:`   | 768px     |
| `lg:`   | 1024px    |
| `xl:`   | 1280px    |
| `2xl:`  | 1536px    |

### Variantes de estado
`hover:`, `focus:`, `focus-within:`, `focus-visible:`, `active:`, `visited:`, `disabled:`, `checked:`, `group-hover:`, `peer-hover:`, `dark:`, `first:`, `last:`, `odd:`, `even:`

---

## Paleta de Cores

### Cores neutras
`slate`, `gray`, `zinc`, `neutral`, `stone` — escalas 50/100/200/300/400/500/600/700/800/900/950

### Cores cromáticas
`red`, `orange`, `amber`, `yellow`, `lime`, `green`, `emerald`, `teal`, `cyan`, `sky`, `blue`, `indigo`, `violet`, `purple`, `fuchsia`, `pink`, `rose` — escalas 50/100/200/300/400/500/600/700/800/900/950

### Especiais
`black`, `white`, `transparent`, `current`, `inherit`

---

## Checklist de Implementação

### Legenda
- `[ ]` Pendente
- `[x]` Implementado
- `[~]` Parcial

---

## 1. Layout

### 1.1 Display
- [x] `block` → `display: block`
- [x] `inline-block` → `display: inline-block`
- [x] `inline` → `display: inline`
- [x] `flex` → `display: flex`
- [x] `inline-flex` → `display: inline-flex`
- [x] `grid` → `display: grid`
- [x] `inline-grid` → `display: inline-grid`
- [x] `contents` → `display: contents`
- [x] `flow-root` → `display: flow-root`
- [x] `table` → `display: table`
- [x] `inline-table` → `display: inline-table`
- [x] `table-caption` → `display: table-caption`
- [x] `table-cell` → `display: table-cell`
- [x] `table-column` → `display: table-column`
- [x] `table-column-group` → `display: table-column-group`
- [x] `table-footer-group` → `display: table-footer-group`
- [x] `table-header-group` → `display: table-header-group`
- [x] `table-row-group` → `display: table-row-group`
- [x] `table-row` → `display: table-row`
- [x] `list-item` → `display: list-item`
- [x] `hidden` → `display: none`

### 1.2 Position
- [x] `static` → `position: static`
- [x] `fixed` → `position: fixed`
- [x] `absolute` → `position: absolute`
- [x] `relative` → `position: relative`
- [x] `sticky` → `position: sticky`

### 1.3 Top / Right / Bottom / Left
Valores da escala de espaçamento + frações + `auto` + `full` + `px` + arbitrários `[value]`
- [x] `top-{n}` → `top: calc(0.25rem * n)`
- [x] `right-{n}` → `right: calc(0.25rem * n)`
- [x] `bottom-{n}` → `bottom: calc(0.25rem * n)`
- [x] `left-{n}` → `left: calc(0.25rem * n)`
- [x] `inset-{n}` → `top/right/bottom/left`
- [x] `inset-x-{n}` → `left/right`
- [x] `inset-y-{n}` → `top/bottom`
- [x] `top-auto`, `right-auto`, `bottom-auto`, `left-auto`
- [x] `top-full`, `right-full`, `bottom-full`, `left-full` → 100%
- [x] `top-1/2`, `right-1/2`, etc. → 50%
- [x] `top-1/3`, `top-2/3`, `top-1/4`, `top-3/4`

### 1.4 Z-Index
- [x] `z-0` → `z-index: 0`
- [x] `z-10` → `z-index: 10`
- [x] `z-20` → `z-index: 20`
- [x] `z-30` → `z-index: 30`
- [x] `z-40` → `z-index: 40`
- [x] `z-50` → `z-index: 50`
- [x] `z-auto` → `z-index: auto`
- [x] `z-[{n}]` → arbitrário

### 1.5 Float & Clear
- [x] `float-start` → `float: inline-start`
- [x] `float-end` → `float: inline-end`
- [x] `float-right` → `float: right`
- [x] `float-left` → `float: left`
- [x] `float-none` → `float: none`
- [x] `clear-start`, `clear-end`, `clear-left`, `clear-right`, `clear-both`, `clear-none`

### 1.6 Overflow
- [x] `overflow-auto` → `overflow: auto`
- [x] `overflow-hidden` → `overflow: hidden`
- [x] `overflow-clip` → `overflow: clip`
- [x] `overflow-visible` → `overflow: visible`
- [x] `overflow-scroll` → `overflow: scroll`
- [x] `overflow-x-auto`, `overflow-x-hidden`, `overflow-x-clip`, `overflow-x-visible`, `overflow-x-scroll`
- [x] `overflow-y-auto`, `overflow-y-hidden`, `overflow-y-clip`, `overflow-y-visible`, `overflow-y-scroll`

### 1.7 Visibility
- [x] `visible` → `visibility: visible`
- [x] `invisible` → `visibility: hidden`
- [x] `visibility: collapse`

### 1.8 Object Fit & Position
- [x] `object-contain` → `object-fit: contain`
- [x] `object-cover` → `object-fit: cover`
- [x] `object-fill` → `object-fit: fill`
- [x] `object-none` → `object-fit: none`
- [x] `object-scale-down` → `object-fit: scale-down`
- [x] `object-center`, `object-top`, `object-bottom`, `object-left`, `object-right`, `object-left-top`, etc.

### 1.9 Aspect Ratio
- [x] `aspect-auto` → `aspect-ratio: auto`
- [x] `aspect-square` → `aspect-ratio: 1 / 1`
- [x] `aspect-video` → `aspect-ratio: 16 / 9`
- [x] `aspect-[{value}]` → arbitrário

### 1.10 Columns
- [x] `columns-1` até `columns-12`
- [x] `columns-auto`
- [x] `columns-3xs` até `columns-7xl`

### 1.11 Break
- [x] `break-before-auto`, `break-before-avoid`, `break-before-all`, `break-before-page`, `break-before-column`
- [x] `break-inside-auto`, `break-inside-avoid`, `break-inside-avoid-page`, `break-inside-avoid-column`
- [x] `break-after-auto`, `break-after-avoid`, `break-after-all`, `break-after-page`, `break-after-column`

### 1.12 Container
- [~] `container` → `width: 100%; margin: auto` (base implementado; responsive max-width por breakpoint na Fase 7)

### 1.13 Box Decoration Break
- [x] `box-decoration-clone`, `box-decoration-slice`

### 1.14 Box Sizing
- [x] `box-border` → `box-sizing: border-box`
- [x] `box-content` → `box-sizing: content-box`

### 1.15 Isolation
- [x] `isolate` → `isolation: isolate`
- [x] `isolation-auto` → `isolation: auto`

---

## 2. Flexbox

### 2.1 Flex Direction
- [x] `flex-row` → `flex-direction: row`
- [x] `flex-row-reverse` → `flex-direction: row-reverse`
- [x] `flex-col` → `flex-direction: column`
- [x] `flex-col-reverse` → `flex-direction: column-reverse`

### 2.2 Flex Wrap
- [x] `flex-wrap` → `flex-wrap: wrap`
- [x] `flex-wrap-reverse` → `flex-wrap: wrap-reverse`
- [x] `flex-nowrap` → `flex-wrap: nowrap`

### 2.3 Flex
- [x] `flex-1` → `flex: 1 1 0%`
- [x] `flex-auto` → `flex: 1 1 auto`
- [x] `flex-initial` → `flex: 0 1 auto`
- [x] `flex-none` → `flex: none`
- [x] `flex-[{value}]` → arbitrário

### 2.4 Flex Grow
- [x] `grow` → `flex-grow: 1`
- [x] `grow-0` → `flex-grow: 0`
- [x] `grow-[{value}]` → arbitrário

### 2.5 Flex Shrink
- [x] `shrink` → `flex-shrink: 1`
- [x] `shrink-0` → `flex-shrink: 0`
- [x] `shrink-[{value}]` → arbitrário

### 2.6 Flex Basis
- [x] `basis-{n}` → `flex-basis: calc(0.25rem * n)` (escala spacing)
- [x] `basis-auto`, `basis-full`, `basis-1/2`, `basis-1/3`, `basis-2/3`, etc.

### 2.7 Order
- [x] `order-first` → `order: -9999`
- [x] `order-last` → `order: 9999`
- [x] `order-none` → `order: 0`
- [x] `order-1` até `order-12`
- [x] `order-[{n}]` → arbitrário

---

## 3. Grid

### 3.1 Grid Template Columns
- [x] `grid-cols-1` até `grid-cols-12` → `grid-template-columns: repeat(n, minmax(0, 1fr))`
- [x] `grid-cols-none` → `grid-template-columns: none`
- [x] `grid-cols-subgrid`
- [x] `grid-cols-[{value}]` → arbitrário

### 3.2 Grid Template Rows
- [x] `grid-rows-1` até `grid-rows-12`
- [x] `grid-rows-none`, `grid-rows-subgrid`

### 3.3 Column Span
- [x] `col-auto` → `grid-column: auto`
- [x] `col-span-1` até `col-span-12` → `grid-column: span n / span n`
- [x] `col-span-full` → `grid-column: 1 / -1`
- [x] `col-start-1` até `col-start-13`, `col-start-auto`
- [x] `col-end-1` até `col-end-13`, `col-end-auto`

### 3.4 Row Span
- [x] `row-auto` → `grid-row: auto`
- [x] `row-span-1` até `row-span-12`
- [x] `row-span-full`
- [x] `row-start-1` até `row-start-13`, `row-start-auto`
- [x] `row-end-1` até `row-end-13`, `row-end-auto`

### 3.5 Grid Auto Flow
- [x] `grid-flow-row` → `grid-auto-flow: row`
- [x] `grid-flow-col` → `grid-auto-flow: column`
- [x] `grid-flow-dense` → `grid-auto-flow: dense`
- [x] `grid-flow-row-dense`, `grid-flow-col-dense`

### 3.6 Grid Auto Columns / Rows
- [x] `auto-cols-auto`, `auto-cols-min`, `auto-cols-max`, `auto-cols-fr`
- [x] `auto-rows-auto`, `auto-rows-min`, `auto-rows-max`, `auto-rows-fr`

---

## 4. Alinhamento (Flex + Grid)

### 4.1 Justify Content
- [x] `justify-normal`
- [x] `justify-start` → `justify-content: flex-start`
- [x] `justify-end` → `justify-content: flex-end`
- [x] `justify-center` → `justify-content: center`
- [x] `justify-between` → `justify-content: space-between`
- [x] `justify-around` → `justify-content: space-around`
- [x] `justify-evenly` → `justify-content: space-evenly`
- [x] `justify-stretch`

### 4.2 Justify Items
- [x] `justify-items-start`, `justify-items-end`, `justify-items-center`, `justify-items-stretch`, `justify-items-normal`

### 4.3 Justify Self
- [x] `justify-self-auto`, `justify-self-start`, `justify-self-end`, `justify-self-center`, `justify-self-stretch`

### 4.4 Align Content
- [x] `content-normal`, `content-start`, `content-end`, `content-center`
- [x] `content-between`, `content-around`, `content-evenly`, `content-baseline`, `content-stretch`

### 4.5 Align Items
- [x] `items-start` → `align-items: flex-start`
- [x] `items-end` → `align-items: flex-end`
- [x] `items-center` → `align-items: center`
- [x] `items-baseline` → `align-items: baseline`
- [x] `items-stretch` → `align-items: stretch`

### 4.6 Align Self
- [x] `self-auto`, `self-start`, `self-end`, `self-center`, `self-stretch`, `self-baseline`

### 4.7 Place Content / Items / Self
- [x] `place-content-{value}` → shorthand align-content + justify-content
- [x] `place-items-{value}` → shorthand align-items + justify-items
- [x] `place-self-{value}` → shorthand align-self + justify-self

### 4.8 Gap
- [x] `gap-{n}` → `gap: calc(0.25rem * n)` (escala spacing)
- [x] `gap-x-{n}` → `column-gap`
- [x] `gap-y-{n}` → `row-gap`
- [x] `gap-px`

---

## 5. Spacing

### 5.1 Padding
- [x] `p-{n}` → `padding` (escala spacing)
- [x] `px-{n}` → `padding-inline` (left + right)
- [x] `py-{n}` → `padding-block` (top + bottom)
- [x] `pt-{n}` → `padding-top`
- [x] `pr-{n}` → `padding-right`
- [x] `pb-{n}` → `padding-bottom`
- [x] `pl-{n}` → `padding-left`
- [x] `ps-{n}` → `padding-inline-start`
- [x] `pe-{n}` → `padding-inline-end`
- [x] `p-px`, `px-px`, `py-px`, etc.

### 5.2 Margin
- [x] `m-{n}` → `margin`
- [x] `mx-{n}` → `margin-inline`
- [x] `my-{n}` → `margin-block`
- [x] `mt-{n}` → `margin-top`
- [x] `mr-{n}` → `margin-right`
- [x] `mb-{n}` → `margin-bottom`
- [x] `ml-{n}` → `margin-left`
- [x] `ms-{n}` → `margin-inline-start`
- [x] `me-{n}` → `margin-inline-end`
- [x] `m-auto`, `mx-auto`, `my-auto`, etc.
- [x] Negativos: `-m-{n}`, `-mx-{n}`, etc.

### 5.3 Space Between (Divide spacing)
- [x] `space-x-{n}` → `margin-left` nos filhos (exceto primeiro)
- [x] `space-y-{n}` → `margin-top` nos filhos (exceto primeiro)
- [x] `space-x-reverse`, `space-y-reverse`

---

## 6. Sizing

### 6.1 Width
- [x] `w-{n}` → `width: calc(0.25rem * n)`
- [x] `w-auto` → `width: auto`
- [x] `w-px` → `width: 1px`
- [x] `w-full` → `width: 100%`
- [x] `w-screen` → `width: 100vw`
- [x] `w-min` → `width: min-content`
- [x] `w-max` → `width: max-content`
- [x] `w-fit` → `width: fit-content`
- [x] `w-1/2`, `w-1/3`, `w-2/3`, `w-1/4`, `w-3/4`, `w-1/5`, `w-2/5`, `w-3/5`, `w-4/5`, `w-1/6`, `w-5/6`
- [x] `w-3xs` até `w-7xl` (escala container)
- [x] `w-dvw`, `w-svw`, `w-lvw`, `w-dvh`, `w-svh`, `w-lvh`

### 6.2 Min-Width
- [x] `min-w-{n}`, `min-w-px`, `min-w-full`, `min-w-min`, `min-w-max`, `min-w-fit`
- [x] `min-w-0`

### 6.3 Max-Width
- [x] `max-w-{n}`, `max-w-none`, `max-w-full`, `max-w-min`, `max-w-max`, `max-w-fit`
- [x] `max-w-xs` (20rem), `max-w-sm` (24rem), `max-w-md` (28rem), `max-w-lg` (32rem)
- [x] `max-w-xl`, `max-w-2xl`, `max-w-3xl`, `max-w-4xl`, `max-w-5xl`, `max-w-6xl`, `max-w-7xl`
- [x] `max-w-screen-sm`, `max-w-screen-md`, `max-w-screen-lg`, `max-w-screen-xl`
- [x] `max-w-prose`

### 6.4 Height
- [x] `h-{n}` → `height: calc(0.25rem * n)`
- [x] `h-auto`, `h-px`, `h-full`, `h-screen`, `h-min`, `h-max`, `h-fit`
- [x] `h-dvh`, `h-svh`, `h-lvh`, `h-dvw`, `h-svw`, `h-lvw`

### 6.5 Min-Height
- [x] `min-h-{n}`, `min-h-0`, `min-h-px`, `min-h-full`, `min-h-screen`, `min-h-min`, `min-h-max`, `min-h-fit`
- [x] `min-h-dvh`, `min-h-svh`, `min-h-lvh`

### 6.6 Max-Height
- [x] `max-h-{n}`, `max-h-none`, `max-h-px`, `max-h-full`, `max-h-screen`, `max-h-min`, `max-h-max`, `max-h-fit`
- [x] `max-h-dvh`, `max-h-svh`, `max-h-lvh`

### 6.7 Size (width + height simultâneo)
- [x] `size-{n}` → `width` + `height`
- [x] `size-auto`, `size-px`, `size-full`, `size-min`, `size-max`, `size-fit`

---

## 7. Typography

### 7.1 Font Family
- [x] `font-sans` → `font-family: ui-sans-serif, system-ui, ...`
- [x] `font-serif` → `font-family: ui-serif, Georgia, ...`
- [x] `font-mono` → `font-family: ui-monospace, SFMono-Regular, ...`
- [x] `font-[{name}]` → arbitrary font (e.g. `font-[Inter]` for Google Fonts; use `_` for spaces in names: `font-[Open_Sans]`)

### 7.2 Font Size
- [x] `text-xs` → `font-size: 0.75rem; line-height: 1rem`
- [x] `text-sm` → `font-size: 0.875rem; line-height: 1.25rem`
- [x] `text-base` → `font-size: 1rem; line-height: 1.5rem`
- [x] `text-lg` → `font-size: 1.125rem; line-height: 1.75rem`
- [x] `text-xl` → `font-size: 1.25rem; line-height: 1.75rem`
- [x] `text-2xl` → `font-size: 1.5rem; line-height: 2rem`
- [x] `text-3xl` → `font-size: 1.875rem; line-height: 2.25rem`
- [x] `text-4xl` → `font-size: 2.25rem; line-height: 2.5rem`
- [x] `text-5xl` → `font-size: 3rem; line-height: 1`
- [x] `text-6xl` → `font-size: 3.75rem; line-height: 1`
- [x] `text-7xl` → `font-size: 4.5rem; line-height: 1`
- [x] `text-8xl` → `font-size: 6rem; line-height: 1`
- [x] `text-9xl` → `font-size: 8rem; line-height: 1`
- [x] `text-[{value}]` → arbitrário

### 7.3 Font Weight
- [x] `font-thin` → `font-weight: 100`
- [x] `font-extralight` → `font-weight: 200`
- [x] `font-light` → `font-weight: 300`
- [x] `font-normal` → `font-weight: 400`
- [x] `font-medium` → `font-weight: 500`
- [x] `font-semibold` → `font-weight: 600`
- [x] `font-bold` → `font-weight: 700`
- [x] `font-extrabold` → `font-weight: 800`
- [x] `font-black` → `font-weight: 900`

### 7.4 Font Style
- [x] `italic` → `font-style: italic`
- [x] `not-italic` → `font-style: normal`

### 7.5 Font Smoothing
- [x] `antialiased` → `-webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale`
- [x] `subpixel-antialiased` → `-webkit-font-smoothing: auto; -moz-osx-font-smoothing: auto`

### 7.6 Letter Spacing
- [x] `tracking-tighter` → `letter-spacing: -0.05em`
- [x] `tracking-tight` → `letter-spacing: -0.025em`
- [x] `tracking-normal` → `letter-spacing: 0em`
- [x] `tracking-wide` → `letter-spacing: 0.025em`
- [x] `tracking-wider` → `letter-spacing: 0.05em`
- [x] `tracking-widest` → `letter-spacing: 0.1em`

### 7.7 Line Height
- [x] `leading-none` → `line-height: 1`
- [x] `leading-tight` → `line-height: 1.25`
- [x] `leading-snug` → `line-height: 1.375`
- [x] `leading-normal` → `line-height: 1.5`
- [x] `leading-relaxed` → `line-height: 1.625`
- [x] `leading-loose` → `line-height: 2`
- [x] `leading-{n}` → `line-height: calc(0.25rem * n)`

### 7.8 Text Align
- [x] `text-left` → `text-align: left`
- [x] `text-center` → `text-align: center`
- [x] `text-right` → `text-align: right`
- [x] `text-justify` → `text-align: justify`
- [x] `text-start`, `text-end`

### 7.9 Text Color
- [x] `text-{color}-{shade}` → `color: {value}` (paleta completa)
- [x] `text-white`, `text-black`, `text-transparent`, `text-current`, `text-inherit`
- [x] `text-{color}-{shade}/{opacity}` → com opacidade

### 7.10 Text Decoration
- [x] `underline` → `text-decoration-line: underline`
- [x] `overline` → `text-decoration-line: overline`
- [x] `line-through` → `text-decoration-line: line-through`
- [x] `no-underline` → `text-decoration-line: none`
- [x] `decoration-{color}` → `text-decoration-color`
- [x] `decoration-solid`, `decoration-double`, `decoration-dotted`, `decoration-dashed`, `decoration-wavy`
- [x] `decoration-auto`, `decoration-from-font`, `decoration-0`, `decoration-1`, `decoration-2`, `decoration-4`, `decoration-8`
- [x] `underline-offset-auto`, `underline-offset-0`, `underline-offset-1`, `underline-offset-2`, `underline-offset-4`, `underline-offset-8`

### 7.11 Text Transform
- [x] `uppercase` → `text-transform: uppercase`
- [x] `lowercase` → `text-transform: lowercase`
- [x] `capitalize` → `text-transform: capitalize`
- [x] `normal-case` → `text-transform: none`

### 7.12 Text Overflow
- [x] `truncate` → `overflow: hidden; text-overflow: ellipsis; white-space: nowrap`
- [x] `text-ellipsis` → `text-overflow: ellipsis`
- [x] `text-clip` → `text-overflow: clip`

### 7.13 Text Wrap
- [x] `text-wrap` → `text-wrap: wrap`
- [x] `text-nowrap` → `text-wrap: nowrap`
- [x] `text-balance` → `text-wrap: balance`
- [x] `text-pretty` → `text-wrap: pretty`

### 7.14 Text Indent
- [x] `indent-{n}` → `text-indent: calc(0.25rem * n)`
- [x] `indent-px`

### 7.15 Vertical Align
- [x] `align-baseline`, `align-top`, `align-middle`, `align-bottom`
- [x] `align-text-top`, `align-text-bottom`, `align-sub`, `align-super`

### 7.16 White Space
- [x] `whitespace-normal`, `whitespace-nowrap`, `whitespace-pre`
- [x] `whitespace-pre-line`, `whitespace-pre-wrap`, `whitespace-break-spaces`

### 7.17 Word Break
- [x] `break-normal` → `word-break: normal; overflow-wrap: normal`
- [x] `break-words` → `overflow-wrap: break-word`
- [x] `break-all` → `word-break: break-all`
- [x] `break-keep` → `word-break: keep-all`

### 7.18 Line Clamp
- [x] `line-clamp-1` até `line-clamp-6`
- [x] `line-clamp-none`

### 7.19 List Style
- [x] `list-none`, `list-disc`, `list-decimal`
- [x] `list-inside`, `list-outside`
- [x] `list-image-none`

### 7.20 Font Variant Numeric
- [x] `normal-nums`, `ordinal`, `slashed-zero`
- [x] `lining-nums`, `oldstyle-nums`
- [x] `proportional-nums`, `tabular-nums`
- [x] `diagonal-fractions`, `stacked-fractions`

**Stage 7 (Typography) — completo**

---

## 8. Backgrounds

### 8.1 Background Color
- [x] `bg-{color}-{shade}` → `background-color` (paleta completa)
- [x] `bg-white`, `bg-black`, `bg-transparent`, `bg-current`, `bg-inherit`
- [x] `bg-{color}-{shade}/{opacity}` → com opacidade

### 8.2 Background Image / Gradient
- [x] `bg-none` → `background-image: none`
- [x] `bg-linear-to-t`, `bg-linear-to-tr`, `bg-linear-to-r`, `bg-linear-to-br`
- [x] `bg-linear-to-b`, `bg-linear-to-bl`, `bg-linear-to-l`, `bg-linear-to-tl`
- [x] `bg-radial`
- [x] `bg-conic`
- [x] `from-{color}` → `--tw-gradient-from`
- [x] `via-{color}` → `--tw-gradient-via`
- [x] `to-{color}` → `--tw-gradient-to`
- [x] `from-{n}%`, `via-{n}%`, `to-{n}%` → posição do stop

### 8.3 Background Size
- [x] `bg-auto` → `background-size: auto`
- [x] `bg-cover` → `background-size: cover`
- [x] `bg-contain` → `background-size: contain`

### 8.4 Background Position
- [x] `bg-center`, `bg-top`, `bg-bottom`, `bg-left`, `bg-right`
- [x] `bg-left-top`, `bg-left-bottom`, `bg-right-top`, `bg-right-bottom`

### 8.5 Background Repeat
- [x] `bg-repeat`, `bg-no-repeat`
- [x] `bg-repeat-x`, `bg-repeat-y`, `bg-repeat-round`, `bg-repeat-space`

### 8.6 Background Attachment
- [x] `bg-fixed`, `bg-local`, `bg-scroll`

### 8.7 Background Clip
- [x] `bg-clip-border`, `bg-clip-padding`, `bg-clip-content`, `bg-clip-text`

### 8.8 Background Origin
- [x] `bg-origin-border`, `bg-origin-padding`, `bg-origin-content`

---

## 9. Borders

### 9.1 Border Width
- [x] `border` → `border-width: 1px`
- [x] `border-0`, `border-2`, `border-4`, `border-8`
- [x] `border-t`, `border-r`, `border-b`, `border-l`
- [x] `border-t-0`, `border-t-2`, `border-t-4`, `border-t-8`
- [x] `border-x`, `border-y`
- [x] `border-x-0`, `border-x-2`, etc.
- [x] `border-s`, `border-e`, `border-bs`, `border-be`

### 9.2 Border Color
- [x] `border-{color}-{shade}` → `border-color` (paleta completa)
- [x] `border-white`, `border-black`, `border-transparent`
- [x] `border-t-{color}`, `border-r-{color}`, `border-b-{color}`, `border-l-{color}`

### 9.3 Border Style
- [x] `border-solid` → `border-style: solid`
- [x] `border-dashed` → `border-style: dashed`
- [x] `border-dotted` → `border-style: dotted`
- [x] `border-double` → `border-style: double`
- [x] `border-hidden` → `border-style: hidden`
- [x] `border-none` → `border-style: none`

### 9.4 Border Radius
- [x] `rounded-none` → `border-radius: 0`
- [x] `rounded-sm` → `border-radius: 0.125rem`
- [x] `rounded` → `border-radius: 0.25rem`
- [x] `rounded-md` → `border-radius: 0.375rem`
- [x] `rounded-lg` → `border-radius: 0.5rem`
- [x] `rounded-xl` → `border-radius: 0.75rem`
- [x] `rounded-2xl` → `border-radius: 1rem`
- [x] `rounded-3xl` → `border-radius: 1.5rem`
- [x] `rounded-full` → `border-radius: 9999px`
- [x] `rounded-t-{size}`, `rounded-r-{size}`, `rounded-b-{size}`, `rounded-l-{size}`
- [x] `rounded-tl-{size}`, `rounded-tr-{size}`, `rounded-bl-{size}`, `rounded-br-{size}`
- [x] `rounded-ss-{size}`, `rounded-se-{size}`, `rounded-es-{size}`, `rounded-ee-{size}`

### 9.5 Outline
- [x] `outline-none` → `outline: none`
- [x] `outline`, `outline-dashed`, `outline-dotted`, `outline-double`
- [x] `outline-0`, `outline-1`, `outline-2`, `outline-4`, `outline-8`
- [x] `outline-{color}-{shade}`
- [x] `outline-offset-0`, `outline-offset-1`, `outline-offset-2`, `outline-offset-4`, `outline-offset-8`

### 9.6 Ring
- [x] `ring` → `box-shadow: 0 0 0 3px var(--tw-ring-color)`
- [x] `ring-0`, `ring-1`, `ring-2`, `ring-4`, `ring-8`
- [x] `ring-inset`
- [x] `ring-{color}-{shade}`
- [x] `ring-offset-0`, `ring-offset-1`, `ring-offset-2`, `ring-offset-4`, `ring-offset-8`
- [x] `ring-offset-{color}-{shade}`

### 9.7 Divide
- [x] `divide-x`, `divide-x-0`, `divide-x-2`, `divide-x-4`, `divide-x-8`
- [x] `divide-y`, `divide-y-0`, `divide-y-2`, `divide-y-4`, `divide-y-8`
- [x] `divide-x-reverse`, `divide-y-reverse`
- [x] `divide-{color}-{shade}`
- [x] `divide-solid`, `divide-dashed`, `divide-dotted`, `divide-double`, `divide-none`

---

## 10. Effects

### 10.1 Box Shadow
- [x] `shadow-2xs`
- [x] `shadow-xs`
- [x] `shadow-sm`
- [x] `shadow` (default)
- [x] `shadow-md`
- [x] `shadow-lg`
- [x] `shadow-xl`
- [x] `shadow-2xl`
- [x] `shadow-inner`
- [x] `shadow-none`
- [x] `shadow-{color}-{shade}` → `--tw-shadow-color` (combine com shadow-sm/lg para cor)

### 10.2 Opacity
- [x] `opacity-0`, `opacity-5`, `opacity-10`, `opacity-15`, `opacity-20`, `opacity-25`
- [x] `opacity-30`, `opacity-35`, `opacity-40`, `opacity-45`, `opacity-50`
- [x] `opacity-55`, `opacity-60`, `opacity-65`, `opacity-70`, `opacity-75`
- [x] `opacity-80`, `opacity-85`, `opacity-90`, `opacity-95`, `opacity-100`

### 10.3 Mix Blend Mode
- [x] `mix-blend-normal`, `mix-blend-multiply`, `mix-blend-screen`, `mix-blend-overlay`
- [x] `mix-blend-darken`, `mix-blend-lighten`, `mix-blend-color-dodge`, `mix-blend-color-burn`
- [x] `mix-blend-hard-light`, `mix-blend-soft-light`, `mix-blend-difference`, `mix-blend-exclusion`
- [x] `mix-blend-hue`, `mix-blend-saturation`, `mix-blend-color`, `mix-blend-luminosity`
- [x] `mix-blend-plus-darker`, `mix-blend-plus-lighter`

### 10.4 Background Blend Mode
- [x] `bg-blend-{value}` (mesmos valores do mix-blend)

---

## 11. Filters

### 11.1 Filter
- [x] `blur-none`, `blur-sm`, `blur`, `blur-md`, `blur-lg`, `blur-xl`, `blur-2xl`, `blur-3xl`
- [x] `brightness-0`, `brightness-50`, `brightness-75`, `brightness-90`, `brightness-95`, `brightness-100`, `brightness-105`, `brightness-110`, `brightness-125`, `brightness-150`, `brightness-200`
- [x] `contrast-0`, `contrast-50`, `contrast-75`, `contrast-100`, `contrast-125`, `contrast-150`, `contrast-200`
- [x] `drop-shadow-none`, `drop-shadow-sm`, `drop-shadow`, `drop-shadow-md`, `drop-shadow-lg`, `drop-shadow-xl`, `drop-shadow-2xl`
- [x] `grayscale-0`, `grayscale`
- [x] `hue-rotate-0`, `hue-rotate-15`, `hue-rotate-30`, `hue-rotate-60`, `hue-rotate-90`, `hue-rotate-180`
- [x] `invert-0`, `invert`
- [x] `saturate-0`, `saturate-50`, `saturate-100`, `saturate-150`, `saturate-200`
- [x] `sepia-0`, `sepia`

### 11.2 Backdrop Filter
- [x] `backdrop-blur-{size}` (same sizes as blur)
- [x] `backdrop-brightness-{value}`
- [x] `backdrop-contrast-{value}`
- [x] `backdrop-grayscale-{value}`
- [x] `backdrop-hue-rotate-{value}`
- [x] `backdrop-invert-{value}`
- [x] `backdrop-opacity-{value}`
- [x] `backdrop-saturate-{value}`
- [x] `backdrop-sepia-{value}`

---

## 12. Transitions & Animation

### 12.1 Transition Property
- [x] `transition-none` → `transition-property: none`
- [x] `transition-all` → `transition-property: all`
- [x] `transition` → default (color, bg, border, outline, text-decoration, fill, stroke, opacity, shadow, transform)
- [x] `transition-colors` → color-related
- [x] `transition-opacity` → `transition-property: opacity`
- [x] `transition-shadow` → `transition-property: box-shadow`
- [x] `transition-transform` → transform-related

### 12.2 Transition Duration
- [x] `duration-0`, `duration-75`, `duration-100`, `duration-150`, `duration-200`
- [x] `duration-300`, `duration-500`, `duration-700`, `duration-1000`

### 12.3 Transition Timing Function
- [x] `ease-linear` → `transition-timing-function: linear`
- [x] `ease-in` → `cubic-bezier(0.4, 0, 1, 1)`
- [x] `ease-out` → `cubic-bezier(0, 0, 0.2, 1)`
- [x] `ease-in-out` → `cubic-bezier(0.4, 0, 0.2, 1)`

### 12.4 Transition Delay
- [x] `delay-0`, `delay-75`, `delay-100`, `delay-150`, `delay-200`, `delay-300`, `delay-500`, `delay-700`, `delay-1000`

### 12.5 Animation
- [x] `animate-none` → `animation: none`
- [x] `animate-spin` → `animation: spin 1s linear infinite` + @keyframes spin
- [x] `animate-ping` → `animation: ping 1s cubic-bezier(0,0,0.2,1) infinite` + @keyframes ping
- [x] `animate-pulse` → `animation: pulse 2s cubic-bezier(0.4,0,0.6,1) infinite` + @keyframes pulse
- [x] `animate-bounce` → `animation: bounce 1s infinite` + @keyframes bounce

---

## 13. Transforms

### 13.1 Scale
- [x] `scale-0`, `scale-50`, `scale-75`, `scale-90`, `scale-95`, `scale-100`
- [x] `scale-105`, `scale-110`, `scale-125`, `scale-150`
- [x] `scale-x-{value}`, `scale-y-{value}`
- [x] `-scale-{value}` (negative = mirror)

### 13.2 Rotate
- [x] `rotate-0`, `rotate-1`, `rotate-2`, `rotate-3`, `rotate-6`, `rotate-12`
- [x] `rotate-45`, `rotate-90`, `rotate-180`
- [x] `-rotate-{value}` (negative)
- [x] `rotate-x-{value}`, `rotate-y-{value}` (3D)

### 13.3 Translate
- [x] `translate-x-{n}` → spacing scale (0.25rem × n)
- [x] `translate-y-{n}`
- [x] `translate-x-1/2`, `translate-x-full`, `translate-x-px` (and 1/3, 2/3, 1/4, 3/4, 1/6, 5/6)
- [x] `-translate-x-{n}`, `-translate-y-{n}` (negative)
- [x] `translate-z-{n}` (3D)

### 13.4 Skew
- [x] `skew-x-0`, `skew-x-1`, `skew-x-2`, `skew-x-3`, `skew-x-6`, `skew-x-12`
- [x] `skew-y-0`, `skew-y-1`, etc.
- [x] `-skew-x-{value}`, `-skew-y-{value}`

### 13.5 Transform Origin
- [x] `origin-center`, `origin-top`, `origin-top-right`, `origin-right`
- [x] `origin-bottom-right`, `origin-bottom`, `origin-bottom-left`
- [x] `origin-left`, `origin-top-left`

### 13.6 Perspective
- [x] `perspective-none`, `perspective-dramatic`, `perspective-near`, `perspective-normal`, `perspective-midrange`, `perspective-distant`

---

## 14. Interactivity

### 14.1 Cursor
- [x] `cursor-auto`, `cursor-default`, `cursor-pointer`, `cursor-wait`
- [x] `cursor-text`, `cursor-move`, `cursor-help`, `cursor-not-allowed`
- [x] `cursor-none`, `cursor-context-menu`, `cursor-progress`
- [x] `cursor-cell`, `cursor-crosshair`, `cursor-vertical-text`, `cursor-alias`
- [x] `cursor-copy`, `cursor-no-drop`, `cursor-grab`, `cursor-grabbing`
- [x] `cursor-all-scroll`, `cursor-col-resize`, `cursor-row-resize`
- [x] `cursor-n-resize`, `cursor-e-resize`, `cursor-s-resize`, `cursor-w-resize`
- [x] `cursor-ne-resize`, `cursor-nw-resize`, `cursor-se-resize`, `cursor-sw-resize`
- [x] `cursor-ew-resize`, `cursor-ns-resize`, `cursor-nesw-resize`, `cursor-nwse-resize`
- [x] `cursor-zoom-in`, `cursor-zoom-out`

### 14.2 Pointer Events
- [x] `pointer-events-none` → `pointer-events: none`
- [x] `pointer-events-auto` → `pointer-events: auto`

### 14.3 Resize
- [x] `resize-none`, `resize`, `resize-y`, `resize-x`

### 14.4 User Select
- [x] `select-none`, `select-text`, `select-all`, `select-auto`

### 14.5 Scroll
- [x] `scroll-auto` → `scroll-behavior: auto`
- [x] `scroll-smooth` → `scroll-behavior: smooth`
- [x] `scroll-m-{n}`, `scroll-p-{n}` (and directional variants: mt, mr, mb, ml, mx, my, ms, me, pt, pr, pb, pl, px, py, ps, pe)
- [x] `snap-none`, `snap-x`, `snap-y`, `snap-both` → `scroll-snap-type`
- [x] `snap-mandatory`, `snap-proximity`
- [x] `snap-start`, `snap-end`, `snap-center`, `snap-align-none`
- [x] `snap-normal`, `snap-always`

### 14.6 Touch Action
- [x] `touch-auto`, `touch-none`, `touch-pan-x`, `touch-pan-left`, `touch-pan-right`
- [x] `touch-pan-y`, `touch-pan-up`, `touch-pan-down`, `touch-pinch-zoom`, `touch-manipulation`

### 14.7 Will Change
- [x] `will-change-auto`, `will-change-scroll`, `will-change-contents`, `will-change-transform`

### 14.8 Appearance
- [x] `appearance-none` → `appearance: none`
- [x] `appearance-auto` → `appearance: auto`

### 14.9 Caret Color
- [x] `caret-{color}-{shade}` → `caret-color`
- [x] `caret-transparent`, `caret-current`

### 14.10 Accent Color
- [x] `accent-{color}-{shade}` → `accent-color`
- [x] `accent-auto`


---

## 16. Tables

- [x] `border-collapse` → `border-collapse: collapse`
- [x] `border-separate` → `border-collapse: separate`
- [x] `border-spacing-{n}` → `border-spacing`
- [x] `border-spacing-x-{n}`, `border-spacing-y-{n}`
- [x] `table-auto` → `table-layout: auto`
- [x] `table-fixed` → `table-layout: fixed`
- [x] `caption-top` → `caption-side: top`
- [x] `caption-bottom` → `caption-side: bottom`


---

## 18. Misc / Utilitários Extras

### 18.1 Content
- [x] `content-none` → `content: none`
- [x] `content-['']`, `content-[{value}]` (arbitrary values, `_` → space)

### 18.2 Overscroll
- [x] `overscroll-auto`, `overscroll-contain`, `overscroll-none`
- [x] `overscroll-x-auto`, `overscroll-x-contain`, `overscroll-x-none`
- [x] `overscroll-y-auto`, `overscroll-y-contain`, `overscroll-y-none`


---

## 19. Variantes (Modificadores)

### 19.1 Responsivas
- [x] `sm:`, `md:`, `lg:`, `xl:`, `2xl:` → `@media (min-width: ...)`
- [x] `max-sm:`, `max-md:`, `max-lg:`, `max-xl:`, `max-2xl:` → `@media (max-width: ...)`

### 19.2 Estado de Elemento
- [x] `hover:` → `:hover`
- [x] `focus:` → `:focus`
- [x] `focus-within:` → `:focus-within`
- [x] `focus-visible:` → `:focus-visible`
- [x] `active:` → `:active`
- [x] `visited:` → `:visited`
- [x] `disabled:` → `:disabled`
- [x] `enabled:` → `:enabled`
- [x] `checked:` → `:checked`
- [x] `indeterminate:` → `:indeterminate`
- [x] `required:` → `:required`
- [x] `valid:` → `:valid`
- [x] `invalid:` → `:invalid`
- [x] `placeholder:` → `::placeholder`
- [x] `first:` → `:first-child`
- [x] `last:` → `:last-child`
- [x] `only:` → `:only-child`
- [x] `odd:` → `:nth-child(odd)`
- [x] `even:` → `:nth-child(even)`
- [x] `empty:` → `:empty`

### 19.3 Pseudo-elementos
- [x] `before:` → `::before`
- [x] `after:` → `::after`
- [x] `placeholder:` → `::placeholder`
- [x] `selection:` → `::selection`
- [x] `first-line:` → `::first-line`
- [x] `first-letter:` → `::first-letter`
- [x] `marker:` → `::marker`

### 19.4 Dark Mode
- [x] `dark:` → `@media (prefers-color-scheme: dark)`

### 19.5 Motion
- [x] `motion-safe:` → `@media (prefers-reduced-motion: no-preference)`
- [x] `motion-reduce:` → `@media (prefers-reduced-motion: reduce)`

### 19.6 Print
- [x] `print:` → `@media print`

### 19.7 Group / Peer
- [x] `group-hover:`, `group-focus:`, `group-active:`, `group-disabled:`, `group-focus-within:`
- [x] `peer-hover:`, `peer-focus:`, `peer-checked:`, `peer-disabled:`, `peer-focus-within:`

### 19.8 Container Queries
- [x] `@sm:`, `@md:`, `@lg:`, `@xl:`, `@2xl:`, `@3xl:`, `@4xl:`, `@5xl:`, `@6xl:`, `@7xl:`

### 19.9 Arbitrary Values
- [x] `{property}-[{value}]` → many utilities support `[]` arbitrary syntax
- [ ] `{property}-({variable})` → `var(--variable)` (partial support)

---

## 20. Ordem de Implementação Sugerida

1. **[Fase 1 — Core]** Spacing, Sizing, Display, Flexbox, Colors
2. **[Fase 2 — Typography]** Font size, weight, color, align, transform
3. **[Fase 3 — Backgrounds & Borders]** bg-color, border-*, rounded-*, ring
4. **[Fase 4 — Layout avançado]** Grid, Position, Z-index, Overflow
5. **[Fase 5 — Effects]** Shadow, Opacity, Filters, Backdrop
6. **[Fase 6 — Transforms & Transitions]** Scale, Rotate, Translate, animate
7. **[Fase 7 — Variantes]** hover, focus, dark, responsive breakpoints
8. **[Fase 8 — Interactivity & SVG]** Cursor, pointer-events, fill, stroke
9. **[Fase 9 — Completude]** Tables, Accessibility, Misc, Arbitrary values

---

## Notas de Implementação Rust

```rust
// Assinatura esperada do gerador
pub fn generate_css(html: &str) -> String

// Internamente:
// 1. Regex/scan para extrair class="..."
// 2. Para cada token de classe:
//    a. Parsear variante (hover:, md:, dark:)
//    b. Resolver a regra CSS base
//    c. Envolver em media query / seletor se necessário
// 3. Deduplicar regras
// 4. Serializar CSS final
```

### Estrutura de dados recomendada

```rust
struct CssRule {
    selector: String,   // e.g. ".flex", ".hover\\:bg-blue-500:hover"
    properties: Vec<(String, String)>,  // [("display", "flex")]
    media_query: Option<String>,  // e.g. "@media (min-width: 768px)"
}
```
