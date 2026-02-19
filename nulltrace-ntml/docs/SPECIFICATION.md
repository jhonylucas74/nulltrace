# NTML Specification v0.2.0

## Introdução

**NullTrace Markup Language (NTML)** é uma linguagem declarativa baseada em YAML para criar interfaces de usuário no jogo NullTrace. Esta especificação define a sintaxe, semântica e regras de validação do NTML.

## Objetivos de Design

1. **Segurança**: Prevenir injeções e exploits através de validação rigorosa
2. **Simplicidade**: Sintaxe YAML legível e fácil de aprender
3. **Expressividade**: Rico conjunto de componentes e estilos
4. **Type-Safety**: Validação estrita de tipos e propriedades
5. **Consistência**: Comportamento previsível e determinístico

## Estrutura do Documento

### Formatos

NTML suporta dois formatos de documento:

#### Formato Clássico (v0.1.0 — ainda válido)

Um único componente raiz na raiz do documento. Sem `head` e sem `body`. Documentos v0.1.0 continuam funcionando sem alterações.

```yaml
ComponentType:
  property1: value1
  property2: value2
  children:
    - ChildComponent1:
        ...
    - ChildComponent2:
        ...
```

#### Formato Completo (v0.2.0)

Quando o documento inclui uma seção `head`, a estrutura muda para `head` + `body`:

```yaml
head:
  title: "Minha Página"
  description: "Painel de status do sistema"
  tags: [hud, sistema, monitoramento]

  fonts:
    - family: "Roboto Mono"
      weights: [400, 700]

  scripts:
    - src: "scripts/main.lua"

  imports:
    - src: "components/nav-bar.ntml"
      as: "NavBar"

body:
  Container:
    children:
      - NavBar:
          title: "Meu App"
      - Text:
          text: "Hello"
```

**Regra de compatibilidade:** Se a chave `head` estiver presente, a chave `body` é obrigatória. Se nenhuma das duas estiver presente, o documento é tratado como formato clássico.

### Regras Globais

1. **Um único componente raiz**: Documentos com zero ou múltiplos componentes raiz são inválidos
2. **Profundidade máxima**: Máximo de 20 níveis de aninhamento
3. **Case sensitivity**: Nomes de componentes e propriedades são case-sensitive
4. **Naming convention**:
   - Componentes: PascalCase (e.g., `Container`, `ProgressBar`)
   - Propriedades: camelCase (e.g., `backgroundColor`, `fontSize`)

## Seção Head

A seção `head` é opcional e permite declarar metadados da página, importar fonts, scripts Lua e componentes externos.

### Metadados

```yaml
head:
  title: "Nome da Página"         # obrigatório quando head presente
  description: "Descrição breve"  # opcional
  author: "NomeDoJogador"         # opcional
  tags: [hud, status, monitoramento]  # opcional — usado para indexação
```

**Validação:**
- `title`: string não-vazia; obrigatório se `head` estiver presente
- `description`: string; opcional
- `author`: string; opcional
- `tags`: array de strings; cada tag deve ser não-vazia, sem espaços, lowercase; máximo de **10 tags**

### Importação de Fonts

Fontes são carregadas do **Google Fonts**. O usuário especifica apenas o nome da família e os pesos desejados — sem caminhos de arquivo.

```yaml
head:
  fonts:
    - family: "Roboto Mono"
      weights: [400, 700]

    - family: "Inter"
      weights: [300, 400, 600]
```

Após declarar uma font no `head`, ela pode ser referenciada pelo `family` em qualquer propriedade `fontFamily` do documento:

```yaml
body:
  Text:
    text: "Hello"
    style:
      fontFamily: "Roboto Mono"  # referencia a font importada
```

**Validação:**
- `family`: string não-vazia (nome exato de uma família disponível no Google Fonts)
- `weights`: array não-vazio de inteiros; cada peso deve ser 100-900 em incrementos de 100
- Máximo de **10 fonts** por documento

### Importação de Scripts Lua

```yaml
head:
  scripts:
    - src: "scripts/main.lua"
    - src: "scripts/helpers.lua"
```

**Validação:**
- `src`: caminho na whitelist com extensão `.lua`
- Máximo de **5 scripts** por documento
- Scripts são executados na ordem declarada

### Importação de Componentes

```yaml
head:
  imports:
    - src: "components/nav-bar.ntml"
      as: "NavBar"
    - src: "components/footer.ntml"
      as: "Footer"
```

