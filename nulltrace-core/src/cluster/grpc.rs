//! gRPC GameService implementation (Ping, Login, SayHello, TerminalStream, GetDiskUsage, RestoreDisk).
//! Used by the unified cluster binary.

use super::bin_programs;
use super::db::email_account_service::EmailAccountService;
use super::db::email_service::EmailService;
use super::db::faction_service::FactionService;
use super::db::fs_service::FsService;
use super::db::player_service::PlayerService;
use super::db::shortcuts_service::ShortcutsService;
use super::db::user_service::{UserService, VmUser};
use super::db::vm_service::VmService;
use super::mailbox_hub::{MailboxHub, MailboxEvent};
use super::process_run_hub::{ProcessRunHub, RunProcessStreamMsg};
use super::process_spy_hub::{ProcessSpyConnection, ProcessSpyDownstreamMsg, ProcessSpyHub};
use super::terminal_hub::{SessionReady, TerminalHub};
use super::vm_manager::ProcessSnapshot;
use std::collections::HashMap;
use dashmap::DashMap;
use game::terminal_server_message::Msg as TerminalServerMsg;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub mod game {
    tonic::include_proto!("game");
}

use game::game_service_server::GameService;
use game::process_spy_server_message::Msg as ProcessSpyServerMsg;
use game::run_process_response::Msg as RunProcessResponseMsg;
use game::{
    CopyPathRequest, CopyPathResponse, CreateFactionRequest, CreateFactionResponse, FsEntry,
    GetDiskUsageRequest, GetDiskUsageResponse, GetHomePathRequest, GetHomePathResponse,
    GetPlayerProfileRequest, GetPlayerProfileResponse, GetProcessListRequest, GetProcessListResponse,
    GetRankingRequest, GetRankingResponse, GetSysinfoRequest, GetSysinfoResponse, HelloRequest,
    HelloResponse, LeaveFactionRequest, LeaveFactionResponse, ListFsRequest, ListFsResponse,
    InjectStdin, LoginRequest, LoginResponse, MovePathRequest, MovePathResponse,
    OpenCodeRun, OpenTerminal, PingRequest, PingResponse, ProcessEntry, ProcessGone, ProcessListSnapshot,
    ProcessSpyClientMessage, ProcessSpyOpened, ProcessSpyServerMessage, ProcessSpyStdout,
    ProcessSpyError, KillProcess, LuaScriptSpawned, RankingEntry, RefreshTokenRequest, RefreshTokenResponse,
    SpawnLuaScript,
    CreateFolderRequest, CreateFolderResponse, RenamePathRequest, RenamePathResponse, RestoreDiskRequest, RestoreDiskResponse, SetPreferredThemeRequest,
    SetPreferredThemeResponse, SetShortcutsRequest, SetShortcutsResponse, StdinChunk, StdinData,
    PromptReady, StdoutData, SubscribePid, TerminalClientMessage, TerminalClosed, TerminalError, TerminalOpened,
    TerminalServerMessage, UnsubscribePid, WriteFileRequest, WriteFileResponse,
    ReadFileRequest, ReadFileResponse,
    EmptyTrashRequest, EmptyTrashResponse,
    GetInstalledStoreAppsRequest, GetInstalledStoreAppsResponse,
    InstallStoreAppRequest, InstallStoreAppResponse,
    UninstallStoreAppRequest, UninstallStoreAppResponse,
    RunProcessFinished, RunProcessRequest, RunProcessResponse,
    // Email RPCs
    DeleteEmailRequest, DeleteEmailResponse, GetEmailsRequest, GetEmailsResponse,
    MailboxStreamMessage, MailboxStreamRequest, MarkEmailReadRequest, MarkEmailReadResponse,
    MoveEmailRequest, MoveEmailResponse, SendEmailRequest, SendEmailResponse,
    EmailMessage as GrpcEmailMessage,
};

pub struct ClusterGameService {
    player_service: Arc<PlayerService>,
    vm_service: Arc<VmService>,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    faction_service: Arc<FactionService>,
    shortcuts_service: Arc<ShortcutsService>,
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: MailboxHub,
    terminal_hub: Arc<TerminalHub>,
    process_spy_hub: Arc<ProcessSpyHub>,
    process_run_hub: Arc<ProcessRunHub>,
    process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
    vm_lua_memory_store: Arc<DashMap<Uuid, u64>>,
}

impl ClusterGameService {
    pub fn new(
        player_service: Arc<PlayerService>,
        vm_service: Arc<VmService>,
        fs_service: Arc<FsService>,
        user_service: Arc<UserService>,
        faction_service: Arc<FactionService>,
        shortcuts_service: Arc<ShortcutsService>,
        email_service: Arc<EmailService>,
        email_account_service: Arc<EmailAccountService>,
        mailbox_hub: MailboxHub,
        terminal_hub: Arc<TerminalHub>,
        process_spy_hub: Arc<ProcessSpyHub>,
        process_run_hub: Arc<ProcessRunHub>,
        process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
        vm_lua_memory_store: Arc<DashMap<Uuid, u64>>,
    ) -> Self {
        Self {
            player_service,
            vm_service,
            fs_service,
            user_service,
            faction_service,
            shortcuts_service,
            email_service,
            email_account_service,
            mailbox_hub,
            terminal_hub,
            process_spy_hub,
            process_run_hub,
            process_snapshot_store,
            vm_lua_memory_store,
        }
    }

    /// Authenticate a request by validating the JWT token from metadata
    fn authenticate_request<T>(&self, request: &Request<T>) -> Result<crate::auth::Claims, Status> {
        let metadata = request.metadata();
        let token = metadata
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        crate::auth::validate_token(token, &crate::auth::get_jwt_secret())
            .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))
    }
}

#[tonic::async_trait]
impl GameService for ClusterGameService {
    type TerminalStreamStream = ReceiverStream<Result<TerminalServerMessage, Status>>;
    type ProcessSpyStreamStream = ReceiverStream<Result<ProcessSpyServerMessage, Status>>;
    type RunProcessStream = ReceiverStream<Result<RunProcessResponse, Status>>;
    type MailboxStreamStream = ReceiverStream<Result<MailboxStreamMessage, Status>>;

