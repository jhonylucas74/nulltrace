# gRPC Integration - Frontend to Backend Communication

This document explains how the NullTrace application implements gRPC communication between the React frontend (via Tauri) and the Rust backend (cluster server).

## Architecture Overview

The system uses **native gRPC** (HTTP/2) for communication. The architecture consists of:

- **Frontend**: React application invoking Tauri commands for gRPC actions
- **Tauri (Rust)**: gRPC client using Tonic; exposes `grpc_ping` and `grpc_login` commands
- **Backend**: Rust server using Tonic with gRPC-Web layer (for any future non-Tauri clients)
- **Protocol**: Protocol Buffers (protobuf) for message serialization
- **Transport**: HTTP/2 for Tauri→backend; Tauri commands bridge JavaScript↔Rust

## Protocol Definition

The communication protocol is defined in `game.proto`:

```protobuf
service GameService {
  rpc SayHello (HelloRequest) returns (HelloResponse);
  rpc Ping (PingRequest) returns (PingResponse);
  rpc Login (LoginRequest) returns (LoginResponse);
}
```

### Available RPCs

1. **Ping** - Health check that returns server timestamp
2. **Login** - User authentication with username/password
3. **SayHello** - Simple greeting service (demo)

## Backend Implementation

### Server Setup (nulltrace-core/src/cluster/main.rs)

The Rust backend uses **Tonic** with the **GrpcWebLayer** to handle both native gRPC and gRPC-Web:

```rust
let grpc_addr = "[::1]:50051".parse().unwrap();
let game_svc = ClusterGameService::new(player_service.clone());
let game_server = GameServiceServer::new(game_svc);

tokio::spawn(async move {
    Server::builder()
        .accept_http1(true)              // Enable HTTP/1.1 for gRPC-Web
        .layer(GrpcWebLayer::new())      // Add gRPC-Web support
        .add_service(game_server)
        .serve(grpc_addr)
        .await
        .expect("gRPC server failed");
});
```

**Key configuration:**
- Listens on `[::1]:50051` (localhost IPv6)
- Accepts HTTP/1.1 (gRPC-Web) and HTTP/2 (native gRPC)
- Uses `GrpcWebLayer` for gRPC-Web compatibility

### Service Implementation (nulltrace-core/src/cluster/grpc.rs)

The `ClusterGameService` struct implements the `GameService` trait. See the source for full implementation.

## Tauri gRPC Client

### Location

- **Client**: `nulltrace-client/src-tauri/src/grpc.rs`
- **Proto**: `nulltrace-client/proto/game.proto` (compiled via `build.rs`)

### Build Configuration (build.rs)

```rust
tonic_build::compile_protos("../proto/game.proto").expect("Failed to compile game.proto");
```

### Tauri Commands

```rust
#[tauri::command]
pub async fn grpc_ping() -> Result<PingResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client.ping(tonic::Request::new(PingRequest {})).await?;
    Ok(PingResponse { server_time_ms: response.into_inner().server_time_ms })
}

#[tauri::command]
pub async fn grpc_login(username: String, password: String) -> Result<LoginResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client.login(tonic::Request::new(LoginRequest { username, password })).await?;
    let inner = response.into_inner();
    Ok(LoginResponse {
        success: inner.success,
        player_id: inner.player_id,
        error_message: inner.error_message,
    })
}
```

### URL Configuration

The gRPC backend URL is configurable via the `NULLTRACE_GRPC_URL` environment variable. Default: `http://127.0.0.1:50051`.

## Frontend Implementation

### GrpcContext (nulltrace-client/src/contexts/GrpcContext.tsx)

The `GrpcContext` provides gRPC functionality by invoking Tauri commands:

```typescript
import { invoke } from "@tauri-apps/api/core";

export interface GrpcContextValue {
  ping: () => Promise<PingResponseMessage>;
  login: (username: string, password: string) => Promise<LoginResponseMessage>;
}

export function GrpcProvider({ children }: { children: React.ReactNode }) {
  const value = useMemo<GrpcContextValue>(() => ({
    ping: () => invoke<PingResponseMessage>("grpc_ping"),
    login: (username: string, password: string) =>
      invoke<LoginResponseMessage>("grpc_login", { username, password }),
  }), []);

  return <GrpcContext.Provider value={value}>{children}</GrpcContext.Provider>;
}
```

