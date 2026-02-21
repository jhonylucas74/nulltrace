//! gRPC client for GameService (Ping, Login, TerminalStream). Used by Tauri commands to communicate with nulltrace-core.

mod game {
    tonic::include_proto!("game");
}

use game::game_service_client::GameServiceClient;
use game::process_spy_client_message::Msg as ProcessSpyClientMsg;
use game::process_spy_server_message::Msg as ProcessSpyServerMsg;
use game::terminal_client_message::Msg as TerminalClientMsg;
use game::terminal_server_message::Msg as TerminalServerMsg;
use game::run_process_response::Msg as RunProcessResponseMsg;
use game::{
    CopyPathRequest, CreateFolderRequest, CreateFactionRequest, GetDiskUsageRequest, GetHomePathRequest,
    GetPlayerProfileRequest, GetProcessListRequest, GetRankingRequest, GetSysinfoRequest,
    InjectStdin, Interrupt, KillProcess, LeaveFactionRequest, ListFsRequest, LoginRequest,
    MovePathRequest, OpenCodeRun, OpenTerminal, PingRequest, ProcessListSnapshot, ProcessSpyClientMessage,
    ProcessSpyOpened, RenamePathRequest, RestoreDiskRequest, RunProcessRequest, SetPreferredThemeRequest,
    SetShortcutsRequest, SpawnLuaScript, StdinData, SubscribePid, TerminalClientMessage,
    TerminalOpened, UnsubscribePid, WriteFileRequest, ReadFileRequest, EmptyTrashRequest,
    GetInstalledStoreAppsRequest, InstallStoreAppRequest, UninstallStoreAppRequest,
};
use std::collections::HashMap;
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
    pub preferred_theme: String,
    pub shortcuts_overrides: String,
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
        preferred_theme: response.preferred_theme,
        shortcuts_overrides: response.shortcuts_overrides,
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
pub async fn grpc_disk_usage(token: String) -> Result<DiskUsageResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetDiskUsageRequest {});
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

/// One process entry for grpc_get_process_list response.
#[derive(serde::Serialize)]
pub struct ProcessListEntry {
    pub pid: u64,
    pub name: String,
    pub username: String,
    pub status: String,
    pub memory_bytes: u64,
}

/// Response for grpc_get_process_list command (processes + disk in one call).
#[derive(serde::Serialize)]
pub struct GetProcessListResponse {
    pub processes: Vec<ProcessListEntry>,
    pub disk_used_bytes: i64,
    pub disk_total_bytes: i64,
    pub error_message: String,
    pub vm_lua_memory_bytes: u64,
}

/// Tauri command: Get process list and disk usage for the player's VM (single round-trip for System Monitor).
#[tauri::command]
pub async fn grpc_get_process_list(token: String) -> Result<GetProcessListResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetProcessListRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_process_list(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();

    let processes = response
        .processes
        .into_iter()
        .map(|p| ProcessListEntry {
            pid: p.pid,
            name: p.name,
            username: p.username,
            status: p.status,
            memory_bytes: p.memory_bytes,
        })
        .collect();

    Ok(GetProcessListResponse {
        processes,
        disk_used_bytes: response.disk_used_bytes,
        disk_total_bytes: response.disk_total_bytes,
        error_message: response.error_message,
        vm_lua_memory_bytes: response.vm_lua_memory_bytes,
    })
}

/// Response for grpc_sysinfo command.
#[derive(serde::Serialize)]
pub struct SysinfoResponse {
    pub cpu_cores: i32,
    pub memory_mb: i32,
    pub disk_mb: i32,
    pub error_message: String,
}

