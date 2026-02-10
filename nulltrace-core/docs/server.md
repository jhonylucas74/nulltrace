# Server & gRPC

> Files: `src/server/main.rs`, `proto/game.proto`

The `server` binary exposes a gRPC API for external communication with the VM cluster.

## Protocol (Protobuf)

```protobuf
syntax = "proto3";
package game;

service GameService {
  rpc SayHello (HelloRequest) returns (HelloResponse);
}

message HelloRequest {
  string player_name = 1;
}

message HelloResponse {
  string greeting = 1;
}
```

Currently the service has only an example RPC (`SayHello`). The plan is to expand to:
- Remotely create/destroy VMs
- Send Lua scripts for execution
- Query cluster state and metrics

## Server

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    Server::builder()
        .add_service(GameServiceServer::new(MyGameService::default()))
        .serve(addr)
        .await?;
}
```

- **Address**: `[::1]:50051` (IPv6 localhost)
- **Framework**: `tonic` (gRPC for Rust)
- **Serialization**: `prost` (protobuf)
- **Build**: `build.rs` automatically compiles the `.proto` into Rust code

## Infrastructure

The `docker-compose.yml` provisions a Redis instance for inter-node communication:

```yaml
services:
  redis:
    image: redis:latest
    ports:
      - '6379:6379'
```

Redis will be used to coordinate multiple cluster nodes (distributing VMs across servers).
