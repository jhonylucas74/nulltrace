use game::game_service_server::{GameService, GameServiceServer};
use game::{
    HelloRequest, HelloResponse, LoginRequest, LoginResponse, PingRequest, PingResponse,
    TerminalClientMessage, TerminalServerMessage,
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
            error_message: "Use the unified cluster binary for login".to_string(),
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
