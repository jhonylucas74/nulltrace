//! gRPC client for GameService (Ping, Login). Used by Tauri commands to communicate with nulltrace-core.

mod game {
    tonic::include_proto!("game");
}

use game::game_service_client::GameServiceClient;
use game::{LoginRequest, PingRequest};

/// Default gRPC backend URL. Overridable via env for custom deployments.
fn grpc_url() -> String {
    std::env::var("NULLTRACE_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

/// Response for grpc_login command.
#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub player_id: String,
    pub error_message: String,
}

/// Response for grpc_ping command.
#[derive(serde::Serialize)]
pub struct PingResponse {
    pub server_time_ms: i64,
}

/// Tauri command: Ping the backend. Returns server time in ms.
#[tauri::command]
pub async fn grpc_ping() -> Result<PingResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .ping(tonic::Request::new(PingRequest {}))
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(PingResponse {
        server_time_ms: response.server_time_ms,
    })
}

/// Tauri command: Login with username and password.
#[tauri::command]
pub async fn grpc_login(username: String, password: String) -> Result<LoginResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .login(tonic::Request::new(LoginRequest { username, password }))
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(LoginResponse {
        success: response.success,
        player_id: response.player_id,
        error_message: response.error_message,
    })
}
