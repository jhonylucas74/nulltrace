//! gRPC GameService implementation (Ping, Login, SayHello, TerminalStream). Used by the unified cluster binary.

use super::db::player_service::PlayerService;
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
    HelloRequest, HelloResponse, LoginRequest, LoginResponse, OpenTerminal, PingRequest,
    PingResponse, StdinData, StdoutData, TerminalClientMessage, TerminalClosed, TerminalError,
    TerminalOpened, TerminalServerMessage,
};

pub struct ClusterGameService {
    player_service: Arc<PlayerService>,
    terminal_hub: Arc<TerminalHub>,
}

impl ClusterGameService {
    pub fn new(player_service: Arc<PlayerService>, terminal_hub: Arc<TerminalHub>) -> Self {
        Self {
            player_service,
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
}

#[cfg(test)]
mod tests {
    use super::super::db::{self, player_service::PlayerService};
    use super::super::terminal_hub::new_hub;
    use super::*;
    use std::sync::Arc;
    use tonic::Request;

    #[tokio::test]
    async fn test_grpc_login_success() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool));
        let name = format!("grpcuser_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "secret").await.unwrap();

        let svc = ClusterGameService::new(player_service, new_hub());
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
        let player_service = Arc::new(PlayerService::new(pool));
        let name = format!("grpcwrong_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "right").await.unwrap();

        let svc = ClusterGameService::new(player_service, new_hub());
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
        let player_service = Arc::new(PlayerService::new(pool));
        let svc = ClusterGameService::new(player_service, new_hub());

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
        let player_service = Arc::new(PlayerService::new(pool));
        let svc = ClusterGameService::new(player_service, new_hub());

        let res = svc.ping(Request::new(PingRequest {})).await.unwrap();
        let out = res.into_inner();
        assert!(out.server_time_ms > 0);
    }
}
