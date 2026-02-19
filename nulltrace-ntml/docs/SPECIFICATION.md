# NTML Specification v0.3.0

## Introdução

**NullTrace Markup Language (NTML)** é uma linguagem declarativa baseada em XML para criar interfaces de usuário no jogo NullTrace. Esta especificação define a sintaxe, semântica e regras de validação do NTML.

## Objetivos de Design

1. **Segurança**: Prevenir injeções e exploits através de validação rigorosa
2. **Simplicidade**: Sintaxe XML/HTML-like legível e fácil de aprender
3. **Expressividade**: Rico conjunto de componentes e estilos
4. **Type-Safety**: Validação estrita de tipos e propriedades
5. **Consistência**: Comportamento previsível e determinístico

## Estrutura do Documento

### Formatos

NTML suporta dois formatos de documento:

#### Formato Clássico

Um único componente raiz na raiz do documento. Sem `head` e sem `body`. Melhor para páginas simples.

```xml
<ComponentType property1="value1" property2="value2">
  <ChildComponent1 />
  <ChildComponent2 prop="val" />
</ComponentType>
```

#### Formato Completo (head + body)

Quando o documento inclui uma seção `head`, a estrutura muda para `head` + `body`:

```xml
<head>
  <title>Minha Página</title>
  <description>Painel de status do sistema</description>
  <tags>hud sistema monitoramento</tags>

  <font family="Roboto Mono" weights="400,700" />

  <script src="scripts/main.lua" />

  <import src="components/nav-bar.ntml" as="NavBar" />
</head>
<body>
  <Container>
    <NavBar title="Meu App" />
    <Text text="Hello" />
  </Container>
</body>
```

**Regra de compatibilidade:** Se o elemento `head` estiver presente, o elemento `body` é obrigatório. Se nenhum dos dois estiver presente, o documento é tratado como formato clássico.

### Atributos

Propriedades de componentes são passadas como atributos XML:

```xml
<Text text="Hello" style="fontSize:24; color:#00ff00" class="font-mono" />
```

- **`class`**: Nomes de classes CSS separados por espaço (ex.: Tailwind). O renderizador emite o atributo `class` no HTML.
- **`style`**: Propriedades de estilo NTML no formato `chave:valor; chave2:valor2`.
- Demais atributos: propriedades específicas do componente (ex.: `text`, `action`, `src`).

### Regras Globais

1. **Um único componente raiz**: Documentos com zero ou múltiplos componentes raiz são inválidos
2. **Profundidade máxima**: Máximo de 20 níveis de aninhamento
3. **Case sensitivity**: Nomes de componentes e propriedades são case-sensitive
4. **Naming convention**:
   - Componentes: PascalCase (e.g., `Container`, `ProgressBar`)
   - Atributos/propriedades: camelCase (e.g., `backgroundColor`, `fontSize`)

## Seção Head

A seção `head` é opcional e permite declarar metadados da página, importar fonts, scripts Lua e componentes externos.

### Metadados

```xml
<head>
  <title>Nome da Página</title>         <!-- obrigatório quando head presente -->
  <description>Descrição breve</description>  <!-- opcional -->
  <author>NomeDoJogador</author>         <!-- opcional -->
  <tags>hud status monitoramento</tags> <!-- opcional — separados por espaço -->
</head>
```

**Validação:**
- `title`: string não-vazia; obrigatório se `head` estiver presente
- `description`: string; opcional
- `author`: string; opcional
- `tags`: texto com tokens separados por espaço; cada tag deve ser não-vazia, sem espaços internos, lowercase; máximo de **10 tags**

### Importação de Fonts

Fontes são carregadas do **Google Fonts**. O usuário especifica apenas o nome da família e os pesos desejados.

```xml
<head>
  <font family="Roboto Mono" weights="400,700" />
  <font family="Inter" weights="300,400,600" />
</head>
```

Após declarar uma font no `head`, ela pode ser referenciada pelo `family` em qualquer propriedade `fontFamily` do documento:

```xml
<body>
  <Text text="Hello" style="fontFamily:Roboto Mono" />
</body>
```

