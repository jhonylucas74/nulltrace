#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

/// Max size for shortcuts_overrides JSON (bytes) to avoid abuse.
const MAX_SHORTCUTS_JSON_BYTES: usize = 4096;

/// Valid shortcut action IDs (must match client ShortcutActionId).
const VALID_ACTION_IDS: &[&str] = &[
    "appLauncher",
    "toggleGrid",
    "nextWorkspace",
    "prevWorkspace",
    "nextWorkspaceAlt",
    "prevWorkspaceAlt",
    "goToWorkspace1",
    "goToWorkspace2",
    "goToWorkspace3",
    "goToWorkspace4",
    "goToWorkspace5",
    "goToWorkspace6",
    "goToWorkspace7",
    "goToWorkspace8",
    "goToWorkspace9",
];

pub struct ShortcutsService {
    pool: PgPool,
}

impl ShortcutsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get shortcuts overrides for a player. Returns JSON string (e.g. "{}" or "{\"appLauncher\":[\"Alt\",\" \"]}").
    pub async fn get_shortcuts(&self, player_id: Uuid) -> Result<String, sqlx::Error> {
        let row = sqlx::query_scalar::<_, Option<String>>(
            r#"SELECT overrides::text FROM player_shortcuts WHERE player_id = $1"#,
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.flatten().unwrap_or_else(|| "{}".to_string()))
    }

    /// Set shortcuts overrides. Validates that json is an object with only valid action IDs and array-of-string values.
    pub async fn set_shortcuts(&self, player_id: Uuid, overrides_json: &str) -> Result<(), sqlx::Error> {
        if overrides_json.len() > MAX_SHORTCUTS_JSON_BYTES {
            return Err(sqlx::Error::Protocol("shortcuts_overrides too large".into()));
        }
        let value: serde_json::Value =
            serde_json::from_str(overrides_json).map_err(|e| sqlx::Error::Protocol(format!("invalid JSON: {}", e).into()))?;
        let obj = value
            .as_object()
            .ok_or_else(|| sqlx::Error::Protocol("shortcuts_overrides must be a JSON object".into()))?;
        for (key, val) in obj {
            if !VALID_ACTION_IDS.contains(&key.as_str()) {
                return Err(sqlx::Error::Protocol(format!("invalid shortcut action id: {}", key).into()));
            }
            let arr = val
                .as_array()
                .ok_or_else(|| sqlx::Error::Protocol(format!("shortcut value for {} must be array of strings", key).into()))?;
            for v in arr {
                if !v.is_string() {
                    return Err(sqlx::Error::Protocol(format!("shortcut keys for {} must be strings", key).into()));
                }
            }
        }
        sqlx::query(
            r#"
            INSERT INTO player_shortcuts (player_id, overrides, updated_at)
            VALUES ($1, $2::jsonb, now())
            ON CONFLICT (player_id) DO UPDATE SET overrides = $2::jsonb, updated_at = now()
            "#,
        )
        .bind(player_id)
        .bind(overrides_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
