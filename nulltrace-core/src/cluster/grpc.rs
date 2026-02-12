//! gRPC GameService implementation (Ping, Login, SayHello, TerminalStream, GetDiskUsage, RestoreDisk).
//! Used by the unified cluster binary.

use super::bin_programs;
use super::db::fs_service::FsService;
use super::db::player_service::PlayerService;
use super::db::user_service::{UserService, VmUser};
use super::db::vm_service::VmService;
use super::terminal_hub::{SessionReady, TerminalHub};
use game::terminal_server_message::Msg as TerminalServerMsg;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub mod game {
    tonic::include_proto!("game");
}

use game::game_service_server::GameService;
use game::{
    CopyPathRequest, CopyPathResponse, FsEntry, GetDiskUsageRequest, GetDiskUsageResponse,
    GetHomePathRequest, GetHomePathResponse, HelloRequest, HelloResponse, ListFsRequest,
    ListFsResponse, LoginRequest, LoginResponse, MovePathRequest, MovePathResponse, OpenTerminal,
    PingRequest, PingResponse, RefreshTokenRequest, RefreshTokenResponse, RenamePathRequest,
    RenamePathResponse, RestoreDiskRequest, RestoreDiskResponse, StdinData, StdoutData,
    TerminalClientMessage, TerminalClosed, TerminalError, TerminalOpened, TerminalServerMessage,
};

pub struct ClusterGameService {
    player_service: Arc<PlayerService>,
    vm_service: Arc<VmService>,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    terminal_hub: Arc<TerminalHub>,
}

impl ClusterGameService {
    pub fn new(
        player_service: Arc<PlayerService>,
        vm_service: Arc<VmService>,
        fs_service: Arc<FsService>,
        user_service: Arc<UserService>,
        terminal_hub: Arc<TerminalHub>,
    ) -> Self {
        Self {
            player_service,
            vm_service,
            fs_service,
            user_service,
            terminal_hub,
        }
    }

    /// Authenticate a request by validating the JWT token from metadata
    fn authenticate_request<T>(&self, request: &Request<T>) -> Result<crate::auth::Claims, Status> {
        let metadata = request.metadata();
        let token = metadata
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        crate::auth::validate_token(token, &crate::auth::get_jwt_secret())
            .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))
    }
}

