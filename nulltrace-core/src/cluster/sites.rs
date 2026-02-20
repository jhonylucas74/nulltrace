//! Site VMs: load from `sites/<dns_name>/config.yaml` and optional `www/` folder.
//! On server start we reset (destroy if exists) and create each site VM, then seed files from www/.

use super::db::fs_service::FsService;
use super::db::player_service::PlayerService;
use super::db::vm_service::{VmConfig, VmService};
use super::vm_manager::VmManager;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SiteConfig {
    pub hostname: String,
    pub dns_name: Option<String>,
    /// Fixed IP for this site VM (e.g. "10.0.1.100"). Avoids IP collision on hot restart.
    pub ip: Option<String>,
    pub cpu_cores: i16,
    pub memory_mb: i32,
    pub disk_mb: i32,
    #[serde(default)]
    pub bootstrap: String,
}

/// Base path for sites directory (e.g. nulltrace-core/sites when running from crate).
pub fn sites_base_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sites")
}

/// Load site VMs: for each folder in `sites_base` that has a config.yaml, reset (destroy if exists)
/// and create the VM, then seed /var/www from the folder's www/ and write /etc/bootstrap.
pub async fn load_site_vms(
    manager: &mut VmManager,
    fs_service: &Arc<FsService>,
    player_service: &Arc<PlayerService>,
    vm_service: &Arc<VmService>,
    sites_base: &Path,
) -> Result<(), String> {
    let webserver = player_service
        .get_by_username(super::db::player_service::WEBSERVER_USERNAME)
        .await
        .map_err(|e| format!("get webserver: {}", e))?
        .ok_or_else(|| "Webserver player not found".to_string())?;

    let owner_id = webserver.id;

    let read_dir = tokio::fs::read_dir(sites_base)
        .await
        .map_err(|e| format!("read sites dir {}: {}", sites_base.display(), e))?;

    let mut entries = Vec::new();
    let mut read = read_dir;
    loop {
        let entry = read
            .next_entry()
            .await
            .map_err(|e| format!("read_dir next: {}", e))?;
        let Some(entry) = entry else { break };
        entries.push(entry);
    }

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let site_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("invalid site dir name: {}", path.display()))?
            .to_string();

        let config_path = path.join("config.yaml");
        let config_bytes = match tokio::fs::read(&config_path).await {
            Ok(b) => b,
            Err(_) => continue, // skip dirs without config.yaml
        };

        let config: SiteConfig = serde_yaml::from_slice(&config_bytes)
            .map_err(|e| format!("parse {}: {}", config_path.display(), e))?;

        let dns_name = config
            .dns_name
            .clone()
            .unwrap_or_else(|| site_id.clone());

        // Find existing VM for this site (owner = webserver, dns_name = site)
        let existing_vms = vm_service
            .get_vms_by_owner_id(owner_id)
            .await
            .map_err(|e| format!("get_vms_by_owner_id: {}", e))?;

        if let Some(existing) = existing_vms.iter().find(|v| v.dns_name.as_deref() == Some(dns_name.as_str())) {
            manager
                .destroy_vm(existing.id)
                .await
                .map_err(|e| format!("destroy existing VM {}: {}", dns_name, e))?;
            println!("[cluster] Destroyed existing VM for site {}", dns_name);
        }

        let vm_config = VmConfig {
            hostname: config.hostname.clone(),
            dns_name: Some(dns_name.clone()),
            cpu_cores: config.cpu_cores,
            memory_mb: config.memory_mb,
            disk_mb: config.disk_mb,
            ip: config.ip.clone(),
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: Some(owner_id),
        };

        let (record, _) = manager
            .create_vm(vm_config)
            .await
            .map_err(|e| format!("create VM {}: {}", dns_name, e))?;

        // Create /var/www and seed from sites/<site_id>/www/
        fs_service
            .mkdir(record.id, "/var/www", "root")
            .await
            .map_err(|e| format!("mkdir /var/www: {}", e))?;

        let www_dir = path.join("www");
        if www_dir.is_dir() {
            seed_www_dir(fs_service, record.id, &www_dir, "/var/www").await?;
        }

        // Write /etc/bootstrap
        let bootstrap = config.bootstrap.trim();
        if !bootstrap.is_empty() {
            fs_service
                .write_file(
                    record.id,
                    "/etc/bootstrap",
                    bootstrap.as_bytes(),
                    None,
                    "root",
                )
                .await
                .map_err(|e| format!("write /etc/bootstrap: {}", e))?;
        }

        println!("[cluster] ✓ Site VM created: {}", dns_name);
    }

    Ok(())
}

/// Copy files from host `www_dir` into VM at `vm_www_path` (iterative to avoid async recursion).
async fn seed_www_dir(
    fs_service: &FsService,
    vm_id: Uuid,
    www_dir: &Path,
    vm_www_path: &str,
) -> Result<(), String> {
    let mut stack: Vec<(PathBuf, String)> = vec![(www_dir.to_path_buf(), vm_www_path.to_string())];

    while let Some((host_path, vm_path)) = stack.pop() {
        let mut read = tokio::fs::read_dir(&host_path)
            .await
            .map_err(|e| format!("read_dir {}: {}", host_path.display(), e))?;

        while let Some(entry) = read
            .next_entry()
            .await
            .map_err(|e| format!("read_dir next: {}", e))?
        {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("invalid name: {}", path.display()))?;

            let child_vm_path = if vm_path == "/" {
                format!("/{}", name)
            } else {
                format!("{}/{}", vm_path.trim_end_matches('/'), name)
            };

            if path.is_dir() {
                fs_service
                    .mkdir(vm_id, &child_vm_path, "root")
                    .await
                    .map_err(|e| format!("mkdir {}: {}", child_vm_path, e))?;
                stack.push((path, child_vm_path));
            } else {
                let content = tokio::fs::read(&path)
                    .await
                    .map_err(|e| format!("read {}: {}", path.display(), e))?;
                let mime = mime_for_path(name);
                fs_service
                    .write_file(vm_id, &child_vm_path, &content, Some(mime), "root")
                    .await
                    .map_err(|e| format!("write {}: {}", child_vm_path, e))?;
            }
        }
    }

    Ok(())
}

fn mime_for_path(name: &str) -> &'static str {
    if name.ends_with(".ntml") {
        "application/x-ntml"
    } else if name.ends_with(".txt") || name.ends_with(".md") {
        "text/plain"
    } else if name.ends_with(".html") {
        "text/html"
    } else {
        "application/octet-stream"
    }
}
