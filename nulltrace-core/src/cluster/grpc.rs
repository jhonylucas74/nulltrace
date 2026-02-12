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
    GetDiskUsageRequest, GetDiskUsageResponse, HelloRequest, HelloResponse, LoginRequest,
    LoginResponse, OpenTerminal, PingRequest, PingResponse, RestoreDiskRequest, RestoreDiskResponse,
    StdinData, StdoutData, TerminalClientMessage, TerminalClosed, TerminalError,
    TerminalOpened, TerminalServerMessage,
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
                error_message: "Username is required".to_string(),
            }));
        }

        let player = self
            .player_service
            .verify_password(&username, &password)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match player {
            Some(p) => Ok(Response::new(LoginResponse {
                success: true,
                player_id: p.id.to_string(),
                error_message: String::new(),
            })),
            None => Ok(Response::new(LoginResponse {
                success: false,
                player_id: String::new(),
                error_message: "Invalid credentials".to_string(),
            })),
        }
    }

    async fn terminal_stream(
        &self,
        request: Request<tonic::Streaming<TerminalClientMessage>>,
    ) -> Result<Response<Self::TerminalStreamStream>, Status> {
        let mut client_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);
        let terminal_hub = Arc::clone(&self.terminal_hub);

        let first = client_stream
            .message()
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?
            .ok_or_else(|| Status::invalid_argument("stream closed before OpenTerminal"))?;
        let player_id_str = match first.msg {
            Some(game::terminal_client_message::Msg::OpenTerminal(OpenTerminal { player_id })) => {
                player_id
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
        let player_id = Uuid::parse_str(&player_id_str)
            .map_err(|_| Status::invalid_argument("invalid player_id uuid"))?;
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
        let player_id_str = request.into_inner().player_id;
        let player_id = Uuid::parse_str(&player_id_str)
            .map_err(|_| Status::invalid_argument("invalid player_id uuid"))?;

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
        let player_id_str = request.into_inner().player_id;
        let player_id = Uuid::parse_str(&player_id_str)
            .map_err(|_| Status::invalid_argument("invalid player_id uuid"))?;

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
