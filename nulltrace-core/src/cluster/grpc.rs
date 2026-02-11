//! gRPC GameService implementation (Ping, Login, SayHello). Used by the unified cluster binary.

use super::db::player_service::PlayerService;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub mod game {
    tonic::include_proto!("game");
}

use game::game_service_server::GameService;
use game::{
    HelloRequest, HelloResponse, LoginRequest, LoginResponse, PingRequest, PingResponse,
};

pub struct ClusterGameService {
    player_service: Arc<PlayerService>,
}

impl ClusterGameService {
    pub fn new(player_service: Arc<PlayerService>) -> Self {
        Self { player_service }
    }
}

#[tonic::async_trait]
impl GameService for ClusterGameService {
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
}

#[cfg(test)]
mod tests {
    use super::super::db::{self, player_service::PlayerService};
    use super::*;
    use std::sync::Arc;
    use tonic::Request;

    #[tokio::test]
    async fn test_grpc_login_success() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool));
        let name = format!("grpcuser_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "secret").await.unwrap();

        let svc = ClusterGameService::new(player_service);
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

        let svc = ClusterGameService::new(player_service);
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
        let svc = ClusterGameService::new(player_service);

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
        let svc = ClusterGameService::new(player_service);

        let res = svc.ping(Request::new(PingRequest {})).await.unwrap();
        let out = res.into_inner();
        assert!(out.server_time_ms > 0);
    }
}