Após importar, o alias PascalCase pode ser usado como se fosse um componente built-in dentro de `body`.

**Validação:**
- `src`: caminho na whitelist com extensão `.ntml`; o arquivo referenciado deve ser um arquivo de componente (ter a chave `component`)
- `as`: PascalCase, não pode conflitar com nomes de componentes built-in
- Máximo de **10 imports** por documento

## Componentes

### Propriedade `id` (Nova — v0.2.0)

Todos os componentes aceitam uma propriedade `id` opcional, que permite que scripts Lua façam referência ao componente pelo identificador:

```yaml
Text:
  id: "status-text"
  text: "Aguardando..."
```

**Validação:**
- `id`: string não-vazia, único no documento

### Categoria: Layout

#### Container

Contêiner básico para agrupar elementos.

**Propriedades:**
- `id` (opcional): String única
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

**Exemplo:**
```yaml
Container:
  style:
    padding: 16
  children:
    - Text:
        text: "Content"
```

#### Flex

Layout flexível com controle de direção e alinhamento.

**Propriedades:**
- `id` (opcional): String única
- `direction` (opcional): `"row"` | `"column"`
- `justify` (opcional): `"start"` | `"center"` | `"end"` | `"spaceBetween"` | `"spaceAround"` | `"spaceEvenly"`
- `align` (opcional): `"start"` | `"center"` | `"end"` | `"stretch"`
- `gap` (opcional): Número (pixels)
- `wrap` (opcional): Boolean
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

**Validação:**
- `gap` deve ser não-negativo

#### Grid

Layout em grade.

**Propriedades:**
- `id` (opcional): String única
- `columns` (obrigatório): Número ou Array de strings
- `rows` (opcional): Número ou Array de strings
- `gap` (opcional): Número ou `{ row: Número, column: Número }`
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

**Validação:**
- `columns` deve ser > 0 se número
- `columns` não pode ser array vazio
- `gap` valores devem ser não-negativos

#### Stack

Empilha elementos (z-index).

**Propriedades:**
- `id` (opcional): String única
- `alignment` (opcional): `"topLeft"` | `"topCenter"` | `"topRight"` | `"centerLeft"` | `"center"` | `"centerRight"` | `"bottomLeft"` | `"bottomCenter"` | `"bottomRight"`
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

#### Row

Atalho para Flex com `direction: row`.

**Propriedades:**
- `id` (opcional): String única
- `justify` (opcional): JustifyContent
- `align` (opcional): AlignItems
- `gap` (opcional): Número não-negativo
- `wrap` (opcional): Boolean
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

#### Column

Atalho para Flex com `direction: column`.

**Propriedades:** Idênticas a Row

### Categoria: Content

#### Text

Exibe texto.

**Propriedades:**
- `id` (opcional): String única
- `text` (obrigatório): String não-vazia
- `style` (opcional): Objeto Style

**Validação:**
- `text` não pode ser vazio

#### Image

Exibe imagem.

**Propriedades:**
- `id` (opcional): String única
- `src` (obrigatório): String não-vazia (caminho da imagem)
- `alt` (opcional): String (texto alternativo)
- `fit` (opcional): `"cover"` | `"contain"` | `"fill"` | `"none"` | `"scaleDown"`
- `style` (opcional): Objeto Style

**Validação:**
- `src` não pode ser vazio
- Em runtime: `src` deve estar na whitelist de assets

#### Icon

Exibe ícone.

**Propriedades:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `size` (opcional): Número positivo
- `style` (opcional): Objeto Style

**Validação:**
- `name` não pode ser vazio
- `size` deve ser > 0 se fornecido

### Categoria: Interactive

#### Button

Botão clicável.

**Propriedades:**
- `id` (opcional): String única
- `action` (obrigatório): String não-vazia
- `variant` (opcional): `"primary"` | `"secondary"` | `"danger"` | `"ghost"`
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

**Validação:**
- `action` não pode ser vazio

**Formato do `action` (v0.2.0):**
- `"functionName"` — chama a função Lua `functionName()` de um script importado
- `"namespace:eventName"` — dispara evento de jogo diretamente (comportamento v0.1.0)

#### Input

Campo de entrada de texto.

**Propriedades:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `placeholder` (opcional): String
- `value` (opcional): String
- `type` (opcional): `"text"` | `"password"` | `"number"`
- `maxLength` (opcional): Número inteiro positivo
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style

**Validação:**
- `name` não pode ser vazio

