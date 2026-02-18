# NTML Integration Guide

How to integrate NTML into the NullTrace project.

## Overview

The `nulltrace-ntml` crate is designed to be shared between:
- **nulltrace-core** (Rust backend)
- **nulltrace-client** (Tauri/React frontend)

Both can use the same library for consistent parsing and validation.

## 🔧 Backend Integration (nulltrace-core)

### 1. Add Dependency

In `nulltrace-core/Cargo.toml`:

```toml
[dependencies]
nulltrace-ntml = { path = "../nulltrace-ntml" }
```

### 2. Parse Player UIs

```rust
use nulltrace_ntml::{parse_ntml, NtmlError, Component};

pub struct PlayerProgram {
    pub id: String,
    pub owner_id: String,
    pub ui_yaml: String,
    pub ui_component: Option<Component>,
}

impl PlayerProgram {
    pub fn validate_ui(&mut self) -> Result<(), NtmlError> {
        let component = parse_ntml(&self.ui_yaml)?;
        self.ui_component = Some(component);
        Ok(())
    }
}

// In gRPC handler
pub async fn upload_program_ui(
    &self,
    request: Request<UploadUiRequest>,
) -> Result<Response<UploadUiResponse>, Status> {
    let ui_yaml = request.get_ref().yaml_content.clone();

    // Validate NTML
    match parse_ntml(&ui_yaml) {
        Ok(component) => {
            // Save to database
            self.db.save_program_ui(program_id, ui_yaml).await?;

            Ok(Response::new(UploadUiResponse {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            Ok(Response::new(UploadUiResponse {
                success: false,
                error: Some(format!("{}", e)),
            }))
        }
    }
}
```

### 3. Validate Assets

```rust
use nulltrace_ntml::Component;

const ALLOWED_ASSETS: &[&str] = &[
    "player-avatar.png",
    "terminal-bg.png",
    "icon-hack.png",
    "icon-shield.png",
    // ... more assets
];

fn validate_component_assets(component: &Component) -> Result<(), String> {
    match component {
        Component::Image(img) => {
            if !ALLOWED_ASSETS.contains(&img.src.as_str()) {
                return Err(format!("Asset not whitelisted: {}", img.src));
            }
        }
        Component::Container(c) => {
            if let Some(children) = &c.children {
                for child in children {
                    validate_component_assets(child)?;
                }
            }
        }
        // ... handle other components with children
        _ => {}
    }
    Ok(())
}
```

### 4. Store in Database

```sql
CREATE TABLE program_uis (
    id UUID PRIMARY KEY,
    program_id UUID NOT NULL REFERENCES programs(id),
    yaml_content TEXT NOT NULL,
    validated_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
);
```

```rust
pub async fn save_program_ui(
    &self,
    program_id: Uuid,
    yaml: String,
) -> Result<(), Error> {
    // Validate first
    parse_ntml(&yaml)?;

    sqlx::query!(
        r#"
        INSERT INTO program_uis (id, program_id, yaml_content, validated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (program_id) DO UPDATE
        SET yaml_content = $3, validated_at = NOW()
        "#,
        Uuid::new_v4(),
        program_id,
        yaml,
    )
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

## 🎨 Frontend Integration (nulltrace-client)

### 1. Create Tauri Command

In `src-tauri/src/lib.rs`:

```rust
use nulltrace_ntml::{parse_ntml, Component, NtmlError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NtmlValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub component: Option<Component>,
}

