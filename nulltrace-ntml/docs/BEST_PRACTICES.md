# NTML Best Practices

Guia de boas práticas para escrever NTML eficiente, seguro e manutenível.

## 📋 Estrutura de Arquivos

### Organização

```
ui/
├── components/
│   ├── hud.ntml           # HUD do jogo
│   ├── terminal.ntml      # Interface do terminal
│   └── menus/
│       ├── main-menu.ntml
│       └── settings.ntml
├── themes/
│   ├── dark.yaml
│   └── light.yaml
└── shared/
    ├── buttons.ntml
    └── dialogs.ntml
```

### Convenções de Nomenclatura

- **Arquivos**: Use kebab-case: `main-menu.ntml`, `player-stats.ntml`
- **Actions**: Use snake_case: `hack_system`, `send_message`
- **Nomes de inputs**: Use snake_case: `username_input`, `password_field`

## 🎨 Estilos

### Use Variáveis de Tema

❌ **Ruim:**
```yaml
Text:
  text: "Hello"
  style:
    color: "#4a90e2"  # Cor hardcoded
```

✅ **Bom:**
```yaml
Text:
  text: "Hello"
  style:
    color: "$theme.colors.primary"  # Reutilizável
```

### Agrupe Estilos Relacionados

❌ **Ruim:**
```yaml
style:
  paddingTop: 8
  paddingBottom: 8
  paddingLeft: 16
  paddingRight: 16
```

✅ **Bom:**
```yaml
style:
  paddingVertical: 8
  paddingHorizontal: 16
```

### Evite Valores Mágicos

❌ **Ruim:**
```yaml
style:
  width: 347  # Por que 347?
  padding: 23
```

✅ **Bom:**
```yaml
style:
  width: 400  # Container padrão
  padding: 16  # 1rem de padding
```

## 🏗️ Estrutura de Componentes

### Mantenha Componentes Simples

❌ **Ruim:** Componente monolítico de 500 linhas

✅ **Bom:** Divida em componentes menores e reutilizáveis

```yaml
# player-stats.ntml
Column:
  gap: 12
  children:
    - !include health-bar.ntml
    - !include energy-bar.ntml
    - !include skill-indicators.ntml
```

### Use Layout Apropriado

**Container**: Quando não precisa de layout especial
```yaml
Container:
  style:
    padding: 16
  children: [...]
```

**Flex/Row/Column**: Para layouts flexíveis
```yaml
Row:
  gap: 8
  align: center
  children: [...]
```

**Grid**: Para layouts de grade uniformes
```yaml
Grid:
  columns: 3
  gap: 8
  children: [...]
```

**Stack**: Para overlays e camadas
```yaml
Stack:
  alignment: center
  children:
    - Image: ...  # Background
    - Text: ...   # Overlay
```

### Hierarquia de Componentes

```yaml
Container (root)
└── Column (layout principal)
    ├── Container (header)
    │   └── Row
    │       ├── Icon
    │       └── Text
    ├── Container (content)
    │   └── Grid
    │       ├── Badge
    │       └── Badge
    └── Container (footer)
        └── Row
            ├── Button
            └── Button
```

## 🔒 Segurança

### Validar Actions

Sempre valide actions no servidor:

```rust
fn validate_action(action: &str) -> Result<(), Error> {
    match action {
        "hack_system" | "scan_target" | "send_message" => Ok(()),
        _ => Err(Error::InvalidAction(action.to_string()))
    }
}
```

### Asset Whitelisting

Configure whitelist de assets permitidos:

```rust
const ALLOWED_IMAGES: &[&str] = &[
    "player-avatar.png",
    "background.png",
    "icon-hack.png",
];

fn validate_image_src(src: &str) -> bool {
    ALLOWED_IMAGES.contains(&src)
}
```

### Limitar Profundidade

O parser já limita a 20 níveis, mas evite estruturas muito profundas:

❌ **Ruim:** 15 níveis de aninhamento

✅ **Bom:** Máximo 5-6 níveis na prática

## ⚡ Performance

### Minimize Número de Componentes

❌ **Ruim:**
```yaml
Row:
  children:
    - Container:
        children:
          - Container:
              children:
                - Text:
                    text: "Hello"
```

✅ **Bom:**
```yaml
Text:
  text: "Hello"
```

### Use Spacer ao Invés de Containers Vazios

❌ **Ruim:**
```yaml
- Container:
    style:
      height: 16
```

✅ **Bom:**
```yaml
- Spacer:
    size: 16
```

### Reutilize Componentes

Use sistemas de templates/includes quando disponíveis para evitar duplicação.

## 📝 Legibilidade

### Use Comentários

