//! gRPC AdminService implementation. Admin-only API for cluster management.

use super::admin_auth;
use super::cluster_snapshot;
use super::db::admin_service::AdminService as AdminDbService;
use admin::admin_service_server::AdminService as AdminServiceTrait;
use admin::{
    AdminLoginRequest, AdminLoginResponse, GetClusterStatsRequest, GetClusterStatsResponse,
    GetNetworkTopologyRequest, GetNetworkTopologyResponse, ListVmsRequest, ListVmsResponse,
    NetworkEdge, NetworkNode, VmInfo,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub mod admin {
    tonic::include_proto!("admin");
}

/// Authenticate admin request by validating JWT with role "admin".
fn authenticate_admin_request<T>(request: &Request<T>) -> Result<admin_auth::AdminClaims, Status> {
    let metadata = request.metadata();
    let token = metadata
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

    admin_auth::validate_admin_token(token, &crate::auth::get_jwt_secret())
        .map_err(|e| Status::unauthenticated(format!("Invalid admin token: {}", e)))
}

pub struct ClusterAdminService {
    admin_service: Arc<AdminDbService>,
    cluster_snapshot: Arc<std::sync::RwLock<cluster_snapshot::ClusterSnapshot>>,
}

impl ClusterAdminService {
    pub fn new(
        admin_service: Arc<AdminDbService>,
        cluster_snapshot: Arc<std::sync::RwLock<cluster_snapshot::ClusterSnapshot>>,
    ) -> Self {
        Self {
            admin_service,
            cluster_snapshot,
        }
    }
}

#[tonic::async_trait]
impl AdminServiceTrait for ClusterAdminService {
    async fn admin_login(
        &self,
        request: Request<AdminLoginRequest>,
    ) -> Result<Response<AdminLoginResponse>, Status> {
        let AdminLoginRequest { email, password } = request.into_inner();

        if email.is_empty() || password.is_empty() {
            return Ok(Response::new(AdminLoginResponse {
                success: false,
                token: String::new(),
                error_message: "Email and password are required".to_string(),
            }));
        }

        let admin = self
            .admin_service
            .verify_password(&email, &password)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::unauthenticated("Invalid email or password"))?;

        let token = admin_auth::generate_admin_token(
            admin.id,
            &admin.email,
            &crate::auth::get_jwt_secret(),
        )
        .map_err(|e| Status::internal(format!("Failed to generate token: {}", e)))?;

        Ok(Response::new(AdminLoginResponse {
            success: true,
            token,
            error_message: String::new(),
        }))
    }

    async fn list_vms(
        &self,
        request: Request<ListVmsRequest>,
    ) -> Result<Response<ListVmsResponse>, Status> {
        authenticate_admin_request(&request)?;
        let snap = self
            .cluster_snapshot
            .read()
            .map_err(|e| Status::internal(format!("Snapshot lock error: {}", e)))?;
        let vms: Vec<VmInfo> = snap
            .vms
            .iter()
            .map(|v| VmInfo {
                id: v.id.to_string(),
                hostname: v.hostname.clone(),
                dns_name: v.dns_name.clone().unwrap_or_default(),
                ip: v.ip.clone().unwrap_or_default(),
                subnet: v.subnet.clone().unwrap_or_default(),
                gateway: v.gateway.clone().unwrap_or_default(),
                cpu_cores: v.cpu_cores as i32,
                memory_mb: v.memory_mb,
                disk_mb: v.disk_mb,
                owner_id: v.owner_id.map(|u| u.to_string()).unwrap_or_default(),
                real_memory_bytes: v.real_memory_bytes,
                disk_used_bytes: v.disk_used_bytes,
                ticks_per_second: v.ticks_per_second,
                remaining_ticks: v.remaining_ticks,
            })
            .collect();
        Ok(Response::new(ListVmsResponse { vms }))
    }

    async fn get_cluster_stats(
        &self,
        request: Request<GetClusterStatsRequest>,
    ) -> Result<Response<GetClusterStatsResponse>, Status> {
        authenticate_admin_request(&request)?;
        let snap = self
            .cluster_snapshot
            .read()
            .map_err(|e| Status::internal(format!("Snapshot lock error: {}", e)))?;
        Ok(Response::new(GetClusterStatsResponse {
            vm_count: snap.vms.len() as u64,
            tick_count: snap.tick_count,
            uptime_secs: snap.uptime_secs,
            effective_tps: snap.effective_tps(),
        }))
    }

    async fn get_network_topology(
        &self,
        request: Request<GetNetworkTopologyRequest>,
    ) -> Result<Response<GetNetworkTopologyResponse>, Status> {
        authenticate_admin_request(&request)?;
        let snap = self
            .cluster_snapshot
            .read()
            .map_err(|e| Status::internal(format!("Snapshot lock error: {}", e)))?;
        let (topology_nodes, topology_edges) = snap.build_network_topology();
        let nodes: Vec<NetworkNode> = topology_nodes
            .into_iter()
            .map(|n| NetworkNode {
                id: n.id,
                label: n.label,
                ip: n.ip,
                subnet: n.subnet,
                node_type: n.node_type,
            })
            .collect();
        let edges: Vec<NetworkEdge> = topology_edges
            .into_iter()
            .map(|e| NetworkEdge {
                source_id: e.source_id,
                target_id: e.target_id,
                same_subnet: e.same_subnet,
            })
            .collect();
        Ok(Response::new(GetNetworkTopologyResponse { nodes, edges }))
    }
}
