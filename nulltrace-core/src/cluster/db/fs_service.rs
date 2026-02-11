#![allow(dead_code)]

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct FsEntry {
    pub name: String,
    pub node_type: String,
    pub size_bytes: i64,
    pub permissions: String,
    pub owner: String,
}

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
            SELECT c.data, n.mime_type
            FROM fs_contents c
            JOIN fs_nodes n ON n.id = c.node_id
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

        match existing {
            Some(node_id) => {
                // Update existing file
                sqlx::query(
                    "UPDATE fs_nodes SET size_bytes = $1, mime_type = $2, updated_at = now() WHERE id = $3",
                )
                .bind(size)
                .bind(mime_type)
                .bind(node_id)
                .execute(&self.pool)
                .await?;

                sqlx::query("UPDATE fs_contents SET data = $1 WHERE node_id = $2")
                    .bind(data)
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

                sqlx::query("INSERT INTO fs_contents (node_id, data) VALUES ($1, $2)")
                    .bind(node_id)
                    .bind(data)
                    .execute(&self.pool)
                    .await?;

                Ok(node_id)
            }
        }
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

        // Create /home/user
        fs_svc.mkdir(vm_id, "/home/user", "root").await.unwrap();

        let entries = fs_svc.ls(vm_id, "/home").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "user");
        assert_eq!(entries[0].node_type, "directory");

        // Create nested /home/user/documents
        fs_svc.mkdir(vm_id, "/home/user/documents", "root").await.unwrap();

        let entries = fs_svc.ls(vm_id, "/home/user").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "documents");

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

        fs_svc.mkdir(vm_id, "/home/user", "root").await.unwrap();
        fs_svc
            .write_file(vm_id, "/home/user/notes.txt", b"important", None, "root")
            .await
            .unwrap();

        // Delete /home/user -> should cascade delete notes.txt
        let deleted = fs_svc.rm(vm_id, "/home/user").await.unwrap();
        assert!(deleted);

        let result = fs_svc.resolve_path(vm_id, "/home/user").await.unwrap();
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
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "alice");
        assert_eq!(entries[0].owner, "alice");

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