**Usage in components:**
```typescript
const { login } = useGrpc();

const handleLogin = async () => {
  const response = await login(username, password);
  if (response.success) {
    // Handle successful login
  } else {
    // Show error message
    alert(response.error_message);
  }
};
```

**Note:** The app must run inside Tauri for gRPC to work. The `invoke()` calls will fail when running in a bare browser (e.g. `npm run dev` without Tauri).

## Communication Flow

### Example: Login Request

1. **User initiates login:**
   ```typescript
   const response = await login("haru", "password123");
   ```

2. **Frontend invokes Tauri command:**
   ```typescript
   invoke("grpc_login", { username: "haru", password: "password123" })
   ```

3. **Tauri (Rust) makes gRPC call:**
   - Connects to `http://127.0.0.1:50051` via tonic
   - Sends `LoginRequest` over HTTP/2
   - Receives `LoginResponse`

4. **Backend processes:**
   - Deserializes protobuf message
   - Calls `ClusterGameService::login()`
   - Queries database via `PlayerService`
   - Returns `LoginResponse`

5. **Frontend receives response:**
   - Tauri returns serialized object to JavaScript
   - Typed response is available to the application

## Protocol Buffers

### Schema Location

- **Proto file**: `nulltrace-client/proto/game.proto` (and `nulltrace-core/proto/game.proto` - keep in sync)
- **Tauri**: Compiles via `tonic-build` in `build.rs`
- **Backend**: Compiles via `tonic-build` in nulltrace-core

### Type Safety

Both sides have strongly-typed messages:

**TypeScript (GrpcContext):**
```typescript
export interface LoginResponseMessage {
  success: boolean;
  player_id: string;
  error_message: string;
}
```

**Rust (Tauri commands):**
```rust
pub struct LoginResponse {
    pub success: bool,
    pub player_id: String,
    pub error_message: String,
}
```

## Security Considerations

1. **Password handling:**
   - Passwords are transmitted over the wire (should use HTTPS in production)
   - Backend hashes passwords using bcrypt before storage
   - Plaintext passwords never stored in database

2. **Input validation:**
   - Backend validates username is not empty
   - Protobuf schema enforces type safety

3. **Error handling:**
   - Generic error messages for authentication failures
   - No user enumeration (same error for wrong user vs wrong password)

## Testing

The backend includes comprehensive tests:

```rust
#[tokio::test]
async fn test_grpc_login_success() {
    let pool = db::test_pool().await;
    let player_service = Arc::new(PlayerService::new(pool));
    let name = format!("grpcuser_{}", uuid::Uuid::new_v4());
    player_service.create_player(&name, "secret").await.unwrap();

    let svc = ClusterGameService::new(player_service);
    let res = svc.login(Request::new(LoginRequest {
        username: name.clone(),
        password: "secret".to_string(),
    })).await.unwrap();

    let out = res.into_inner();
    assert!(out.success);
    assert!(!out.player_id.is_empty());
}
```

## Performance Characteristics

- **Binary protocol:** Protobuf provides compact serialization
- **HTTP/2:** Native gRPC uses HTTP/2 for efficient multiplexing
- **Unary RPCs:** Simple request-response pattern (no streaming yet)
- **Direct connection:** Tauri connects directly to backend (no browser proxy)

## Troubleshooting

### Common Issues

**Connection refused:**
- Confirm backend is running on port 50051
- Check `NULLTRACE_GRPC_URL` if using a custom endpoint
- Ensure firewall allows localhost connections

**Invoke fails in browser:**
- gRPC only works when running inside Tauri. Use `npm run tauri dev` for development.

**Protobuf mismatch:**
- Ensure `nulltrace-client/proto/game.proto` and `nulltrace-core/proto/game.proto` stay in sync
- Rebuild after proto changes (`cargo build` in src-tauri)

## References

- **Tonic documentation:** https://docs.rs/tonic
- **Protocol Buffers:** https://protobuf.dev
- **Tauri invoke:** https://v2.tauri.app/develop/calling-rust/
