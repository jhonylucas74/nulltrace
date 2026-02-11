use game::game_service_server::{GameService, GameServiceServer};
use game::{
    HelloRequest, HelloResponse, LoginRequest, LoginResponse, PingRequest, PingResponse,
};
use tonic::{Request, Response, Status, transport::Server};

pub mod game {
    tonic::include_proto!("game");
}

#[derive(Default)]
pub struct MyGameService {}

#[tonic::async_trait]
impl GameService for MyGameService {
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