**Validação:**
- `family`: string não-vazia (nome exato de uma família disponível no Google Fonts)
- `weights`: lista de inteiros separados por vírgula; cada peso deve ser 100-900 em incrementos de 100
- Máximo de **10 fonts** por documento

### Importação de Scripts Lua

```xml
<head>
  <script src="scripts/main.lua" />
  <script src="scripts/helpers.lua" />
</head>
```

**Validação:**
- `src`: caminho na whitelist com extensão `.lua`
- Máximo de **5 scripts** por documento
- Scripts são executados na ordem declarada

### Importação de Componentes

```xml
<head>
  <import src="components/nav-bar.ntml" as="NavBar" />
  <import src="components/footer.ntml" as="Footer" />
</head>
```

Após importar, o alias PascalCase pode ser usado como se fosse um componente built-in dentro de `body`.

**Validação:**
- `src`: caminho na whitelist com extensão `.ntml`
- `as`: PascalCase, não pode conflitar com nomes de componentes built-in
- Máximo de **10 imports** por documento

## Componentes

### Propriedade `id`

Todos os componentes aceitam um atributo `id` opcional, que permite que scripts Lua façam referência ao componente pelo identificador:

```xml
<Text id="status-text" text="Aguardando..." />
```

**Validação:**
- `id`: string não-vazia, único no documento

### Categoria: Layout

#### Container

Contêiner básico para agrupar elementos.

**Atributos:**
- `id` (opcional): String única
- `style` (opcional): String de estilos
- `class` (opcional): Classes CSS
- Filhos: qualquer Component

**Exemplo:**
```xml
<Container style="padding:16">
  <Text text="Content" />
</Container>
```

#### Flex

Layout flexível com controle de direção e alinhamento.

**Atributos:**
- `id` (opcional): String única
- `direction` (opcional): `"row"` | `"column"`
- `justify` (opcional): `"start"` | `"center"` | `"end"` | `"spaceBetween"` | `"spaceAround"` | `"spaceEvenly"`
- `align` (opcional): `"start"` | `"center"` | `"end"` | `"stretch"`
- `gap` (opcional): Número (pixels)
- `wrap` (opcional): `"true"` | `"false"`
- `style` / `class` (opcionais)
- Filhos: qualquer Component

**Validação:**
- `gap` deve ser não-negativo

#### Grid

Layout em grade.

**Atributos:**
- `id` (opcional): String única
- `columns` (obrigatório): Número ou string de definições (ex.: `"1fr 2fr 1fr"`)
- `rows` (opcional): Número ou string de definições
- `gap` (opcional): Número
- `style` / `class` (opcionais)
- Filhos: qualquer Component

**Validação:**
- `columns` deve ser > 0 se número
- `gap` valores devem ser não-negativos

#### Stack

Empilha elementos (z-index).

**Atributos:**
- `id` (opcional): String única
- `alignment` (opcional): `"topLeft"` | `"topCenter"` | `"topRight"` | `"centerLeft"` | `"center"` | `"centerRight"` | `"bottomLeft"` | `"bottomCenter"` | `"bottomRight"`
- `style` / `class` (opcionais)
- Filhos: qualquer Component

#### Row

Atalho para Flex com `direction: row`.

**Atributos:**
- `id` (opcional): String única
- `justify` (opcional): JustifyContent
- `align` (opcional): AlignItems
- `gap` (opcional): Número não-negativo
- `wrap` (opcional): Boolean string
- `style` / `class` (opcionais)
- Filhos: qualquer Component

#### Column

Atalho para Flex com `direction: column`.

**Atributos:** Idênticos a Row

### Categoria: Content

#### Text

Exibe texto.

**Atributos:**
- `id` (opcional): String única
- `text` (obrigatório): String não-vazia
- `style` / `class` (opcionais)

**Validação:**
- `text` não pode ser vazio

#### Image

Exibe imagem.

**Atributos:**
- `id` (opcional): String única
- `src` (obrigatório): String não-vazia (caminho da imagem)
- `alt` (opcional): String (texto alternativo)
- `fit` (opcional): `"cover"` | `"contain"` | `"fill"` | `"none"` | `"scaleDown"`
- `style` / `class` (opcionais)

