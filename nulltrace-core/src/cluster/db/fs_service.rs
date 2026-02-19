#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Full stat for a path (used by find and fs.stat Lua API).
#[derive(Debug, Clone, FromRow)]
pub struct FsStat {
    pub node_type: String,
    pub size_bytes: i64,
    pub owner: String,
    pub updated_at: DateTime<Utc>,
}

/// SHA-256 hash of content as hex string (64 chars).
fn hash_content(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, FromRow)]
pub struct FsEntry {
    pub name: String,
    pub node_type: String,
    pub size_bytes: i64,
    pub permissions: String,
    pub owner: String,
}

/// Default README for Documents (seed and restore disk).
const DEFAULT_README_MD: &[u8] = b"# Documents\n\nThis folder contains sample files for testing terminal commands.\n\n- Use `find .` to list files recursively.\n- Use `grep pattern file` to search inside files.\n- Use `cat filename` to print file contents.\n- Use `lua hello.lua` to run the sample script.\n";

/// Default notes file (plain text with a phrase).
const DEFAULT_NOTES_TXT: &[u8] = b"The quick brown fox jumps over the lazy dog.\n\nWelcome to your VM. Edit this file or add new ones.\n";

/// Sample Lua script runnable with: lua hello.lua
const DEFAULT_HELLO_LUA: &[u8] = b"-- Sample script for testing.\nprint(\"Hello from Lua!\")\nprint(\"Current time: \" .. (os.date and os.date() or \"N/A\"))\n";

/// Default todo list (for grep/find tests).
const DEFAULT_TODO_TXT: &[u8] = b"TODO:\n- Try: find Documents -name '*.txt'\n- Try: grep hello Documents\n- Try: cat README.md\n";

/// Sample log file (multi-line, for grep tests).
const DEFAULT_SAMPLE_LOG: &[u8] = b"[INFO] Application started\n[DEBUG] Loading config\n[INFO] Ready\n[WARN] Low disk space\n[ERROR] Connection timeout\n";

pub struct FsService {
    pool: PgPool,
}

