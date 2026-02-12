//! gRPC client for GameService (Ping, Login, TerminalStream). Used by Tauri commands to communicate with nulltrace-core.

mod game {
    tonic::include_proto!("game");
}

use game::game_service_client::GameServiceClient;
use game::terminal_client_message::Msg as TerminalClientMsg;
use game::terminal_server_message::Msg as TerminalServerMsg;
use game::{
    LoginRequest, OpenTerminal, PingRequest, StdinData, TerminalClientMessage, TerminalOpened,
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
    let response = client
        .terminal_stream(tonic::Request::new(stream))
        .await
        .map_err(|e| e.to_string())?;
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