#### Checkbox

Caixa de seleção.

**Propriedades:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `label` (opcional): String
- `checked` (opcional): Boolean
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style

#### Radio

Botão de rádio.

**Propriedades:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `value` (obrigatório): String não-vazia
- `label` (opcional): String
- `checked` (opcional): Boolean
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style

**Validação:**
- `name` e `value` não podem ser vazios

#### Select

Menu dropdown.

**Propriedades:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `options` (obrigatório): Array não-vazio de `{ label: String, value: String }`
- `value` (opcional): String
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style

**Validação:**
- `name` não pode ser vazio
- `options` deve ter pelo menos um item

### Categoria: Display

#### ProgressBar

Barra de progresso.

**Propriedades:**
- `id` (opcional): String única
- `value` (obrigatório): Número
- `max` (opcional): Número positivo (default: 100)
- `variant` (opcional): `"default"` | `"success"` | `"warning"` | `"danger"`
- `showLabel` (opcional): Boolean
- `style` (opcional): Objeto Style

**Validação:**
- `value` deve estar entre 0 e `max`
- `max` deve ser > 0 se fornecido

#### Badge

Selo ou etiqueta.

**Propriedades:**
- `id` (opcional): String única
- `text` (obrigatório): String não-vazia
- `variant` (opcional): `"default"` | `"primary"` | `"success"` | `"warning"` | `"danger"`
- `style` (opcional): Objeto Style

**Validação:**
- `text` não pode ser vazio

#### Divider

Linha divisória.

**Propriedades:**
- `id` (opcional): String única
- `orientation` (opcional): `"horizontal"` | `"vertical"`
- `style` (opcional): Objeto Style

#### Spacer

Espaço vazio.

**Propriedades:**
- `size` (obrigatório): Número ou `"auto"`

### Categoria: Documento e código

#### Code

Código inline ou em bloco, com highlight de sintaxe opcional.

**Propriedades:**
- `id` (opcional): String única
- `text` (obrigatório): String (conteúdo do código)
- `language` (opcional): String (ex.: `lua`, `python`)
- `block` (opcional): Boolean (default: false) — se true, renderiza como `<pre><code>`
- `style` (opcional): Objeto Style

#### Markdown

Renderiza conteúdo em markdown como HTML (títulos, listas, tabelas, etc.). O conteúdo é parseado e sanitizado.

**Propriedades:**
- `id` (opcional): String única
- `content` (obrigatório): String (fonte markdown)
- `style` (opcional): Objeto Style

#### List e ListItem

Lista ordenada ou não ordenada.

**List — Propriedades:**
- `id` (opcional): String única
- `ordered` (opcional): Boolean (default: false) — false = `<ul>`, true = `<ol>`
- `children` (obrigatório): Array de ListItem
- `style` (opcional): Objeto Style

**ListItem — Propriedades:**
- `id` (opcional): String única
- `children` (obrigatório): Array de Components
- `style` (opcional): Objeto Style

#### Heading

Título semântico (h1, h2, h3).

**Propriedades:**
- `id` (opcional): String única
- `level` (obrigatório): 1 | 2 | 3
- `text` (obrigatório): String não-vazia
- `style` (opcional): Objeto Style

#### Table

Tabela de dados com linha de cabeçalho e linhas de corpo.

**Propriedades:**
- `id` (opcional): String única
- `headers` (obrigatório): Array de strings (cabeçalhos das colunas)
- `rows` (obrigatório): Array de arrays de strings (cada array interno é uma linha)
- `style` (opcional): Objeto Style

#### Blockquote

Bloco de citação.

**Propriedades:**
- `id` (opcional): String única
- `children` (obrigatório): Array de Components
- `style` (opcional): Objeto Style

#### Pre

Texto pré-formatado (sem highlight de sintaxe).

**Propriedades:**
- `id` (opcional): String única
- `text` (obrigatório): String
- `style` (opcional): Objeto Style

#### Details

Seção colapsável com resumo.

**Propriedades:**
- `id` (opcional): String única
- `summary` (obrigatório): String (rótulo do toggle)
- `children` (obrigatório): Array de Components (corpo expansível)
- `open` (opcional): Boolean (default: false) — se expandido por padrão
- `style` (opcional): Objeto Style

## Sistema de Estilos

### Objeto Style

Todas as propriedades de estilo são opcionais. Além das propriedades listadas abaixo, o objeto Style aceita:

- **`classes`** (opcional): String com nomes de classes CSS separados por espaço (ex.: para Tailwind). O renderizador emite o atributo `class` no HTML (valor sanitizado). O documento deve incluir o CSS correspondente (ex.: Tailwind) para que as classes tenham efeito.

### Dimensões

```typescript
interface DimensionStyles {
  width?: number | "auto"
  height?: number | "auto"
  minWidth?: number
  maxWidth?: number
  minHeight?: number
  maxHeight?: number
}
```

### Espaçamento

```typescript
interface SpacingStyles {
  // Padding
  padding?: number
  paddingVertical?: number
  paddingHorizontal?: number
  paddingTop?: number
  paddingRight?: number
  paddingBottom?: number
  paddingLeft?: number

  // Margin
  margin?: number
  marginVertical?: number
  marginHorizontal?: number
  marginTop?: number
  marginRight?: number
  marginBottom?: number
  marginLeft?: number
}
```

### Cores

```typescript
interface ColorStyles {
  color?: Color
  backgroundColor?: Color
  borderColor?: Color
  opacity?: number  // 0.0 - 1.0
}

type Color = HexColor | NamedColor
type HexColor = string  // "#RRGGBB" format
type NamedColor = "red" | "blue" | "green" | "white" | "black" |
                  "transparent" | "yellow" | "orange" | "purple" |
                  "pink" | "gray" | "grey"
```

**Validação de Cores:**
- Hex: Deve ser exatamente 6 dígitos hexadecimais precedidos por `#`
- Named: Deve ser uma das cores nomeadas suportadas
- `opacity`: Deve estar entre 0.0 e 1.0 (inclusivo)

### Tipografia

```typescript
interface TypographyStyles {
  fontSize?: number
  fontWeight?: number | "normal" | "bold"  // 100-900 in increments of 100
  fontFamily?: "sans" | "serif" | "monospace" | "game" | string  // string = family de font importada via head
  textAlign?: "left" | "center" | "right" | "justify"
  textTransform?: "none" | "uppercase" | "lowercase" | "capitalize"
  letterSpacing?: number
  lineHeight?: number  // Must be non-negative
  textDecoration?: "none" | "underline" | "line-through"
}
```

**Validação:**
- `fontWeight` numérico: Deve ser 100-900 em incrementos de 100
- `lineHeight`: Deve ser não-negativo
- `fontFamily` string customizada: Deve corresponder a um `family` declarado no `head.fonts`

### Bordas

```typescript
interface BorderStyles {
  borderWidth?: number
  borderTopWidth?: number
  borderRightWidth?: number
  borderBottomWidth?: number
  borderLeftWidth?: number
  borderStyle?: "solid" | "dashed" | "dotted"
  borderRadius?: number
  borderTopLeftRadius?: number
  borderTopRightRadius?: number
  borderBottomLeftRadius?: number
  borderBottomRightRadius?: number
}
```

### Sombras

```typescript
type Shadow = ShadowPreset | CustomShadow

type ShadowPreset = "small" | "medium" | "large"

interface CustomShadow {
  shadowColor: Color
  shadowOffset: { x: number, y: number }
  shadowBlur: number
  shadowOpacity: number  // 0.0 - 1.0
}
```

### Posicionamento

```typescript
interface PositionStyles {
  position?: "relative" | "absolute"
  top?: number
  right?: number
  bottom?: number
  left?: number
  zIndex?: number
}
```

### Flex Item

```typescript
interface FlexItemStyles {
  flex?: number  // Must be non-negative
  alignSelf?: "start" | "center" | "end" | "stretch"
}
```

### Display

```typescript
interface DisplayStyles {
  display?: "flex" | "none"
  overflow?: "visible" | "hidden" | "scroll" | "auto"
  cursor?: "default" | "pointer" | "not-allowed" | "text"
}
```

### Classes CSS (Tailwind)

```typescript
// No objeto Style
classes?: string   // Ex.: "p-4 bg-gray-100 rounded"
```

**Validação:** Se presente, `classes` deve conter apenas caracteres seguros (alfanuméricos, espaços, `-`, `_`, `:`, `/`, `.`) para evitar injeção em atributos.

## Sistema de Temas

### Estrutura do Theme

```typescript
interface Theme {
  colors?: Record<string, Color>
  spacing?: Record<string, number>
  borderRadius?: Record<string, number>
  typography?: Record<string, number>
}
```

### Referências de Tema

Sintaxe: `$theme.<category>.<key>`