**Validação:**
- `src` não pode ser vazio
- Em runtime: `src` deve estar na whitelist de assets

#### Icon

Exibe ícone.

**Atributos:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `size` (opcional): Número positivo
- `style` / `class` (opcionais)

**Validação:**
- `name` não pode ser vazio
- `size` deve ser > 0 se fornecido

### Categoria: Interactive

#### Button

Botão clicável.

**Atributos:**
- `id` (opcional): String única
- `action` (obrigatório): String não-vazia
- `variant` (opcional): `"primary"` | `"secondary"` | `"danger"` | `"ghost"`
- `disabled` (opcional): `"true"` | `"false"`
- `style` / `class` (opcionais)
- Filhos: qualquer Component

**Validação:**
- `action` não pode ser vazio

**Formato do `action`:**
- `"functionName"` — chama a função Lua `functionName()` de um script importado
- `"namespace:eventName"` — dispara evento de jogo diretamente

#### Input

Campo de entrada de texto.

**Atributos:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `placeholder` (opcional): String
- `value` (opcional): String
- `type` (opcional): `"text"` | `"password"` | `"number"`
- `maxLength` (opcional): Número inteiro positivo
- `disabled` (opcional): Boolean string
- `style` / `class` (opcionais)

**Validação:**
- `name` não pode ser vazio

#### Checkbox

Caixa de seleção.

**Atributos:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `label` (opcional): String
- `checked` (opcional): Boolean string
- `disabled` (opcional): Boolean string
- `style` / `class` (opcionais)

#### Radio

Botão de rádio.

**Atributos:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `value` (obrigatório): String não-vazia
- `label` (opcional): String
- `checked` (opcional): Boolean string
- `disabled` (opcional): Boolean string
- `style` / `class` (opcionais)

**Validação:**
- `name` e `value` não podem ser vazios

#### Select

Menu dropdown.

**Atributos:**
- `id` (opcional): String única
- `name` (obrigatório): String não-vazia
- `value` (opcional): String
- `disabled` (opcional): Boolean string
- `style` / `class` (opcionais)
- Filhos: elementos `<option label="..." value="..." />` (pelo menos um)

**Validação:**
- `name` não pode ser vazio
- Deve ter pelo menos um filho `<option>`

### Categoria: Display

#### ProgressBar

Barra de progresso.

**Atributos:**
- `id` (opcional): String única
- `value` (obrigatório): Número
- `max` (opcional): Número positivo (default: 100)
- `variant` (opcional): `"default"` | `"success"` | `"warning"` | `"danger"`
- `showLabel` (opcional): Boolean string
- `style` / `class` (opcionais)

**Validação:**
- `value` deve estar entre 0 e `max`
- `max` deve ser > 0 se fornecido

#### Badge

Selo ou etiqueta.

**Atributos:**
- `id` (opcional): String única
- `text` (obrigatório): String não-vazia
- `variant` (opcional): `"default"` | `"primary"` | `"success"` | `"warning"` | `"danger"`
- `style` / `class` (opcionais)

**Validação:**
- `text` não pode ser vazio

#### Divider

Linha divisória.

**Atributos:**
- `id` (opcional): String única
- `orientation` (opcional): `"horizontal"` | `"vertical"`
- `style` / `class` (opcionais)

#### Spacer

Espaço vazio.

**Atributos:**
- `size` (obrigatório): Número ou `"auto"`

### Categoria: Documento e código

#### Code

Código inline ou em bloco, com highlight de sintaxe opcional. O conteúdo pode ser passado via atributo `text` ou como conteúdo do elemento (suporta CDATA para conteúdo multilinha).

**Atributos:**
- `id` (opcional): String única
- `text` (opcional se conteúdo presente): String (conteúdo do código)
- `language` (opcional): String (ex.: `lua`, `python`)
- `block` (opcional): Boolean string (default: false) — se true, renderiza como `<pre><code>`
- `style` / `class` (opcionais)

**Exemplos:**
```xml
<Code text="local x = 1" language="lua" />

<Code language="lua" block="true"><![CDATA[
function hello()
  return "world"
end
]]></Code>
```

