//! gRPC client for AdminService. Used by Tauri commands.

mod admin {
    tonic::include_proto!("admin");
}

use admin::admin_service_client::AdminServiceClient;
use admin::{
    AdminLoginRequest, GetClusterStatsRequest, GetNetworkTopologyRequest, ListVmsRequest,
};

fn grpc_url() -> String {
    std::env::var("NULLTRACE_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

fn auth_metadata(token: &str) -> Result<tonic::metadata::AsciiMetadataValue, String> {
    format!("Bearer {}", token)
        .parse()
        .map_err(|_| "Invalid token format".to_string())
}

/// Tauri command: Admin login with email and password.
#[tauri::command]
pub async fn admin_login(email: String, password: String) -> Result<AdminLoginResponse, String> {
    let url = grpc_url();
    eprintln!("[admin_login] connecting to gRPC at {}", url);
    let mut client = AdminServiceClient::connect(url.clone()).await.map_err(|e| {
        eprintln!("[admin_login] connect failed: {}", e);
        e.to_string()
    })?;
    eprintln!("[admin_login] connected, calling AdminLogin RPC");
    let response = client
        .admin_login(tonic::Request::new(AdminLoginRequest { email: email.clone(), password }))
        .await
        .map_err(|e| {
            eprintln!("[admin_login] RPC failed: {}", e);
            e.to_string()
        })?
        .into_inner();
    eprintln!(
        "[admin_login] RPC ok success={} has_token={} error_message={:?}",
        response.success,
        !response.token.is_empty(),
        response.error_message
    );
    Ok(AdminLoginResponse {
        success: response.success,
        token: response.token,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct AdminLoginResponse {
    pub success: bool,
    pub token: String,
    pub error_message: String,
}

/// Tauri command: List all running VMs (requires admin token).
#[tauri::command]
pub async fn list_vms(token: String) -> Result<ListVmsResponse, String> {
    let url = grpc_url();
    let mut client = AdminServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let mut request = tonic::Request::new(ListVmsRequest::default());
    request.metadata_mut().insert("authorization", auth_metadata(&token)?);
    let response = client.list_vms(request).await.map_err(|e| e.to_string())?.into_inner();
    Ok(ListVmsResponse {
        vms: response
            .vms
            .into_iter()
            .map(|v| VmInfoResponse {
                id: v.id,
                hostname: v.hostname,
                dns_name: v.dns_name,
                ip: v.ip,
                subnet: v.subnet,
                gateway: v.gateway,
                cpu_cores: v.cpu_cores,
                memory_mb: v.memory_mb,
                disk_mb: v.disk_mb,
                owner_id: v.owner_id,
                real_memory_bytes: v.real_memory_bytes,
                disk_used_bytes: v.disk_used_bytes,
                ticks_per_second: v.ticks_per_second,
                remaining_ticks: v.remaining_ticks,
            })
            .collect(),
    })
}

#[derive(serde::Serialize)]
pub struct ListVmsResponse {
    pub vms: Vec<VmInfoResponse>,
}

#[derive(serde::Serialize)]
pub struct VmInfoResponse {
    pub id: String,
    pub hostname: String,
    pub dns_name: String,
    pub ip: String,
    pub subnet: String,
    pub gateway: String,
    pub cpu_cores: i32,
    pub memory_mb: i32,
    pub disk_mb: i32,
    pub owner_id: String,
    pub real_memory_bytes: u64,
    pub disk_used_bytes: i64,
    pub ticks_per_second: u32,
    pub remaining_ticks: u32,
}

/// Tauri command: Get cluster stats (requires admin token).
#[tauri::command]
pub async fn get_cluster_stats(token: String) -> Result<GetClusterStatsResponse, String> {
    let url = grpc_url();
    let mut client = AdminServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let mut request = tonic::Request::new(GetClusterStatsRequest::default());
    request.metadata_mut().insert("authorization", auth_metadata(&token)?);
    let response = client
        .get_cluster_stats(request)
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(GetClusterStatsResponse {
        vm_count: response.vm_count,
        tick_count: response.tick_count,
        uptime_secs: response.uptime_secs,
        effective_tps: response.effective_tps,
    })
}

#[derive(serde::Serialize)]
pub struct GetClusterStatsResponse {
    pub vm_count: u64,
    pub tick_count: u64,
    pub uptime_secs: f64,
    pub effective_tps: f64,
}

/// Tauri command: Get network topology (requires admin token).
#[tauri::command]
pub async fn get_network_topology(token: String) -> Result<GetNetworkTopologyResponse, String> {
    let url = grpc_url();
    let mut client = AdminServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let mut request = tonic::Request::new(GetNetworkTopologyRequest::default());
    request.metadata_mut().insert("authorization", auth_metadata(&token)?);
    let response = client
        .get_network_topology(request)
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(GetNetworkTopologyResponse {
        nodes: response
            .nodes
            .into_iter()
            .map(|n| NetworkNodeResponse {
                id: n.id,
                label: n.label,
                ip: n.ip,
                subnet: n.subnet,
                node_type: n.node_type,
            })
            .collect(),
        edges: response
            .edges
            .into_iter()
            .map(|e| NetworkEdgeResponse {
                source_id: e.source_id,
                target_id: e.target_id,
                same_subnet: e.same_subnet,
            })
            .collect(),
    })
}

#[derive(serde::Serialize)]
pub struct GetNetworkTopologyResponse {
    pub nodes: Vec<NetworkNodeResponse>,
    pub edges: Vec<NetworkEdgeResponse>,
}

#[derive(serde::Serialize)]
pub struct NetworkNodeResponse {
    pub id: String,
    pub label: String,
    pub ip: String,
    pub subnet: String,
    pub node_type: String,
}

#[derive(serde::Serialize)]
pub struct NetworkEdgeResponse {
    pub source_id: String,
    pub target_id: String,
    pub same_subnet: bool,
}
