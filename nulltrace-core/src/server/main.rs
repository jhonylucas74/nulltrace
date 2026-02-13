use game::game_service_server::{GameService, GameServiceServer};
use game::{
    CopyPathRequest, CopyPathResponse, CreateFactionRequest, CreateFactionResponse,
    GetDiskUsageRequest, GetDiskUsageResponse, GetHomePathRequest, GetHomePathResponse,
    GetPlayerProfileRequest, GetPlayerProfileResponse, GetProcessListRequest, GetProcessListResponse,
    GetRankingRequest, GetRankingResponse, HelloRequest, HelloResponse, LeaveFactionRequest,
    LeaveFactionResponse, ListFsRequest, ListFsResponse, LoginRequest, LoginResponse,
    MovePathRequest, MovePathResponse, PingRequest, PingResponse, RefreshTokenRequest,
    RefreshTokenResponse, RenamePathRequest, RenamePathResponse, RestoreDiskRequest,
    RestoreDiskResponse, TerminalClientMessage, TerminalServerMessage,
};
use tonic::{Request, Response, Status, transport::Server};
use tokio_stream::wrappers::ReceiverStream;

pub mod game {
    tonic::include_proto!("game");
}

use game::terminal_server_message::Msg as TerminalServerMsg;
use game::TerminalError;

#[derive(Default)]
pub struct MyGameService {}

type TerminalStreamStream = ReceiverStream<Result<TerminalServerMessage, Status>>;

#[tonic::async_trait]
impl GameService for MyGameService {
    type TerminalStreamStream = TerminalStreamStream;

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
        _request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        // Standalone server has no DB; always reject login.
        Ok(Response::new(LoginResponse {
            success: false,
            player_id: String::new(),
            token: String::new(),
            error_message: "Use the unified cluster binary for login".to_string(),
        }))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(RefreshTokenResponse {
            success: false,
            token: String::new(),
            error_message: "Use the unified cluster binary for token refresh".to_string(),
        }))
    }

    async fn terminal_stream(
        &self,
        _request: Request<tonic::Streaming<TerminalClientMessage>>,
    ) -> Result<Response<TerminalStreamStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(TerminalServerMessage {
                msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                    message: "Use the unified cluster binary for terminal".to_string(),
                })),
            }))
            .await;
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_disk_usage(
        &self,
        _request: Request<GetDiskUsageRequest>,
    ) -> Result<Response<GetDiskUsageResponse>, Status> {
        Ok(Response::new(GetDiskUsageResponse {
            used_bytes: 0,
            total_bytes: 0,
            error_message: "Use the unified cluster binary for disk operations".to_string(),
        }))
    }

    async fn restore_disk(
        &self,
        _request: Request<RestoreDiskRequest>,
    ) -> Result<Response<RestoreDiskResponse>, Status> {
        Ok(Response::new(RestoreDiskResponse {
            success: false,
            error_message: "Use the unified cluster binary for disk operations".to_string(),
        }))
    }

    async fn get_home_path(
        &self,
        _request: Request<GetHomePathRequest>,
    ) -> Result<Response<GetHomePathResponse>, Status> {
        Ok(Response::new(GetHomePathResponse {
            home_path: String::new(),
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn list_fs(
        &self,
        _request: Request<ListFsRequest>,
    ) -> Result<Response<ListFsResponse>, Status> {
        Ok(Response::new(ListFsResponse {
            entries: vec![],
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn copy_path(
        &self,
        _request: Request<CopyPathRequest>,
    ) -> Result<Response<CopyPathResponse>, Status> {
        Ok(Response::new(CopyPathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn move_path(
        &self,
        _request: Request<MovePathRequest>,
    ) -> Result<Response<MovePathResponse>, Status> {
        Ok(Response::new(MovePathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn rename_path(
        &self,
        _request: Request<RenamePathRequest>,
    ) -> Result<Response<RenamePathResponse>, Status> {
        Ok(Response::new(RenamePathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn get_process_list(
        &self,
        _request: Request<GetProcessListRequest>,
    ) -> Result<Response<GetProcessListResponse>, Status> {
        Ok(Response::new(GetProcessListResponse {
            processes: vec![],
            disk_used_bytes: 0,
            disk_total_bytes: 0,
            error_message: "Use the unified cluster binary for process list".to_string(),
        }))
    }

    async fn get_sysinfo(
        &self,
        _request: Request<game::GetSysinfoRequest>,
    ) -> Result<Response<game::GetSysinfoResponse>, Status> {
        Ok(Response::new(game::GetSysinfoResponse {
            cpu_cores: 0,
            memory_mb: 0,
            disk_mb: 0,
            error_message: "Use the unified cluster binary for sysinfo".to_string(),
        }))
    }

    async fn get_ranking(
        &self,
        _request: Request<GetRankingRequest>,
    ) -> Result<Response<GetRankingResponse>, Status> {
        Ok(Response::new(GetRankingResponse {
            entries: vec![],
            error_message: "Use the unified cluster binary for ranking".to_string(),
        }))
    }

    async fn get_player_profile(
        &self,
        _request: Request<GetPlayerProfileRequest>,
    ) -> Result<Response<GetPlayerProfileResponse>, Status> {
        Ok(Response::new(GetPlayerProfileResponse {
            rank: 0,
            points: 0,
            faction_id: String::new(),
            faction_name: String::new(),
            error_message: "Use the unified cluster binary for profile".to_string(),
        }))
    }

    async fn create_faction(
        &self,
        _request: Request<CreateFactionRequest>,
    ) -> Result<Response<CreateFactionResponse>, Status> {
        Ok(Response::new(CreateFactionResponse {
            faction_id: String::new(),
            name: String::new(),
            error_message: "Use the unified cluster binary for factions".to_string(),
        }))
    }

    async fn leave_faction(
        &self,
        _request: Request<LeaveFactionRequest>,
    ) -> Result<Response<LeaveFactionResponse>, Status> {
        Ok(Response::new(LeaveFactionResponse {
            success: false,
            error_message: "Use the unified cluster binary for factions".to_string(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let game_service = MyGameService::default();

    println!("Server listening on {}", addr);

    Server::builder()
        .add_service(GameServiceServer::new(game_service))
        .serve(addr)
        .await?;

    Ok(())
}