Categorias:
- `colors`: Cores do tema
- `spacing`: Valores de espaçamento
- `borderRadius`: Valores de border radius
- `typography`: Tamanhos de fonte

**Exemplo:**
```yaml
Text:
  text: "Hello"
  style:
    color: "$theme.colors.primary"
    fontSize: "$theme.typography.heading"
```

**Validação:**
- Referência deve começar com `$theme.`
- Deve ter formato `$theme.<category>.<key>`
- Categoria deve existir no tema
- Key deve existir na categoria

## Sistema de Scripting Lua

Scripts Lua permitem adicionar comportamento dinâmico às páginas NTML. Eles são declarados no `head.scripts` e ficam em arquivos `.lua` separados.

### Modelo de Execução

Scripts Lua rodam em uma **sandbox isolada** sem acesso ao sistema de arquivos ou APIs do sistema operacional. A comunicação com a UI é feita pela API `ui`; a comunicação com o backend é feita pela API `http`.

Scripts são carregados na ordem declarada em `head.scripts`. Funções definidas em qualquer script ficam disponíveis globalmente para os `action` dos botões e eventos do runtime.

### API `ui` — Interação com a Interface

```lua
-- Lê o valor atual de um Input, Checkbox ou Select pelo name
local valor = ui.get_value("input-name")   -- retorna string ou nil

-- Altera o texto de um componente Text pelo id
ui.set_text("meu-text-id", "novo texto")

-- Mostra ou oculta um componente pelo id
ui.set_visible("meu-componente-id", false)

-- Altera o value de um ProgressBar pelo id
ui.set_value("minha-barra-id", 75)

-- Altera o disabled de um Button ou Input pelo id
ui.set_disabled("meu-botao-id", true)
```

### API `http` — Comunicação com o Backend

A API `http` expõe chamadas HTTP ao estilo axios: `http.get`, `http.post`, `http.put`, `http.patch` e `http.delete`. Todas retornam uma tabela de resposta com `status`, `ok` e `data`.

```lua
-- GET
local res = http.get("/api/player/stats")
if res.ok then
  ui.set_value("cpu-bar", res.data.cpu)
end

-- POST com body
local res = http.post("/api/auth/login", {
  username = ui.get_value("username"),
  password = ui.get_value("password"),
})

-- PUT / PATCH / DELETE
local res = http.put("/api/settings", { theme = "dark" })
local res = http.patch("/api/profile", { bio = "hacker" })
local res = http.delete("/api/session")
```

**Campos da resposta:**

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `res.ok` | boolean | `true` se status 2xx |
| `res.status` | number | Código HTTP (200, 404, etc.) |
| `res.data` | table ou string | Body da resposta (parseado automaticamente se JSON) |
| `res.error` | string ou nil | Mensagem de erro, se houver |

### Convenção de Handlers de Button

Quando um `Button` tem `action: "nomeDaFuncao"` (sem `:`), o runtime chama a função Lua de mesmo nome ao clicar:

```lua
-- Chamado por: Button com action: "fazerLogin"
function fazerLogin()
  local usuario = ui.get_value("username")
  local senha   = ui.get_value("password")
  local res = http.post("/api/auth/login", {
    username = usuario,
    password = senha,
  })
  if res.ok then
    ui.set_visible("feedback", false)
  else
    ui.set_text("feedback", "Falha: " .. (res.error or res.status))
    ui.set_visible("feedback", true)
  end
end

-- Chamado por: Button com action: "limparFormulario"
function limparFormulario()
  ui.set_text("feedback-text", "")
  ui.set_visible("erro-msg", false)
end
```

### Restrições de Segurança

| Restrição | Valor |
|-----------|-------|
| Módulos bloqueados | `io`, `os`, `file`, `require`, `loadfile`, `dofile`, `coroutine`, `debug` |
| Timeout por handler | **200ms** |
| Máximo de linhas por arquivo `.lua` | **500 linhas** |
| Máximo de scripts por documento | **5** |
| Globals permitidas | Apenas `ui`, `http` e funções padrão seguras de Lua (`math`, `string`, `table`, `pairs`, `ipairs`, `tostring`, `tonumber`, `type`) |

### Exemplo Completo de Script

