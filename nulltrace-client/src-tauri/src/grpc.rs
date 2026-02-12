//! gRPC client for GameService (Ping, Login, TerminalStream). Used by Tauri commands to communicate with nulltrace-core.

mod game {
    tonic::include_proto!("game");
}

use game::game_service_client::GameServiceClient;
use game::terminal_client_message::Msg as TerminalClientMsg;
use game::terminal_server_message::Msg as TerminalServerMsg;
use game::{
    CopyPathRequest, GetDiskUsageRequest, GetHomePathRequest, ListFsRequest, LoginRequest,
    MovePathRequest, OpenTerminal, PingRequest, RenamePathRequest, RestoreDiskRequest, StdinData,
    TerminalClientMessage, TerminalOpened,
};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

/// Default gRPC backend URL. Overridable via env for custom deployments.
fn grpc_url() -> String {
    std::env::var("NULLTRACE_GRPC_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

/// Response for grpc_login command.
#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub player_id: String,
    pub token: String,
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
        token: response.token,
        error_message: response.error_message,
    })
}

/// Response for grpc_refresh_token command.
#[derive(serde::Serialize)]
pub struct RefreshTokenCommandResponse {
    pub success: bool,
    pub token: String,
    pub error_message: String,
}

/// Tauri command: Refresh JWT token.
#[tauri::command]
pub async fn grpc_refresh_token(
    current_token: String,
) -> Result<RefreshTokenCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .refresh_token(tonic::Request::new(game::RefreshTokenRequest {
            current_token,
        }))
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(RefreshTokenCommandResponse {
        success: response.success,
        token: response.token,
        error_message: response.error_message,
    })
}

/// Response for grpc_disk_usage command.
#[derive(serde::Serialize)]
pub struct DiskUsageResponse {
    pub used_bytes: i64,
    pub total_bytes: i64,
    pub error_message: String,
}

