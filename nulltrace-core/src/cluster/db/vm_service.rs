#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct VmRecord {
    pub id: Uuid,
    pub hostname: String,
    pub dns_name: Option<String>,
    pub cpu_cores: i16,
    pub memory_mb: i32,
    pub disk_mb: i32,
    pub status: String,
    pub ip: Option<String>,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub mac: Option<String>,
    pub owner_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct VmConfig {
    pub hostname: String,
    pub dns_name: Option<String>,
    pub cpu_cores: i16,
    pub memory_mb: i32,
    pub disk_mb: i32,
    pub ip: Option<String>,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub mac: Option<String>,
    pub owner_id: Option<Uuid>,
}

pub struct VmService {
    pool: PgPool,
}

impl VmService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the database pool (e.g. for creating VM Lua states).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn create_vm(&self, id: Uuid, config: VmConfig) -> Result<VmRecord, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmRecord>(
            r#"
            INSERT INTO vms (id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status, ip, subnet, gateway, mac, owner_id)
            VALUES ($1, $2, $3, $4, $5, $6, 'running', $7, $8, $9, $10, $11)
            RETURNING id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                      ip, subnet, gateway, mac, owner_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&config.hostname)
        .bind(&config.dns_name)
        .bind(config.cpu_cores)
        .bind(config.memory_mb)
        .bind(config.disk_mb)
        .bind(&config.ip)
        .bind(&config.subnet)
        .bind(&config.gateway)
        .bind(&config.mac)
        .bind(config.owner_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_vm(&self, vm_id: Uuid) -> Result<Option<VmRecord>, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmRecord>(
            r#"
            SELECT id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                   ip, subnet, gateway, mac, owner_id, created_at, updated_at
            FROM vms WHERE id = $1
            "#,
        )
        .bind(vm_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn restore_running_vms(&self) -> Result<Vec<VmRecord>, sqlx::Error> {
        let recs = sqlx::query_as::<_, VmRecord>(
            r#"
            SELECT id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                   ip, subnet, gateway, mac, owner_id, created_at, updated_at
            FROM vms WHERE status IN ('running', 'crashed')
            ORDER BY created_at
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs)
    }

    /// Restore only player-owned VMs (owner_id IS NOT NULL) with status 'running' or 'crashed'.
    /// Used in game mode to avoid loading test VMs.
    pub async fn restore_player_vms(&self) -> Result<Vec<VmRecord>, sqlx::Error> {
        let recs = sqlx::query_as::<_, VmRecord>(
            r#"
            SELECT id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                   ip, subnet, gateway, mac, owner_id, created_at, updated_at
            FROM vms
            WHERE status IN ('running', 'crashed')
              AND owner_id IS NOT NULL
            ORDER BY created_at
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs)
    }

    pub async fn set_status(&self, vm_id: Uuid, status: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE vms SET status = $1, updated_at = now() WHERE id = $2")
            .bind(status)
            .bind(vm_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_vm(&self, vm_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM vms WHERE id = $1")
            .bind(vm_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_all(&self) -> Result<Vec<VmRecord>, sqlx::Error> {
        let recs = sqlx::query_as::<_, VmRecord>(
            r#"
            SELECT id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                   ip, subnet, gateway, mac, owner_id, created_at, updated_at
            FROM vms ORDER BY created_at
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs)
    }

    /// Returns the VM owned by the given player (owner_id), if any.
    pub async fn get_vm_by_owner_id(&self, owner_id: Uuid) -> Result<Option<VmRecord>, sqlx::Error> {
        let rec = sqlx::query_as::<_, VmRecord>(
            r#"
            SELECT id, hostname, dns_name, cpu_cores, memory_mb, disk_mb, status,
                   ip, subnet, gateway, mac, owner_id, created_at, updated_at
            FROM vms WHERE owner_id = $1
            LIMIT 1
            "#,
        )
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> VmConfig {
        VmConfig {
            hostname: name.to_string(),
            dns_name: None,
            cpu_cores: 2,
            memory_mb: 1024,
            disk_mb: 20480,
            ip: Some("10.0.1.10".to_string()),
            subnet: Some("10.0.1.0/24".to_string()),
            gateway: Some("10.0.1.1".to_string()),
            mac: Some("02:00:0a:00:01:0a".to_string()),
            owner_id: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_vm() {
        let pool = super::super::test_pool().await;
        let service = VmService::new(pool);
        let id = Uuid::new_v4();

        // Create
        let vm = service.create_vm(id, test_config("test-web-srv")).await.unwrap();
        assert_eq!(vm.id, id);
        assert_eq!(vm.hostname, "test-web-srv");
        assert_eq!(vm.cpu_cores, 2);
        assert_eq!(vm.memory_mb, 1024);
        assert_eq!(vm.status, "running");
        assert_eq!(vm.ip.as_deref(), Some("10.0.1.10"));

        // Get
        let fetched = service.get_vm(id).await.unwrap().unwrap();
        assert_eq!(fetched.id, id);
        assert_eq!(fetched.hostname, "test-web-srv");

        // Cleanup
        service.delete_vm(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_nonexistent_vm() {
        let pool = super::super::test_pool().await;
        let service = VmService::new(pool);

        let result = service.get_vm(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_status() {
        let pool = super::super::test_pool().await;
        let service = VmService::new(pool);
        let id = Uuid::new_v4();

        service.create_vm(id, test_config("test-status-vm")).await.unwrap();

        // Change to stopped
        service.set_status(id, "stopped").await.unwrap();
        let vm = service.get_vm(id).await.unwrap().unwrap();
        assert_eq!(vm.status, "stopped");

        // Change to crashed
        service.set_status(id, "crashed").await.unwrap();
        let vm = service.get_vm(id).await.unwrap().unwrap();
        assert_eq!(vm.status, "crashed");

        // Cleanup
        service.delete_vm(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_restore_running_vms() {
        let pool = super::super::test_pool().await;
        let service = VmService::new(pool);

        let id_running = Uuid::new_v4();
        let id_stopped = Uuid::new_v4();
        let id_crashed = Uuid::new_v4();

        service.create_vm(id_running, test_config("vm-running")).await.unwrap();
        service.create_vm(id_stopped, test_config("vm-stopped")).await.unwrap();
        service.create_vm(id_crashed, test_config("vm-crashed")).await.unwrap();

        // All start as 'running', change two
        service.set_status(id_stopped, "stopped").await.unwrap();
        service.set_status(id_crashed, "crashed").await.unwrap();

        let restored = service.restore_running_vms().await.unwrap();
        let restored_ids: Vec<Uuid> = restored.iter().map(|r| r.id).collect();

        // running and crashed should be restored, stopped should not
        assert!(restored_ids.contains(&id_running));
        assert!(restored_ids.contains(&id_crashed));
        assert!(!restored_ids.contains(&id_stopped));

        // Cleanup
        service.delete_vm(id_running).await.unwrap();
        service.delete_vm(id_stopped).await.unwrap();
        service.delete_vm(id_crashed).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_vm() {
        let pool = super::super::test_pool().await;
        let service = VmService::new(pool);
        let id = Uuid::new_v4();

        service.create_vm(id, test_config("vm-to-delete")).await.unwrap();
        assert!(service.get_vm(id).await.unwrap().is_some());

        service.delete_vm(id).await.unwrap();
        assert!(service.get_vm(id).await.unwrap().is_none());
    }
}
