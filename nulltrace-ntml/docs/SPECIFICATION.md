# NTML Specification v0.1.0

## Introdução

**NullTrace Markup Language (NTML)** é uma linguagem declarativa baseada em YAML para criar interfaces de usuário no jogo NullTrace. Esta especificação define a sintaxe, semântica e regras de validação do NTML.

## Objetivos de Design

1. **Segurança**: Prevenir injeções e exploits através de validação rigorosa
2. **Simplicidade**: Sintaxe YAML legível e fácil de aprender
3. **Expressividade**: Rico conjunto de componentes e estilos
4. **Type-Safety**: Validação estrita de tipos e propriedades
5. **Consistência**: Comportamento previsível e determinístico

## Estrutura do Documento

### Formato

Todo documento NTML é um arquivo YAML válido que define **exatamente um** componente raiz.

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

### Regras Globais

1. **Um único componente raiz**: Documentos com zero ou múltiplos componentes raiz são inválidos
2. **Profundidade máxima**: Máximo de 20 níveis de aninhamento
3. **Case sensitivity**: Nomes de componentes e propriedades são case-sensitive
4. **Naming convention**:
   - Componentes: PascalCase (e.g., `Container`, `ProgressBar`)
   - Propriedades: camelCase (e.g., `backgroundColor`, `fontSize`)

## Componentes

### Categoria: Layout

#### Container

Contêiner básico para agrupar elementos.

**Propriedades:**
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
- `alignment` (opcional): `"topLeft"` | `"topCenter"` | `"topRight"` | `"centerLeft"` | `"center"` | `"centerRight"` | `"bottomLeft"` | `"bottomCenter"` | `"bottomRight"`
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

#### Row

Atalho para Flex com `direction: row`.

**Propriedades:**
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
- `text` (obrigatório): String não-vazia
- `style` (opcional): Objeto Style

**Validação:**
- `text` não pode ser vazio

#### Image

Exibe imagem.

**Propriedades:**
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
- `action` (obrigatório): String não-vazia
- `variant` (opcional): `"primary"` | `"secondary"` | `"danger"` | `"ghost"`
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style
- `children` (opcional): Array de Components

**Validação:**
- `action` não pode ser vazio

#### Input

Campo de entrada de texto.

**Propriedades:**
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
- `name` (obrigatório): String não-vazia
- `label` (opcional): String
- `checked` (opcional): Boolean
- `disabled` (opcional): Boolean
- `style` (opcional): Objeto Style

#### Radio

Botão de rádio.

**Propriedades:**
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
- `text` (obrigatório): String não-vazia
- `variant` (opcional): `"default"` | `"primary"` | `"success"` | `"warning"` | `"danger"`
- `style` (opcional): Objeto Style

**Validação:**
- `text` não pode ser vazio

#### Divider

Linha divisória.

**Propriedades:**
- `orientation` (opcional): `"horizontal"` | `"vertical"`
- `style` (opcional): Objeto Style

#### Spacer

Espaço vazio.

**Propriedades:**
- `size` (obrigatório): Número ou `"auto"`

## Sistema de Estilos

### Objeto Style

Todas as propriedades de estilo são opcionais.

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
  fontFamily?: "sans" | "serif" | "monospace" | "game"
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

## Validação

### Fases de Validação

1. **Parse YAML**: Valida sintaxe YAML
2. **Parse Component**: Valida estrutura de componentes
3. **Type Validation**: Valida tipos de propriedades
4. **Semantic Validation**: Valida regras semânticas (ranges, não-vazio, etc)
5. **Theme Resolution**: Resolve referências de tema (se aplicável)

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
}
```

## Segurança

### Prevenção de Exploits

1. **Profundidade de Aninhamento**: Máximo 20 níveis previne stack overflow
2. **Validação de Tipos**: Previne type confusion attacks
3. **Whitelist de Assets**: Apenas imagens/icons aprovados
4. **Validação de Actions**: Actions devem ser validadas no runtime
5. **Ranges de Valores**: Previne valores absurdos (opacity > 1, etc)
6. **No Code Execution**: NTML é puramente declarativo

### Sandboxing

- Componentes não têm acesso ao sistema de arquivos
- Não há execução de código arbitrário
- Actions são strings validadas pelo servidor
- Imagens devem estar em whitelist

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

Versão atual: **0.1.0** (Alpha)

## Conformidade

Implementações NTML devem:

1. ✅ Validar todos os documentos segundo esta especificação
2. ✅ Rejeitar documentos inválidos com mensagens de erro claras
3. ✅ Implementar todas as regras de validação
4. ✅ Suportar todos os componentes e propriedades definidos
5. ✅ Seguir as regras de segurança

## Exemplos Completos

### UI de Status do Jogador

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

### Formulário de Login

```yaml
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

    - Checkbox:
        name: "remember"
        label: "Remember me"

    - Button:
        action: "login"
        variant: primary
        children:
          - Text:
              text: "LOGIN"
              style:
                fontWeight: bold
```

## Referências

- [YAML 1.2 Specification](https://yaml.org/spec/1.2/spec.html)
- [CSS Box Model](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Box_Model)
- [Flexbox Guide](https://css-tricks.com/snippets/css/a-guide-to-flexbox/)