#[tauri::command]
pub fn validate_ntml(yaml: String) -> NtmlValidationResult {
    match parse_ntml(&yaml) {
        Ok(component) => NtmlValidationResult {
            valid: true,
            error: None,
            component: Some(component),
        },
        Err(e) => NtmlValidationResult {
            valid: false,
            error: Some(format!("{}", e)),
            component: None,
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            validate_ntml,
            // ... other commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Use in React

Create a hook for NTML validation:

```typescript
// hooks/useNtmlValidator.ts
import { invoke } from '@tauri-apps/api/tauri';

interface NtmlValidationResult {
  valid: boolean;
  error?: string;
  component?: any;
}

export function useNtmlValidator() {
  const [isValidating, setIsValidating] = useState(false);
  const [result, setResult] = useState<NtmlValidationResult | null>(null);

  const validate = async (yaml: string) => {
    setIsValidating(true);
    try {
      const result = await invoke<NtmlValidationResult>('validate_ntml', { yaml });
      setResult(result);
      return result;
    } catch (error) {
      console.error('Validation error:', error);
      return { valid: false, error: String(error) };
    } finally {
      setIsValidating(false);
    }
  };

  return { validate, isValidating, result };
}
```

### 3. Create UI Editor Component

```tsx
// components/NtmlEditor.tsx
import { useState } from 'react';
import { useNtmlValidator } from '../hooks/useNtmlValidator';

export function NtmlEditor() {
  const [yaml, setYaml] = useState(`Container:
  style:
    padding: 16
  children:
    - Text:
        text: "Hello World"
`);
  const { validate, isValidating, result } = useNtmlValidator();

  const handleValidate = async () => {
    await validate(yaml);
  };

  return (
    <div className="ntml-editor">
      <div className="editor-pane">
        <h3>NTML Editor</h3>
        <textarea
          value={yaml}
          onChange={(e) => setYaml(e.target.value)}
          className="yaml-input"
          rows={20}
          cols={60}
        />
        <button onClick={handleValidate} disabled={isValidating}>
          {isValidating ? 'Validating...' : 'Validate'}
        </button>
      </div>

      <div className="preview-pane">
        <h3>Validation Result</h3>
        {result && (
          <div className={result.valid ? 'valid' : 'invalid'}>
            {result.valid ? (
              <>
                <p>✓ Valid NTML!</p>
                <pre>{JSON.stringify(result.component, null, 2)}</pre>
              </>
            ) : (
              <>
                <p>✗ Invalid NTML</p>
                <pre className="error">{result.error}</pre>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
```

### 4. Upload to Server

```typescript
// services/programService.ts
import { invoke } from '@tauri-apps/api/tauri';

export async function uploadProgramUI(programId: string, yaml: string) {
  // Validate locally first
  const validation = await invoke('validate_ntml', { yaml });

  if (!validation.valid) {
    throw new Error(validation.error);
  }

  // Send to server via gRPC
  const response = await grpcClient.uploadProgramUI({
    programId,
    yamlContent: yaml,
  });

  return response;
}
```

## 🎮 Runtime Rendering

### Browser NTML → HTML Renderer (nulltrace-client)

The **Browser** app in nulltrace-client converts NTML to safe HTML for display in an iframe. The renderer (`ntml_html.rs`) supports:

- **All NTML components** (Container, Flex, Row, Column, Grid, Stack, Text, Image, Icon, Button, Input, Checkbox, Radio, Select, ProgressBar, Badge, Divider, Spacer)
- **Full style support** (dimensions, spacing, typography, borders, shadow, position, flex, display, overflow, cursor)
- **Layout props** (Flex/Row/Column justify, align, wrap; Grid columns/rows; Stack alignment; Divider orientation; Spacer size: "auto")
- **Image resolution** via `base_url` (paths resolved against page URL)
- **Image fit** (object-fit: cover, contain, fill, none, scale-down)
- **Lua scripts** (head.scripts) and **patches** (ui.set_text, etc.)

The Tauri command `ntml_to_html` accepts `yaml`, `imports`, and optional `base_url`. The `ntml_runtime` uses `ntml_to_html_with_imports_and_patches` for re-renders after Lua handlers. See [docs/ui-markup-language.md](../docs/ui-markup-language.md) for the full spec.

### Convert NTML to React Components (Alternative)

```typescript
// renderers/NtmlRenderer.tsx
import React from 'react';

interface NtmlComponent {
  Container?: { style?: any; children?: NtmlComponent[] };
  Text?: { text: string; style?: any };
  Button?: { action: string; children?: NtmlComponent[]; style?: any };
  // ... other components
}

export function NtmlRenderer({ component }: { component: NtmlComponent }) {
  if (component.Container) {
    return (
      <div style={convertStyle(component.Container.style)}>
        {component.Container.children?.map((child, i) => (
          <NtmlRenderer key={i} component={child} />
        ))}
      </div>
    );
  }

  if (component.Text) {
    return (
      <span style={convertStyle(component.Text.style)}>
        {component.Text.text}
      </span>
    );
  }

  if (component.Button) {
    return (
      <button
        onClick={() => handleAction(component.Button.action)}
        style={convertStyle(component.Button.style)}
      >
        {component.Button.children?.map((child, i) => (
          <NtmlRenderer key={i} component={child} />
        ))}
      </button>
    );
  }

  // ... handle other components

  return null;
}

function convertStyle(style?: any) {
  if (!style) return {};

  return {
    padding: style.padding,
    backgroundColor: style.backgroundColor,
    color: style.color,
    fontSize: style.fontSize,
    // ... convert other styles
  };
}

function handleAction(action: string) {
  console.log('Action triggered:', action);
  // Dispatch to game logic
}
```

## 🔒 Security Considerations

### 1. Always Validate Server-Side

```rust
// Never trust client validation
pub async fn upload_ui(yaml: String) -> Result<()> {
    // ALWAYS validate on server
    let component = parse_ntml(&yaml)?;

    // Validate assets
    validate_assets(&component)?;

    // Save
    Ok(())
}
```

### 2. Asset Whitelisting

```rust
// Only allow pre-approved assets
fn validate_image_src(src: &str) -> bool {
    ALLOWED_IMAGES.contains(&src)
}
```

### 3. Action Validation

```rust
// Validate actions at runtime
fn validate_action(action: &str) -> Result<()> {
    match action {
        "hack_system" | "scan_target" | "send_message" => Ok(()),
        _ => Err(Error::InvalidAction(action.to_string()))
    }
}
```

### 4. Rate Limiting

```rust
// Limit UI upload frequency
pub async fn can_upload_ui(player_id: &str) -> bool {
    let last_upload = get_last_upload_time(player_id).await;
    last_upload.elapsed() > Duration::from_secs(60) // 1 min cooldown
}
```

## 📊 Database Schema

```sql
-- Programs table
CREATE TABLE programs (
    id UUID PRIMARY KEY,
    player_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    code TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Program UIs table
CREATE TABLE program_uis (
    id UUID PRIMARY KEY,
    program_id UUID NOT NULL UNIQUE,
    yaml_content TEXT NOT NULL,
    validated_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
);

-- UI upload history (for audit)
CREATE TABLE ui_upload_history (
    id UUID PRIMARY KEY,
    program_id UUID NOT NULL,
    yaml_content TEXT NOT NULL,
    validation_result BOOLEAN NOT NULL,
    error_message TEXT,
    uploaded_at TIMESTAMP NOT NULL DEFAULT NOW(),
    FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
);
```

## 🧪 Testing

### Backend Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_ui() {
        let yaml = r#"
        Text:
          text: "Hello"
        "#;

        assert!(parse_ntml(yaml).is_ok());
    }

    #[test]
    fn test_reject_invalid_ui() {
        let yaml = r#"
        InvalidComponent:
          text: "Hello"
        "#;

        assert!(parse_ntml(yaml).is_err());
    }
}
```

### Frontend Tests

```typescript
// tests/ntml.test.ts
import { invoke } from '@tauri-apps/api/tauri';

describe('NTML Validation', () => {
  it('validates correct NTML', async () => {
    const yaml = `
Text:
  text: "Hello"
    `;

    const result = await invoke('validate_ntml', { yaml });
    expect(result.valid).toBe(true);
  });

  it('rejects invalid NTML', async () => {
    const yaml = `
InvalidComponent:
  text: "Hello"
    `;

    const result = await invoke('validate_ntml', { yaml });
    expect(result.valid).toBe(false);
    expect(result.error).toBeTruthy();
  });
});
```

## 🚀 Deployment

### 1. Build Release

```bash
cd nulltrace-ntml
cargo build --release
```

### 2. Update Dependencies

```bash
# In nulltrace-core
cargo update nulltrace-ntml

# In nulltrace-client
cd src-tauri
cargo update nulltrace-ntml
```

### 3. Version Management

Use semantic versioning in `Cargo.toml`:

```toml
[package]
name = "nulltrace-ntml"
version = "0.1.0"  # Increment on changes
```

## 📝 Example Workflow

1. **Player creates UI in editor**
   - Types NTML in editor
   - Real-time validation feedback

2. **Validate locally**
   - Tauri command validates NTML
   - Shows errors immediately

3. **Upload to server**
   - Send validated NTML via gRPC
   - Server re-validates (security)

4. **Store in database**
   - Save YAML and validation timestamp
   - Keep upload history

5. **Render in game**
   - Fetch NTML from server
   - Parse to components
   - Render with React

## 🔗 Complete Example

See `examples/` directory for:
- `game-hud.ntml` - Complete game HUD
- `terminal-ui.ntml` - Terminal interface
- `mission-briefing.ntml` - Mission screen

---

**Next Steps:**
1. Add `nulltrace-ntml` dependency to backend
2. Create Tauri command for validation
3. Implement UI editor in frontend
4. Add gRPC endpoint for UI upload
5. Create database schema
6. Implement renderer