```lua
-- scripts/login.lua

function fazerLogin()
  local usuario = ui.get_value("username")
  local senha   = ui.get_value("password")

  if not usuario or usuario == "" then
    ui.set_text("feedback", "Usuário obrigatório.")
    ui.set_visible("feedback", true)
    return
  end

  ui.set_disabled("btn-login", true)
  ui.set_text("feedback", "Autenticando...")

  local res = http.post("/api/auth/login", {
    username = usuario,
    password = senha,
  })

  ui.set_disabled("btn-login", false)

  if res.ok then
    ui.set_visible("feedback", false)
  else
    ui.set_text("feedback", "Falha: " .. (res.error or tostring(res.status)))
    ui.set_visible("feedback", true)
  end
end
```

## Componentes Importáveis

Componentes importáveis permitem definir blocos de UI reutilizáveis em arquivos `.ntml` separados e usá-los em múltiplas páginas.

### Formato de Arquivo de Componente

Um arquivo de componente tem a chave `component` na raiz (em vez de um nome de componente built-in), seguida opcionalmente por `props` e obrigatoriamente por `body`:

```yaml
# components/nav-bar.ntml
component: NavBar

props:
  - name: title
    type: string
    default: "Navigation"
  - name: accentColor
    type: color
    default: "#00ff00"
  - name: showBack
    type: boolean
    default: false

body:
  Row:
    align: center
    gap: 12
    style:
      padding: 12
      backgroundColor: "#111111"
    children:
      - Text:
          text: "{props.title}"
          style:
            fontSize: 18
            fontWeight: bold
            color: "{props.accentColor}"
```

### Tipos de Props

| Tipo | Descrição | Exemplo de `default` |
|------|-----------|----------------------|
| `string` | Texto | `default: "Hello"` |
| `number` | Numérico | `default: 0` |
| `boolean` | Booleano | `default: false` |
| `color` | Cor (hex ou named) | `default: "#ffffff"` |

### Interpolação de Props

Use `{props.nome}` como valor de qualquer propriedade dentro do `body` do componente:

```yaml
Text:
  text: "{props.title}"         # interpolação em string
  style:
    color: "{props.accentColor}" # interpolação em color
    fontSize: "{props.size}"     # interpolação em number
```

**Regras de interpolação:**
- `{props.nome}` deve ser o valor inteiro da propriedade (não é possível concatenar: `"Olá {props.nome}"` não é válido — use Lua para isso)
- O tipo do valor interpolado deve ser compatível com o tipo esperado pela propriedade

### Uso de Componente Importado

```yaml
head:
  title: "Dashboard"
  imports:
    - src: "components/nav-bar.ntml"
      as: "NavBar"

body:
  Column:
    children:
      - NavBar:
          title: "Meu App"
          accentColor: "#ff6600"
          showBack: false
      - Text:
          text: "Conteúdo aqui"
```

### Validação de Componentes

**Arquivo de componente:**
- `component`: PascalCase, não pode ser nome de componente built-in
- `props[].name`: camelCase, único dentro do componente
- `props[].type`: deve ser um dos tipos suportados (`string`, `number`, `boolean`, `color`)
- `props[].default`: se ausente, a prop é **obrigatória** ao instanciar o componente
- Arquivos de componente **não podem** ter `head` (sem imports aninhados — previne dependências circulares)

**Ao instanciar:**
- Props desconhecidas (não declaradas no arquivo de componente) → erro de validação
- Prop obrigatória ausente → erro de validação
- Tipo de valor incompatível com o tipo declarado → erro de validação

## Validação

### Fases de Validação

1. **Parse YAML**: Valida sintaxe YAML
2. **Parse Head** (se presente): Valida estrutura e campos do `head`
3. **Parse Component**: Valida estrutura de componentes
4. **Type Validation**: Valida tipos de propriedades
5. **Semantic Validation**: Valida regras semânticas (ranges, não-vazio, etc)
6. **Theme Resolution**: Resolve referências de tema (se aplicável)
7. **Prop Resolution** (v0.2.0): Resolve e valida interpolações `{props.*}` em componentes importados

### Erros

Todos os erros incluem informações contextuais:

```rust
pub enum NtmlError {
    // Erros v0.1.0
    ParseError { line: usize, column: usize, message: String },
    ValidationError(String),
    InvalidComponent { component: String, reason: String },
    InvalidProperty { component: String, property: String, reason: String },
    InvalidStyle { property: String, reason: String },
    InvalidColor { value: String, reason: String },
    InvalidDimension { value: String },
    InvalidEnum { property: String, value: String, expected: String },
    MissingProperty { component: String, property: String },
    MaxNestingDepthExceeded { max_depth: usize },
    ThemeVariableNotFound { variable: String },
    InvalidThemeReference { reference: String },
    AssetNotWhitelisted { path: String },
    InvalidAction { action: String },
    MultipleRootComponents,
    EmptyDocument,
    ValueOutOfRange { property: String, value: String, range: String },

    // Erros do head (v0.2.0)
    MissingBody,                                // head presente mas body ausente
    MissingTitle,                               // head presente mas title ausente
    InvalidFontFamily { family: String },        // nome de família vazio
    ScriptLimitExceeded { max: usize },         // mais de 5 scripts
    ImportLimitExceeded { max: usize },         // mais de 10 imports
    FontLimitExceeded { max: usize },           // mais de 10 fonts
    InvalidImportAlias { alias: String },       // alias não é PascalCase ou conflita com built-in
    InvalidTag { tag: String },                 // tag com espaços, uppercase ou vazia
    TagLimitExceeded { max: usize },            // mais de 10 tags

    // Erros de componentes importáveis (v0.2.0)
    UnknownImportedComponent { name: String },  // usado mas não importado via head.imports
    MissingRequiredProp { component: String, prop: String },
    InvalidPropType { component: String, prop: String, expected: String },
    UnknownProp { component: String, prop: String },
    CircularComponentImport { path: String },
    ComponentFileHasHead { path: String },       // arquivo de componente não pode ter head

    // Erros de Lua (v0.2.0)
    LuaScriptTooLong { src: String, max_lines: usize },
    LuaHandlerTimeout { handler: String, timeout_ms: u64 },
    LuaRuntimeError { src: String, message: String },
    LuaFunctionNotFound { action: String },     // action sem ":" mas sem função Lua correspondente
}
```

## Segurança

### Prevenção de Exploits

1. **Profundidade de Aninhamento**: Máximo 20 níveis previne stack overflow
2. **Validação de Tipos**: Previne type confusion attacks
3. **Whitelist de Assets**: Apenas imagens/icons/fonts/scripts aprovados
4. **Validação de Actions**: Actions com `:` são validadas no runtime; sem `:` chamam Lua
5. **Ranges de Valores**: Previne valores absurdos (opacity > 1, etc)
6. **Sandbox Lua**: Scripts Lua rodam em ambiente isolado, sem acesso ao sistema

### Sandbox Lua

- Módulos proibidos: `io`, `os`, `file`, `require`, `loadfile`, `dofile`, `coroutine`, `debug`
- Timeout de **200ms** por execução de handler — previne loops infinitos
- Limite de **500 linhas** por arquivo — previne scripts excessivamente complexos
- Máximo de **5 scripts** por documento
- Apenas APIs `ui` e `http` disponíveis para comunicação externa

### Sandboxing de Componentes

- Componentes importados não podem importar outros componentes
- Sem herança de contexto Lua entre componentes — cada página tem seu próprio contexto de script
- Props passadas a componentes são validadas por tipo antes da renderização

### Sandboxing Geral

- Componentes não têm acesso ao sistema de arquivos
- Actions são strings validadas pelo servidor
- Imagens devem estar em whitelist
- Paths de scripts e componentes devem estar na whitelist de assets

## Extensibilidade

### Adicionando Novos Componentes

1. Adicionar struct em `components.rs`
2. Adicionar variant ao enum `Component`
3. Adicionar parser em `parser.rs`
4. Adicionar validador em `validator.rs`
5. Atualizar documentação

### Adicionando Novas Propriedades de Estilo

1. Adicionar campo ao struct `Style` em `style.rs`
2. Adicionar validação (se necessário) em `validator.rs`
3. Atualizar documentação

## Versionamento

NTML segue Semantic Versioning:

- **Major**: Mudanças incompatíveis (remover componentes, mudar validação)
- **Minor**: Novos recursos compatíveis (novos componentes, propriedades)
- **Patch**: Bug fixes

### Changelog

#### v0.2.0
- **Novo**: Seção `head` com suporte a metadados, fonts, scripts e imports
- **Novo**: Sistema de scripting Lua com sandbox
- **Novo**: Componentes importáveis com sistema de props e interpolação
- **Novo**: Propriedade `id` em todos os componentes para integração com Lua
- **Novo**: `fontFamily` aceita fontes customizadas declaradas no `head`
- **Novo**: `action` em Button suporta chamada de funções Lua (sem `:`)
- **Novo**: API `http` para comunicação com backend (http.get, http.post, http.put, http.patch, http.delete)
- **Alterado**: Fonts carregadas do Google Fonts via `family` + `weights` (sem `src` local)
- **Novo**: `head.tags` — array de strings para indexação de páginas

