#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct VmUser {
    pub id: Uuid,
    pub vm_id: Uuid,
    pub username: String,
    pub uid: i32,
    pub home_dir: String,
    pub shell: String,
    pub password_hash: Option<String>,
    pub is_root: bool,
    pub created_at: DateTime<Utc>,
}

pub struct UserService {
    pool: PgPool,
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(
        &self,
        vm_id: Uuid,
        username: &str,
        uid: i32,
        password_hash: Option<&str>,
        is_root: bool,
        home_dir: &str,
        shell: &str,
    ) -> Result<VmUser, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmUser>(
            r#"
            INSERT INTO vm_users (vm_id, username, uid, password_hash, is_root, home_dir, shell)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, vm_id, username, uid, home_dir, shell, password_hash, is_root, created_at
            "#,
        )
        .bind(vm_id)
        .bind(username)
        .bind(uid)
        .bind(password_hash)
        .bind(is_root)
        .bind(home_dir)
        .bind(shell)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_user(
        &self,
        vm_id: Uuid,
        username: &str,
    ) -> Result<Option<VmUser>, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmUser>(
            r#"
            SELECT id, vm_id, username, uid, home_dir, shell, password_hash, is_root, created_at
            FROM vm_users WHERE vm_id = $1 AND username = $2
            "#,
        )
        .bind(vm_id)
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_user_by_uid(
        &self,
        vm_id: Uuid,
        uid: i32,
    ) -> Result<Option<VmUser>, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmUser>(
            r#"
            SELECT id, vm_id, username, uid, home_dir, shell, password_hash, is_root, created_at
            FROM vm_users WHERE vm_id = $1 AND uid = $2
            "#,
        )
        .bind(vm_id)
        .bind(uid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn list_users(&self, vm_id: Uuid) -> Result<Vec<VmUser>, sqlx::Error> {
        let recs = sqlx::query_as::<_, VmUser>(
            r#"
            SELECT id, vm_id, username, uid, home_dir, shell, password_hash, is_root, created_at
            FROM vm_users WHERE vm_id = $1 ORDER BY uid
            "#,
        )
        .bind(vm_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(recs)
    }

    pub async fn verify_password(
        &self,
        vm_id: Uuid,
        username: &str,
        password: &str,
    ) -> Result<bool, sqlx::Error> {
        let user = self.get_user(vm_id, username).await?;

        match user {
            Some(u) => match u.password_hash {
                Some(stored_hash) => Ok(stored_hash == hash_password(password)),
                None => Ok(false),
            },
            None => Ok(false),
        }
    }

    pub async fn set_password(
        &self,
        vm_id: Uuid,
        username: &str,
        password: &str,
    ) -> Result<bool, sqlx::Error> {
        let hash = hash_password(password);
        let result = sqlx::query(
            "UPDATE vm_users SET password_hash = $1 WHERE vm_id = $2 AND username = $3",
        )
        .bind(&hash)
        .bind(vm_id)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_user(
        &self,
        vm_id: Uuid,
        username: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM vm_users WHERE vm_id = $1 AND username = $2",
        )
        .bind(vm_id)
        .bind(username)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all vm_users for a VM (used before disk restore / re-bootstrap).
    pub async fn delete_all_for_vm(&self, vm_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM vm_users WHERE vm_id = $1")
            .bind(vm_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Create default users for a new VM: root (uid=0) and user (uid=1000).
    pub async fn bootstrap_users(&self, vm_id: Uuid) -> Result<Vec<VmUser>, sqlx::Error> {
        let mut users = Vec::new();

        let root = self
            .create_user(
                vm_id,
                "root",
                0,
                Some(&hash_password("toor")),
                true,
                "/root",
                "/bin/sh",
            )
            .await?;
        users.push(root);

        let user = self
            .create_user(
                vm_id,
                "user",
                1000,
                Some(&hash_password("password")),
                false,
                "/home/user",
                "/bin/sh",
            )
            .await?;
        users.push(user);

        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::vm_service::{VmConfig, VmService};

    async fn setup_vm(pool: &PgPool) -> (Uuid, VmService) {
        let vm_service = VmService::new(pool.clone());
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

        (vm_id, vm_service)
    }

    #[tokio::test]
    async fn test_bootstrap_creates_default_users() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let users = user_svc.bootstrap_users(vm_id).await.unwrap();
        assert_eq!(users.len(), 2);

        // root
        assert_eq!(users[0].username, "root");
        assert_eq!(users[0].uid, 0);
        assert!(users[0].is_root);
        assert_eq!(users[0].home_dir, "/root");

        // user
        assert_eq!(users[1].username, "user");
        assert_eq!(users[1].uid, 1000);
        assert!(!users[1].is_root);
        assert_eq!(users[1].home_dir, "/home/user");

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_verify_password_correct() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        assert!(user_svc.verify_password(vm_id, "root", "toor").await.unwrap());
        assert!(user_svc.verify_password(vm_id, "user", "password").await.unwrap());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_verify_password_wrong() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        assert!(!user_svc.verify_password(vm_id, "root", "wrongpass").await.unwrap());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_verify_password_nonexistent_user() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        assert!(!user_svc.verify_password(vm_id, "nobody", "test").await.unwrap());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_users() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        let users = user_svc.list_users(vm_id).await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].uid, 0); // ordered by uid
        assert_eq!(users[1].uid, 1000);

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_password() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        // Change root password
        assert!(user_svc.set_password(vm_id, "root", "newpass").await.unwrap());

        // Old password should fail, new should work
        assert!(!user_svc.verify_password(vm_id, "root", "toor").await.unwrap());
        assert!(user_svc.verify_password(vm_id, "root", "newpass").await.unwrap());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_user() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        assert!(user_svc.delete_user(vm_id, "user").await.unwrap());
        assert!(user_svc.get_user(vm_id, "user").await.unwrap().is_none());

        // root should still exist
        assert!(user_svc.get_user(vm_id, "root").await.unwrap().is_some());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_vm_delete_cascades_to_users() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        // Delete the VM
        vm_svc.delete_vm(vm_id).await.unwrap();

        // Users should be gone
        let users = user_svc.list_users(vm_id).await.unwrap();
        assert!(users.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_by_uid() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id).await.unwrap();

        let root = user_svc.get_user_by_uid(vm_id, 0).await.unwrap().unwrap();
        assert_eq!(root.username, "root");

        let user = user_svc.get_user_by_uid(vm_id, 1000).await.unwrap().unwrap();
        assert_eq!(user.username, "user");

        assert!(user_svc.get_user_by_uid(vm_id, 999).await.unwrap().is_none());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[test]
    fn test_hash_password_deterministic() {
        let h1 = hash_password("toor");
        let h2 = hash_password("toor");
        assert_eq!(h1, h2);
        // SHA-256 of "toor"
        assert_eq!(h1.len(), 64); // 256 bits = 64 hex chars
    }

    #[test]
    fn test_hash_password_different_inputs() {
        let h1 = hash_password("toor");
        let h2 = hash_password("password");
        assert_ne!(h1, h2);
    }

    #[tokio::test]
    async fn test_create_user_custom_fields() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let user = user_svc
            .create_user(
                vm_id,
                "admin",
                500,
                Some(&hash_password("admin123")),
                false,
                "/home/admin",
                "/bin/bash",
            )
            .await
            .unwrap();

        assert_eq!(user.username, "admin");
        assert_eq!(user.uid, 500);
        assert_eq!(user.home_dir, "/home/admin");
        assert_eq!(user.shell, "/bin/bash");
        assert!(!user.is_root);
        assert!(user.password_hash.is_some());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_user_no_password() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let user = user_svc
            .create_user(vm_id, "nologin", 65534, None, false, "/nonexistent", "/usr/sbin/nologin")
            .await
            .unwrap();

        assert!(user.password_hash.is_none());

        // verify_password should fail for user with no password
        assert!(!user_svc.verify_password(vm_id, "nologin", "anything").await.unwrap());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_duplicate_username_fails() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc
            .create_user(vm_id, "dupuser", 100, None, false, "/home/dupuser", "/bin/sh")
            .await
            .unwrap();

        // Same username, different uid — should fail (UNIQUE(vm_id, username))
        let result = user_svc
            .create_user(vm_id, "dupuser", 101, None, false, "/home/dupuser2", "/bin/sh")
            .await;
        assert!(result.is_err());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_duplicate_uid_fails() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc
            .create_user(vm_id, "userA", 200, None, false, "/home/a", "/bin/sh")
            .await
            .unwrap();

        // Different username, same uid — should fail (UNIQUE(vm_id, uid))
        let result = user_svc
            .create_user(vm_id, "userB", 200, None, false, "/home/b", "/bin/sh")
            .await;
        assert!(result.is_err());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let deleted = user_svc.delete_user(vm_id, "ghost").await.unwrap();
        assert!(!deleted);

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_password_nonexistent_user() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let changed = user_svc.set_password(vm_id, "ghost", "pass").await.unwrap();
        assert!(!changed);

        vm_svc.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_users_isolated_between_vms() {
        let pool = super::super::test_pool().await;
        let (vm_id_a, vm_svc_a) = setup_vm(&pool).await;
        let (vm_id_b, vm_svc_b) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        user_svc.bootstrap_users(vm_id_a).await.unwrap();

        // VM B should have no users
        let users_b = user_svc.list_users(vm_id_b).await.unwrap();
        assert!(users_b.is_empty());

        // VM A should have 2 users
        let users_a = user_svc.list_users(vm_id_a).await.unwrap();
        assert_eq!(users_a.len(), 2);

        // Password from VM A should not work on VM B
        assert!(!user_svc.verify_password(vm_id_b, "root", "toor").await.unwrap());

        vm_svc_a.delete_vm(vm_id_a).await.unwrap();
        vm_svc_b.delete_vm(vm_id_b).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_users_empty_vm() {
        let pool = super::super::test_pool().await;
        let (vm_id, vm_svc) = setup_vm(&pool).await;
        let user_svc = UserService::new(pool);

        let users = user_svc.list_users(vm_id).await.unwrap();
        assert!(users.is_empty());

        vm_svc.delete_vm(vm_id).await.unwrap();
    }
}
