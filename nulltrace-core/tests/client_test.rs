use tonic::transport::Channel;
use game::game_service_client::GameServiceClient;
use game::HelloRequest;

pub mod game {
    tonic::include_proto!("game");
}

#[tokio::test]
async fn test_say_hello() {
    let mut client = GameServiceClient::connect("http://[::1]:50051")
        .await
        .expect("Failed to connect to server");

    let request = tonic::Request::new(HelloRequest {
        player_name: "Jhony".to_string(),
    });

    let response = client.say_hello(request).await.expect("RPC failed");

    assert_eq!(
        response.into_inner().greeting,
        "Hello, Jhony! Welcome to the game!"
    );
}