/// Tauri command: Get disk usage for the player's VM.
#[tauri::command]
pub async fn grpc_disk_usage(
    player_id: String,
    token: String,
) -> Result<DiskUsageResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetDiskUsageRequest {
        player_id: player_id.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_disk_usage(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(DiskUsageResponse {
        used_bytes: response.used_bytes,
        total_bytes: response.total_bytes,
        error_message: response.error_message,
    })
}

/// Response for grpc_restore_disk command.
#[derive(serde::Serialize)]
pub struct RestoreDiskCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Restore disk (wipe and recreate default files) for the player's VM.
#[tauri::command]
pub async fn grpc_restore_disk(
    player_id: String,
    token: String,
) -> Result<RestoreDiskCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(RestoreDiskRequest {
        player_id: player_id.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .restore_disk(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(RestoreDiskCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Tauri command: Get home path for the player's VM.
#[tauri::command]
pub async fn grpc_get_home_path(
    player_id: String,
    token: String,
) -> Result<GetHomePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetHomePathRequest {
        player_id: player_id.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_home_path(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(GetHomePathCommandResponse {
        home_path: response.home_path,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct GetHomePathCommandResponse {
    pub home_path: String,
    pub error_message: String,
}

/// Tauri command: List files and folders at path.
#[tauri::command]
pub async fn grpc_list_fs(
    player_id: String,
    path: String,
    token: String,
) -> Result<ListFsCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(ListFsRequest {
        player_id: player_id.clone(),
        path: path.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .list_fs(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(ListFsCommandResponse {
        entries: response
            .entries
            .into_iter()
            .map(|e| ListFsEntry {
                name: e.name,
                node_type: e.node_type,
                size_bytes: e.size_bytes,
            })
            .collect(),
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct ListFsCommandResponse {
    pub entries: Vec<ListFsEntry>,
    pub error_message: String,
}

#[derive(serde::Serialize)]
pub struct ListFsEntry {
    pub name: String,
    pub node_type: String,
    pub size_bytes: i64,
}

/// Tauri command: Copy file or folder.
#[tauri::command]
pub async fn grpc_copy_path(
    player_id: String,
    src_path: String,
    dest_path: String,
    token: String,
) -> Result<CopyPathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(CopyPathRequest {
        player_id: player_id.clone(),
        src_path: src_path.clone(),
        dest_path: dest_path.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .copy_path(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(CopyPathCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct CopyPathCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Move file or folder.
#[tauri::command]
pub async fn grpc_move_path(
    player_id: String,
    src_path: String,
    dest_path: String,
    token: String,
) -> Result<MovePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(MovePathRequest {
        player_id: player_id.clone(),
        src_path: src_path.clone(),
        dest_path: dest_path.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .move_path(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(MovePathCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct MovePathCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Rename file or folder.
#[tauri::command]
pub async fn grpc_rename_path(
    player_id: String,
    path: String,
    new_name: String,
    token: String,
) -> Result<RenamePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(RenamePathRequest {
        player_id: player_id.clone(),
        path: path.clone(),
        new_name: new_name.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .rename_path(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(RenamePathCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct RenamePathCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Shared state: session_id -> sender for stdin (so terminal_send_stdin can push lines).
pub type TerminalSessionsState = Arc<Mutex<std::collections::HashMap<String, mpsc::Sender<String>>>>;

/// Create initial state for terminal sessions.
pub fn new_terminal_sessions() -> TerminalSessionsState {
    Arc::new(Mutex::new(std::collections::HashMap::new()))
}

/// Tauri command: Open terminal stream for the given player. Returns session_id. Emits "terminal-output" events with { sessionId, type: 'stdout'|'closed'|'error', data? }.
#[tauri::command]
pub async fn terminal_connect(
    player_id: String,
    token: String,
    app: tauri::AppHandle,
    sessions: tauri::State<'_, TerminalSessionsState>,
) -> Result<String, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let (client_tx, client_rx) = mpsc::channel(32);
    let _ = client_tx
        .send(TerminalClientMessage {
            msg: Some(TerminalClientMsg::OpenTerminal(OpenTerminal {
                player_id: player_id.clone(),
            })),
        })
        .await
        .map_err(|e| e.to_string())?;

    let stream = ReceiverStream::new(client_rx);
    let mut request = tonic::Request::new(stream);
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .terminal_stream(request)
        .await
        .map_err(|e| {
            if e.to_string().contains("Unauthenticated") || e.to_string().contains("UNAUTHENTICATED") {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?;
    let mut server_rx = response.into_inner();

    let first = server_rx
        .next()
        .await
        .ok_or("stream closed before TerminalOpened")?
        .map_err(|e| e.to_string())?;
    let session_id = match first.msg {
        Some(TerminalServerMsg::TerminalOpened(TerminalOpened { session_id })) => session_id,
        Some(TerminalServerMsg::TerminalError(e)) => return Err(e.message),
        _ => return Err("expected TerminalOpened".to_string()),
    };

    let (stdin_tx, stdin_rx) = mpsc::channel(32);
    sessions.lock().unwrap().insert(session_id.clone(), stdin_tx);

    let app_emit = app.clone();
    let session_id_task = session_id.clone();
    tokio::spawn(async move {
        let mut server_rx = server_rx;
        let mut stdin_rx = stdin_rx;
        let client_tx = client_tx;

        loop {
            tokio::select! {
                msg = server_rx.next() => {
                    match msg {
                        Some(Ok(m)) => {
                            let payload: Result<serde_json::Value, String> = match m.msg {
                                Some(TerminalServerMsg::Stdout(s)) => Ok(serde_json::json!({
                                    "sessionId": session_id_task,
                                    "type": "stdout",
                                    "data": String::from_utf8_lossy(&s.data),
                                })),
                                Some(TerminalServerMsg::TerminalClosed(_)) => {
                                    let _ = app_emit.emit("terminal-output", serde_json::json!({
                                        "sessionId": session_id_task,
                                        "type": "closed",
                                    }));
                                    break;
                                }
                                Some(TerminalServerMsg::TerminalError(e)) => {
                                    let _ = app_emit.emit("terminal-output", serde_json::json!({
                                        "sessionId": session_id_task,
                                        "type": "error",
                                        "data": e.message,
                                    }));
                                    break;
                                }
                                _ => continue,
                            };
                            if let Ok(p) = payload {
                                let _ = app_emit.emit("terminal-output", p);
                            }
                        }
                        Some(Err(_)) | None => break,
                    }
                }
                stdin_msg = stdin_rx.recv() => {
                    match stdin_msg {
                        Some(line) => {
                            let _ = client_tx
                                .send(TerminalClientMessage {
                                    msg: Some(TerminalClientMsg::Stdin(StdinData {
                                        data: line.into_bytes(),
                                    })),
                                })
                                .await;
                        }
                        None => break,
                    }
                }
            }
        }
    });

    Ok(session_id)
}

/// Tauri command: Send a line to the terminal session (shell stdin).
#[tauri::command]
pub async fn terminal_send_stdin(
    session_id: String,
    data: String,
    sessions: tauri::State<'_, TerminalSessionsState>,
) -> Result<(), String> {
    let tx = sessions
        .lock()
        .unwrap()
        .get(&session_id)
        .cloned()
        .ok_or("session not found")?;
    tx.send(data).await.map_err(|e| e.to_string())
}

/// Tauri command: Disconnect terminal session (removes from map; stream task will exit when sender is dropped).
#[tauri::command]
pub fn terminal_disconnect(
    session_id: String,
    sessions: tauri::State<'_, TerminalSessionsState>,
) {
    sessions.lock().unwrap().remove(&session_id);
}