#### Markdown

Renderiza conteúdo em markdown como HTML. O conteúdo é parseado e sanitizado.

**Atributos:**
- `id` (opcional): String única
- `content` (obrigatório): String (fonte markdown)
- `style` / `class` (opcionais)

#### List e ListItem

Lista ordenada ou não ordenada.

**List — Atributos:**
- `id` (opcional): String única
- `ordered` (opcional): Boolean string (default: false)
- `style` / `class` (opcionais)
- Filhos: elementos ListItem

**ListItem — Atributos:**
- `id` (opcional): String única
- `style` / `class` (opcionais)
- Filhos: qualquer Component

#### Heading

Título semântico (h1, h2, h3).

**Atributos:**
- `id` (opcional): String única
- `level` (obrigatório): `"1"` | `"2"` | `"3"`
- `text` (obrigatório): String não-vazia
- `style` / `class` (opcionais)

#### Table

Tabela de dados com linha de cabeçalho e linhas de corpo.

**Atributos:**
- `id` (opcional): String única
- `headers` (obrigatório): String com cabeçalhos separados por vírgula (ex.: `"Name,Score"`)
- `rows` (obrigatório): String com linhas separadas por `|`, células por vírgula (ex.: `"Alice,100|Bob,85"`)
- `style` / `class` (opcionais)

#### Blockquote

Bloco de citação.

**Atributos:**
- `id` (opcional): String única
- `style` / `class` (opcionais)
- Filhos: qualquer Component

#### Pre

Texto pré-formatado (sem highlight de sintaxe).

**Atributos:**
- `id` (opcional): String única
- `text` (obrigatório): String
- `style` / `class` (opcionais)

#### Details

Seção colapsável com resumo.

**Atributos:**
- `id` (opcional): String única
- `summary` (obrigatório): String (rótulo do toggle)
- `open` (opcional): Boolean string (default: false)
- `style` / `class` (opcionais)
- Filhos: qualquer Component (corpo expansível)

## Sistema de Estilos

### Atributo `style`

O atributo `style` aceita pares `chave:valor` separados por `;`:

```xml
<Text text="Hello" style="fontSize:24; fontWeight:bold; color:#00ff00; backgroundColor:#111" />
```

### Atributo `class`

O atributo `class` aceita nomes de classes CSS separados por espaço (ex.: Tailwind):

```xml
<Container class="p-4 bg-gray-900 rounded-lg">
  <Text text="Content" class="text-white font-mono" />
</Container>
```

**Validação:** Se presente, `class` deve conter apenas caracteres seguros (alfanuméricos, espaços, `-`, `_`, `:`, `/`, `.`) para evitar injeção em atributos.

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
  fontFamily?: "sans" | "serif" | "monospace" | "game" | string  // string = family declarada no head
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
- `fontFamily` string customizada: Deve corresponder a um `family` declarado no `head`

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
```xml
<Text text="Hello" style="color:$theme.colors.primary; fontSize:$theme.typography.heading" />
```

**Validação:**
- Referência deve começar com `$theme.`
- Deve ter formato `$theme.<category>.<key>`
- Categoria deve existir no tema
- Key deve existir na categoria

## Sistema de Scripting Lua

Scripts Lua permitem adicionar comportamento dinâmico às páginas NTML. Eles são declarados no `head` como elementos `<script src="..." />` e ficam em arquivos `.lua` separados.

### Modelo de Execução

Scripts Lua rodam em uma **sandbox isolada** sem acesso ao sistema de arquivos ou APIs do sistema operacional. A comunicação com a UI é feita pela API `ui`; a comunicação com o backend é feita pela API `http`.

Scripts são carregados na ordem declarada no `head`. Funções definidas em qualquer script ficam disponíveis globalmente para os `action` dos botões e eventos do runtime.

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

Quando um `Button` tem `action="nomeDaFuncao"` (sem `:`), o runtime chama a função Lua de mesmo nome ao clicar:

```lua
-- Chamado por: <Button action="fazerLogin">
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

-- Chamado por: <Button action="limparFormulario">
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

Um arquivo de componente tem uma seção `<props>` (opcional) seguida de `<body>`:

```xml
<!-- components/nav-bar.ntml -->
<props>
  <prop name="title" type="string" default="Navigation" />
  <prop name="accentColor" type="color" default="#00ff00" />
  <prop name="showBack" type="boolean" default="false" />
</props>
<body>
  <Row align="center" gap="12" style="padding:12; backgroundColor:#111111">
    <Text text="{props.title}" style="fontSize:18; fontWeight:bold; color:{props.accentColor}" />
  </Row>
</body>
```

O nome do componente vem do alias definido no `import` (o arquivo em si não declara um nome).

### Tipos de Props

| Tipo | Descrição | Exemplo de `default` |
|------|-----------|----------------------|
| `string` | Texto | `default="Hello"` |
| `number` | Numérico | `default="0"` |
| `boolean` | Booleano | `default="false"` |
| `color` | Cor (hex ou named) | `default="#ffffff"` |

### Interpolação de Props

Use `{props.nome}` como valor de qualquer atributo dentro do `<body>` do componente:

```xml
<Text text="{props.title}" style="color:{props.accentColor}; fontSize:{props.size}" />
```

**Regras de interpolação:**
- `{props.nome}` deve ser o valor inteiro do atributo (não é possível concatenar: `"Olá {props.nome}"` não é válido — use Lua para isso)
- O tipo do valor interpolado deve ser compatível com o tipo esperado pela propriedade

### Uso de Componente Importado

```xml
<head>
  <title>Dashboard</title>
  <import src="components/nav-bar.ntml" as="NavBar" />
</head>
<body>
  <Column>
    <NavBar title="Meu App" accentColor="#ff6600" showBack="false" />
    <Text text="Conteúdo aqui" />
  </Column>
</body>
```

### Validação de Componentes

**Arquivo de componente:**
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

1. **Parse XML**: Valida sintaxe XML
2. **Parse Head** (se presente): Valida estrutura e elementos do `head`
3. **Parse Component**: Valida estrutura de componentes
4. **Type Validation**: Valida tipos de propriedades
5. **Semantic Validation**: Valida regras semânticas (ranges, não-vazio, etc)
6. **Theme Resolution**: Resolve referências de tema (se aplicável)
7. **Prop Resolution**: Resolve e valida interpolações `{props.*}` em componentes importados

### Erros

Todos os erros incluem informações contextuais:

```rust
pub enum NtmlError {
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

    // Erros do head
    MissingBody,                                // head presente mas body ausente
    MissingTitle,                               // head presente mas title ausente
    InvalidFontFamily { family: String },        // nome de família vazio
    ScriptLimitExceeded { max: usize },         // mais de 5 scripts
    ImportLimitExceeded { max: usize },         // mais de 10 imports
    FontLimitExceeded { max: usize },           // mais de 10 fonts
    InvalidImportAlias { alias: String },       // alias não é PascalCase ou conflita com built-in
    InvalidTag { tag: String },                 // tag com uppercase ou vazia
    TagLimitExceeded { max: usize },            // mais de 10 tags

    // Erros de componentes importáveis
    UnknownImportedComponent { name: String },  // usado mas não importado via head
    MissingRequiredProp { component: String, prop: String },
    InvalidPropType { component: String, prop: String, expected: String },
    UnknownProp { component: String, prop: String },
    CircularComponentImport { path: String },
    ComponentFileHasHead { path: String },