impl FsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolve a path like "/home/user/file.txt" walking the tree node by node.
    /// Returns the node UUID if found.
    pub async fn resolve_path(
        &self,
        vm_id: Uuid,
        path: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

        // Find root node
        let root: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fs_nodes WHERE vm_id = $1 AND parent_id IS NULL AND name = '/'",
        )
        .bind(vm_id)
        .fetch_optional(&self.pool)
        .await?;

        let mut current_id = match root {
            Some(id) => id,
            None => return Ok(None),
        };

        if parts.is_empty() {
            return Ok(Some(current_id));
        }

        for part in parts {
            let node: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM fs_nodes WHERE vm_id = $1 AND parent_id = $2 AND name = $3",
            )
            .bind(vm_id)
            .bind(current_id)
            .bind(part)
            .fetch_optional(&self.pool)
            .await?;

            match node {
                Some(id) => current_id = id,
                None => return Ok(None),
            }
        }

        Ok(Some(current_id))
    }

    /// Returns node_type ("file" or "directory") if path exists, None otherwise.
    pub async fn node_type_at(&self, vm_id: Uuid, path: &str) -> Result<Option<String>, sqlx::Error> {
        let node_id = match self.resolve_path(vm_id, path).await? {
            Some(id) => id,
            None => return Ok(None),
        };
        let row: Option<(String,)> = sqlx::query_as("SELECT node_type FROM fs_nodes WHERE id = $1")
            .bind(node_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    /// Returns full stat (type, size, owner, updated_at) for path, or None if not found.
    pub async fn stat_at(&self, vm_id: Uuid, path: &str) -> Result<Option<FsStat>, sqlx::Error> {
        let node_id = match self.resolve_path(vm_id, path).await? {
            Some(id) => id,
            None => return Ok(None),
        };
        let row = sqlx::query_as::<_, FsStat>(
            "SELECT node_type, size_bytes, owner, updated_at FROM fs_nodes WHERE id = $1",
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List entries in a directory.
    pub async fn ls(&self, vm_id: Uuid, path: &str) -> Result<Vec<FsEntry>, sqlx::Error> {
        let dir_id = match self.resolve_path(vm_id, path).await? {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let rows = sqlx::query_as::<_, FsEntry>(
            r#"
            SELECT name, node_type, size_bytes, permissions, owner
            FROM fs_nodes WHERE vm_id = $1 AND parent_id = $2
            ORDER BY node_type DESC, name
            "#,
        )
        .bind(vm_id)
        .bind(dir_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// List directory entries as preformatted lines (name, type, size, owner columns).
    /// Returns a simple Vec<String> so the Lua side only has to print; avoids building a nested table.
    pub async fn ls_formatted(&self, vm_id: Uuid, path: &str) -> Result<Vec<String>, sqlx::Error> {
        let entries = self.ls(vm_id, path).await?;
        if entries.is_empty() {
            return Ok(vec![]);
        }
        const MIN_NAME: usize = 8;
        const MIN_TYPE: usize = 9;
        const MIN_SIZE: usize = 6;
        const MIN_OWNER: usize = 4;
        let mut w_name = MIN_NAME;
        let mut w_type = MIN_TYPE;
        let mut w_size = MIN_SIZE;
        let mut w_owner = MIN_OWNER;
        for e in &entries {
            w_name = w_name.max(e.name.len());
            w_type = w_type.max(e.node_type.len());
            w_size = w_size.max(e.size_bytes.to_string().len());
            w_owner = w_owner.max(e.owner.len());
        }
        fn pad_right(s: &str, w: usize) -> String {
            let n = s.len();
            format!("{}{}", s, " ".repeat(if n >= w { 0 } else { w - n }))
        }
        fn pad_left(s: &str, w: usize) -> String {
            let n = s.len();
            format!("{}{}", " ".repeat(if n >= w { 0 } else { w - n }), s)
        }
        let lines: Vec<String> = entries
            .iter()
            .map(|e| {
                format!(
                    "{}  {}  {}  {}",
                    pad_right(&e.name, w_name),
                    pad_right(&e.node_type, w_type),
                    pad_left(&e.size_bytes.to_string(), w_size),
                    pad_right(&e.owner, w_owner),
                )
            })
            .collect();
        Ok(lines)
    }

    /// Read file content as bytes + mime_type.
    pub async fn read_file(
        &self,
        vm_id: Uuid,
        path: &str,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, sqlx::Error> {
        let node_id = match self.resolve_path(vm_id, path).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let row: Option<(Vec<u8>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT b.data, n.mime_type
            FROM fs_contents c
            JOIN fs_nodes n ON n.id = c.node_id
            JOIN blob_store b ON b.hash = c.content_hash
            WHERE c.node_id = $1
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Write a file. Creates the file if it doesn't exist, updates content if it does.
    /// Parent directories must already exist.
    pub async fn write_file(
        &self,
        vm_id: Uuid,
        path: &str,
        data: &[u8],
        mime_type: Option<&str>,
        owner: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let (parent_path, file_name) = split_path(path);

        let parent_id = match self.resolve_path(vm_id, parent_path).await? {
            Some(id) => id,
            None => return Err(sqlx::Error::RowNotFound),
        };

        // Check if file already exists
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fs_nodes WHERE vm_id = $1 AND parent_id = $2 AND name = $3",
        )
        .bind(vm_id)
        .bind(parent_id)
        .bind(file_name)
        .fetch_optional(&self.pool)
        .await?;

        let size = data.len() as i64;
        let hash = hash_content(data);

        // Ensure blob exists (insert if not already present)
        sqlx::query(
            "INSERT INTO blob_store (hash, data) VALUES ($1, $2) ON CONFLICT (hash) DO NOTHING",
        )
        .bind(&hash)
        .bind(data)
        .execute(&self.pool)
        .await?;

        match existing {
            Some(node_id) => {
                // Update existing file — point to new blob (copy-on-write: old blob untouched)
                sqlx::query(
                    "UPDATE fs_nodes SET size_bytes = $1, mime_type = $2, updated_at = now() WHERE id = $3",
                )
                .bind(size)
                .bind(mime_type)
                .bind(node_id)
                .execute(&self.pool)
                .await?;

                sqlx::query("UPDATE fs_contents SET content_hash = $1 WHERE node_id = $2")
                    .bind(&hash)
                    .bind(node_id)
                    .execute(&self.pool)
                    .await?;

                Ok(node_id)
            }
            None => {
                // Create new file + content
                let node_id: Uuid = sqlx::query_scalar(
                    r#"
                    INSERT INTO fs_nodes (vm_id, parent_id, name, node_type, mime_type, size_bytes, owner)
                    VALUES ($1, $2, $3, 'file', $4, $5, $6)
                    RETURNING id
                    "#,
                )
                .bind(vm_id)
                .bind(parent_id)
                .bind(file_name)
                .bind(mime_type)
                .bind(size)
                .bind(owner)
                .fetch_one(&self.pool)
                .await?;

                sqlx::query("INSERT INTO fs_contents (node_id, content_hash) VALUES ($1, $2)")
                    .bind(node_id)
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;

                Ok(node_id)
            }
        }
    }

    /// Standard subdirectories created inside each user's home (Documents, Downloads, etc.).
    pub const STANDARD_HOME_SUBDIRS: &[&str] = &[
        "Documents",
        "Downloads",
        "Images",
        "Desktop",
        "Music",
        "Videos",
        "Trash",
    ];

    /// Create standard home subdirs (Documents, Downloads, Images, etc.) under home_path if they don't exist.
    pub async fn ensure_standard_home_subdirs(
        &self,
        vm_id: Uuid,
        home_path: &str,
        owner: &str,
    ) -> Result<(), sqlx::Error> {
        for name in Self::STANDARD_HOME_SUBDIRS {
            let subdir = if home_path == "/" {
                format!("/{name}")
            } else {
                format!("{home_path}/{name}")
            };
            if self.resolve_path(vm_id, &subdir).await?.is_none() {
                self.mkdir(vm_id, &subdir, owner).await?;
            }
        }
        Ok(())
    }

    /// Default files created in the owner's Documents folder (VM seed and restore disk).
    /// Used so the user can test find, grep, cat, etc.
    pub async fn seed_default_documents(
        &self,
        vm_id: Uuid,
        documents_path: &str,
        owner: &str,
    ) -> Result<(), sqlx::Error> {
        let files: &[(&str, &[u8], Option<&str>)] = &[
            ("README.md", DEFAULT_README_MD, Some("text/markdown")),
            ("notes.txt", DEFAULT_NOTES_TXT, Some("text/plain")),
            ("hello.lua", DEFAULT_HELLO_LUA, Some("application/x-nulltrace-lua")),
            ("todo.txt", DEFAULT_TODO_TXT, Some("text/plain")),
            ("sample.log", DEFAULT_SAMPLE_LOG, Some("text/plain")),
        ];
        for (name, content, mime) in files {
            let path = if documents_path.ends_with('/') {
                format!("{documents_path}{name}")
            } else {
                format!("{documents_path}/{name}")
            };
            self.write_file(vm_id, &path, content, *mime, owner)
                .await?;
        }
        Ok(())
    }

    /// Create a directory. Parent must exist.
    pub async fn mkdir(&self, vm_id: Uuid, path: &str, owner: &str) -> Result<Uuid, sqlx::Error> {
        let (parent_path, dir_name) = split_path(path);

        let parent_id = match self.resolve_path(vm_id, parent_path).await? {
            Some(id) => id,
            None => return Err(sqlx::Error::RowNotFound),
        };

        let node_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO fs_nodes (vm_id, parent_id, name, node_type, owner)
            VALUES ($1, $2, $3, 'directory', $4)
            RETURNING id
            "#,
        )
        .bind(vm_id)
        .bind(parent_id)
        .bind(dir_name)
        .bind(owner)
        .fetch_one(&self.pool)
        .await?;

        Ok(node_id)
    }

    /// Remove a file or directory (cascade deletes contents and children).
    pub async fn rm(&self, vm_id: Uuid, path: &str) -> Result<bool, sqlx::Error> {
        let node_id = match self.resolve_path(vm_id, path).await? {
            Some(id) => id,
            None => return Ok(false),
        };

        sqlx::query("DELETE FROM fs_nodes WHERE id = $1")
            .bind(node_id)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }

    /// Bootstrap the base filesystem for a new VM.
    /// Creates: / , /home, /etc, /tmp, /bin, /var, /var/log
    pub async fn bootstrap_fs(&self, vm_id: Uuid) -> Result<(), sqlx::Error> {
        // Create root "/"
        let root_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO fs_nodes (vm_id, parent_id, name, node_type)
            VALUES ($1, NULL, '/', 'directory')
            RETURNING id
            "#,
        )
        .bind(vm_id)
        .fetch_one(&self.pool)
        .await?;

        // Top-level directories
        let dirs = ["home", "etc", "tmp", "bin", "var"];
        let mut var_id: Option<Uuid> = None;
        let mut home_id: Option<Uuid> = None;

        for dir in dirs {
            let id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO fs_nodes (vm_id, parent_id, name, node_type)
                VALUES ($1, $2, $3, 'directory')
                RETURNING id
                "#,
            )
            .bind(vm_id)
            .bind(root_id)
            .bind(dir)
            .fetch_one(&self.pool)
            .await?;

            if dir == "var" {
                var_id = Some(id);
            }
            if dir == "home" {
                home_id = Some(id);
            }
        }

        // /root (default home for root user)
        sqlx::query(
            r#"
            INSERT INTO fs_nodes (vm_id, parent_id, name, node_type)
            VALUES ($1, $2, 'root', 'directory')
            "#,
        )
        .bind(vm_id)
        .bind(root_id)
        .execute(&self.pool)
        .await?;
        self.ensure_standard_home_subdirs(vm_id, "/root", "root")
            .await?;

        // /home/user (default home for non-root user)
        if let Some(home_id) = home_id {
            sqlx::query(
                r#"
                INSERT INTO fs_nodes (vm_id, parent_id, name, node_type)
                VALUES ($1, $2, 'user', 'directory')
                "#,
            )
            .bind(vm_id)
            .bind(home_id)
            .execute(&self.pool)
            .await?;
            self.ensure_standard_home_subdirs(vm_id, "/home/user", "user")
                .await?;
        }

        // /var/log
        if let Some(var_id) = var_id {
            sqlx::query(
                r#"
                INSERT INTO fs_nodes (vm_id, parent_id, name, node_type)
                VALUES ($1, $2, 'log', 'directory')
                "#,
            )
            .bind(vm_id)
            .bind(var_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Migrate existing fs_contents from inline data to blob_store (content-addressable).
    /// Call after 007 migration, before 008. Idempotent if content_hash already set.
    pub async fn migrate_fs_contents_to_blob_store(pool: &PgPool) -> Result<(), sqlx::Error> {
        // Check if data column exists (pre-008 schema)
        let has_data: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = 'fs_contents' AND column_name = 'data'
            )
            "#,
        )
        .fetch_one(pool)
        .await?;

        if !has_data {
            return Ok(());
        }

        #[derive(Debug, sqlx::FromRow)]
        struct Row {
            node_id: Uuid,
            data: Vec<u8>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT node_id, data FROM fs_contents WHERE content_hash IS NULL",
        )
        .fetch_all(pool)
        .await?;

        for row in rows {
            let hash = hash_content(&row.data);
            sqlx::query(
                "INSERT INTO blob_store (hash, data) VALUES ($1, $2) ON CONFLICT (hash) DO NOTHING",
            )
            .bind(&hash)
            .bind(&row.data)
            .execute(pool)
            .await?;

            sqlx::query("UPDATE fs_contents SET content_hash = $1 WHERE node_id = $2")
                .bind(&hash)
                .bind(row.node_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    /// Remove orphaned blobs from blob_store (not referenced by any fs_contents).
    /// Call at startup after migrations to prevent volume growth when VMs are destroyed.
    pub async fn cleanup_orphan_blobs(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM blob_store b
            WHERE NOT EXISTS (
                SELECT 1 FROM fs_contents c WHERE c.content_hash = b.hash
            )
            "#,
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Total bytes used by files in this VM (sum of size_bytes for node_type = 'file').
    pub async fn disk_usage_bytes(&self, vm_id: Uuid) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(size_bytes), 0)::BIGINT FROM fs_nodes WHERE vm_id = $1 AND node_type = 'file'",
        )
        .bind(vm_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Recursively copy a file or directory from src_path to dest_path.
    /// dest_path is the full destination path (e.g. /home/user/Downloads/file.txt for a file).
    pub async fn copy_path_recursive(
        &self,
        vm_id: Uuid,
        src_path: &str,
        dest_path: &str,
        owner: &str,
    ) -> Result<(), sqlx::Error> {
        let node_type = self
            .node_type_at(vm_id, src_path)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        if node_type == "file" {
            let (data, mime_type) = self
                .read_file(vm_id, src_path)
                .await?
                .ok_or_else(|| sqlx::Error::RowNotFound)?;
            self.write_file(
                vm_id,
                dest_path,
                &data,
                mime_type.as_deref(),
                owner,
            )
            .await?;
            return Ok(());
        }

        // Directory: use queue (BFS) to avoid async recursion
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((src_path.to_string(), dest_path.to_string()));
        while let Some((src, dest)) = queue.pop_front() {
            self.mkdir(vm_id, &dest, owner).await?;
            let children = self.ls(vm_id, &src).await?;
            let src_base = src.trim_end_matches('/');
            let dest_base = dest.trim_end_matches('/');
            for child in children {
                let src_child = format!("{}/{}", src_base, child.name);
                let dest_child = format!("{}/{}", dest_base, child.name);
                let child_type = self
                    .node_type_at(vm_id, &src_child)
                    .await?
                    .ok_or_else(|| sqlx::Error::RowNotFound)?;
                if child_type == "file" {
                    let (data, mime_type) = self
                        .read_file(vm_id, &src_child)
                        .await?
                        .ok_or_else(|| sqlx::Error::RowNotFound)?;
                    self.write_file(
                        vm_id,
                        &dest_child,
                        &data,
                        mime_type.as_deref(),
                        owner,
                    )
                    .await?;
                } else {
                    queue.push_back((src_child, dest_child));
                }
            }
        }
        Ok(())
    }

    /// Move a file or directory by updating its parent_id. Fails if dest already has a child with the same name.
    pub async fn move_node(
        &self,
        vm_id: Uuid,
        src_path: &str,
        dest_parent_path: &str,
    ) -> Result<(), sqlx::Error> {
        let (_src_parent, src_name) = split_path(src_path);
        let src_node_id = self
            .resolve_path(vm_id, src_path)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;
        let dest_parent_id = self
            .resolve_path(vm_id, dest_parent_path)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Cannot move directory into itself or a descendant
        let src_norm = src_path.trim_end_matches('/');
        let dest_norm = dest_parent_path.trim_end_matches('/');
        if dest_norm == src_norm || dest_norm.starts_with(&format!("{}/", src_norm)) {
            return Err(sqlx::Error::RowNotFound);
        }

        // Check for name conflict
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fs_nodes WHERE vm_id = $1 AND parent_id = $2 AND name = $3",
        )
        .bind(vm_id)
        .bind(dest_parent_id)
        .bind(&src_name)
        .fetch_optional(&self.pool)
        .await?;
        if existing.is_some() {
            return Err(sqlx::Error::RowNotFound);
        }

        sqlx::query("UPDATE fs_nodes SET parent_id = $1, updated_at = now() WHERE id = $2")
            .bind(dest_parent_id)
            .bind(src_node_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Rename a file or directory. Fails if a sibling already has the new name.
    pub async fn rename_node(
        &self,
        vm_id: Uuid,
        path: &str,
        new_name: &str,
    ) -> Result<(), sqlx::Error> {
        if new_name.is_empty() || new_name.contains('/') || new_name == "." || new_name == ".." {
            return Err(sqlx::Error::RowNotFound);
        }
        let node_id = self
            .resolve_path(vm_id, path)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;
        let row: Option<(Option<Uuid>,)> = sqlx::query_as(
            "SELECT parent_id FROM fs_nodes WHERE id = $1",
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;
        let parent_id = match row.and_then(|r| r.0) {
            Some(id) => id,
            None => return Err(sqlx::Error::RowNotFound),
        };
        // Check for name conflict with siblings
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fs_nodes WHERE vm_id = $1 AND parent_id = $2 AND name = $3",
        )
        .bind(vm_id)
        .bind(parent_id)
        .bind(new_name)
        .fetch_optional(&self.pool)
        .await?;
        if existing.is_some() && existing != Some(node_id) {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query("UPDATE fs_nodes SET name = $1, updated_at = now() WHERE id = $2")
            .bind(new_name)
            .bind(node_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all filesystem data for a VM.
    pub async fn destroy_fs(&self, vm_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM fs_nodes WHERE vm_id = $1")
            .bind(vm_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Split "/home/user/file.txt" into ("/home/user", "file.txt")
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(pos) => {
            let parent = if pos == 0 { "/" } else { &path[..pos] };
            let name = &path[pos + 1..];
            (parent, name)
        }
        None => ("/", path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::vm_service::{VmConfig, VmService};

    #[test]
    fn test_split_path_root_file() {
        let (parent, name) = split_path("/file.txt");
        assert_eq!(parent, "/");
        assert_eq!(name, "file.txt");
    }

    #[test]
    fn test_split_path_nested() {
        let (parent, name) = split_path("/home/user/doc.txt");
        assert_eq!(parent, "/home/user");
        assert_eq!(name, "doc.txt");
    }

    #[test]
    fn test_split_path_single_dir() {
        let (parent, name) = split_path("/etc");
        assert_eq!(parent, "/");
        assert_eq!(name, "etc");
    }

    // ── Helper: creates a VM + bootstraps its filesystem, returns the vm_id ──

    async fn setup_vm(pool: &sqlx::PgPool) -> (Uuid, VmService, FsService) {
        let vm_service = VmService::new(pool.clone());
        let fs_service = FsService::new(pool.clone());
        let vm_id = Uuid::new_v4();

        vm_service
            .create_vm(
                vm_id,
                VmConfig {
                    hostname: format!("test-{}", &vm_id.to_string()[..8]),
                    dns_name: None,
                    cpu_cores: 1,
                    memory_mb: 512,
                    disk_mb: 10240,
                    ip: None,
                    subnet: None,
                    gateway: None,
                    mac: None,
                    owner_id: None,
                },
            )
            .await
            .unwrap();

        fs_service.bootstrap_fs(vm_id).await.unwrap();

        (vm_id, vm_service, fs_service)
    }

    async fn cleanup(vm_service: &VmService, vm_id: Uuid) {
        vm_service.delete_vm(vm_id).await.unwrap();
    }

    // ── Bootstrap tests ──

    #[tokio::test]
    async fn test_bootstrap_creates_base_dirs() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Root should exist
        let root = fs_svc.resolve_path(vm_id, "/").await.unwrap();
        assert!(root.is_some());

        // List root
        let entries = fs_svc.ls(vm_id, "/").await.unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"home"));
        assert!(names.contains(&"etc"));
        assert!(names.contains(&"tmp"));
        assert!(names.contains(&"bin"));
        assert!(names.contains(&"var"));

        // All should be directories
        assert!(entries.iter().all(|e| e.node_type == "directory"));

        // /var/log should exist
        let var_log = fs_svc.resolve_path(vm_id, "/var/log").await.unwrap();
        assert!(var_log.is_some());

        cleanup(&vm_svc, vm_id).await;
    }

    // ── Resolve path tests ──

    #[tokio::test]
    async fn test_resolve_nonexistent_path() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        let result = fs_svc.resolve_path(vm_id, "/nope/nothing").await.unwrap();
        assert!(result.is_none());

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_resolve_no_fs() {
        let pool = super::super::test_pool().await;
        let fs_svc = FsService::new(pool);

        // Random UUID with no filesystem
        let result = fs_svc.resolve_path(Uuid::new_v4(), "/").await.unwrap();
        assert!(result.is_none());
    }

    // ── mkdir tests ──

    #[tokio::test]
    async fn test_mkdir_and_ls() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Bootstrap already creates /home/user; list /home and expect user
        let entries = fs_svc.ls(vm_id, "/home").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "user" && e.node_type == "directory"));

        // Create nested /home/user/documents (bootstrap already created Documents, Downloads, etc.)
        fs_svc.mkdir(vm_id, "/home/user/documents", "root").await.unwrap();

        let entries = fs_svc.ls(vm_id, "/home/user").await.unwrap();
        assert!(
            entries.iter().any(|e| e.name == "documents"),
            "/home/user should contain documents, got: {:?}",
            entries.iter().map(|e| &e.name).collect::<Vec<_>>()
        );

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_mkdir_fails_without_parent() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // /nonexistent doesn't exist, so /nonexistent/child should fail
        let result = fs_svc.mkdir(vm_id, "/nonexistent/child", "root").await;
        assert!(result.is_err());

        cleanup(&vm_svc, vm_id).await;
    }

    // ── write / read file tests ──

    #[tokio::test]
    async fn test_write_and_read_text_file() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        let content = b"Hello, NullTrace!";
        fs_svc
            .write_file(vm_id, "/tmp/hello.txt", content, Some("text/plain"), "root")
            .await
            .unwrap();

        // Read back
        let (data, mime) = fs_svc.read_file(vm_id, "/tmp/hello.txt").await.unwrap().unwrap();
        assert_eq!(data, content);
        assert_eq!(mime.as_deref(), Some("text/plain"));

        // Verify it appears in ls
        let entries = fs_svc.ls(vm_id, "/tmp").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");
        assert_eq!(entries[0].node_type, "file");
        assert_eq!(entries[0].size_bytes, content.len() as i64);

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_write_binary_file() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Fake PNG header bytes
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0xFE];
        fs_svc
            .write_file(vm_id, "/tmp/image.png", &png_bytes, Some("image/png"), "root")
            .await
            .unwrap();

        let (data, mime) = fs_svc.read_file(vm_id, "/tmp/image.png").await.unwrap().unwrap();
        assert_eq!(data, png_bytes);
        assert_eq!(mime.as_deref(), Some("image/png"));

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_overwrite_file() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Write v1
        fs_svc
            .write_file(vm_id, "/etc/config.txt", b"v1", Some("text/plain"), "root")
            .await
            .unwrap();

        // Overwrite with v2
        fs_svc
            .write_file(vm_id, "/etc/config.txt", b"version2-updated", Some("text/plain"), "root")
            .await
            .unwrap();

        let (data, _) = fs_svc.read_file(vm_id, "/etc/config.txt").await.unwrap().unwrap();
        assert_eq!(data, b"version2-updated");

        // Still only 1 file in /etc
        let entries = fs_svc.ls(vm_id, "/etc").await.unwrap();
        assert_eq!(entries.len(), 1);

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        let result = fs_svc.read_file(vm_id, "/tmp/nope.txt").await.unwrap();
        assert!(result.is_none());

        cleanup(&vm_svc, vm_id).await;
    }

    // ── blob store (content-addressable) tests ──

    #[tokio::test]
    async fn test_blob_deduplication() {
        let pool = super::super::test_pool().await;
        let (vm_id_1, vm_svc_1, fs_svc) = setup_vm(&pool).await;
        let (vm_id_2, vm_svc_2, _) = setup_vm(&pool).await;

        let content = b"identical content for both VMs";
        fs_svc
            .write_file(vm_id_1, "/tmp/shared.txt", content, None, "root")
            .await
            .unwrap();
        fs_svc
            .write_file(vm_id_2, "/tmp/shared.txt", content, None, "root")
            .await
            .unwrap();

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM blob_store WHERE data = $1",
        )
        .bind(content)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(count.0, 1, "identical content should be stored once in blob_store");

        cleanup(&vm_svc_1, vm_id_1).await;
        cleanup(&vm_svc_2, vm_id_2).await;
    }

    #[tokio::test]
    async fn test_blob_copy_on_write() {
        let pool = super::super::test_pool().await;
        let (vm_id_1, vm_svc_1, fs_svc) = setup_vm(&pool).await;
        let (vm_id_2, vm_svc_2, _) = setup_vm(&pool).await;

        let original = b"original content";
        let modified = b"modified by VM1";

        fs_svc
            .write_file(vm_id_1, "/tmp/file.txt", original, None, "root")
            .await
            .unwrap();
        fs_svc
            .write_file(vm_id_2, "/tmp/file.txt", original, None, "root")
            .await
            .unwrap();

        let (data_1, _) = fs_svc.read_file(vm_id_1, "/tmp/file.txt").await.unwrap().unwrap();
        let (data_2, _) = fs_svc.read_file(vm_id_2, "/tmp/file.txt").await.unwrap().unwrap();
        assert_eq!(data_1, original);
        assert_eq!(data_2, original);

        fs_svc
            .write_file(vm_id_1, "/tmp/file.txt", modified, None, "root")
            .await
            .unwrap();

        let (data_1, _) = fs_svc.read_file(vm_id_1, "/tmp/file.txt").await.unwrap().unwrap();
        let (data_2, _) = fs_svc.read_file(vm_id_2, "/tmp/file.txt").await.unwrap().unwrap();
        assert_eq!(data_1, modified, "VM1 should see modified content");
        assert_eq!(data_2, original, "VM2 should still see original (copy-on-write)");

        let original_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM blob_store WHERE data = $1",
        )
        .bind(original)
        .fetch_one(&pool)
        .await
        .unwrap();
        let modified_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM blob_store WHERE data = $1",
        )
        .bind(modified)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(original_count.0, 1, "original blob should exist for VM2");
        assert_eq!(modified_count.0, 1, "modified blob should exist for VM1");

        cleanup(&vm_svc_1, vm_id_1).await;
        cleanup(&vm_svc_2, vm_id_2).await;
    }

    // ── rm tests ──

    #[tokio::test]
    async fn test_rm_file() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        fs_svc
            .write_file(vm_id, "/tmp/delete_me.txt", b"bye", None, "root")
            .await
            .unwrap();

        let deleted = fs_svc.rm(vm_id, "/tmp/delete_me.txt").await.unwrap();
        assert!(deleted);

        // Should be gone
        let result = fs_svc.read_file(vm_id, "/tmp/delete_me.txt").await.unwrap();
        assert!(result.is_none());

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_rm_directory_cascades() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        fs_svc.mkdir(vm_id, "/home/customuser", "root").await.unwrap();
        fs_svc
            .write_file(vm_id, "/home/customuser/notes.txt", b"important", None, "root")
            .await
            .unwrap();

        // Delete /home/customuser -> should cascade delete notes.txt
        let deleted = fs_svc.rm(vm_id, "/home/customuser").await.unwrap();
        assert!(deleted);

        let result = fs_svc.resolve_path(vm_id, "/home/customuser").await.unwrap();
        assert!(result.is_none());

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_rm_nonexistent() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        let deleted = fs_svc.rm(vm_id, "/nope").await.unwrap();
        assert!(!deleted);

        cleanup(&vm_svc, vm_id).await;
    }

    // ── destroy_fs tests ──

    #[tokio::test]
    async fn test_destroy_fs_cleans_everything() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Write some files
        fs_svc.write_file(vm_id, "/tmp/a.txt", b"a", None, "root").await.unwrap();
        fs_svc.write_file(vm_id, "/etc/b.txt", b"b", None, "root").await.unwrap();

        // Destroy all
        fs_svc.destroy_fs(vm_id).await.unwrap();

        // Root should be gone
        let root = fs_svc.resolve_path(vm_id, "/").await.unwrap();
        assert!(root.is_none());

        cleanup(&vm_svc, vm_id).await;
    }

    // ── VM cascade delete tests ──

    #[tokio::test]
    async fn test_vm_delete_cascades_to_fs() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        fs_svc.write_file(vm_id, "/tmp/test.txt", b"data", None, "root").await.unwrap();

        // Delete the VM entirely
        vm_svc.delete_vm(vm_id).await.unwrap();

        // Filesystem should be gone too
        let root = fs_svc.resolve_path(vm_id, "/").await.unwrap();
        assert!(root.is_none());
    }

    // ── Owner tests ──

    #[tokio::test]
    async fn test_write_file_sets_owner() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        fs_svc
            .write_file(vm_id, "/tmp/userfile.txt", b"hello", Some("text/plain"), "johndoe")
            .await
            .unwrap();

        let entries = fs_svc.ls(vm_id, "/tmp").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "userfile.txt");
        assert_eq!(entries[0].owner, "johndoe");

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_mkdir_sets_owner() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        fs_svc.mkdir(vm_id, "/home/alice", "alice").await.unwrap();

        let entries = fs_svc.ls(vm_id, "/home").await.unwrap();
        let alice = entries.iter().find(|e| e.name == "alice").expect("alice dir should exist");
        assert_eq!(alice.owner, "alice");

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_bootstrap_dirs_owned_by_root() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        let entries = fs_svc.ls(vm_id, "/").await.unwrap();
        // All bootstrap dirs should be owned by root (DB default)
        for entry in &entries {
            assert_eq!(entry.owner, "root", "dir {} should be owned by root", entry.name);
        }

        cleanup(&vm_svc, vm_id).await;
    }

    #[tokio::test]
    async fn test_overwrite_preserves_original_owner() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc, fs_svc) = setup_vm(&pool).await;

        // Create file as "alice"
        fs_svc
            .write_file(vm_id, "/tmp/shared.txt", b"v1", None, "alice")
            .await
            .unwrap();

        // Overwrite as "bob" — existing file update doesn't change owner
        fs_svc
            .write_file(vm_id, "/tmp/shared.txt", b"v2", None, "bob")
            .await
            .unwrap();

        let entries = fs_svc.ls(vm_id, "/tmp").await.unwrap();
        assert_eq!(entries[0].owner, "alice"); // owner stays as original creator

        cleanup(&vm_svc, vm_id).await;
    }
}