#[tonic::async_trait]
impl GameService for ClusterGameService {
    type TerminalStreamStream = ReceiverStream<Result<TerminalServerMessage, Status>>;
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let player_name = request.into_inner().player_name;
        let reply = HelloResponse {
            greeting: format!("Hello, {}! Welcome to the game!", player_name),
        };
        Ok(Response::new(reply))
    }

    async fn ping(
        &self,
        _request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        let server_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(Response::new(PingResponse { server_time_ms }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let LoginRequest { username, password } = request.into_inner();
        let username = username.trim().to_string();
        if username.is_empty() {
            return Ok(Response::new(LoginResponse {
                success: false,
                player_id: String::new(),
                token: String::new(),
                error_message: "Username is required".to_string(),
            }));
        }

        let player = self
            .player_service
            .verify_password(&username, &password)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match player {
            Some(p) => {
                // Generate JWT token
                let token = crate::auth::generate_token(
                    p.id,
                    &p.username,
                    &crate::auth::get_jwt_secret(),
                )
                .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

                Ok(Response::new(LoginResponse {
                    success: true,
                    player_id: p.id.to_string(),
                    token,
                    error_message: String::new(),
                }))
            }
            None => Ok(Response::new(LoginResponse {
                success: false,
                player_id: String::new(),
                token: String::new(),
                error_message: "Invalid credentials".to_string(),
            })),
        }
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let current_token = request.into_inner().current_token;

        // Validate current token
        let claims = crate::auth::validate_token(&current_token, &crate::auth::get_jwt_secret())
            .map_err(|_| Status::unauthenticated("Invalid token"))?;

        // Parse player_id from claims
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        // Issue new token with fresh 24-hour expiry
        let new_token = crate::auth::generate_token(
            player_id,
            &claims.username,
            &crate::auth::get_jwt_secret(),
        )
        .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

        Ok(Response::new(RefreshTokenResponse {
            success: true,
            token: new_token,
            error_message: String::new(),
        }))
    }

    async fn terminal_stream(
        &self,
        request: Request<tonic::Streaming<TerminalClientMessage>>,
    ) -> Result<Response<Self::TerminalStreamStream>, Status> {
        // Authenticate request before processing stream
        let claims = self.authenticate_request(&request)?;
        let authenticated_player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let mut client_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);
        let terminal_hub = Arc::clone(&self.terminal_hub);

        let first = client_stream
            .message()
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?
            .ok_or_else(|| Status::invalid_argument("stream closed before OpenTerminal"))?;
        let _player_id_str = match first.msg {
            Some(game::terminal_client_message::Msg::OpenTerminal(OpenTerminal { player_id: _ })) => {
                // Ignore player_id from message, use authenticated one
                authenticated_player_id.to_string()
            }
            _ => {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                            message: "first message must be OpenTerminal".to_string(),
                        })),
                    }))
                    .await;
                return Ok(Response::new(ReceiverStream::new(rx)));
            }
        };
        // Use authenticated player_id
        let player_id = authenticated_player_id;
        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut hub = terminal_hub.lock().unwrap();
            hub.pending_opens.push((player_id, response_tx));
        }
        let ready: SessionReady = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            response_rx,
        )
        .await
        {
            Ok(Ok(Ok(ready))) => ready,
            Ok(Ok(Err(e))) => {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                            message: e,
                        })),
                    }))
                    .await;
                return Ok(Response::new(ReceiverStream::new(rx)));
            }
            Ok(Err(_)) | Err(_) => {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                            message: "terminal open timeout or channel closed".to_string(),
                        })),
                    }))
                    .await;
                return Ok(Response::new(ReceiverStream::new(rx)));
            }
        };
        let _ = tx
            .send(Ok(TerminalServerMessage {
                msg: Some(TerminalServerMsg::TerminalOpened(TerminalOpened {
                    session_id: ready.session_id.to_string(),
                })),
            }))
            .await;

        let hub_remove = Arc::clone(&terminal_hub);
        let stdin_tx = ready.stdin_tx;
        let sid = ready.session_id;
        tokio::spawn(async move {
            while let Ok(Some(msg)) = client_stream.message().await {
                if let Some(game::terminal_client_message::Msg::Stdin(StdinData { data })) = msg.msg
                {
                    if let Ok(s) = String::from_utf8(data) {
                        let _ = stdin_tx.send(s).await;
                    }
                }
            }
            drop(stdin_tx);
            let mut hub = hub_remove.lock().unwrap();
            hub.sessions.remove(&sid);
        });

        let mut stdout_rx = ready.stdout_rx;
        tokio::spawn(async move {
            while let Some(chunk) = stdout_rx.recv().await {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::Stdout(StdoutData {
                            data: chunk.into_bytes(),
                        })),
                    }))
                    .await;
            }
            let _ = tx
                .send(Ok(TerminalServerMessage {
                    msg: Some(TerminalServerMsg::TerminalClosed(TerminalClosed {})),
                }))
                .await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_disk_usage(
        &self,
        request: Request<GetDiskUsageRequest>,
    ) -> Result<Response<GetDiskUsageResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let used_bytes = self
            .fs_service
            .disk_usage_bytes(vm.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let total_bytes = (vm.disk_mb as i64) * 1024 * 1024;

        Ok(Response::new(GetDiskUsageResponse {
            used_bytes,
            total_bytes,
            error_message: String::new(),
        }))
    }

    async fn restore_disk(
        &self,
        request: Request<RestoreDiskRequest>,
    ) -> Result<Response<RestoreDiskResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let record = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let vm_id = record.id;

        // Delete phase: remove all fs_nodes (cascade fs_contents), then vm_users
        self.fs_service
            .destroy_fs(vm_id)
            .await
            .map_err(|e| Status::internal(format!("destroy_fs: {}", e)))?;

        self.user_service
            .delete_all_for_vm(vm_id)
            .await
            .map_err(|e| Status::internal(format!("delete_all_for_vm: {}", e)))?;

        // Bootstrap phase: same as create_vm (minus NIC/DNS)
        self.fs_service
            .bootstrap_fs(vm_id)
            .await
            .map_err(|e| Status::internal(format!("bootstrap_fs: {}", e)))?;

        for (name, source) in bin_programs::DEFAULT_BIN_PROGRAMS {
            let path = format!("/bin/{}", name);
            self.fs_service
                .write_file(
                    vm_id,
                    &path,
                    source.as_bytes(),
                    Some("application/x-nulltrace-lua"),
                    "root",
                )
                .await
                .map_err(|e| Status::internal(format!("write {}: {}", path, e)))?;
        }

        let mut users: Vec<VmUser> = self
            .user_service
            .bootstrap_users(vm_id)
            .await
            .map_err(|e| Status::internal(format!("bootstrap_users: {}", e)))?;

        if let Some(owner_id) = record.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            let owner_home = format!("/home/{}", player.username);
            let owner_user = self
                .user_service
                .create_user(
                    vm_id,
                    &player.username,
                    1001,
                    Some(&player.password_hash),
                    true,
                    &owner_home,
                    "/bin/sh",
                )
                .await
                .map_err(|e| Status::internal(format!("create_user: {}", e)))?;
            users.push(owner_user);
        }

        for user in &users {
            if self
                .fs_service
                .resolve_path(vm_id, &user.home_dir)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .is_none()
            {
                self.fs_service
                    .mkdir(vm_id, &user.home_dir, &user.username)
                    .await
                    .map_err(|e| Status::internal(format!("mkdir: {}", e)))?;
            }
            self.fs_service
                .ensure_standard_home_subdirs(vm_id, &user.home_dir, &user.username)
                .await
                .map_err(|e| Status::internal(format!("ensure_standard_home_subdirs: {}", e)))?;
        }

        let passwd_content: String = users
            .iter()
            .map(|u| {
                let gid = if u.is_root { 0 } else { u.uid };
                format!(
                    "{}:x:{}:{}:{}:{}:{}",
                    u.username, u.uid, gid, u.username, u.home_dir, u.shell
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(vm_id, "/etc/passwd", passwd_content.as_bytes(), Some("text/plain"), "root")
            .await
            .map_err(|e| Status::internal(format!("write /etc/passwd: {}", e)))?;

        let shadow_content: String = users
            .iter()
            .map(|u| {
                let hash = u.password_hash.as_deref().unwrap_or("!");
                format!("{}:{}:19000:0:99999:7:::", u.username, hash)
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(
                vm_id,
                "/etc/shadow",
                shadow_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await
            .map_err(|e| Status::internal(format!("write /etc/shadow: {}", e)))?;

        Ok(Response::new(RestoreDiskResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_home_path(
        &self,
        request: Request<GetHomePathRequest>,
    ) -> Result<Response<GetHomePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let home_path = if let Some(owner_id) = vm.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            format!("/home/{}", player.username)
        } else {
            "/home/user".to_string()
        };

        Ok(Response::new(GetHomePathResponse {
            home_path,
            error_message: String::new(),
        }))
    }

    async fn list_fs(
        &self,
        request: Request<ListFsRequest>,
    ) -> Result<Response<ListFsResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let ListFsRequest { path, .. } = request.into_inner();

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let home_path = if let Some(owner_id) = vm.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            format!("/home/{}", player.username)
        } else {
            "/home/user".to_string()
        };

        if !path_under_home(&path, &home_path) {
            return Ok(Response::new(ListFsResponse {
                entries: vec![],
                error_message: "Path must be under home".to_string(),
            }));
        }

        let entries = self
            .fs_service
            .ls(vm.id, &path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let proto_entries: Vec<FsEntry> = entries
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                node_type: e.node_type,
                size_bytes: e.size_bytes,
            })
            .collect();

        Ok(Response::new(ListFsResponse {
            entries: proto_entries,
            error_message: String::new(),
        }))
    }

    async fn copy_path(
        &self,
        request: Request<CopyPathRequest>,
    ) -> Result<Response<CopyPathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let CopyPathRequest {
            src_path,
            dest_path,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&src_path, &owner.1) || !path_under_home(&dest_path, &owner.1) {
            return Ok(Response::new(CopyPathResponse {
                success: false,
                error_message: "Paths must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .copy_path_recursive(vm.id, &src_path, &dest_path, &owner.0)
            .await
        {
            Ok(()) => Ok(Response::new(CopyPathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(CopyPathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn move_path(
        &self,
        request: Request<MovePathRequest>,
    ) -> Result<Response<MovePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let MovePathRequest {
            src_path,
            dest_path,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&src_path, &owner.1) || !path_under_home(&dest_path, &owner.1) {
            return Ok(Response::new(MovePathResponse {
                success: false,
                error_message: "Paths must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .move_node(vm.id, &src_path, &dest_path)
            .await
        {
            Ok(()) => Ok(Response::new(MovePathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(MovePathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn rename_path(
        &self,
        request: Request<RenamePathRequest>,
    ) -> Result<Response<RenamePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let RenamePathRequest {
            path,
            new_name,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&path, &owner.1) {
            return Ok(Response::new(RenamePathResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .rename_node(vm.id, &path, &new_name)
            .await
        {
            Ok(()) => Ok(Response::new(RenamePathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(RenamePathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }
}

/// Returns true if path is under home (normalized, no traversal).
fn path_under_home(path: &str, home: &str) -> bool {
    let path_norm = normalize_path(path);
    let home_norm = normalize_path(home);
    path_norm == home_norm
        || (path_norm.starts_with(&home_norm) && path_norm.len() > home_norm.len() && path_norm.as_bytes().get(home_norm.len()) == Some(&b'/'))
}

fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    let mut resolved: Vec<&str> = Vec::new();
    for p in parts {
        if p == ".." {
            resolved.pop();
        } else {
            resolved.push(p);
        }
    }
    if resolved.is_empty() {
        "/".to_string()
    } else {
        "/".to_string() + &resolved.join("/")
    }
}

async fn vm_and_owner(
    svc: &ClusterGameService,
    player_id: Uuid,
) -> Result<
    (
        super::db::vm_service::VmRecord,
        (String, String),
    ),
    Status,
> {
    let vm = svc
        .vm_service
        .get_vm_by_owner_id(player_id)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::not_found("No VM found for this player"))?;
    let home_path = if let Some(owner_id) = vm.owner_id {
        let player = svc
            .player_service
            .get_by_id(owner_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::internal("Owner player not found"))?;
        (player.username.clone(), format!("/home/{}", player.username))
    } else {
        ("user".to_string(), "/home/user".to_string())
    };
    Ok((vm, home_path))
}

#[cfg(test)]
mod tests {
    use super::super::db::{
        self, fs_service::FsService, player_service::PlayerService, user_service::UserService,
        vm_service::VmService,
    };
    use super::super::terminal_hub::new_hub;
    use super::*;
    use std::sync::Arc;
    use tonic::Request;

    fn test_cluster_game_service(pool: &sqlx::PgPool) -> ClusterGameService {
        ClusterGameService::new(
            Arc::new(PlayerService::new(pool.clone())),
            Arc::new(VmService::new(pool.clone())),
            Arc::new(FsService::new(pool.clone())),
            Arc::new(UserService::new(pool.clone())),
            new_hub(),
        )
    }

    #[tokio::test]
    async fn test_grpc_login_success() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let name = format!("grpcuser_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "secret").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let res = svc
            .login(Request::new(LoginRequest {
                username: name.clone(),
                password: "secret".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(out.success);
        assert!(!out.player_id.is_empty());
        assert!(out.error_message.is_empty());
    }

    #[tokio::test]
    async fn test_grpc_login_wrong_password() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let name = format!("grpcwrong_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "right").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let res = svc
            .login(Request::new(LoginRequest {
                username: name,
                password: "wrong".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(!out.success);
        assert!(out.player_id.is_empty());
        assert_eq!(out.error_message, "Invalid credentials");
    }

    #[tokio::test]
    async fn test_grpc_login_empty_username() {
        let pool = db::test_pool().await;
        let svc = test_cluster_game_service(&pool);

        let res = svc
            .login(Request::new(LoginRequest {
                username: "".to_string(),
                password: "any".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(!out.success);
        assert_eq!(out.error_message, "Username is required");
    }

    #[tokio::test]
    async fn test_grpc_ping_returns_time() {
        let pool = db::test_pool().await;
        let svc = test_cluster_game_service(&pool);

        let res = svc.ping(Request::new(PingRequest {})).await.unwrap();
        let out = res.into_inner();
        assert!(out.server_time_ms > 0);
    }
}