/// Tauri command: Get VM specs (CPU, RAM total, disk total) for the player's VM.
#[tauri::command]
pub async fn grpc_sysinfo(token: String) -> Result<SysinfoResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetSysinfoRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_sysinfo(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(SysinfoResponse {
        cpu_cores: response.cpu_cores,
        memory_mb: response.memory_mb,
        disk_mb: response.disk_mb,
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
pub async fn grpc_restore_disk(token: String) -> Result<RestoreDiskCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(RestoreDiskRequest {});
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

/// Single entry in ranking response.
#[derive(serde::Serialize)]
pub struct RankingEntryResponse {
    pub rank: u32,
    pub player_id: String,
    pub username: String,
    pub points: i32,
    pub faction_id: String,
    pub faction_name: String,
}

/// Response for grpc_get_ranking command.
#[derive(serde::Serialize)]
pub struct GetRankingCommandResponse {
    pub entries: Vec<RankingEntryResponse>,
    pub error_message: String,
}

/// Tauri command: Get player ranking (authenticated).
#[tauri::command]
pub async fn grpc_get_ranking(token: String) -> Result<GetRankingCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetRankingRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_ranking(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(GetRankingCommandResponse {
        entries: response
            .entries
            .into_iter()
            .map(|e| RankingEntryResponse {
                rank: e.rank,
                player_id: e.player_id,
                username: e.username,
                points: e.points,
                faction_id: e.faction_id,
                faction_name: e.faction_name,
            })
            .collect(),
        error_message: response.error_message,
    })
}

/// Response for grpc_get_player_profile command.
#[derive(serde::Serialize)]
pub struct GetPlayerProfileCommandResponse {
    pub rank: u32,
    pub points: i32,
    pub faction_id: String,
    pub faction_name: String,
    pub error_message: String,
    pub preferred_theme: String,
    pub shortcuts_overrides: String,
}

/// Tauri command: Get current player profile (rank, points, faction).
#[tauri::command]
pub async fn grpc_get_player_profile(token: String) -> Result<GetPlayerProfileCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetPlayerProfileRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_player_profile(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(GetPlayerProfileCommandResponse {
        rank: response.rank,
        points: response.points,
        faction_id: response.faction_id,
        faction_name: response.faction_name,
        error_message: response.error_message,
        preferred_theme: response.preferred_theme,
        shortcuts_overrides: response.shortcuts_overrides,
    })
}

/// Response for grpc_set_shortcuts command.
#[derive(serde::Serialize)]
pub struct SetShortcutsCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Set keyboard shortcut overrides (authenticated).
#[tauri::command]
pub async fn grpc_set_shortcuts(
    token: String,
    shortcuts_overrides_json: String,
) -> Result<SetShortcutsCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(SetShortcutsRequest {
        shortcuts_overrides_json,
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .set_shortcuts(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(SetShortcutsCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Response for grpc_set_preferred_theme command.
#[derive(serde::Serialize)]
pub struct SetPreferredThemeCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Set preferred UI theme (authenticated).
#[tauri::command]
pub async fn grpc_set_preferred_theme(
    token: String,
    preferred_theme: String,
) -> Result<SetPreferredThemeCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(SetPreferredThemeRequest { preferred_theme });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .set_preferred_theme(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(SetPreferredThemeCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Response for grpc_create_faction command.
#[derive(serde::Serialize)]
pub struct CreateFactionCommandResponse {
    pub faction_id: String,
    pub name: String,
    pub error_message: String,
}

/// Tauri command: Create a faction (authenticated). Creator joins it.
#[tauri::command]
pub async fn grpc_create_faction(name: String, token: String) -> Result<CreateFactionCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(CreateFactionRequest { name: name.clone() });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .create_faction(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(CreateFactionCommandResponse {
        faction_id: response.faction_id,
        name: response.name,
        error_message: response.error_message,
    })
}

/// Response for grpc_leave_faction command.
#[derive(serde::Serialize)]
pub struct LeaveFactionCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Leave current faction (authenticated).
#[tauri::command]
pub async fn grpc_leave_faction(token: String) -> Result<LeaveFactionCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(LeaveFactionRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .leave_faction(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(LeaveFactionCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Tauri command: Get home path for the player's VM.
#[tauri::command]
pub async fn grpc_get_home_path(token: String) -> Result<GetHomePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetHomePathRequest {});
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
pub async fn grpc_list_fs(path: String, token: String) -> Result<ListFsCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(ListFsRequest {
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
    src_path: String,
    dest_path: String,
    token: String,
) -> Result<CopyPathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(CopyPathRequest {
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
    src_path: String,
    dest_path: String,
    token: String,
) -> Result<MovePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(MovePathRequest {
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
    path: String,
    new_name: String,
    token: String,
) -> Result<RenamePathCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(RenamePathRequest {
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

/// Tauri command: Write file (creates or overwrites). Empty content creates a new empty file.
#[tauri::command]
pub async fn grpc_write_file(
    path: String,
    content: String,
    token: String,
) -> Result<WriteFileCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(WriteFileRequest {
        path: path.clone(),
        content: content.into_bytes(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .write_file(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(WriteFileCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct WriteFileCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Create a folder at the given path. Parent must exist.
#[tauri::command]
pub async fn grpc_create_folder(path: String, token: String) -> Result<CreateFolderCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(CreateFolderRequest { path: path.clone() });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .create_folder(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(CreateFolderCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct CreateFolderCommandResponse {
    pub success: bool,
    pub error_message: String,
}

#[derive(serde::Serialize)]
pub struct RunProcessCommandResponse {
    pub stdout: String,
    pub exit_code: i32,
}

/// Tauri command: Run a VM binary with args, collect full stdout and exit code.
#[tauri::command]
pub async fn grpc_run_process(
    bin_name: String,
    args: Vec<String>,
    token: String,
) -> Result<RunProcessCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(RunProcessRequest {
        bin_name: bin_name.clone(),
        args: args.clone(),
    });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let mut stream = client
        .run_process(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();

    let mut stdout = String::new();
    let mut exit_code = 0i32;
    while let Some(msg) = stream.message().await.map_err(|e| e.to_string())? {
        match msg.msg {
            Some(RunProcessResponseMsg::StdoutChunk(data)) => {
                stdout.push_str(&String::from_utf8_lossy(&data));
            }
            Some(RunProcessResponseMsg::Finished(f)) => {
                exit_code = f.exit_code;
                break;
            }
            None => {}
        }
    }
    Ok(RunProcessCommandResponse { stdout, exit_code })
}

/// Tauri command: Read file content from the VM. Returns UTF-8 string content.
#[tauri::command]
pub async fn grpc_read_file(path: String, token: String) -> Result<ReadFileCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(ReadFileRequest { path: path.clone() });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .read_file(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();

    let content = String::from_utf8_lossy(&response.content).into_owned();
    Ok(ReadFileCommandResponse {
        success: response.success,
        error_message: response.error_message,
        content,
    })
}

#[derive(serde::Serialize)]
pub struct ReadFileCommandResponse {
    pub success: bool,
    pub error_message: String,
    pub content: String,
}

/// Tauri command: Permanently delete all items in the user's Trash folder.
#[tauri::command]
pub async fn grpc_empty_trash(token: String) -> Result<EmptyTrashCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(EmptyTrashRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .empty_trash(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(EmptyTrashCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

#[derive(serde::Serialize)]
pub struct EmptyTrashCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Response for grpc_get_installed_store_apps command.
#[derive(serde::Serialize)]
pub struct GetInstalledStoreAppsCommandResponse {
    pub app_types: Vec<String>,
    pub error_message: String,
}

/// Tauri command: Get list of installed store apps from the VM file.
#[tauri::command]
pub async fn grpc_get_installed_store_apps(
    token: String,
) -> Result<GetInstalledStoreAppsCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(GetInstalledStoreAppsRequest {});
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .get_installed_store_apps(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(GetInstalledStoreAppsCommandResponse {
        app_types: response.app_types,
        error_message: response.error_message,
    })
}

/// Response for grpc_install_store_app / grpc_uninstall_store_app commands.
#[derive(serde::Serialize)]
pub struct InstallUninstallStoreAppCommandResponse {
    pub success: bool,
    pub error_message: String,
}

/// Tauri command: Install a store app (append to VM file).
#[tauri::command]
pub async fn grpc_install_store_app(
    app_type: String,
    token: String,
) -> Result<InstallUninstallStoreAppCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(InstallStoreAppRequest { app_type });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .install_store_app(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(InstallUninstallStoreAppCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Tauri command: Uninstall a store app (remove from VM file).
#[tauri::command]
pub async fn grpc_uninstall_store_app(
    app_type: String,
    token: String,
) -> Result<InstallUninstallStoreAppCommandResponse, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let mut request = tonic::Request::new(UninstallStoreAppRequest { app_type });
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token)
            .parse()
            .map_err(|e| format!("Invalid token: {:?}", e))?,
    );

    let response = client
        .uninstall_store_app(request)
        .await
        .map_err(|e| {
            if e.code() == tonic::Code::Unauthenticated {
                return "UNAUTHENTICATED".to_string();
            }
            e.to_string()
        })?
        .into_inner();
    Ok(InstallUninstallStoreAppCommandResponse {
        success: response.success,
        error_message: response.error_message,
    })
}

/// Input that can be sent to the terminal stream: stdin data or interrupt (Ctrl+C).
pub enum TerminalInput {
    Stdin(String),
    Interrupt,
}

/// Shared state: session_id -> sender for terminal input (stdin or interrupt).
pub type TerminalSessionsState =
    Arc<Mutex<std::collections::HashMap<String, mpsc::Sender<TerminalInput>>>>;

/// Create initial state for terminal sessions.
pub fn new_terminal_sessions() -> TerminalSessionsState {
    Arc::new(Mutex::new(std::collections::HashMap::new()))
}

/// Tauri command: Open terminal stream for the given player. Returns session_id. Emits "terminal-output" events with { sessionId, type: 'stdout'|'closed'|'error', data? }.
#[tauri::command]
pub async fn terminal_connect(
    token: String,
    app: tauri::AppHandle,
    sessions: tauri::State<'_, TerminalSessionsState>,
) -> Result<String, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let (client_tx, client_rx) = mpsc::channel(32);
    let _ = client_tx
        .send(TerminalClientMessage {
            msg: Some(TerminalClientMsg::OpenTerminal(OpenTerminal {})),
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
                                Some(TerminalServerMsg::PromptReady(_)) => Ok(serde_json::json!({
                                    "sessionId": session_id_task,
                                    "type": "prompt_ready",
                                })),
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
                        Some(TerminalInput::Stdin(line)) => {
                            let _ = client_tx
                                .send(TerminalClientMessage {
                                    msg: Some(TerminalClientMsg::Stdin(StdinData {
                                        data: line.into_bytes(),
                                    })),
                                })
                                .await;
                        }
                        Some(TerminalInput::Interrupt) => {
                            let _ = client_tx
                                .send(TerminalClientMessage {
                                    msg: Some(TerminalClientMsg::Interrupt(Interrupt {})),
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

/// Tauri command: Open a Code Run session (run Lua script at path). Uses same TerminalStream; returns session_id. Emits "terminal-output" like terminal_connect. Use terminal_send_stdin/terminal_disconnect for stdin and stop.
#[tauri::command]
pub async fn code_run_connect(
    token: String,
    path: String,
    app: tauri::AppHandle,
    sessions: tauri::State<'_, TerminalSessionsState>,
) -> Result<String, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let (client_tx, client_rx) = mpsc::channel(32);
    let _ = client_tx
        .send(TerminalClientMessage {
            msg: Some(TerminalClientMsg::OpenCodeRun(OpenCodeRun { path })),
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
                                Some(TerminalServerMsg::PromptReady(_)) => Ok(serde_json::json!({
                                    "sessionId": session_id_task,
                                    "type": "prompt_ready",
                                })),
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
                        Some(TerminalInput::Stdin(line)) => {
                            let _ = client_tx
                                .send(TerminalClientMessage {
                                    msg: Some(TerminalClientMsg::Stdin(StdinData {
                                        data: line.into_bytes(),
                                    })),
                                })
                                .await;
                        }
                        Some(TerminalInput::Interrupt) => {
                            let _ = client_tx
                                .send(TerminalClientMessage {
                                    msg: Some(TerminalClientMsg::Interrupt(Interrupt {})),
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
    tx.send(TerminalInput::Stdin(data))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Send interrupt (Ctrl+C) to the terminal session; kills the foreground process.
#[tauri::command]
pub async fn terminal_send_interrupt(
    session_id: String,
    sessions: tauri::State<'_, TerminalSessionsState>,
) -> Result<(), String> {
    let tx = sessions
        .lock()
        .unwrap()
        .get(&session_id)
        .cloned()
        .ok_or("session not found")?;
    tx.send(TerminalInput::Interrupt)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Disconnect terminal session (removes from map; stream task will exit when sender is dropped).
#[tauri::command]
pub fn terminal_disconnect(
    session_id: String,
    sessions: tauri::State<'_, TerminalSessionsState>,
) {
    sessions.lock().unwrap().remove(&session_id);
}

// ─── Process Spy (Proc Spy) stream ─────────────────────────────────────────

/// Commands sent from Tauri commands to the process spy stream task.
pub enum ProcessSpyCommand {
    Subscribe(u64),
    Unsubscribe(u64),
    Stdin(u64, String),
    SpawnLuaScript(String),
    KillProcess(u64),
}

/// Shared state: multiple process spy connections (connection_id -> command sender).
/// Each connection (Code Run, Proc Spy app, etc.) has its own stream and connection_id.
pub struct ProcessSpyState {
    /// connection_id -> sender for commands to that stream task
    pub connections: HashMap<String, mpsc::Sender<ProcessSpyCommand>>,
}

pub fn new_process_spy_state() -> Arc<Mutex<ProcessSpyState>> {
    Arc::new(Mutex::new(ProcessSpyState {
        connections: HashMap::new(),
    }))
}

/// Tauri command: Open Process Spy stream. Returns connection_id. Emits process-spy-opened, process-spy-process-list, process-spy-stdout, process-spy-stdin, process-spy-process-gone, process-spy-error, process-spy-closed.
/// Multiple connections are allowed (e.g. Code app Run and Proc Spy app can both be connected).
#[tauri::command]
pub async fn process_spy_connect(
    token: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<String, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;

    let (client_tx, client_rx) = mpsc::channel(32);
    let _ = client_tx
        .send(ProcessSpyClientMessage {
            msg: Some(ProcessSpyClientMsg::OpenProcessSpy(game::OpenProcessSpy {})),
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
        .process_spy_stream(request)
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
        .ok_or("stream closed before ProcessSpyOpened")?
        .map_err(|e| e.to_string())?;
    match first.msg {
        Some(ProcessSpyServerMsg::ProcessSpyOpened(ProcessSpyOpened {})) => {}
        Some(ProcessSpyServerMsg::ProcessSpyError(e)) => return Err(e.message),
        _ => return Err("expected ProcessSpyOpened".to_string()),
    }

    let connection_id = uuid::Uuid::new_v4().to_string();
    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
    let state_clone = state.inner().clone();
    let connection_id_clone = connection_id.clone();
    {
        let mut s = state.lock().unwrap();
        s.connections.insert(connection_id.clone(), cmd_tx);
    }

    let app_emit = app.clone();
    tokio::spawn(async move {
        let mut server_rx = server_rx;
        let mut client_tx = client_tx;
        loop {
            tokio::select! {
                msg = server_rx.next() => {
                    match msg {
                        Some(Ok(m)) => {
                            match m.msg {
                                Some(ProcessSpyServerMsg::ProcessListSnapshot(ProcessListSnapshot { processes })) => {
                                    let list: Vec<serde_json::Value> = processes
                                        .into_iter()
                                        .map(|p| serde_json::json!({
                                            "pid": p.pid,
                                            "name": p.name,
                                            "username": p.username,
                                            "status": p.status,
                                            "memory_bytes": p.memory_bytes,
                                            "args": p.args,
                                        }))
                                        .collect();
                                    let _ = app_emit.emit("process-spy-process-list", serde_json::json!({ "processes": list }));
                                }
                                Some(ProcessSpyServerMsg::ProcessSpyStdout(s)) => {
                                    let _ = app_emit.emit("process-spy-stdout", serde_json::json!({
                                        "connectionId": connection_id_clone,
                                        "pid": s.pid,
                                        "data": String::from_utf8_lossy(&s.data),
                                    }));
                                }
                                Some(ProcessSpyServerMsg::StdinChunk(s)) => {
                                    let _ = app_emit.emit("process-spy-stdin", serde_json::json!({
                                        "pid": s.pid,
                                        "data": String::from_utf8_lossy(&s.data),
                                    }));
                                }
                                Some(ProcessSpyServerMsg::ProcessGone(p)) => {
                                    let _ = app_emit.emit("process-spy-process-gone", serde_json::json!({
                                        "connectionId": connection_id_clone,
                                        "pid": p.pid,
                                    }));
                                }
                                Some(ProcessSpyServerMsg::LuaScriptSpawned(s)) => {
                                    let _ = app_emit.emit("process-spy-lua-script-spawned", serde_json::json!({
                                        "connectionId": connection_id_clone,
                                        "pid": s.pid,
                                    }));
                                }
                                Some(ProcessSpyServerMsg::ProcessSpyError(e)) => {
                                    let _ = app_emit.emit("process-spy-error", serde_json::json!({ "message": e.message }));
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Some(Err(_)) | None => {
                            let _ = app_emit.emit("process-spy-closed", ());
                            break;
                        }
                    }
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ProcessSpyCommand::Subscribe(pid)) => {
                            let _ = client_tx
                                .send(ProcessSpyClientMessage {
                                    msg: Some(ProcessSpyClientMsg::SubscribePid(SubscribePid { pid })),
                                })
                                .await;
                        }
                        Some(ProcessSpyCommand::Unsubscribe(pid)) => {
                            let _ = client_tx
                                .send(ProcessSpyClientMessage {
                                    msg: Some(ProcessSpyClientMsg::UnsubscribePid(UnsubscribePid { pid })),
                                })
                                .await;
                        }
                        Some(ProcessSpyCommand::Stdin(pid, data)) => {
                            let _ = client_tx
                                .send(ProcessSpyClientMessage {
                                    msg: Some(ProcessSpyClientMsg::InjectStdin(InjectStdin {
                                        pid,
                                        data: data.into_bytes(),
                                    })),
                                })
                                .await;
                        }
                        Some(ProcessSpyCommand::SpawnLuaScript(path)) => {
                            let _ = client_tx
                                .send(ProcessSpyClientMessage {
                                    msg: Some(ProcessSpyClientMsg::SpawnLuaScript(SpawnLuaScript { path })),
                                })
                                .await;
                        }
                        Some(ProcessSpyCommand::KillProcess(pid)) => {
                            let _ = client_tx
                                .send(ProcessSpyClientMessage {
                                    msg: Some(ProcessSpyClientMsg::KillProcess(KillProcess { pid })),
                                })
                                .await;
                        }
                        None => break,
                    }
                }
            }
        }
        let mut s = state_clone.lock().unwrap();
        s.connections.remove(&connection_id_clone);
        let _ = app_emit.emit("process-spy-closed", serde_json::json!({ "connectionId": connection_id_clone }));
    });

    let _ = app.emit("process-spy-opened", serde_json::json!({ "connectionId": connection_id }));
    Ok(connection_id)
}

/// Tauri command: Subscribe to a PID (open a tab). Call after process_spy_connect.
#[tauri::command]
pub async fn process_spy_subscribe(
    connection_id: String,
    pid: u64,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connections.get(&connection_id).cloned()
    };
    let tx = tx.ok_or("Process Spy not connected or connection id mismatch".to_string())?;
    tx.send(ProcessSpyCommand::Subscribe(pid))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Unsubscribe from a PID (close tab).
#[tauri::command]
pub async fn process_spy_unsubscribe(
    connection_id: String,
    pid: u64,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connections.get(&connection_id).cloned()
    };
    let tx = tx.ok_or("Process Spy not connected or connection id mismatch".to_string())?;
    tx.send(ProcessSpyCommand::Unsubscribe(pid))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Inject stdin into a process.
#[tauri::command]
pub async fn process_spy_stdin(
    connection_id: String,
    pid: u64,
    data: String,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connections.get(&connection_id).cloned()
    };
    let tx = tx.ok_or("Process Spy not connected or connection id mismatch".to_string())?;
    tx.send(ProcessSpyCommand::Stdin(pid, data))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Spawn a Lua script in the VM (for Code app Run). Returns immediately; listen for process-spy-lua-script-spawned to get pid.
#[tauri::command]
pub async fn process_spy_spawn_lua_script(
    connection_id: String,
    path: String,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connections.get(&connection_id).cloned()
    };
    let tx = tx.ok_or("Process Spy not connected or connection id mismatch".to_string())?;
    tx.send(ProcessSpyCommand::SpawnLuaScript(path))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Kill a process in the VM (for Code app Stop).
#[tauri::command]
pub async fn process_spy_kill_process(
    connection_id: String,
    pid: u64,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) -> Result<(), String> {
    let tx = {
        let s = state.lock().unwrap();
        s.connections.get(&connection_id).cloned()
    };
    let tx = tx.ok_or("Process Spy not connected or connection id mismatch".to_string())?;
    tx.send(ProcessSpyCommand::KillProcess(pid))
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: Disconnect Process Spy stream. Dropping the sender makes the stream task exit and remove the connection.
#[tauri::command]
pub fn process_spy_disconnect(
    connection_id: String,
    state: tauri::State<'_, Arc<Mutex<ProcessSpyState>>>,
) {
    let mut s = state.lock().unwrap();
    s.connections.remove(&connection_id);
    // Dropping the sender makes the stream task's cmd_rx.recv() return None and the task exits
}

// ─── Email Commands ─────────────────────────────────────────────────────────

/// Single email message returned by grpc_get_emails.
#[derive(serde::Serialize, Clone)]
pub struct EmailMessageResponse {
    pub id: String,
    pub from_address: String,
    pub to_address: String,
    pub subject: String,
    pub body: String,
    pub folder: String,
    pub read: bool,
    pub sent_at_ms: i64,
}

/// Tauri command: Fetch emails for a given address and folder.
#[tauri::command]
pub async fn grpc_get_emails(
    email_address: String,
    mail_token: String,
    folder: String,
) -> Result<Vec<EmailMessageResponse>, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .get_emails(tonic::Request::new(game::GetEmailsRequest {
            email_address,
            mail_token,
            folder,
        }))
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(response
        .emails
        .into_iter()
        .map(|e| EmailMessageResponse {
            id: e.id,
            from_address: e.from_address,
            to_address: e.to_address,
            subject: e.subject,
            body: e.body,
            folder: e.folder,
            read: e.read,
            sent_at_ms: e.sent_at_ms,
        })
        .collect())
}

/// Tauri command: Send an email.
#[tauri::command]
pub async fn grpc_send_email(
    from_address: String,
    mail_token: String,
    to_address: String,
    subject: String,
    body: String,
    cc_address: Option<String>,
    bcc_address: Option<String>,
) -> Result<(), String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .send_email(tonic::Request::new(game::SendEmailRequest {
            from_address,
            mail_token,
            to_address,
            subject,
            body,
            cc_address: cc_address.unwrap_or_default(),
            bcc_address: bcc_address.unwrap_or_default(),
        }))
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if !response.success && !response.error_message.is_empty() {
        return Err(response.error_message);
    }
    Ok(())
}

/// Tauri command: Mark an email as read or unread.
#[tauri::command]
pub async fn grpc_mark_email_read(
    email_address: String,
    mail_token: String,
    email_id: String,
    read: bool,
) -> Result<(), String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    client
        .mark_email_read(tonic::Request::new(game::MarkEmailReadRequest {
            email_address,
            mail_token,
            email_id,
            read,
        }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Tauri command: Move an email to a different folder.
#[tauri::command]
pub async fn grpc_move_email(
    email_address: String,
    mail_token: String,
    email_id: String,
    folder: String,
) -> Result<(), String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    client
        .move_email(tonic::Request::new(game::MoveEmailRequest {
            email_address,
            mail_token,
            email_id,
            folder,
        }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Tauri command: Permanently delete an email.
#[tauri::command]
pub async fn grpc_delete_email(
    email_address: String,
    mail_token: String,
    email_id: String,
) -> Result<(), String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    client
        .delete_email(tonic::Request::new(game::DeleteEmailRequest {
            email_address,
            mail_token,
            email_id,
        }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Shared state for mailbox stream connections: conn_id -> JoinHandle.
pub struct MailboxConnectionsState {
    pub handles: HashMap<String, tokio::task::JoinHandle<()>>,
}

pub fn new_mailbox_connections() -> Arc<Mutex<MailboxConnectionsState>> {
    Arc::new(Mutex::new(MailboxConnectionsState {
        handles: HashMap::new(),
    }))
}

/// Tauri command: Connect to the mailbox real-time stream.
/// Returns a conn_id. Emits "mailbox_event" Tauri events with payloads:
///   { type: "new_email", email: EmailMessageResponse }
///   { type: "unread_count", count: number }
#[tauri::command]
pub async fn mailbox_connect(
    email_address: String,
    mail_token: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<MailboxConnectionsState>>>,
) -> Result<String, String> {
    let url = grpc_url();
    let mut client = GameServiceClient::connect(url).await.map_err(|e| e.to_string())?;
    let response = client
        .mailbox_stream(tonic::Request::new(game::MailboxStreamRequest {
            email_address: email_address.clone(),
            mail_token: mail_token.clone(),
        }))
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = response.into_inner();
    let conn_id = uuid::Uuid::new_v4().to_string();
    let app_emit = app.clone();
    let handle = tokio::spawn(async move {
        while let Some(result) = stream.message().await.ok().flatten() {
            let payload = match result.payload {
                Some(game::mailbox_stream_message::Payload::NewEmail(email)) => {
                    serde_json::json!({
                        "type": "new_email",
                        "email": {
                            "id": email.id,
                            "from_address": email.from_address,
                            "to_address": email.to_address,
                            "subject": email.subject,
                            "body": email.body,
                            "folder": email.folder,
                            "read": email.read,
                            "sent_at_ms": email.sent_at_ms,
                        }
                    })
                }
                Some(game::mailbox_stream_message::Payload::UnreadCount(count)) => {
                    serde_json::json!({
                        "type": "unread_count",
                        "count": count,
                    })
                }
                None => continue,
            };
            let _ = app_emit.emit("mailbox_event", payload);
        }
    });
    state.lock().unwrap().handles.insert(conn_id.clone(), handle);
    Ok(conn_id)
}

/// Tauri command: Disconnect from the mailbox stream.
#[tauri::command]
pub fn mailbox_disconnect(
    conn_id: String,
    state: tauri::State<'_, Arc<Mutex<MailboxConnectionsState>>>,
) {
    if let Some(handle) = state.lock().unwrap().handles.remove(&conn_id) {
        handle.abort();
    }
}
