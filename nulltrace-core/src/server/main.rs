use game::game_service_server::{GameService, GameServiceServer};
use game::{HelloRequest, HelloResponse};
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