    async fn run_process(
        &self,
        request: Request<RunProcessRequest>,
    ) -> Result<Response<Self::RunProcessStream>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let RunProcessRequest { bin_name, args } = request.into_inner();
        if bin_name.is_empty() {
            return Err(Status::invalid_argument("bin_name is required"));
        }
        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut hub = self.process_run_hub.lock().unwrap();
            hub.pending_runs.push((player_id, bin_name, args, response_tx));
        }
        let run_rx = match tokio::time::timeout(std::time::Duration::from_secs(15), response_rx).await {
            Ok(Ok(Ok(rx))) => rx,
            Ok(Ok(Err(e))) => {
                return Err(Status::internal(e));
            }
            Ok(Err(_)) => {
                return Err(Status::deadline_exceeded("run process response channel closed"));
            }
            Err(_) => {
                return Err(Status::deadline_exceeded("run process start timeout"));
            }
        };
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut run_rx = run_rx;
            while let Some(msg) = run_rx.recv().await {
                let is_finished = matches!(&msg, RunProcessStreamMsg::Finished(_));
                let response = match msg {
                    RunProcessStreamMsg::Stdout(s) => RunProcessResponse {
                        msg: Some(RunProcessResponseMsg::StdoutChunk(s.into_bytes())),
                    },
                    RunProcessStreamMsg::Finished(code) => RunProcessResponse {
                        msg: Some(RunProcessResponseMsg::Finished(RunProcessFinished {
                            exit_code: code,
                        })),
                    },
                };
                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
                if is_finished {
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

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
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let LoginRequest { username, password } = request.into_inner();
        let username = username.trim().to_string();
        if username.is_empty() {
            return Ok(Response::new(LoginResponse {
                success: false,
                player_id: String::new(),
                token: String::new(),
                error_message: "Username is required".to_string(),
                preferred_theme: String::new(),
                shortcuts_overrides: String::new(),
            }));
        }

        let player = self
            .player_service
            .verify_password(&username, &password)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match player {
            Some(p) => {
                // Generate JWT token
                let token = crate::auth::generate_token(
                    p.id,
                    &p.username,
                    &crate::auth::get_jwt_secret(),
                )
                .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

                let preferred_theme = p
                    .preferred_theme
                    .as_deref()
                    .unwrap_or("githubdark")
                    .to_string();
                let shortcuts_overrides = self
                    .shortcuts_service
                    .get_shortcuts(p.id)
                    .await
                    .unwrap_or_else(|_| "{}".to_string());
                Ok(Response::new(LoginResponse {
                    success: true,
                    player_id: p.id.to_string(),
                    token,
                    error_message: String::new(),
                    preferred_theme,
                    shortcuts_overrides,
                }))
            }
            None => Ok(Response::new(LoginResponse {
                success: false,
                player_id: String::new(),
                token: String::new(),
                error_message: "Invalid credentials".to_string(),
                preferred_theme: String::new(),
                shortcuts_overrides: String::new(),
            })),
        }
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let current_token = request.into_inner().current_token;

        // Validate current token
        let claims = crate::auth::validate_token(&current_token, &crate::auth::get_jwt_secret())
            .map_err(|_| Status::unauthenticated("Invalid token"))?;

        // Parse player_id from claims
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        // Issue new token with fresh 24-hour expiry
        let new_token = crate::auth::generate_token(
            player_id,
            &claims.username,
            &crate::auth::get_jwt_secret(),
        )
        .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

        Ok(Response::new(RefreshTokenResponse {
            success: true,
            token: new_token,
            error_message: String::new(),
        }))
    }

    async fn terminal_stream(
        &self,
        request: Request<tonic::Streaming<TerminalClientMessage>>,
    ) -> Result<Response<Self::TerminalStreamStream>, Status> {
        // Authenticate request before processing stream
        let claims = self.authenticate_request(&request)?;
        let authenticated_player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let mut client_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(16);
        let terminal_hub = Arc::clone(&self.terminal_hub);

        let first = client_stream
            .message()
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?
            .ok_or_else(|| Status::invalid_argument("stream closed before OpenTerminal or OpenCodeRun"))?;
        let player_id = authenticated_player_id;
        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut hub = terminal_hub.lock().unwrap();
            match first.msg {
                Some(game::terminal_client_message::Msg::OpenTerminal(OpenTerminal {})) => {
                    hub.pending_opens.push((player_id, response_tx));
                }
                Some(game::terminal_client_message::Msg::OpenCodeRun(OpenCodeRun { path })) => {
                    hub.pending_code_runs.push((player_id, path, response_tx));
                }
                _ => {
                    drop(hub);
                    let _ = tx
                        .send(Ok(TerminalServerMessage {
                            msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                                message: "first message must be OpenTerminal or OpenCodeRun".to_string(),
                            })),
                        }))
                        .await;
                    return Ok(Response::new(ReceiverStream::new(rx)));
                }
            }
        }
        let ready: SessionReady = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            response_rx,
        )
        .await
        {
            Ok(Ok(Ok(ready))) => ready,
            Ok(Ok(Err(e))) => {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                            message: e,
                        })),
                    }))
                    .await;
                return Ok(Response::new(ReceiverStream::new(rx)));
            }
            Ok(Err(_)) | Err(_) => {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                            message: "terminal open timeout or channel closed".to_string(),
                        })),
                    }))
                    .await;
                return Ok(Response::new(ReceiverStream::new(rx)));
            }
        };
        let _ = tx
            .send(Ok(TerminalServerMessage {
                msg: Some(TerminalServerMsg::TerminalOpened(TerminalOpened {
                    session_id: ready.session_id.to_string(),
                })),
            }))
            .await;

        let hub_remove = Arc::clone(&terminal_hub);
        let stdin_tx = ready.stdin_tx;
        let sid = ready.session_id;
        tokio::spawn(async move {
            while let Ok(Some(msg)) = client_stream.message().await {
                match msg.msg {
                    Some(game::terminal_client_message::Msg::Stdin(StdinData { data })) => {
                        if let Ok(s) = String::from_utf8(data) {
                            let _ = stdin_tx.send(s).await;
                        }
                    }
                    Some(game::terminal_client_message::Msg::Interrupt(_)) => {
                        let mut hub = hub_remove.lock().unwrap();
                        if let Some(session) = hub.sessions.get(&sid) {
                            let (vm_id, pid) = (session.vm_id, session.pid);
                            hub.pending_interrupts.push((vm_id, pid));
                        }
                    }
                    _ => {}
                }
            }
            drop(stdin_tx);
            let mut hub = hub_remove.lock().unwrap();
            let kill_info = hub.sessions.get(&sid).map(|s| (s.vm_id, s.pid));
            hub.sessions.remove(&sid);
            if let Some((vm_id, pid)) = kill_info {
                hub.pending_kills.push((vm_id, pid));
            }
        });

        let mut stdout_rx = ready.stdout_rx;
        let mut error_rx = ready.error_rx;
        let mut prompt_ready_rx = ready.prompt_ready_rx;
        tokio::spawn(async move {
            let mut send_closed = true;
            loop {
                tokio::select! {
                    chunk = stdout_rx.recv() => {
                        match chunk {
                            Some(s) => {
                                let _ = tx
                                    .send(Ok(TerminalServerMessage {
                                        msg: Some(TerminalServerMsg::Stdout(StdoutData {
                                            data: s.into_bytes(),
                                        })),
                                    }))
                                    .await;
                            }
                            None => {
                                // stdout closed; error may have been sent before session drop - drain it
                                if let Ok(msg) = error_rx.try_recv() {
                                    let _ = tx
                                        .send(Ok(TerminalServerMessage {
                                            msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                                                message: msg,
                                            })),
                                        }))
                                        .await;
                                    send_closed = false;
                                }
                                break;
                            }
                        }
                    }
                    _ = prompt_ready_rx.recv() => {
                        let _ = tx
                            .send(Ok(TerminalServerMessage {
                                msg: Some(TerminalServerMsg::PromptReady(PromptReady {})),
                            }))
                            .await;
                    }
                    err = error_rx.recv() => {
                        if let Some(msg) = err {
                            let _ = tx
                                .send(Ok(TerminalServerMessage {
                                    msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                                        message: msg,
                                    })),
                                }))
                                .await;
                            send_closed = false;
                        }
                        break;
                    }
                }
            }
            if send_closed {
                let _ = tx
                    .send(Ok(TerminalServerMessage {
                        msg: Some(TerminalServerMsg::TerminalClosed(TerminalClosed {})),
                    }))
                    .await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn process_spy_stream(
        &self,
        request: Request<tonic::Streaming<ProcessSpyClientMessage>>,
    ) -> Result<Response<Self::ProcessSpyStreamStream>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let mut client_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let process_spy_hub = Arc::clone(&self.process_spy_hub);

        let first = client_stream
            .message()
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?
            .ok_or_else(|| Status::invalid_argument("stream closed before OpenProcessSpy"))?;

        let is_open = matches!(first.msg, Some(game::process_spy_client_message::Msg::OpenProcessSpy(_)));
        if !is_open {
            let _ = tx
                .send(Ok(ProcessSpyServerMessage {
                    msg: Some(ProcessSpyServerMsg::ProcessSpyError(ProcessSpyError {
                        message: "first message must be OpenProcessSpy".to_string(),
                    })),
                }))
                .await;
            return Ok(Response::new(ReceiverStream::new(rx)));
        }

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;
        let vm_id = vm.id;

        let connection_id = Uuid::new_v4();
        let (downstream_tx, mut downstream_rx) = mpsc::channel(64);
        {
            let mut hub = process_spy_hub.lock().unwrap();
            hub.connections.insert(
                connection_id,
                ProcessSpyConnection {
                    player_id,
                    vm_id,
                    downstream_tx,
                    subscriptions: HashMap::new(),
                    sent_initial_list: false,
                },
            );
        }

        let _ = tx
            .send(Ok(ProcessSpyServerMessage {
                msg: Some(ProcessSpyServerMsg::ProcessSpyOpened(ProcessSpyOpened {})),
            }))
            .await;

        let hub_for_recv = Arc::clone(&process_spy_hub);
        tokio::spawn(async move {
            let mut client_stream = client_stream;
            while let Ok(Some(msg)) = client_stream.message().await {
                match msg.msg {
                    Some(game::process_spy_client_message::Msg::SubscribePid(SubscribePid { pid })) => {
                        let (stdin_tx, stdin_rx) = mpsc::channel(32);
                        let mut hub = hub_for_recv.lock().unwrap();
                        let vm_id = hub.connections.get(&connection_id).map(|c| c.vm_id);
                        let cached_stdout = vm_id.and_then(|vm_id| hub.recently_finished_stdout.remove(&(vm_id, pid)));
                        if let Some(conn) = hub.connections.get_mut(&connection_id) {
                            if let Some(cached_stdout) = cached_stdout {
                                // Late subscribe: process already exited but we had cached stdout (e.g. Proc Spy opening a just-exited process)
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::Stdout(pid, cached_stdout));
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::ProcessGone(pid));
                            } else {
                                // Preserve last_stdout_len from existing subscription (e.g. auto-subscribe) so we don't re-send already-sent stdout
                                let last_stdout_len = conn
                                    .subscriptions
                                    .get(&pid)
                                    .map(|s| s.last_stdout_len)
                                    .unwrap_or(0);
                                conn.subscriptions.insert(
                                    pid,
                                    super::process_spy_hub::ProcessSpySubscription {
                                        stdin_tx,
                                        stdin_rx,
                                        last_stdout_len,
                                    },
                                );
                            }
                        }
                    }
                    Some(game::process_spy_client_message::Msg::UnsubscribePid(UnsubscribePid { pid })) => {
                        let mut hub = hub_for_recv.lock().unwrap();
                        if let Some(conn) = hub.connections.get_mut(&connection_id) {
                            conn.subscriptions.remove(&pid);
                        }
                    }
                    Some(game::process_spy_client_message::Msg::InjectStdin(InjectStdin { pid, data })) => {
                        if let Ok(s) = String::from_utf8(data) {
                            let hub = hub_for_recv.lock().unwrap();
                            if let Some(conn) = hub.connections.get(&connection_id) {
                                if let Some(sub) = conn.subscriptions.get(&pid) {
                                    let _ = sub.stdin_tx.try_send(s);
                                }
                            }
                        }
                    }
                    Some(game::process_spy_client_message::Msg::SpawnLuaScript(SpawnLuaScript { path })) => {
                        let vm_id = {
                            let hub = hub_for_recv.lock().unwrap();
                            hub.connections.get(&connection_id).map(|c| c.vm_id)
                        };
                        if let Some(vm_id) = vm_id {
                            let mut hub = hub_for_recv.lock().unwrap();
                            hub.pending_lua_spawns.push((connection_id, vm_id, path));
                        }
                    }
                    Some(game::process_spy_client_message::Msg::KillProcess(KillProcess { pid })) => {
                        let vm_id = {
                            let hub = hub_for_recv.lock().unwrap();
                            hub.connections.get(&connection_id).map(|c| c.vm_id)
                        };
                        if let Some(vm_id) = vm_id {
                            let mut hub = hub_for_recv.lock().unwrap();
                            hub.pending_kills.push((vm_id, pid));
                        }
                    }
                    _ => {}
                }
            }
            let mut hub = hub_for_recv.lock().unwrap();
            hub.connections.remove(&connection_id);
        });

        tokio::spawn(async move {
            while let Some(msg) = downstream_rx.recv().await {
                let server_msg = match msg {
                    ProcessSpyDownstreamMsg::ProcessList(snapshots) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::ProcessListSnapshot(ProcessListSnapshot {
                            processes: snapshots
                                .into_iter()
                                .map(|s| ProcessEntry {
                                    pid: s.pid,
                                    name: s.name,
                                    username: s.username,
                                    status: s.status,
                                    memory_bytes: s.memory_bytes,
                                    args: s.args,
                                })
                                .collect(),
                        })),
                    },
                    ProcessSpyDownstreamMsg::Stdout(pid, data) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::ProcessSpyStdout(ProcessSpyStdout {
                            pid,
                            data: data.into_bytes(),
                        })),
                    },
                    ProcessSpyDownstreamMsg::StdinChunk(pid, data) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::StdinChunk(StdinChunk {
                            pid,
                            data: data.into_bytes(),
                        })),
                    },
                    ProcessSpyDownstreamMsg::ProcessGone(pid) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::ProcessGone(ProcessGone { pid })),
                    },
                    ProcessSpyDownstreamMsg::LuaScriptSpawned(pid) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::LuaScriptSpawned(LuaScriptSpawned { pid })),
                    },
                    ProcessSpyDownstreamMsg::Error(message) => ProcessSpyServerMessage {
                        msg: Some(ProcessSpyServerMsg::ProcessSpyError(ProcessSpyError {
                            message,
                        })),
                    },
                };
                if tx.send(Ok(server_msg)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_disk_usage(
        &self,
        request: Request<GetDiskUsageRequest>,
    ) -> Result<Response<GetDiskUsageResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let used_bytes = self
            .fs_service
            .disk_usage_bytes(vm.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let total_bytes = (vm.disk_mb as i64) * 1024 * 1024;

        Ok(Response::new(GetDiskUsageResponse {
            used_bytes,
            total_bytes,
            error_message: String::new(),
        }))
    }

    async fn get_process_list(
        &self,
        request: Request<GetProcessListRequest>,
    ) -> Result<Response<GetProcessListResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let processes: Vec<ProcessEntry> = self
            .process_snapshot_store
            .get(&vm.id)
            .map(|guard| {
                guard
                    .iter()
                    .map(|s| ProcessEntry {
                        pid: s.pid,
                        name: s.name.clone(),
                        username: s.username.clone(),
                        status: s.status.clone(),
                        memory_bytes: s.memory_bytes,
                        args: s.args.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let disk_used_bytes = self
            .fs_service
            .disk_usage_bytes(vm.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let disk_total_bytes = (vm.disk_mb as i64) * 1024 * 1024;
        let vm_lua_memory_bytes = self
            .vm_lua_memory_store
            .get(&vm.id)
            .map(|g| *g)
            .unwrap_or(0);

        Ok(Response::new(GetProcessListResponse {
            processes,
            disk_used_bytes,
            disk_total_bytes,
            error_message: String::new(),
            vm_lua_memory_bytes,
        }))
    }

    async fn get_sysinfo(
        &self,
        request: Request<GetSysinfoRequest>,
    ) -> Result<Response<GetSysinfoResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        Ok(Response::new(GetSysinfoResponse {
            cpu_cores: vm.cpu_cores as i32,
            memory_mb: vm.memory_mb,
            disk_mb: vm.disk_mb,
            error_message: String::new(),
        }))
    }

    async fn restore_disk(
        &self,
        request: Request<RestoreDiskRequest>,
    ) -> Result<Response<RestoreDiskResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let record = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let vm_id = record.id;

        // Delete phase: remove all fs_nodes (cascade fs_contents), then vm_users
        self.fs_service
            .destroy_fs(vm_id)
            .await
            .map_err(|e| Status::internal(format!("destroy_fs: {}", e)))?;

        self.user_service
            .delete_all_for_vm(vm_id)
            .await
            .map_err(|e| Status::internal(format!("delete_all_for_vm: {}", e)))?;

        // Bootstrap phase: same as create_vm (minus NIC/DNS)
        self.fs_service
            .bootstrap_fs(vm_id)
            .await
            .map_err(|e| Status::internal(format!("bootstrap_fs: {}", e)))?;

        for (name, source) in bin_programs::DEFAULT_BIN_PROGRAMS {
            let path = format!("/bin/{}", name);
            self.fs_service
                .write_file(
                    vm_id,
                    &path,
                    source.as_bytes(),
                    Some("application/x-nulltrace-lua"),
                    "root",
                )
                .await
                .map_err(|e| Status::internal(format!("write {}: {}", path, e)))?;
        }

        let mut users: Vec<VmUser> = self
            .user_service
            .bootstrap_users(vm_id)
            .await
            .map_err(|e| Status::internal(format!("bootstrap_users: {}", e)))?;

        if let Some(owner_id) = record.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            let owner_home = format!("/home/{}", player.username);
            let owner_user = self
                .user_service
                .create_user(
                    vm_id,
                    &player.username,
                    1001,
                    Some(&player.password_hash),
                    true,
                    &owner_home,
                    "/bin/sh",
                )
                .await
                .map_err(|e| Status::internal(format!("create_user: {}", e)))?;
            users.push(owner_user);
        }

        for user in &users {
            if self
                .fs_service
                .resolve_path(vm_id, &user.home_dir)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .is_none()
            {
                self.fs_service
                    .mkdir(vm_id, &user.home_dir, &user.username)
                    .await
                    .map_err(|e| Status::internal(format!("mkdir: {}", e)))?;
            }
            self.fs_service
                .ensure_standard_home_subdirs(vm_id, &user.home_dir, &user.username)
                .await
                .map_err(|e| Status::internal(format!("ensure_standard_home_subdirs: {}", e)))?;
        }

        // Seed default files in owner's Documents (same as create_vm; for testing find, grep, cat, lua)
        if let Some(owner_id) = record.owner_id {
            if let Ok(Some(player)) = self.player_service.get_by_id(owner_id).await {
                let documents_path = format!("/home/{}/Documents", player.username);
                self.fs_service
                    .seed_default_documents(vm_id, &documents_path, &player.username)
                    .await
                    .map_err(|e| Status::internal(format!("seed_default_documents: {}", e)))?;
            }
        }

        let passwd_content: String = users
            .iter()
            .map(|u| {
                let gid = if u.is_root { 0 } else { u.uid };
                format!(
                    "{}:x:{}:{}:{}:{}:{}",
                    u.username, u.uid, gid, u.username, u.home_dir, u.shell
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(vm_id, "/etc/passwd", passwd_content.as_bytes(), Some("text/plain"), "root")
            .await
            .map_err(|e| Status::internal(format!("write /etc/passwd: {}", e)))?;

        let shadow_content: String = users
            .iter()
            .map(|u| {
                let hash = u.password_hash.as_deref().unwrap_or("!");
                format!("{}:{}:19000:0:99999:7:::", u.username, hash)
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(
                vm_id,
                "/etc/shadow",
                shadow_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await
            .map_err(|e| Status::internal(format!("write /etc/shadow: {}", e)))?;

        Ok(Response::new(RestoreDiskResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_home_path(
        &self,
        request: Request<GetHomePathRequest>,
    ) -> Result<Response<GetHomePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let home_path = if let Some(owner_id) = vm.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            format!("/home/{}", player.username)
        } else {
            "/home/user".to_string()
        };

        Ok(Response::new(GetHomePathResponse {
            home_path,
            error_message: String::new(),
        }))
    }

    async fn list_fs(
        &self,
        request: Request<ListFsRequest>,
    ) -> Result<Response<ListFsResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let ListFsRequest { path, .. } = request.into_inner();

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let home_path = if let Some(owner_id) = vm.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::internal("Owner player not found"))?;
            format!("/home/{}", player.username)
        } else {
            "/home/user".to_string()
        };

        if !path_under_home(&path, &home_path) {
            return Ok(Response::new(ListFsResponse {
                entries: vec![],
                error_message: "Path must be under home".to_string(),
            }));
        }

        let entries = self
            .fs_service
            .ls(vm.id, &path)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let proto_entries: Vec<FsEntry> = entries
            .into_iter()
            .map(|e| FsEntry {
                name: e.name,
                node_type: e.node_type,
                size_bytes: e.size_bytes,
            })
            .collect();

        Ok(Response::new(ListFsResponse {
            entries: proto_entries,
            error_message: String::new(),
        }))
    }

    async fn copy_path(
        &self,
        request: Request<CopyPathRequest>,
    ) -> Result<Response<CopyPathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let CopyPathRequest {
            src_path,
            dest_path,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&src_path, &owner.1) || !path_under_home(&dest_path, &owner.1) {
            return Ok(Response::new(CopyPathResponse {
                success: false,
                error_message: "Paths must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .copy_path_recursive(vm.id, &src_path, &dest_path, &owner.0)
            .await
        {
            Ok(()) => Ok(Response::new(CopyPathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(CopyPathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn move_path(
        &self,
        request: Request<MovePathRequest>,
    ) -> Result<Response<MovePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let MovePathRequest {
            src_path,
            dest_path,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&src_path, &owner.1) || !path_under_home(&dest_path, &owner.1) {
            return Ok(Response::new(MovePathResponse {
                success: false,
                error_message: "Paths must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .move_node(vm.id, &src_path, &dest_path)
            .await
        {
            Ok(()) => Ok(Response::new(MovePathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(MovePathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn rename_path(
        &self,
        request: Request<RenamePathRequest>,
    ) -> Result<Response<RenamePathResponse>, Status> {
        // Authenticate request
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let RenamePathRequest {
            path,
            new_name,
            ..
        } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&path, &owner.1) {
            return Ok(Response::new(RenamePathResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .rename_node(vm.id, &path, &new_name)
            .await
        {
            Ok(()) => Ok(Response::new(RenamePathResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(RenamePathResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn create_folder(
        &self,
        request: Request<CreateFolderRequest>,
    ) -> Result<Response<CreateFolderResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let CreateFolderRequest { path } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&path, &owner.1) {
            return Ok(Response::new(CreateFolderResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }

        match self.fs_service.mkdir(vm.id, &path, &owner.1).await {
            Ok(_) => Ok(Response::new(CreateFolderResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateFolderResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let WriteFileRequest { path, content, .. } = request.into_inner();

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(&path, &owner.1) {
            return Ok(Response::new(WriteFileResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }

        match self
            .fs_service
            .write_file(vm.id, &path, &content, Some("text/plain"), &owner.1)
            .await
        {
            Ok(_) => Ok(Response::new(WriteFileResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Ok(Response::new(WriteFileResponse {
                success: false,
                error_message: e.to_string(),
            })),
        }
    }

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let ReadFileRequest { path } = request.into_inner();

        let (vm, _owner) = vm_and_owner(&self, player_id).await?;
        // Allow reading any path on the player's VM (e.g. /etc/mail/default, /etc/mail/<addr>/token).

        match self.fs_service.read_file(vm.id, &path).await {
            Ok(Some((data, _))) => Ok(Response::new(ReadFileResponse {
                success: true,
                error_message: String::new(),
                content: data,
            })),
            Ok(None) => Ok(Response::new(ReadFileResponse {
                success: false,
                error_message: "File not found".to_string(),
                content: vec![],
            })),
            Err(e) => Ok(Response::new(ReadFileResponse {
                success: false,
                error_message: e.to_string(),
                content: vec![],
            })),
        }
    }

    async fn empty_trash(
        &self,
        request: Request<EmptyTrashRequest>,
    ) -> Result<Response<EmptyTrashResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        let trash_path = owner.1.trim_end_matches('/').to_string() + "/Trash";
        if !path_under_home(&trash_path, &owner.1) {
            return Ok(Response::new(EmptyTrashResponse {
                success: false,
                error_message: "Trash path must be under home".to_string(),
            }));
        }

        let entries = match self.fs_service.ls(vm.id, &trash_path).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(Response::new(EmptyTrashResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        };

        for e in entries {
            let child_path = format!("{}/{}", trash_path.trim_end_matches('/'), e.name);
            if let Err(e) = self.fs_service.rm(vm.id, &child_path).await {
                return Ok(Response::new(EmptyTrashResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        }

        Ok(Response::new(EmptyTrashResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_installed_store_apps(
        &self,
        request: Request<GetInstalledStoreAppsRequest>,
    ) -> Result<Response<GetInstalledStoreAppsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        const PATH: &str = "/etc/installed-apps";
        const ALLOWED: &[&str] = &["sound", "network", "minesweeper", "pixelart", "pspy"];
        let content = match self.fs_service.read_file(vm.id, PATH).await {
            Ok(Some((data, _))) => data,
            Ok(None) | Err(_) => {
                return Ok(Response::new(GetInstalledStoreAppsResponse {
                    app_types: vec![],
                    error_message: String::new(),
                }));
            }
        };
        let s = match String::from_utf8(content) {
            Ok(x) => x,
            Err(_) => {
                return Ok(Response::new(GetInstalledStoreAppsResponse {
                    app_types: vec![],
                    error_message: "File is not valid UTF-8".to_string(),
                }));
            }
        };
        let allowed: std::collections::HashSet<&str> = ALLOWED.iter().copied().collect();
        let mut seen = std::collections::HashSet::new();
        let app_types: Vec<String> = s
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .filter(|l| allowed.contains(&l[..]))
            .filter(|l| seen.insert(&l[..]))
            .map(String::from)
            .collect();
        Ok(Response::new(GetInstalledStoreAppsResponse {
            app_types,
            error_message: String::new(),
        }))
    }

    async fn install_store_app(
        &self,
        request: Request<InstallStoreAppRequest>,
    ) -> Result<Response<InstallStoreAppResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        const PATH: &str = "/etc/installed-apps";
        const ALLOWED: &[&str] = &["sound", "network", "minesweeper", "pixelart", "pspy"];
        let InstallStoreAppRequest { app_type, .. } = request.into_inner();
        let app_type = app_type.trim();
        if !ALLOWED.contains(&app_type) {
            return Ok(Response::new(InstallStoreAppResponse {
                success: false,
                error_message: "Invalid app type".to_string(),
            }));
        }

        let content = match self.fs_service.read_file(vm.id, PATH).await {
            Ok(Some((data, _))) => data,
            Ok(None) | Err(_) => Vec::new(),
        };
        let s = String::from_utf8(content).unwrap_or_default();
        let allowed: std::collections::HashSet<&str> = ALLOWED.iter().copied().collect();
        let mut lines: Vec<String> = s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && allowed.contains(&l[..]))
            .collect();
        if lines.contains(&app_type.to_string()) {
            return Ok(Response::new(InstallStoreAppResponse {
                success: true,
                error_message: String::new(),
            }));
        }
        lines.push(app_type.to_string());
        let new_content = lines.join("\n") + "\n";
        if let Err(e) = self
            .fs_service
            .write_file(vm.id, PATH, new_content.as_bytes(), Some("text/plain"), "root")
            .await
        {
            return Ok(Response::new(InstallStoreAppResponse {
                success: false,
                error_message: e.to_string(),
            }));
        }
        Ok(Response::new(InstallStoreAppResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn uninstall_store_app(
        &self,
        request: Request<UninstallStoreAppRequest>,
    ) -> Result<Response<UninstallStoreAppResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        const PATH: &str = "/etc/installed-apps";
        const ALLOWED: &[&str] = &["sound", "network", "minesweeper", "pixelart", "pspy"];
        let UninstallStoreAppRequest { app_type, .. } = request.into_inner();
        let app_type = app_type.trim();

        let content = match self.fs_service.read_file(vm.id, PATH).await {
            Ok(Some((data, _))) => data,
            Ok(None) | Err(_) => {
                return Ok(Response::new(UninstallStoreAppResponse {
                    success: true,
                    error_message: String::new(),
                }));
            }
        };
        let s = String::from_utf8(content).unwrap_or_default();
        let allowed: std::collections::HashSet<&str> = ALLOWED.iter().copied().collect();
        let lines: Vec<String> = s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && allowed.contains(&l[..]))
            .filter(|l| &l[..] != app_type)
            .collect();
        let new_content = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
        if let Err(e) = self
            .fs_service
            .write_file(vm.id, PATH, new_content.as_bytes(), Some("text/plain"), "root")
            .await
        {
            return Ok(Response::new(UninstallStoreAppResponse {
                success: false,
                error_message: e.to_string(),
            }));
        }
        Ok(Response::new(UninstallStoreAppResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_ranking(
        &self,
        request: Request<GetRankingRequest>,
    ) -> Result<Response<GetRankingResponse>, Status> {
        let _ = self.authenticate_request(&request)?;

        let rows = self
            .player_service
            .get_ranking()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut entries = Vec::with_capacity(rows.len());
        for (rank, id, username, points, faction_id) in rows {
            let faction_id_str = faction_id.map(|u| u.to_string()).unwrap_or_default();
            let faction_name = match faction_id {
                Some(fid) => self
                    .faction_service
                    .get_by_id(fid)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?
                    .map(|f| f.name)
                    .unwrap_or_default(),
                None => String::new(),
            };
            entries.push(RankingEntry {
                rank,
                player_id: id.to_string(),
                username,
                points,
                faction_id: faction_id_str,
                faction_name,
            });
        }

        Ok(Response::new(GetRankingResponse {
            entries,
            error_message: String::new(),
        }))
    }

    async fn get_player_profile(
        &self,
        request: Request<GetPlayerProfileRequest>,
    ) -> Result<Response<GetPlayerProfileResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let ranking = self
            .player_service
            .get_ranking()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let player_rank = ranking
            .iter()
            .position(|(_, id, _, _, _)| *id == player_id)
            .map(|i| (i + 1) as u32)
            .unwrap_or(0);

        let player = self
            .player_service
            .get_by_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Player not found"))?;

        let (faction_id_str, faction_name) = match player.faction_id {
            Some(fid) => {
                let name = self
                    .faction_service
                    .get_by_id(fid)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?
                    .map(|f| f.name)
                    .unwrap_or_default();
                (fid.to_string(), name)
            }
            None => (String::new(), String::new()),
        };

        let preferred_theme = player
            .preferred_theme
            .unwrap_or_else(|| "githubdark".to_string());
        let shortcuts_overrides = self
            .shortcuts_service
            .get_shortcuts(player_id)
            .await
            .unwrap_or_else(|_| "{}".to_string());
        Ok(Response::new(GetPlayerProfileResponse {
            rank: player_rank,
            points: player.points,
            faction_id: faction_id_str,
            faction_name,
            error_message: String::new(),
            preferred_theme,
            shortcuts_overrides,
        }))
    }

    async fn set_shortcuts(
        &self,
        request: Request<SetShortcutsRequest>,
    ) -> Result<Response<SetShortcutsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let SetShortcutsRequest {
            shortcuts_overrides_json,
        } = request.into_inner();
        match self
            .shortcuts_service
            .set_shortcuts(player_id, &shortcuts_overrides_json)
            .await
        {
            Ok(()) => Ok(Response::new(SetShortcutsResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(e) => {
                let msg = e.to_string();
                Ok(Response::new(SetShortcutsResponse {
                    success: false,
                    error_message: msg,
                }))
            }
        }
    }

    async fn set_preferred_theme(
        &self,
        request: Request<SetPreferredThemeRequest>,
    ) -> Result<Response<SetPreferredThemeResponse>, Status> {
        const ALLOWED_THEMES: &[&str] = &[
            "latte", "frappe", "macchiato", "mocha", "onedark", "dracula", "githubdark", "monokai",
            "solardark",
        ];
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let SetPreferredThemeRequest { preferred_theme } = request.into_inner();
        let theme = preferred_theme.trim();
        if theme.is_empty() || !ALLOWED_THEMES.contains(&theme) {
            return Ok(Response::new(SetPreferredThemeResponse {
                success: false,
                error_message: "Invalid theme".to_string(),
            }));
        }
        self.player_service
            .set_preferred_theme(player_id, theme)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SetPreferredThemeResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn create_faction(
        &self,
        request: Request<CreateFactionRequest>,
    ) -> Result<Response<CreateFactionResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let name = request.into_inner().name.trim().to_string();
        if name.is_empty() {
            return Ok(Response::new(CreateFactionResponse {
                faction_id: String::new(),
                name: String::new(),
                error_message: "Faction name is required".to_string(),
            }));
        }

        let player = self
            .player_service
            .get_by_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Player not found"))?;

        if player.faction_id.is_some() {
            return Ok(Response::new(CreateFactionResponse {
                faction_id: String::new(),
                name: String::new(),
                error_message: "Already in a faction; leave first".to_string(),
            }));
        }

        let faction = self
            .faction_service
            .create(&name, player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        self.player_service
            .set_faction_id(player_id, Some(faction.id))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateFactionResponse {
            faction_id: faction.id.to_string(),
            name: faction.name,
            error_message: String::new(),
        }))
    }

    async fn leave_faction(
        &self,
        request: Request<LeaveFactionRequest>,
    ) -> Result<Response<LeaveFactionResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        self.player_service
            .set_faction_id(player_id, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(LeaveFactionResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    // ── Email handlers ───────────────────────────────────────────────────────

    async fn get_emails(
        &self,
        request: Request<GetEmailsRequest>,
    ) -> Result<Response<GetEmailsResponse>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.email_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        let records = self
            .email_service
            .list_emails(&req.email_address, &req.folder)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let emails = records
            .into_iter()
            .map(|r| GrpcEmailMessage {
                id: r.id.to_string(),
                from_address: r.from_address,
                to_address: r.to_address,
                subject: r.subject,
                body: r.body,
                folder: r.folder,
                read: r.read,
                sent_at_ms: r.sent_at.timestamp_millis(),
            })
            .collect();
        Ok(Response::new(GetEmailsResponse { emails }))
    }

    async fn send_email(
        &self,
        request: Request<SendEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.from_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        // Insert into main recipient's inbox and notify.
        let inbox_record = self
            .email_service
            .insert_email(&req.from_address, &req.to_address, &req.subject, &req.body, "inbox")
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.to_address, inbox_record);
        // CC: insert into cc recipient's inbox and notify.
        if !req.cc_address.is_empty() {
            if let Ok(cc_record) = self
                .email_service
                .insert_email(&req.from_address, &req.cc_address, &req.subject, &req.body, "inbox")
                .await
            {
                super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.cc_address, cc_record);
            }
        }
        // Bcc: insert into bcc recipient's inbox and notify.
        if !req.bcc_address.is_empty() {
            if let Ok(bcc_record) = self
                .email_service
                .insert_email(&req.from_address, &req.bcc_address, &req.subject, &req.body, "inbox")
                .await
            {
                super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.bcc_address, bcc_record);
            }
        }
        // Insert a copy into sender's sent folder.
        let _ = self
            .email_service
            .insert_email(&req.from_address, &req.from_address, &req.subject, &req.body, "sent")
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SendEmailResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn mark_email_read(
        &self,
        request: Request<MarkEmailReadRequest>,
    ) -> Result<Response<MarkEmailReadResponse>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.email_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        let email_id = Uuid::parse_str(&req.email_id)
            .map_err(|_| Status::invalid_argument("Invalid email_id"))?;
        self.email_service
            .mark_read(email_id, req.read)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        // Push updated unread count so connected clients (e.g. multiple tabs) see the badge update.
        if let Ok(count) = self.email_service.unread_count(&req.email_address).await {
            super::mailbox_hub::notify_unread_count(&self.mailbox_hub, &req.email_address, count);
        }
        Ok(Response::new(MarkEmailReadResponse { success: true }))
    }

    async fn move_email(
        &self,
        request: Request<MoveEmailRequest>,
    ) -> Result<Response<MoveEmailResponse>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.email_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        let email_id = Uuid::parse_str(&req.email_id)
            .map_err(|_| Status::invalid_argument("Invalid email_id"))?;
        self.email_service
            .move_to_folder(email_id, &req.folder)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(MoveEmailResponse { success: true }))
    }

    async fn delete_email(
        &self,
        request: Request<DeleteEmailRequest>,
    ) -> Result<Response<DeleteEmailResponse>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.email_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        let email_id = Uuid::parse_str(&req.email_id)
            .map_err(|_| Status::invalid_argument("Invalid email_id"))?;
        self.email_service
            .delete_email(email_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteEmailResponse { success: true }))
    }

    async fn mailbox_stream(
        &self,
        request: Request<MailboxStreamRequest>,
    ) -> Result<Response<Self::MailboxStreamStream>, Status> {
        let req = request.into_inner();
        let valid = self
            .email_account_service
            .validate_token(&req.email_address, &req.mail_token)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if !valid {
            return Err(Status::unauthenticated("Invalid email token"));
        }
        // Send initial unread count before streaming events.
        let unread = self
            .email_service
            .unread_count(&req.email_address)
            .await
            .unwrap_or(0);
        let mut receiver = super::mailbox_hub::subscribe(&self.mailbox_hub, &req.email_address);
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            // Send initial unread count.
            let _ = tx
                .send(Ok(MailboxStreamMessage {
                    payload: Some(game::mailbox_stream_message::Payload::UnreadCount(unread)),
                }))
                .await;
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let msg = match event {
                            MailboxEvent::NewEmail(record) => MailboxStreamMessage {
                                payload: Some(game::mailbox_stream_message::Payload::NewEmail(
                                    GrpcEmailMessage {
                                        id: record.id.to_string(),
                                        from_address: record.from_address,
                                        to_address: record.to_address,
                                        subject: record.subject,
                                        body: record.body,
                                        folder: record.folder,
                                        read: record.read,
                                        sent_at_ms: record.sent_at.timestamp_millis(),
                                    },
                                )),
                            },
                            MailboxEvent::UnreadCount(count) => MailboxStreamMessage {
                                payload: Some(game::mailbox_stream_message::Payload::UnreadCount(count)),
                            },
                        };
                        if tx.send(Ok(msg)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Skipped messages; continue receiving.
                        continue;
                    }
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Returns true if path is under home (normalized, no traversal).
fn path_under_home(path: &str, home: &str) -> bool {
    let path_norm = normalize_path(path);
    let home_norm = normalize_path(home);
    path_norm == home_norm
        || (path_norm.starts_with(&home_norm) && path_norm.len() > home_norm.len() && path_norm.as_bytes().get(home_norm.len()) == Some(&b'/'))
}

fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    let mut resolved: Vec<&str> = Vec::new();
    for p in parts {
        if p == ".." {
            resolved.pop();
        } else {
            resolved.push(p);
        }
    }
    if resolved.is_empty() {
        "/".to_string()
    } else {
        "/".to_string() + &resolved.join("/")
    }
}

async fn vm_and_owner(
    svc: &ClusterGameService,
    player_id: Uuid,
) -> Result<
    (
        super::db::vm_service::VmRecord,
        (String, String),
    ),
    Status,
> {
    let vm = svc
        .vm_service
        .get_vm_by_owner_id(player_id)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::not_found("No VM found for this player"))?;
    let home_path = if let Some(owner_id) = vm.owner_id {
        let player = svc
            .player_service
            .get_by_id(owner_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::internal("Owner player not found"))?;
        (player.username.clone(), format!("/home/{}", player.username))
    } else {
        ("user".to_string(), "/home/user".to_string())
    };
    Ok((vm, home_path))
}

#[cfg(test)]
mod tests {
    use super::super::db::{
        self, email_account_service::EmailAccountService, email_service::EmailService,
        faction_service::FactionService, fs_service::FsService,
        player_service::PlayerService, shortcuts_service::ShortcutsService, user_service::UserService,
        vm_service::{VmConfig, VmService},
    };
    use super::super::mailbox_hub;
    use super::super::process_run_hub::new_hub as new_process_run_hub;
    use super::super::process_spy_hub::new_hub as new_process_spy_hub;
    use super::super::terminal_hub::new_hub;
    use super::super::vm_manager::ProcessSnapshot;
    use super::*;
    use crate::auth;
    use dashmap::DashMap;
    use std::sync::Arc;
    use tonic::Request;

    fn test_cluster_game_service(pool: &sqlx::PgPool) -> ClusterGameService {
        ClusterGameService::new(
            Arc::new(PlayerService::new(pool.clone())),
            Arc::new(VmService::new(pool.clone())),
            Arc::new(FsService::new(pool.clone())),
            Arc::new(UserService::new(pool.clone())),
            Arc::new(FactionService::new(pool.clone())),
            Arc::new(ShortcutsService::new(pool.clone())),
            Arc::new(EmailService::new(pool.clone())),
            Arc::new(EmailAccountService::new(pool.clone())),
            mailbox_hub::new_hub(),
            new_hub(),
            new_process_spy_hub(),
            new_process_run_hub(),
            Arc::new(DashMap::new()),
            Arc::new(DashMap::new()),
        )
    }

    fn test_cluster_game_service_with_store(
        pool: &sqlx::PgPool,
        process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
    ) -> ClusterGameService {
        ClusterGameService::new(
            Arc::new(PlayerService::new(pool.clone())),
            Arc::new(VmService::new(pool.clone())),
            Arc::new(FsService::new(pool.clone())),
            Arc::new(UserService::new(pool.clone())),
            Arc::new(FactionService::new(pool.clone())),
            Arc::new(ShortcutsService::new(pool.clone())),
            Arc::new(EmailService::new(pool.clone())),
            Arc::new(EmailAccountService::new(pool.clone())),
            mailbox_hub::new_hub(),
            new_hub(),
            new_process_spy_hub(),
            new_process_run_hub(),
            process_snapshot_store,
            Arc::new(DashMap::new()),
        )
    }

    #[tokio::test]
    async fn test_grpc_login_success() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let name = format!("grpcuser_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "secret").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let res = svc
            .login(Request::new(LoginRequest {
                username: name.clone(),
                password: "secret".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(out.success);
        assert!(!out.player_id.is_empty());
        assert!(out.error_message.is_empty());
    }

    #[tokio::test]
    async fn test_grpc_login_wrong_password() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let name = format!("grpcwrong_{}", uuid::Uuid::new_v4());
        player_service.create_player(&name, "right").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let res = svc
            .login(Request::new(LoginRequest {
                username: name,
                password: "wrong".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(!out.success);
        assert!(out.player_id.is_empty());
        assert_eq!(out.error_message, "Invalid credentials");
    }

    #[tokio::test]
    async fn test_grpc_login_empty_username() {
        let pool = db::test_pool().await;
        let svc = test_cluster_game_service(&pool);

        let res = svc
            .login(Request::new(LoginRequest {
                username: "".to_string(),
                password: "any".to_string(),
            }))
            .await
            .unwrap();
        let out = res.into_inner();
        assert!(!out.success);
        assert_eq!(out.error_message, "Username is required");
    }

    #[tokio::test]
    async fn test_grpc_ping_returns_time() {
        let pool = db::test_pool().await;
        let svc = test_cluster_game_service(&pool);

        let res = svc.ping(Request::new(PingRequest {})).await.unwrap();
        let out = res.into_inner();
        assert!(out.server_time_ms > 0);
    }

    #[tokio::test]
    async fn test_grpc_get_process_list_unauthenticated() {
        let pool = db::test_pool().await;
        let svc = test_cluster_game_service(&pool);

        let request = Request::new(GetProcessListRequest {});
        // No authorization metadata
        let res = svc.get_process_list(request).await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_grpc_get_process_list_success() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let name = format!("procuser_{}", Uuid::new_v4());
        let player = player_service.create_player(&name, "secret").await.unwrap();
        let player_id = player.id;

        let vm_id = Uuid::new_v4();
        let vm = vm_service
            .create_vm(
                vm_id,
                VmConfig {
                    hostname: "test-monitor-vm".to_string(),
                    dns_name: None,
                    cpu_cores: 2,
                    memory_mb: 1024,
                    disk_mb: 20480,
                    ip: None,
                    subnet: None,
                    gateway: None,
                    mac: None,
                    owner_id: Some(player_id),
                    create_email_account: true,
                },
            )
            .await
            .unwrap();

        let snapshot = vec![
            ProcessSnapshot {
                pid: 1,
                name: "lua".to_string(),
                username: "user".to_string(),
                status: "running".to_string(),
                memory_bytes: 65_536,
                args: vec!["lua".to_string(), "/tmp/script.lua".to_string()],
            },
            ProcessSnapshot {
                pid: 2,
                name: "init".to_string(),
                username: "root".to_string(),
                status: "finished".to_string(),
                memory_bytes: 32_768,
                args: vec!["init".to_string()],
            },
        ];
        let process_snapshot_store = Arc::new(DashMap::new());
        process_snapshot_store.insert(vm_id, snapshot.clone());

        let svc = test_cluster_game_service_with_store(&pool, process_snapshot_store);

        let token = auth::generate_token(player_id, &name, &auth::get_jwt_secret())
            .expect("generate token");
        let mut request = Request::new(GetProcessListRequest {});
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = svc.get_process_list(request).await.unwrap();
        let out = res.into_inner();

        assert_eq!(out.processes.len(), 2);
        assert_eq!(out.processes[0].pid, 1);
        assert_eq!(out.processes[0].name, "lua");
        assert_eq!(out.processes[0].username, "user");
        assert_eq!(out.processes[0].status, "running");
        assert_eq!(out.processes[0].memory_bytes, 65_536);
        assert_eq!(out.processes[1].pid, 2);
        assert_eq!(out.processes[1].name, "init");
        assert_eq!(out.processes[1].status, "finished");
        assert_eq!(out.processes[1].memory_bytes, 32_768);

        assert_eq!(out.disk_used_bytes, 0, "new VM has no files");
        assert_eq!(out.disk_total_bytes, (vm.disk_mb as i64) * 1024 * 1024);
        assert!(out.error_message.is_empty());

        vm_service.delete_vm(vm_id).await.unwrap();
    }
}