    // Erros de Lua
    LuaScriptTooLong { src: String, max_lines: usize },
    LuaHandlerTimeout { handler: String, timeout_ms: u64 },
    LuaRuntimeError { src: String, message: String },
    LuaFunctionNotFound { action: String },
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

#### v0.3.0
- **Alterado**: Sintaxe migrada de YAML para XML/HTML-like
- Componentes são elementos XML (ex.: `<Text text="Hello" />`)
- Propriedades são atributos XML (ex.: `text="Hello"`, `style="..."`)
- Filhos são elementos filhos aninhados (sem chave `children:`)
- `style.classes` substituído pelo atributo `class`
- Estilos inline via `style="key:val; key2:val2"`
- Seção `head` usa elementos filhos: `<title>`, `<font />`, `<script />`, `<import />`
- Arquivos de componente usam `<props>` / `<body>` em vez de YAML

#### v0.2.0
- **Novo**: Seção `head` com suporte a metadados, fonts, scripts e imports
- **Novo**: Sistema de scripting Lua com sandbox
- **Novo**: Componentes importáveis com sistema de props e interpolação
- **Novo**: Propriedade `id` em todos os componentes para integração com Lua

#### v0.1.0
- Versão inicial — componentes, estilos, temas e validação

Versão atual: **0.3.0**

## Conformidade

Implementações NTML devem:

1. ✅ Validar todos os documentos segundo esta especificação
2. ✅ Rejeitar documentos inválidos com mensagens de erro claras
3. ✅ Implementar todas as regras de validação
4. ✅ Suportar todos os componentes e propriedades definidos
5. ✅ Seguir as regras de segurança

## Exemplos Completos

### UI de Status do Sistema (formato clássico)

```xml
<Container style="padding:24; backgroundColor:#0a0a0a; borderRadius:8; borderWidth:2; borderColor:#00ff00">
  <Column gap="16">
    <Text text="SYSTEM STATUS" style="fontSize:24; fontWeight:bold; color:#00ff00; fontFamily:monospace" />

    <Row gap="12" align="center">
      <Icon name="cpu" size="20" />
      <ProgressBar value="85" variant="success" style="flex:1" />
    </Row>

    <Grid columns="2" gap="8">
      <Badge text="ONLINE" variant="success" />
      <Badge text="LVL 42" variant="primary" />
    </Grid>
  </Column>
</Container>
```

### Formulário de Login com Lua (formato completo)

```xml
<head>
  <title>Login Seguro</title>
  <script src="scripts/login.lua" />
</head>
<body>
  <Column gap="16" style="padding:32; maxWidth:400">
    <Text text="SECURE LOGIN" style="fontSize:28; fontWeight:bold; textAlign:center" />

    <Input name="username" placeholder="Username" type="text" />

    <Input name="password" placeholder="Password" type="password" />

    <Text id="feedback" text="" style="color:red; display:none" />

    <Button id="btn-login" action="fazerLogin" variant="primary">
      <Text text="LOGIN" style="fontWeight:bold" />
    </Button>
  </Column>
</body>
```

### Página com Componente Importado (formato completo)

```xml
<!-- dashboard.ntml -->
<head>
  <title>Dashboard</title>
  <font family="Roboto Mono" weights="400,700" />
  <import src="components/nav-bar.ntml" as="NavBar" />
  <import src="components/stat-card.ntml" as="StatCard" />
</head>
<body>
  <Column>
    <NavBar title="Dashboard" accentColor="#00ff00" />
    <Grid columns="3" gap="16" style="padding:24">
      <StatCard label="CPU" value="85" unit="%" />
      <StatCard label="RAM" value="4096" unit="MB" />
      <StatCard label="Ping" value="12" unit="ms" />
    </Grid>
  </Column>
</body>
```

### Arquivo de Componente com Props

```xml
<!-- components/stat-card.ntml -->
<props>
  <prop name="label" type="string" />
  <prop name="value" type="number" />
  <prop name="unit" type="string" default="" />
  <prop name="variant" type="string" default="default" />
</props>
<body>
  <Container style="padding:16; borderRadius:8; borderWidth:1; borderColor:#333333; backgroundColor:#111111">
    <Text text="{props.label}" style="fontSize:12; color:gray; textTransform:uppercase" />
    <Row align="center" gap="4">
      <Text text="{props.value}" style="fontSize:32; fontWeight:bold; color:#00ff00" />
      <Text text="{props.unit}" style="fontSize:14; color:gray" />
    </Row>
  </Container>
</body>
```

## Referências

- [XML 1.0 Specification](https://www.w3.org/TR/xml/)
- [CSS Box Model](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Box_Model)
- [Flexbox Guide](https://css-tricks.com/snippets/css/a-guide-to-flexbox/)
- [Lua 5.4 Reference Manual](https://www.lua.org/manual/5.4/)
