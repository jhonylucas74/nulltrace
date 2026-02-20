# Tailwind reference

Tailwind CSS v3 utility classes available on every NTML component via the class attribute. No imports needed - just use the class.

---

## Spacing

Scale: 1=4px  2=8px  3=12px  4=16px  5=20px  6=24px  8=32px  10=40px  12=48px  16=64px

**Padding**

```css
p-1   p-2   p-3   p-4   p-5   p-6   p-8   p-10   p-12
px-2  px-3  px-4  px-5  px-6  px-8     py-1  py-2  py-3  py-4
pt-2  pr-3  pb-4  pl-3
```

**Margin**

```css
m-2   m-4   m-auto   mx-auto   mx-2
mt-2  mt-4  mt-6  mt-8    mb-0  mb-2  mb-4
-ml-2   -mt-1
```

**Gap (flex/grid containers)**

```css
gap-1   gap-2   gap-3   gap-4   gap-5   gap-6   gap-8   gap-10   gap-12
gap-x-4   gap-y-2
```

---

## Typography

**Size**

```css
text-xs   text-sm   text-base   text-lg   text-xl
text-2xl  text-3xl  text-4xl    text-5xl  text-6xl
```

**Weight**

```css
font-thin   font-light   font-normal   font-medium   font-semibold   font-bold   font-extrabold
```

**Family / Tracking / Leading**

```css
font-sans   font-mono   font-serif   font-[JetBrains_Mono]

tracking-tighter  tracking-tight  tracking-normal  tracking-wide  tracking-wider  tracking-widest

leading-none  leading-tight  leading-snug  leading-normal  leading-relaxed  leading-loose
```

**Alignment / Transform / Decoration**

```css
text-left   text-center   text-right   text-justify

uppercase   lowercase   capitalize   normal-case   italic   antialiased

underline   line-through   no-underline

truncate   text-ellipsis   whitespace-nowrap   break-words   break-all
```

---

## Colors

Add /15, /30, /50, /80 for opacity variants - e.g. bg-zinc-800/50

**Text**

```css
text-zinc-50    text-zinc-100   text-zinc-200   text-zinc-300
text-zinc-400   text-zinc-500   text-zinc-600   text-zinc-700
text-amber-300  text-amber-400  text-amber-500
text-red-400    text-red-500    text-green-400   text-blue-400
text-yellow-400 text-purple-400 text-cyan-400
```

**Background**

```css
bg-zinc-950   bg-zinc-900   bg-zinc-800   bg-zinc-700
bg-zinc-800/50    bg-amber-500/15   bg-red-500/10
bg-green-500/15   bg-blue-500/15    bg-yellow-500/10
```

**Border**

```css
border-zinc-800   border-zinc-700   border-zinc-600
border-amber-500/30   border-red-500/40   border-green-500/30
```

---

## Borders & Radius

```css
border   border-2   border-4   border-none
border-t   border-b   border-l   border-r

rounded-sm   rounded   rounded-md   rounded-lg   rounded-xl   rounded-2xl   rounded-full
rounded-t-lg   rounded-b-lg   rounded-l-lg   rounded-r-lg
```

---

## Flexbox

Use on Container, or as a complement to Row / Column.

```css
flex-row   flex-col   flex-row-reverse   flex-col-reverse

flex-wrap   flex-nowrap   flex-wrap-reverse

items-start   items-center   items-end   items-stretch   items-baseline

justify-start   justify-center   justify-end   justify-between   justify-around   justify-evenly

flex-1   flex-auto   flex-none   flex-grow   flex-shrink-0

self-start   self-center   self-end   self-stretch
```

---

## Sizing

**Width**

```css
w-full   w-auto   w-screen   w-fit
w-1/2   w-1/3   w-2/3   w-1/4   w-3/4
w-8   w-12   w-16   w-32   w-48   w-52   w-64   w-96
```

**Height**

```css
h-full   h-screen   h-auto   h-fit
h-4   h-6   h-8   h-10   h-12   h-16   h-32   h-64
```

**Min / Max**

```css
min-w-0   min-h-screen
max-w-xs   max-w-sm   max-w-md   max-w-lg   max-w-xl   max-w-2xl   max-w-3xl   max-w-6xl   max-w-full
```

---

## Position & Z-index

```css
static   relative   absolute   fixed   sticky

top-0   top-4   top-24   right-0   bottom-0   left-0   inset-0

z-0   z-10   z-20   z-30   z-40   z-50
```

---

## Effects & Transitions

```css
shadow-sm   shadow   shadow-md   shadow-lg   shadow-xl

opacity-0   opacity-25   opacity-50   opacity-75   opacity-100

transition   transition-colors   transition-opacity   transition-all   transition-transform
duration-100   duration-200   duration-300   ease-in-out

backdrop-blur-sm   backdrop-blur-md   backdrop-blur-lg

hover:bg-zinc-800    hover:text-amber-400    hover:border-zinc-600
focus:outline-none   focus:ring-2   disabled:opacity-50
```

---

## Responsive prefixes

Breakpoints are mobile-first: sm ≥640px · md ≥768px · lg ≥1024px · xl ≥1280px

```ntml
<!-- Hidden on mobile, visible on md+ -->
<Container class="hidden md:block">...</Container>

<!-- Responsive padding -->
<Container class="px-4 md:px-8 lg:px-12">...</Container>

<!-- Responsive text size -->
<Heading text="Title" class="text-2xl md:text-4xl lg:text-5xl font-bold" />

<!-- Responsive columns -->
<Container class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
```

---

## Display & Overflow

```css
block   inline   inline-block   flex   inline-flex   grid   hidden   contents

overflow-hidden   overflow-auto   overflow-scroll   overflow-visible
overflow-x-auto   overflow-y-auto

cursor-pointer   cursor-default   cursor-not-allowed   cursor-wait   cursor-grab

visible   invisible   pointer-events-none   select-none
```