```yaml
# Player HUD - Main overlay showing player stats
Container:
  style:
    padding: 16
  children:
    # Health and energy bars
    - Column:
        gap: 8
        children:
          - ProgressBar:  # HP bar
              value: 85
              variant: danger

          - ProgressBar:  # Energy bar
              value: 60
              variant: success
```

### Indentação Consistente

Use 2 ou 4 espaços (não tabs) e seja consistente.

✅ **Bom:** 2 espaços em todo arquivo
```yaml
Container:
  style:
    padding: 16
  children:
    - Text:
        text: "Hello"
```

### Ordenação de Propriedades

Ordene propriedades de forma consistente:

1. Propriedades obrigatórias primeiro
2. Layout/comportamento
3. Estilo
4. Children/conteúdo

```yaml
Button:
  action: "hack_system"     # 1. Obrigatório
  variant: primary          # 2. Comportamento
  disabled: false           # 2. Comportamento
  style:                    # 3. Estilo
    padding: 16
  children:                 # 4. Conteúdo
    - Text:
        text: "HACK"
```

## 🎯 Responsividade

### Use Flex e Percentuais

```yaml
Row:
  children:
    - Container:
        style:
          flex: 1  # Ocupa espaço disponível
    - Container:
        style:
          flex: 2  # Ocupa 2x mais espaço
```

### Min/Max Dimensions

```yaml
Container:
  style:
    minWidth: 200
    maxWidth: 600
    width: 400  # Preferencial
```

## 🔄 Manutenibilidade

### Separe Lógica de Apresentação

❌ **Ruim:** Lógica de dados misturada com UI

✅ **Bom:** UI apenas define estrutura, dados vêm do backend

### Versionamento

Adicione versão em comentários:

```yaml
# NTML v0.1.0
# Player Stats HUD
# Last updated: 2024-01-15
Container:
  ...
```

### Teste Regularmente

```bash
# Valide após cada alteração
ntml-validate ui/*.ntml

# Automatize em CI/CD
npm run lint-ntml
```

## 🐛 Debug

### Mensagens de Erro

Leia mensagens de erro cuidadosamente:

```
✗ ui.ntml has errors:
  Invalid color value 'not-a-color':
    must be a valid hex color (e.g., #ff0000) or named color (...)
```

### Validação Incremental

Ao criar UIs complexas, valide frequentemente:

1. Crie estrutura básica
2. ✅ Valide
3. Adicione estilos
4. ✅ Valide
5. Adicione conteúdo
6. ✅ Valide

### Debugging com Simplificação

Se algo não funciona, simplifique:

1. Remova estilos
2. Remova children
3. Valide até encontrar o problema

## 📊 Exemplos de Padrões Comuns

### Modal/Dialog

```yaml
Stack:
  alignment: center
  children:
    # Overlay escuro
    - Container:
        style:
          backgroundColor: "#000000"
          opacity: 0.5

    # Dialog
    - Container:
        style:
          backgroundColor: "#1a1a1a"
          padding: 24
          borderRadius: 8
          maxWidth: 400
        children:
          - Column:
              gap: 16
              children:
                - Text:
                    text: "Confirm Action"
                    style:
                      fontSize: 20
                      fontWeight: bold

                - Row:
                    gap: 8
                    children:
                      - Button:
                          action: "confirm"
                          variant: primary
                      - Button:
                          action: "cancel"
                          variant: secondary
```

### Card List

```yaml
Column:
  gap: 12
  children:
    - Container:  # Card 1
        style:
          padding: 16
          backgroundColor: "#1a1a1a"
          borderRadius: 8
        children: [...]

    - Container:  # Card 2
        style:
          padding: 16
          backgroundColor: "#1a1a1a"
          borderRadius: 8
        children: [...]
```

### Form

```yaml
Column:
  gap: 16
  style:
    padding: 24
  children:
    - Text:
        text: "Login"
        style:
          fontSize: 24
          fontWeight: bold

    - Input:
        name: "username"
        placeholder: "Username"

    - Input:
        name: "password"
        placeholder: "Password"
        type: password

    - Button:
        action: "login"
        variant: primary
        children:
          - Text:
              text: "LOGIN"
```

## 🎓 Recursos

- [NTML Specification](./SPECIFICATION.md)
- [Exemplos](../examples/)
- [README](../README.md)

## ✅ Checklist Final

Antes de usar seu NTML em produção:

- [ ] Validado com `ntml-validate`
- [ ] Sem valores hardcoded (usa tema)
- [ ] Comentários explicativos
- [ ] Indentação consistente
- [ ] Actions validadas no backend
- [ ] Assets na whitelist
- [ ] Testado em diferentes tamanhos
- [ ] Profundidade < 10 níveis
- [ ] Componentes reutilizáveis separados
- [ ] Performance aceitável (< 100 componentes na árvore)