#### v0.1.0
- Versão inicial — componentes, estilos, temas e validação

Versão atual: **0.2.0**

## Conformidade

Implementações NTML devem:

1. ✅ Validar todos os documentos segundo esta especificação
2. ✅ Rejeitar documentos inválidos com mensagens de erro claras
3. ✅ Implementar todas as regras de validação
4. ✅ Suportar todos os componentes e propriedades definidos
5. ✅ Seguir as regras de segurança
6. ✅ Manter backward compatibility com documentos v0.1.0

## Exemplos Completos

### UI de Status do Jogador (v0.1.0 — formato clássico)

```yaml
Container:
  style:
    padding: 24
    backgroundColor: "#0a0a0a"
    borderRadius: 8
    borderWidth: 2
    borderColor: "#00ff00"
  children:
    - Column:
        gap: 16
        children:
          - Text:
              text: "SYSTEM STATUS"
              style:
                fontSize: 24
                fontWeight: bold
                color: "#00ff00"
                fontFamily: monospace

          - Row:
              gap: 12
              align: center
              children:
                - Icon:
                    name: "cpu"
                    size: 20
                - ProgressBar:
                    value: 85
                    variant: success
                    style:
                      flex: 1

          - Grid:
              columns: 2
              gap: 8
              children:
                - Badge:
                    text: "ONLINE"
                    variant: success
                - Badge:
                    text: "LVL 42"
                    variant: primary
```

### Formulário de Login com Lua (v0.2.0)

```yaml
head:
  title: "Login Seguro"
  scripts:
    - src: "scripts/login.lua"

body:
  Column:
    gap: 16
    style:
      padding: 32
      maxWidth: 400
    children:
      - Text:
          text: "SECURE LOGIN"
          style:
            fontSize: 28
            fontWeight: bold
            textAlign: center

      - Input:
          name: "username"
          placeholder: "Username"
          type: text

      - Input:
          name: "password"
          placeholder: "Password"
          type: password

      - Text:
          id: "feedback"
          text: ""
          style:
            color: red
            display: none

      - Button:
          id: "btn-login"
          action: "fazerLogin"
          variant: primary
          children:
            - Text:
                text: "LOGIN"
                style:
                  fontWeight: bold
```

### Página com Componente Importado (v0.2.0)

```yaml
# dashboard.ntml
head:
  title: "Dashboard"
  fonts:
    - family: "Roboto Mono"
      weights: [400, 700]
  imports:
    - src: "components/nav-bar.ntml"
      as: "NavBar"
    - src: "components/stat-card.ntml"
      as: "StatCard"

body:
  Column:
    children:
      - NavBar:
          title: "Dashboard"
          accentColor: "#00ff00"
      - Grid:
          columns: 3
          gap: 16
          style:
            padding: 24
          children:
            - StatCard:
                label: "CPU"
                value: 85
                unit: "%"
            - StatCard:
                label: "RAM"
                value: 4096
                unit: "MB"
            - StatCard:
                label: "Ping"
                value: 12
                unit: "ms"
```

### Arquivo de Componente com Props (v0.2.0)

```yaml
# components/stat-card.ntml
component: StatCard

props:
  - name: label
    type: string
  - name: value
    type: number
  - name: unit
    type: string
    default: ""
  - name: variant
    type: string
    default: "default"

body:
  Container:
    style:
      padding: 16
      borderRadius: 8
      borderWidth: 1
      borderColor: "#333333"
      backgroundColor: "#111111"
    children:
      - Text:
          text: "{props.label}"
          style:
            fontSize: 12
            color: gray
            textTransform: uppercase
      - Row:
          align: center
          gap: 4
          children:
            - Text:
                text: "{props.value}"
                style:
                  fontSize: 32
                  fontWeight: bold
                  color: "#00ff00"
            - Text:
                text: "{props.unit}"
                style:
                  fontSize: 14
                  color: gray
```

## Referências

- [YAML 1.2 Specification](https://yaml.org/spec/1.2/spec.html)
- [CSS Box Model](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Box_Model)
- [Flexbox Guide](https://css-tricks.com/snippets/css/a-guide-to-flexbox/)
- [Lua 5.4 Reference Manual](https://www.lua.org/manual/5.4/)
