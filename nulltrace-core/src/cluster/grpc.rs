//! gRPC GameService implementation (Ping, Login, SayHello, TerminalStream, GetDiskUsage, RestoreDisk).
//! Used by the unified cluster binary.

use super::bin_programs;
use super::db::email_account_service::EmailAccountService;
use super::db::email_service::EmailService;
use super::db::faction_invite_service::FactionInviteService;
use super::db::faction_member_service::FactionMemberService;
use super::db::faction_service::FactionService;
use super::db::fs_service::FsService;
use super::db::player_service::PlayerService;
use super::db::shortcuts_service::ShortcutsService;
use super::db::user_service::{UserService, VmUser};
use super::db::vm_service::VmService;
use super::db::wallet_service::{WalletError, WalletService};
use super::db::codelab_service::CodelabService;
use super::db::feed_service::{FeedPostRow, FeedService};
use super::db::hackerboard_dm_service::HackerboardDmService;
use super::db::hackerboard_faction_chat_service::HackerboardFactionChatService;
use super::db::player_block_service::PlayerBlockService;
use super::db::wallet_card_service::WalletCardService;
use super::mailbox_hub::{MailboxHub, MailboxEvent};
use super::process_run_hub::{ProcessRunHub, RunProcessStreamMsg};
use super::process_spy_hub::{ProcessSpyConnection, ProcessSpyDownstreamMsg, ProcessSpyHub};
use super::resource_limits;
use super::terminal_hub::{SessionReady, TerminalHub};
use super::pixel_art_binary::validated_pixel_art_bytes;
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
    GetRankingRequest, GetRankingResponse, GetSysinfoRequest, GetSysinfoResponse,
    UpgradeVmRequest, UpgradeVmResponse, HelloRequest,
    HelloResponse, LeaveFactionRequest, LeaveFactionResponse, ListFsRequest, ListFsResponse,
    AcceptFactionInviteRequest, AcceptFactionInviteResponse,
    DeclineFactionInviteRequest, DeclineFactionInviteResponse,
    FactionInviteEntry, ListFactionInvitesRequest, ListFactionInvitesResponse,
    SendFactionInviteRequest, SendFactionInviteResponse,
    ListOutgoingFactionInvitesRequest, ListOutgoingFactionInvitesResponse,
    OutgoingFactionInviteEntry, CancelFactionInviteRequest, CancelFactionInviteResponse,
    KickFactionMemberRequest, KickFactionMemberResponse,
    UnbanFactionMemberRequest, UnbanFactionMemberResponse,
    ListFactionBannedMembersRequest, ListFactionBannedMembersResponse,
    FactionBannedMemberEntry,
    BlockHackerboardPlayerRequest, BlockHackerboardPlayerResponse,
    UnblockHackerboardPlayerRequest, UnblockHackerboardPlayerResponse,
    ListBlockedPlayersRequest, ListBlockedPlayersResponse, BlockedPlayerEntry,
    HackerboardDmMessageEntry, HackerboardDmThreadEntry, HackerboardFactionMessageEntry,
    ListHackerboardDmMessagesRequest, ListHackerboardDmMessagesResponse,
    ListHackerboardDmThreadsRequest, ListHackerboardDmThreadsResponse,
    ListHackerboardFactionMessagesRequest, ListHackerboardFactionMessagesResponse,
    SendHackerboardDmRequest, SendHackerboardDmResponse,
    SendHackerboardFactionMessageRequest, SendHackerboardFactionMessageResponse,
    InjectStdin, LoginRequest, LoginResponse, MovePathRequest, MovePathResponse,
    OpenCodeRun, OpenTerminal, PingRequest, PingResponse, ProcessEntry, ProcessGone, ProcessListSnapshot,
    ProcessSpyClientMessage, ProcessSpyOpened, ProcessSpyServerMessage, ProcessSpyStdout,
    ProcessSpyError, KillProcess, LuaScriptSpawned, RankingEntry, RefreshTokenRequest, RefreshTokenResponse,
    SpawnLuaScript,
    CreateFolderRequest, CreateFolderResponse, RenamePathRequest, RenamePathResponse, RestoreDiskRequest, RestoreDiskResponse,     SetPreferredThemeRequest,
    SetPreferredThemeResponse,     SetHackerboardLanguagePreferencesRequest,
    SetHackerboardLanguagePreferencesResponse,
    SetHackerboardAvatarFromVmPathRequest, SetHackerboardAvatarFromVmPathResponse,
    SetHackerboardFactionEmblemFromVmPathRequest, SetHackerboardFactionEmblemFromVmPathResponse,
    SetShortcutsRequest, SetShortcutsResponse, StdinChunk, StdinData,
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
    // Wallet RPCs
    GetWalletBalancesRequest, GetWalletBalancesResponse, WalletBalance as GrpcWalletBalance,
    GetWalletTransactionsRequest, GetWalletTransactionsResponse, WalletTransaction as GrpcWalletTx,
    GetWalletKeysRequest, GetWalletKeysResponse, WalletKey as GrpcWalletKey,
    TransferFundsRequest, TransferFundsResponse,
    ResolveTransferKeyRequest, ResolveTransferKeyResponse,
    ConvertFundsRequest, ConvertFundsResponse,
    GetWalletCardsRequest, GetWalletCardsResponse, WalletCard as GrpcWalletCard,
    CreateWalletCardRequest, CreateWalletCardResponse,
    DeleteWalletCardRequest, DeleteWalletCardResponse,
    GetCardTransactionsRequest, GetCardTransactionsResponse, CardTransaction as GrpcCardTx,
    GetCardStatementRequest, GetCardStatementResponse, CardStatement as GrpcCardStatement,
    PayCardBillRequest, PayCardBillResponse,
    PayAccountBillRequest, PayAccountBillResponse,
    // Codelab
    GetCodelabProgressRequest, GetCodelabProgressResponse,
    MarkCodelabSolvedRequest, MarkCodelabSolvedResponse,
    CreateFeedPostRequest, CreateFeedPostResponse, FeedPostEntry, ListFeedPostsRequest,
    ListFeedPostsResponse, ToggleFeedPostLikeRequest, ToggleFeedPostLikeResponse,
};

pub struct ClusterGameService {
    player_service: Arc<PlayerService>,
    vm_service: Arc<VmService>,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    faction_service: Arc<FactionService>,
    faction_invite_service: Arc<FactionInviteService>,
    faction_member_service: Arc<FactionMemberService>,
    player_block_service: Arc<PlayerBlockService>,
    hackerboard_dm_service: Arc<HackerboardDmService>,
    hackerboard_faction_chat_service: Arc<HackerboardFactionChatService>,
    shortcuts_service: Arc<ShortcutsService>,
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    wallet_service: Arc<WalletService>,
    wallet_card_service: Arc<WalletCardService>,
    codelab_service: Arc<CodelabService>,
    feed_service: Arc<FeedService>,
    mailbox_hub: MailboxHub,
    terminal_hub: Arc<TerminalHub>,
    process_spy_hub: Arc<ProcessSpyHub>,
    process_run_hub: Arc<ProcessRunHub>,
    process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
    vm_lua_memory_store: Arc<DashMap<Uuid, u64>>,
    vm_cpu_utilization_store: Arc<DashMap<Uuid, u8>>,
    /// After `upgrade_vm` updates CPU in DB, game loop applies new core count to in-memory `VirtualMachine` (tick budget).
    pending_cpu_core_sync: Arc<DashMap<Uuid, i16>>,
}

impl ClusterGameService {
    pub fn new(
        player_service: Arc<PlayerService>,
        vm_service: Arc<VmService>,
        fs_service: Arc<FsService>,
        user_service: Arc<UserService>,
        faction_service: Arc<FactionService>,
        faction_invite_service: Arc<FactionInviteService>,
        faction_member_service: Arc<FactionMemberService>,
        player_block_service: Arc<PlayerBlockService>,
        hackerboard_dm_service: Arc<HackerboardDmService>,
        hackerboard_faction_chat_service: Arc<HackerboardFactionChatService>,
        shortcuts_service: Arc<ShortcutsService>,
        email_service: Arc<EmailService>,
        email_account_service: Arc<EmailAccountService>,
        wallet_service: Arc<WalletService>,
        wallet_card_service: Arc<WalletCardService>,
        codelab_service: Arc<CodelabService>,
        feed_service: Arc<FeedService>,
        mailbox_hub: MailboxHub,
        terminal_hub: Arc<TerminalHub>,
        process_spy_hub: Arc<ProcessSpyHub>,
        process_run_hub: Arc<ProcessRunHub>,
        process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
        vm_lua_memory_store: Arc<DashMap<Uuid, u64>>,
        vm_cpu_utilization_store: Arc<DashMap<Uuid, u8>>,
        pending_cpu_core_sync: Arc<DashMap<Uuid, i16>>,
    ) -> Self {
        Self {
            player_service,
            vm_service,
            fs_service,
            user_service,
            faction_service,
            faction_invite_service,
            faction_member_service,
            player_block_service,
            hackerboard_dm_service,
            hackerboard_faction_chat_service,
            shortcuts_service,
            email_service,
            email_account_service,
            wallet_service,
            wallet_card_service,
            codelab_service,
            feed_service,
            mailbox_hub,
            terminal_hub,
            process_spy_hub,
            process_run_hub,
            process_snapshot_store,
            vm_lua_memory_store,
            vm_cpu_utilization_store,
            pending_cpu_core_sync,
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

fn feed_post_row_to_proto(r: FeedPostRow) -> FeedPostEntry {
    FeedPostEntry {
        id: r.id.to_string(),
        author_id: r.author_id.to_string(),
        author_username: r.author_username,
        body: r.body,
        language: r.language,
        created_at_ms: r.created_at.timestamp_millis(),
        reply_to_id: r.reply_to_id.map(|u| u.to_string()).unwrap_or_default(),
        post_type: r.post_type,
        like_count: r.like_count,
        liked_by_me: r.liked_by_me,
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
                // Ensure wallet exists for this player (idempotent, ignore errors)
                let _ = self.wallet_service.create_wallet_for_player(p.id).await;
                // Ensure default card exists (shared limit from player_credit_accounts)
                let cards = self.wallet_card_service.get_cards(p.id).await.unwrap_or_default();
                if cards.is_empty() {
                    let _ = self
                        .wallet_card_service
                        .create_card(p.id, Some("Default"), &p.username)
                        .await;
                }
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
                                    cpu_utilization_percent: s.cpu_utilization_percent,
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

        let real_used_bytes = self
            .fs_service
            .disk_usage_bytes(vm.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Return nominal scale for player display (used and total comparable).
        let used_bytes = resource_limits::real_disk_bytes_to_nominal_bytes(real_used_bytes);
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
        let include_resource_metrics = !request.get_ref().omit_resource_metrics;
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
                        cpu_utilization_percent: s.cpu_utilization_percent,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let real_disk_used_bytes = self
            .fs_service
            .disk_usage_bytes(vm.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        // Return nominal scale for player display (used and total comparable).
        let disk_used_bytes = resource_limits::real_disk_bytes_to_nominal_bytes(real_disk_used_bytes);
        let disk_total_bytes = (vm.disk_mb as i64) * 1024 * 1024;
        let vm_lua_memory_bytes = self
            .vm_lua_memory_store
            .get(&vm.id)
            .map(|g| *g)
            .unwrap_or(0);
        let cpu_from_store = self
            .vm_cpu_utilization_store
            .get(&vm.id)
            .map(|g| *g as u32)
            .unwrap_or(0);
        let (cpu_utilization_percent, memory_utilization_percent) = if include_resource_metrics {
            let real_memory_limit_bytes = resource_limits::nominal_ram_mb_to_real_bytes(vm.memory_mb) as u64;
            let mem = if real_memory_limit_bytes > 0 {
                ((vm_lua_memory_bytes as u128 * 100) / real_memory_limit_bytes as u128).min(100) as u32
            } else {
                0
            };
            (cpu_from_store, mem)
        } else {
            (0, 0)
        };

        Ok(Response::new(GetProcessListResponse {
            processes,
            disk_used_bytes,
            disk_total_bytes,
            error_message: String::new(),
            vm_lua_memory_bytes,
            cpu_utilization_percent,
            memory_utilization_percent,
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
            internet_plan_id: vm.internet_plan_id.clone(),
            internet_plan_next_billing_ms: vm.internet_plan_next_billing_ms.unwrap_or(0),
            error_message: String::new(),
        }))
    }

    async fn upgrade_vm(
        &self,
        request: Request<UpgradeVmRequest>,
    ) -> Result<Response<UpgradeVmResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();

        let upgrade_type = super::vm_upgrade_catalog::UpgradeType::from_str(&req.upgrade_type)
            .ok_or_else(|| Status::invalid_argument("Invalid upgrade_type; use cpu, ram, or disk"))?;

        let vm = self
            .vm_service
            .get_vm_by_owner_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("No VM found for this player"))?;

        let (current_value, is_upgrade) = match upgrade_type {
            super::vm_upgrade_catalog::UpgradeType::Cpu => {
                let current = vm.cpu_cores as i32;
                (current, req.new_value > current)
            }
            super::vm_upgrade_catalog::UpgradeType::Ram => {
                let current = vm.memory_mb / 1024;
                (current, req.new_value > current)
            }
            super::vm_upgrade_catalog::UpgradeType::Disk => {
                let current = vm.disk_mb / 1024;
                (current, req.new_value > current)
            }
        };

        if !is_upgrade {
            return Ok(Response::new(UpgradeVmResponse {
                success: false,
                error_message: format!(
                    "New value {} must be greater than current {}",
                    req.new_value, current_value
                ),
            }));
        }

        let price_cents = super::vm_upgrade_catalog::get_price_cents(upgrade_type, req.new_value)
            .ok_or_else(|| Status::invalid_argument("Invalid tier for this upgrade type"))?;

        if price_cents > 0 {
            let desc = match upgrade_type {
                super::vm_upgrade_catalog::UpgradeType::Cpu => "My Computer: CPU upgrade",
                super::vm_upgrade_catalog::UpgradeType::Ram => "My Computer: RAM upgrade",
                super::vm_upgrade_catalog::UpgradeType::Disk => "My Computer: Storage upgrade",
            };
            if let Err(e) = self
                .wallet_service
                .debit(player_id, "USD", price_cents, desc)
                .await
            {
                let msg = match &e {
                    WalletError::InsufficientBalance => "Insufficient balance".to_string(),
                    _ => e.to_string(),
                };
                return Ok(Response::new(UpgradeVmResponse {
                    success: false,
                    error_message: msg,
                }));
            }
        }

        let (cpu_cores, memory_mb, disk_mb) = match upgrade_type {
            super::vm_upgrade_catalog::UpgradeType::Cpu => {
                (Some(req.new_value as i16), None, None)
            }
            super::vm_upgrade_catalog::UpgradeType::Ram => {
                (None, Some(req.new_value * 1024), None)
            }
            super::vm_upgrade_catalog::UpgradeType::Disk => {
                (None, None, Some(req.new_value * 1024))
            }
        };

        self.vm_service
            .update_spec(vm.id, cpu_cores, memory_mb, disk_mb)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if upgrade_type == super::vm_upgrade_catalog::UpgradeType::Cpu {
            self.pending_cpu_core_sync
                .insert(vm.id, req.new_value as i16);
        }

        Ok(Response::new(UpgradeVmResponse {
            success: true,
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
        let claims = self.authenticate_request(&request)?;
        let viewer_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let rows = self
            .player_service
            .get_ranking_for_viewer(viewer_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut entries = Vec::with_capacity(rows.len());
        for (
            rank,
            id,
            username,
            points,
            faction_id,
            faction_creator_id,
            faction_allow_member_invites,
            avatar_pixel,
            emblem_pixel,
        ) in rows
        {
            let faction_id_str = faction_id.map(|u| u.to_string()).unwrap_or_default();
            let faction_creator_str = faction_creator_id
                .map(|u| u.to_string())
                .unwrap_or_default();
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
                faction_creator_id: faction_creator_str,
                faction_allow_member_invites,
                hackerboard_avatar_pixel: avatar_pixel.unwrap_or_default(),
                faction_hackerboard_emblem_pixel: emblem_pixel.unwrap_or_default(),
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
            hackerboard_feed_language_filter: player.hackerboard_feed_language_filter.clone(),
            hackerboard_post_language: player.hackerboard_post_language.clone(),
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

    async fn set_hackerboard_language_preferences(
        &self,
        request: Request<SetHackerboardLanguagePreferencesRequest>,
    ) -> Result<Response<SetHackerboardLanguagePreferencesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let SetHackerboardLanguagePreferencesRequest {
            feed_language_filter,
            post_language,
        } = request.into_inner();
        let feed = feed_language_filter.trim();
        let post = post_language.trim();
        if !PlayerService::is_valid_hackerboard_feed_filter(feed)
            || !PlayerService::is_valid_hackerboard_post_language(post)
        {
            return Ok(Response::new(SetHackerboardLanguagePreferencesResponse {
                success: false,
                error_message: "Invalid feed_language_filter or post_language".to_string(),
            }));
        }
        self.player_service
            .set_hackerboard_language_prefs(player_id, feed, post)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SetHackerboardLanguagePreferencesResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn set_hackerboard_avatar_from_vm_path(
        &self,
        request: Request<SetHackerboardAvatarFromVmPathRequest>,
    ) -> Result<Response<SetHackerboardAvatarFromVmPathResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let SetHackerboardAvatarFromVmPathRequest { vm_path } = request.into_inner();
        let path = vm_path.trim();
        if path.is_empty() {
            return Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
                success: false,
                error_message: "vm_path is required".to_string(),
            }));
        }
        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(path, &owner.1) {
            return Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }
        let data = match self.fs_service.read_file(vm.id, path).await {
            Ok(Some((bytes, _))) => bytes,
            Ok(None) => {
                return Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
                    success: false,
                    error_message: "File not found".to_string(),
                }));
            }
            Err(e) => {
                return Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        };
        let validated = match validated_pixel_art_bytes(&data) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        };
        self.player_service
            .set_hackerboard_avatar_pixel(player_id, Some(&validated))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SetHackerboardAvatarFromVmPathResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn set_hackerboard_faction_emblem_from_vm_path(
        &self,
        request: Request<SetHackerboardFactionEmblemFromVmPathRequest>,
    ) -> Result<Response<SetHackerboardFactionEmblemFromVmPathResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let SetHackerboardFactionEmblemFromVmPathRequest { vm_path } = request.into_inner();
        let path = vm_path.trim();
        if path.is_empty() {
            return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                success: false,
                error_message: "vm_path is required".to_string(),
            }));
        }
        let player = self
            .player_service
            .get_by_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Player not found"))?;
        let faction_id = match player.faction_id {
            Some(id) => id,
            None => {
                return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                    success: false,
                    error_message: "Not in a faction".to_string(),
                }));
            }
        };
        let faction = self
            .faction_service
            .get_by_id(faction_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::internal("Faction not found"))?;
        let creator = faction
            .creator_id
            .ok_or_else(|| Status::failed_precondition("Faction has no creator"))?;
        if creator != player_id {
            return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                success: false,
                error_message: "Only the faction creator can set the emblem".to_string(),
            }));
        }
        let (vm, owner) = vm_and_owner(&self, player_id).await?;
        if !path_under_home(path, &owner.1) {
            return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                success: false,
                error_message: "Path must be under home".to_string(),
            }));
        }
        let data = match self.fs_service.read_file(vm.id, path).await {
            Ok(Some((bytes, _))) => bytes,
            Ok(None) => {
                return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                    success: false,
                    error_message: "File not found".to_string(),
                }));
            }
            Err(e) => {
                return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        };
        let validated = match validated_pixel_art_bytes(&data) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
                    success: false,
                    error_message: e.to_string(),
                }));
            }
        };
        self.faction_service
            .set_hackerboard_emblem_pixel(faction_id, Some(&validated))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(SetHackerboardFactionEmblemFromVmPathResponse {
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

        self.faction_invite_service
            .cancel_pending_sent_by(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(LeaveFactionResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn send_faction_invite(
        &self,
        request: Request<SendFactionInviteRequest>,
    ) -> Result<Response<SendFactionInviteResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let from_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let target_username = request.into_inner().target_username.trim().to_string();
        if target_username.is_empty() {
            return Ok(Response::new(SendFactionInviteResponse {
                invite_id: String::new(),
                error_message: "Username is required".to_string(),
            }));
        }

        let inviter = self
            .player_service
            .get_by_id(from_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Player not found"))?;

        let faction_id = match inviter.faction_id {
            Some(f) => f,
            None => {
                return Ok(Response::new(SendFactionInviteResponse {
                    invite_id: String::new(),
                    error_message: "You are not in a faction".to_string(),
                }));
            }
        };

        let target = match self
            .player_service
            .get_by_username(&target_username)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(p) => p,
            None => {
                return Ok(Response::new(SendFactionInviteResponse {
                    invite_id: String::new(),
                    error_message: "Player not found".to_string(),
                }));
            }
        };

        match self
            .faction_invite_service
            .create_invite(faction_id, from_id, target.id)
            .await
        {
            Ok(id) => Ok(Response::new(SendFactionInviteResponse {
                invite_id: id.to_string(),
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(SendFactionInviteResponse {
                invite_id: String::new(),
                error_message: msg,
            })),
        }
    }

    async fn list_faction_invites(
        &self,
        request: Request<ListFactionInvitesRequest>,
    ) -> Result<Response<ListFactionInvitesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let _ = request.into_inner();

        let rows = self
            .faction_invite_service
            .list_pending_for_player(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let invites = rows
            .into_iter()
            .map(|r| FactionInviteEntry {
                invite_id: r.id.to_string(),
                faction_id: r.faction_id.to_string(),
                faction_name: r.faction_name,
                from_username: r.from_username,
                created_at_ms: r.created_at.timestamp_millis(),
            })
            .collect();

        Ok(Response::new(ListFactionInvitesResponse {
            invites,
            error_message: String::new(),
        }))
    }

    async fn accept_faction_invite(
        &self,
        request: Request<AcceptFactionInviteRequest>,
    ) -> Result<Response<AcceptFactionInviteResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let invite_id_trim = request.into_inner().invite_id.trim().to_string();
        let invite_id = match Uuid::parse_str(&invite_id_trim) {
            Ok(u) => u,
            Err(_) => {
                return Ok(Response::new(AcceptFactionInviteResponse {
                    success: false,
                    error_message: "Invalid invite id".to_string(),
                }));
            }
        };

        match self
            .faction_invite_service
            .accept_invite(invite_id, player_id)
            .await
        {
            Ok(()) => Ok(Response::new(AcceptFactionInviteResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(AcceptFactionInviteResponse {
                success: false,
                error_message: msg,
            })),
        }
    }

    async fn decline_faction_invite(
        &self,
        request: Request<DeclineFactionInviteRequest>,
    ) -> Result<Response<DeclineFactionInviteResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let invite_id_trim = request.into_inner().invite_id.trim().to_string();
        let invite_id = match Uuid::parse_str(&invite_id_trim) {
            Ok(u) => u,
            Err(_) => {
                return Ok(Response::new(DeclineFactionInviteResponse {
                    success: false,
                    error_message: "Invalid invite id".to_string(),
                }));
            }
        };

        match self
            .faction_invite_service
            .decline_invite(invite_id, player_id)
            .await
        {
            Ok(()) => Ok(Response::new(DeclineFactionInviteResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(DeclineFactionInviteResponse {
                success: false,
                error_message: msg,
            })),
        }
    }

    async fn list_outgoing_faction_invites(
        &self,
        request: Request<ListOutgoingFactionInvitesRequest>,
    ) -> Result<Response<ListOutgoingFactionInvitesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let _ = request.into_inner();

        let inviter = self
            .player_service
            .get_by_id(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Player not found"))?;

        let faction_id = match inviter.faction_id {
            Some(f) => f,
            None => {
                return Ok(Response::new(ListOutgoingFactionInvitesResponse {
                    invites: vec![],
                    error_message: "You are not in a faction".to_string(),
                }));
            }
        };

        let rows = self
            .faction_invite_service
            .list_outgoing_pending_for_faction(faction_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let invites = rows
            .into_iter()
            .map(|r| OutgoingFactionInviteEntry {
                invite_id: r.id.to_string(),
                to_username: r.to_username,
                from_username: r.from_username,
                from_player_id: r.from_player_id.to_string(),
                created_at_ms: r.created_at.timestamp_millis(),
            })
            .collect();

        Ok(Response::new(ListOutgoingFactionInvitesResponse {
            invites,
            error_message: String::new(),
        }))
    }

    async fn cancel_faction_invite(
        &self,
        request: Request<CancelFactionInviteRequest>,
    ) -> Result<Response<CancelFactionInviteResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let actor_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let invite_id_trim = request.into_inner().invite_id.trim().to_string();
        let invite_id = match Uuid::parse_str(&invite_id_trim) {
            Ok(u) => u,
            Err(_) => {
                return Ok(Response::new(CancelFactionInviteResponse {
                    success: false,
                    error_message: "Invalid invite id".to_string(),
                }));
            }
        };

        match self
            .faction_invite_service
            .cancel_invite(invite_id, actor_id)
            .await
        {
            Ok(()) => Ok(Response::new(CancelFactionInviteResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(CancelFactionInviteResponse {
                success: false,
                error_message: msg,
            })),
        }
    }

    async fn kick_faction_member(
        &self,
        request: Request<KickFactionMemberRequest>,
    ) -> Result<Response<KickFactionMemberResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let creator_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let inner = request.into_inner();
        let target_username = inner.target_username.trim().to_string();
        if target_username.is_empty() {
            return Ok(Response::new(KickFactionMemberResponse {
                success: false,
                error_message: "Username is required".to_string(),
            }));
        }

        match self
            .faction_member_service
            .kick_member(creator_id, &target_username, inner.ban_from_rejoin)
            .await
        {
            Ok(()) => Ok(Response::new(KickFactionMemberResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(KickFactionMemberResponse {
                success: false,
                error_message: msg,
            })),
        }
    }

    async fn unban_faction_member(
        &self,
        request: Request<UnbanFactionMemberRequest>,
    ) -> Result<Response<UnbanFactionMemberResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let creator_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let target_username = request.into_inner().target_username.trim().to_string();
        if target_username.is_empty() {
            return Ok(Response::new(UnbanFactionMemberResponse {
                success: false,
                error_message: "Username is required".to_string(),
            }));
        }

        match self
            .faction_member_service
            .unban_member(creator_id, &target_username)
            .await
        {
            Ok(()) => Ok(Response::new(UnbanFactionMemberResponse {
                success: true,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(UnbanFactionMemberResponse {
                success: false,
                error_message: msg,
            })),
        }
    }

    async fn list_faction_banned_members(
        &self,
        request: Request<ListFactionBannedMembersRequest>,
    ) -> Result<Response<ListFactionBannedMembersResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let creator_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let _ = request.into_inner();

        match self
            .faction_member_service
            .list_banned_members(creator_id)
            .await
        {
            Ok(rows) => {
                let entries: Vec<FactionBannedMemberEntry> = rows
                    .into_iter()
                    .map(|r| FactionBannedMemberEntry {
                        player_id: r.player_id.to_string(),
                        username: r.username,
                    })
                    .collect();
                Ok(Response::new(ListFactionBannedMembersResponse {
                    entries,
                    error_message: String::new(),
                }))
            }
            Err(msg) => Ok(Response::new(ListFactionBannedMembersResponse {
                entries: vec![],
                error_message: msg,
            })),
        }
    }

    async fn block_hackerboard_player(
        &self,
        request: Request<BlockHackerboardPlayerRequest>,
    ) -> Result<Response<BlockHackerboardPlayerResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let blocker_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let target_username = request.into_inner().target_username.trim().to_string();
        if target_username.is_empty() {
            return Ok(Response::new(BlockHackerboardPlayerResponse {
                error_message: "Username is required".to_string(),
            }));
        }

        let target = match self
            .player_service
            .get_by_username(&target_username)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(p) => p,
            None => {
                return Ok(Response::new(BlockHackerboardPlayerResponse {
                    error_message: "Player not found".to_string(),
                }));
            }
        };

        match self
            .player_block_service
            .block(blocker_id, target.id)
            .await
        {
            Ok(()) => Ok(Response::new(BlockHackerboardPlayerResponse {
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(BlockHackerboardPlayerResponse {
                error_message: msg,
            })),
        }
    }

    async fn unblock_hackerboard_player(
        &self,
        request: Request<UnblockHackerboardPlayerRequest>,
    ) -> Result<Response<UnblockHackerboardPlayerResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let blocker_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let target_username = request.into_inner().target_username.trim().to_string();
        if target_username.is_empty() {
            return Ok(Response::new(UnblockHackerboardPlayerResponse {
                error_message: "Username is required".to_string(),
            }));
        }

        let target = match self
            .player_service
            .get_by_username(&target_username)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(p) => p,
            None => {
                return Ok(Response::new(UnblockHackerboardPlayerResponse {
                    error_message: "Player not found".to_string(),
                }));
            }
        };

        match self
            .player_block_service
            .unblock(blocker_id, target.id)
            .await
        {
            Ok(()) => Ok(Response::new(UnblockHackerboardPlayerResponse {
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(UnblockHackerboardPlayerResponse {
                error_message: msg,
            })),
        }
    }

    async fn list_blocked_players(
        &self,
        request: Request<ListBlockedPlayersRequest>,
    ) -> Result<Response<ListBlockedPlayersResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let blocker_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let _ = request.into_inner();

        let rows = self
            .player_block_service
            .list_blocked_by_blocker(blocker_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let blocked = rows
            .into_iter()
            .map(|r| BlockedPlayerEntry {
                player_id: r.blocked_id.to_string(),
                username: r.username,
            })
            .collect();

        Ok(Response::new(ListBlockedPlayersResponse {
            blocked,
            error_message: String::new(),
        }))
    }

    async fn send_hackerboard_dm(
        &self,
        request: Request<SendHackerboardDmRequest>,
    ) -> Result<Response<SendHackerboardDmResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let from_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let target = req.target_username.trim();
        match self
            .hackerboard_dm_service
            .send_message(from_id, target, &req.body)
            .await
        {
            Ok(id) => Ok(Response::new(SendHackerboardDmResponse {
                message_id: id.to_string(),
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(SendHackerboardDmResponse {
                message_id: String::new(),
                error_message: msg,
            })),
        }
    }

    async fn list_hackerboard_dm_threads(
        &self,
        request: Request<ListHackerboardDmThreadsRequest>,
    ) -> Result<Response<ListHackerboardDmThreadsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let limit = request.into_inner().limit;
        let lim = if limit <= 0 { 50 } else { limit as i64 };
        let rows = self
            .hackerboard_dm_service
            .list_threads(player_id, lim)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let threads = rows
            .into_iter()
            .map(|r| HackerboardDmThreadEntry {
                peer_player_id: r.peer_id.to_string(),
                peer_username: r.peer_username,
                last_message_id: r.last_message_id.to_string(),
                last_body: r.last_body,
                last_created_at_ms: r.last_created_at.timestamp_millis(),
            })
            .collect();
        Ok(Response::new(ListHackerboardDmThreadsResponse {
            threads,
            error_message: String::new(),
        }))
    }

    async fn list_hackerboard_dm_messages(
        &self,
        request: Request<ListHackerboardDmMessagesRequest>,
    ) -> Result<Response<ListHackerboardDmMessagesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let peer_username = req.peer_username.trim();
        if peer_username.is_empty() {
            return Ok(Response::new(ListHackerboardDmMessagesResponse {
                messages: vec![],
                error_message: "peer_username is required".to_string(),
            }));
        }
        let peer = match self.player_service.get_by_username(peer_username).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return Ok(Response::new(ListHackerboardDmMessagesResponse {
                    messages: vec![],
                    error_message: "Player not found".to_string(),
                }));
            }
            Err(e) => return Err(Status::internal(e.to_string())),
        };
        let before = if req.before_message_id.trim().is_empty() {
            None
        } else {
            match Uuid::parse_str(req.before_message_id.trim()) {
                Ok(u) => Some(u),
                Err(_) => {
                    return Ok(Response::new(ListHackerboardDmMessagesResponse {
                        messages: vec![],
                        error_message: "before_message_id must be a valid UUID".to_string(),
                    }));
                }
            }
        };
        let lim = if req.limit <= 0 { 50 } else { req.limit as i64 };
        match self
            .hackerboard_dm_service
            .list_messages(player_id, peer.id, before, lim)
            .await
        {
            Ok(rows) => {
                let messages = rows
                    .into_iter()
                    .map(|m| HackerboardDmMessageEntry {
                        id: m.id.to_string(),
                        from_player_id: m.from_player_id.to_string(),
                        body: m.body,
                        created_at_ms: m.created_at.timestamp_millis(),
                    })
                    .collect();
                Ok(Response::new(ListHackerboardDmMessagesResponse {
                    messages,
                    error_message: String::new(),
                }))
            }
            Err(msg) => Ok(Response::new(ListHackerboardDmMessagesResponse {
                messages: vec![],
                error_message: msg,
            })),
        }
    }

    async fn send_hackerboard_faction_message(
        &self,
        request: Request<SendHackerboardFactionMessageRequest>,
    ) -> Result<Response<SendHackerboardFactionMessageResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let from_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let body = request.into_inner().body;
        match self
            .hackerboard_faction_chat_service
            .send_message(from_id, &body)
            .await
        {
            Ok(id) => Ok(Response::new(SendHackerboardFactionMessageResponse {
                message_id: id.to_string(),
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(SendHackerboardFactionMessageResponse {
                message_id: String::new(),
                error_message: msg,
            })),
        }
    }

    async fn list_hackerboard_faction_messages(
        &self,
        request: Request<ListHackerboardFactionMessagesRequest>,
    ) -> Result<Response<ListHackerboardFactionMessagesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let before = if req.before_message_id.trim().is_empty() {
            None
        } else {
            match Uuid::parse_str(req.before_message_id.trim()) {
                Ok(u) => Some(u),
                Err(_) => {
                    return Ok(Response::new(ListHackerboardFactionMessagesResponse {
                        messages: vec![],
                        error_message: "before_message_id must be a valid UUID".to_string(),
                    }));
                }
            }
        };
        let lim = if req.limit <= 0 { 50 } else { req.limit as i64 };
        match self
            .hackerboard_faction_chat_service
            .list_messages(player_id, before, lim)
            .await
        {
            Ok(rows) => {
                let messages = rows
                    .into_iter()
                    .map(|m| HackerboardFactionMessageEntry {
                        id: m.id.to_string(),
                        from_player_id: m.from_player_id.to_string(),
                        from_username: m.from_username,
                        body: m.body,
                        created_at_ms: m.created_at.timestamp_millis(),
                    })
                    .collect();
                Ok(Response::new(ListHackerboardFactionMessagesResponse {
                    messages,
                    error_message: String::new(),
                }))
            }
            Err(msg) => Ok(Response::new(ListHackerboardFactionMessagesResponse {
                messages: vec![],
                error_message: msg,
            })),
        }
    }

    async fn list_feed_posts(
        &self,
        request: Request<ListFeedPostsRequest>,
    ) -> Result<Response<ListFeedPostsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let language_filter = req.language_filter.trim();
        let filter_opt = if language_filter.is_empty() {
            None
        } else if language_filter == "en" || language_filter == "pt-br" {
            Some(language_filter)
        } else {
            return Err(Status::invalid_argument(
                "language_filter must be empty, en, or pt-br",
            ));
        };
        let limit = if req.limit <= 0 { 50 } else { req.limit };
        let before_trim = req.before_post_id.trim();
        let before_post_id = if before_trim.is_empty() {
            None
        } else {
            Some(
                Uuid::parse_str(before_trim)
                    .map_err(|_| Status::invalid_argument("before_post_id must be a valid UUID"))?,
            )
        };
        let rows = self
            .feed_service
            .list_posts(filter_opt, limit, player_id, before_post_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let posts = rows.into_iter().map(feed_post_row_to_proto).collect();
        Ok(Response::new(ListFeedPostsResponse {
            posts,
            error_message: String::new(),
        }))
    }

    async fn create_feed_post(
        &self,
        request: Request<CreateFeedPostRequest>,
    ) -> Result<Response<CreateFeedPostResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let reply_to = if req.reply_to_post_id.trim().is_empty() {
            None
        } else {
            Some(
                Uuid::parse_str(req.reply_to_post_id.trim())
                    .map_err(|_| Status::invalid_argument("Invalid reply_to_post_id"))?,
            )
        };
        match self
            .feed_service
            .create_post(player_id, &req.body, &req.language, reply_to)
            .await
        {
            Ok(row) => Ok(Response::new(CreateFeedPostResponse {
                post: Some(feed_post_row_to_proto(row)),
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(CreateFeedPostResponse {
                post: None,
                error_message: msg,
            })),
        }
    }

    async fn toggle_feed_post_like(
        &self,
        request: Request<ToggleFeedPostLikeRequest>,
    ) -> Result<Response<ToggleFeedPostLikeResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let post_id = Uuid::parse_str(req.post_id.trim())
            .map_err(|_| Status::invalid_argument("Invalid post_id"))?;
        match self.feed_service.toggle_like(post_id, player_id).await {
            Ok((liked, like_count)) => Ok(Response::new(ToggleFeedPostLikeResponse {
                liked,
                like_count,
                error_message: String::new(),
            })),
            Err(msg) => Ok(Response::new(ToggleFeedPostLikeResponse {
                liked: false,
                like_count: 0,
                error_message: msg,
            })),
        }
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
        let page = req.page;
        let (records, has_more) = self
            .email_service
            .list_emails_page(&req.email_address, &req.folder, page)
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
                cc_address: r.cc_address.unwrap_or_default(),
            })
            .collect();
        Ok(Response::new(GetEmailsResponse {
            emails,
            has_more,
        }))
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
        let cc_for_display = if req.cc_address.is_empty() {
            None
        } else {
            Some(req.cc_address.as_str())
        };
        // Insert into main recipient's inbox and notify.
        let inbox_record = self
            .email_service
            .insert_email(
                &req.from_address,
                &req.to_address,
                &req.subject,
                &req.body,
                "inbox",
                cc_for_display,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.to_address, inbox_record);
        // CC: insert into cc recipient's inbox and notify.
        if !req.cc_address.is_empty() {
            if let Ok(cc_record) = self
                .email_service
                .insert_email(
                    &req.from_address,
                    &req.cc_address,
                    &req.subject,
                    &req.body,
                    "inbox",
                    None,
                )
                .await
            {
                super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.cc_address, cc_record);
            }
        }
        // Bcc: insert into bcc recipient's inbox and notify.
        if !req.bcc_address.is_empty() {
            if let Ok(bcc_record) = self
                .email_service
                .insert_email(
                    &req.from_address,
                    &req.bcc_address,
                    &req.subject,
                    &req.body,
                    "inbox",
                    None,
                )
                .await
            {
                super::mailbox_hub::notify_new_email(&self.mailbox_hub, &req.bcc_address, bcc_record);
            }
        }
        // Insert a copy into sender's sent folder (to_address = actual recipient so UI shows "To: ...").
        let _ = self
            .email_service
            .insert_email(
                &req.from_address,
                &req.to_address,
                &req.subject,
                &req.body,
                "sent",
                cc_for_display,
            )
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
                                        cc_address: record.cc_address.unwrap_or_default(),
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

    // ── Wallet handlers ───────────────────────────────────────────────────────

    async fn get_wallet_balances(
        &self,
        request: Request<GetWalletBalancesRequest>,
    ) -> Result<Response<GetWalletBalancesResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let balances = self
            .wallet_service
            .get_balances(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetWalletBalancesResponse {
            balances: balances
                .into_iter()
                .map(|b| GrpcWalletBalance {
                    currency: b.currency,
                    amount: b.balance,
                })
                .collect(),
            error_message: String::new(),
        }))
    }

    async fn get_wallet_transactions(
        &self,
        request: Request<GetWalletTransactionsRequest>,
    ) -> Result<Response<GetWalletTransactionsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let filter = request.into_inner().filter;
        let filter = if filter.is_empty() { "all".to_string() } else { filter };

        let txns = self
            .wallet_service
            .get_transactions(player_id, &filter)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetWalletTransactionsResponse {
            transactions: txns
                .into_iter()
                .map(|t| GrpcWalletTx {
                    id: t.id.to_string(),
                    tx_type: t.tx_type,
                    currency: t.currency,
                    amount: t.amount,
                    fee: t.fee,
                    description: t.description.unwrap_or_default(),
                    counterpart_address: t.counterpart_address.unwrap_or_default(),
                    created_at_ms: t.created_at.timestamp_millis(),
                })
                .collect(),
            error_message: String::new(),
        }))
    }

    async fn get_wallet_keys(
        &self,
        request: Request<GetWalletKeysRequest>,
    ) -> Result<Response<GetWalletKeysResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let keys = self
            .wallet_service
            .get_keys(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetWalletKeysResponse {
            keys: keys
                .into_iter()
                .map(|k| GrpcWalletKey {
                    currency: k.currency,
                    key_address: k.key_address,
                })
                .collect(),
            error_message: String::new(),
        }))
    }

    async fn transfer_funds(
        &self,
        request: Request<TransferFundsRequest>,
    ) -> Result<Response<TransferFundsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();

        if req.target_address.is_empty() || req.currency.is_empty() || req.amount <= 0 {
            return Err(Status::invalid_argument("target_address, currency and amount are required"));
        }

        self.wallet_service
            .transfer_to_address(player_id, &req.target_address, &req.currency, req.amount)
            .await
            .map_err(wallet_error_to_status)?;

        Ok(Response::new(TransferFundsResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn resolve_transfer_key(
        &self,
        request: Request<ResolveTransferKeyRequest>,
    ) -> Result<Response<ResolveTransferKeyResponse>, Status> {
        let _claims = self.authenticate_request(&request)?;
        let key = request.into_inner().key.trim().to_string();
        if key.is_empty() {
            return Ok(Response::new(ResolveTransferKeyResponse {
                is_valid: false,
                is_usd: false,
                account_holder_name: String::new(),
                target_currency: String::new(),
            }));
        }
        if key.starts_with("fkebank-") {
            let account = self
                .wallet_service
                .fkebank_service()
                .get_by_key(&key)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
            if let Some(acc) = account {
                let holder = if let Some(ref name) = acc.full_name {
                    name.clone()
                } else if acc.owner_type == "player" {
                    self.player_service
                        .get_by_id(acc.owner_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|p| p.username)
                        .unwrap_or_default()
                } else if acc.owner_type == "vm" {
                    self.vm_service
                        .get_vm(acc.owner_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|v| v.hostname)
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                return Ok(Response::new(ResolveTransferKeyResponse {
                    is_valid: true,
                    is_usd: true,
                    account_holder_name: holder,
                    target_currency: "USD".to_string(),
                }));
            }
            return Ok(Response::new(ResolveTransferKeyResponse {
                is_valid: false,
                is_usd: true,
                account_holder_name: String::new(),
                target_currency: String::new(),
            }));
        }
        // Crypto-style key: accept known formats (0x..., bc1q..., or SOL base58 ~44 chars)
        let valid_crypto = key.starts_with("0x") && key.len() == 42
            || key.starts_with("bc1q") && key.len() >= 40
            || (key.len() >= 40 && key.len() <= 50 && !key.contains('-'));
        let target_currency = if key.starts_with("bc1q") {
            "BTC".to_string()
        } else if key.starts_with("0x") && key.len() == 42 {
            "ETH".to_string()
        } else if valid_crypto {
            "SOL".to_string()
        } else {
            String::new()
        };
        Ok(Response::new(ResolveTransferKeyResponse {
            is_valid: valid_crypto,
            is_usd: false,
            account_holder_name: String::new(),
            target_currency,
        }))
    }

    async fn convert_funds(
        &self,
        request: Request<ConvertFundsRequest>,
    ) -> Result<Response<ConvertFundsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();

        if req.from_currency.is_empty() || req.to_currency.is_empty() || req.amount <= 0 {
            return Err(Status::invalid_argument("from_currency, to_currency and amount are required"));
        }

        let converted = self
            .wallet_service
            .convert(player_id, &req.from_currency, &req.to_currency, req.amount)
            .await
            .map_err(wallet_error_to_status)?;

        Ok(Response::new(ConvertFundsResponse {
            success: true,
            converted_amount: converted,
            error_message: String::new(),
        }))
    }

    async fn get_wallet_cards(
        &self,
        request: Request<GetWalletCardsRequest>,
    ) -> Result<Response<GetWalletCardsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let cards = self
            .wallet_card_service
            .get_cards(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let account_credit_limit = self
            .wallet_card_service
            .get_account_credit_limit(player_id)
            .await
            .unwrap_or(20_000);

        let account_total_debt = self
            .wallet_card_service
            .get_account_total_debt(player_id)
            .await
            .unwrap_or(0);

        Ok(Response::new(GetWalletCardsResponse {
            cards: cards.into_iter().map(card_to_grpc).collect(),
            account_credit_limit,
            account_total_debt,
            error_message: String::new(),
        }))
    }

    async fn create_wallet_card(
        &self,
        request: Request<CreateWalletCardRequest>,
    ) -> Result<Response<CreateWalletCardResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();

        let label = if req.label.is_empty() { None } else { Some(req.label.as_str()) };
        let holder_name = claims.username;

        let card = self
            .wallet_card_service
            .create_card(player_id, label, &holder_name)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateWalletCardResponse {
            card: Some(card_to_grpc(card)),
            error_message: String::new(),
        }))
    }

    async fn delete_wallet_card(
        &self,
        request: Request<DeleteWalletCardRequest>,
    ) -> Result<Response<DeleteWalletCardResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let card_id = Uuid::parse_str(&request.into_inner().card_id)
            .map_err(|_| Status::invalid_argument("Invalid card_id"))?;

        let found = self
            .wallet_card_service
            .delete_card(card_id, player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        if !found {
            return Err(Status::not_found("Card not found"));
        }

        Ok(Response::new(DeleteWalletCardResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_card_transactions(
        &self,
        request: Request<GetCardTransactionsRequest>,
    ) -> Result<Response<GetCardTransactionsResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let req = request.into_inner();
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card_id"))?;
        let filter = if req.filter.is_empty() { "all".to_string() } else { req.filter };

        let txns = self
            .wallet_card_service
            .get_card_transactions(card_id, &filter)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (card_label, card_last4) = self
            .wallet_card_service
            .get_cards(player_id)
            .await
            .ok()
            .and_then(|cards| cards.into_iter().find(|c| c.id == card_id))
            .map(|c| (c.label.unwrap_or_default(), c.last4))
            .unwrap_or_default();

        Ok(Response::new(GetCardTransactionsResponse {
            transactions: txns
                .into_iter()
                .map(|t| GrpcCardTx {
                    id: t.id.to_string(),
                    card_id: card_id.to_string(),
                    card_label: card_label.clone(),
                    card_last4: card_last4.clone(),
                    tx_type: t.tx_type,
                    amount: t.amount,
                    description: t.description.unwrap_or_default(),
                    created_at_ms: t.created_at.timestamp_millis(),
                })
                .collect(),
            error_message: String::new(),
        }))
    }

    async fn get_card_statement(
        &self,
        request: Request<GetCardStatementRequest>,
    ) -> Result<Response<GetCardStatementResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let _player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let card_id = Uuid::parse_str(&request.into_inner().card_id)
            .map_err(|_| Status::invalid_argument("Invalid card_id"))?;

        let stmt = self
            .wallet_card_service
            .get_or_create_open_statement(card_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetCardStatementResponse {
            statement: Some(GrpcCardStatement {
                id: stmt.id.to_string(),
                card_id: stmt.card_id.to_string(),
                period_start_ms: stmt.period_start.timestamp_millis(),
                period_end_ms: stmt.period_end.timestamp_millis(),
                total_amount: stmt.total_amount,
                status: stmt.status,
                due_date_ms: stmt.due_date.timestamp_millis(),
            }),
            error_message: String::new(),
        }))
    }

    async fn pay_card_bill(
        &self,
        request: Request<PayCardBillRequest>,
    ) -> Result<Response<PayCardBillResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let card_id = Uuid::parse_str(&request.into_inner().card_id)
            .map_err(|_| Status::invalid_argument("Invalid card_id"))?;

        let paid = self
            .wallet_card_service
            .pay_card_bill(card_id, player_id)
            .await
            .map_err(wallet_error_to_status)?;

        Ok(Response::new(PayCardBillResponse {
            success: true,
            amount_paid: paid,
            error_message: String::new(),
        }))
    }

    async fn pay_account_bill(
        &self,
        request: Request<PayAccountBillRequest>,
    ) -> Result<Response<PayAccountBillResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;

        let paid = self
            .wallet_card_service
            .pay_account_bill(player_id)
            .await
            .map_err(wallet_error_to_status)?;

        Ok(Response::new(PayAccountBillResponse {
            success: true,
            amount_paid: paid,
            error_message: String::new(),
        }))
    }

    async fn mark_codelab_solved(
        &self,
        request: Request<MarkCodelabSolvedRequest>,
    ) -> Result<Response<MarkCodelabSolvedResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let challenge_id = request.into_inner().challenge_id.trim().to_string();
        if challenge_id.is_empty() {
            return Ok(Response::new(MarkCodelabSolvedResponse {
                error_message: "challenge_id is required".to_string(),
            }));
        }
        self.codelab_service
            .mark_solved(player_id, &challenge_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(MarkCodelabSolvedResponse {
            error_message: String::new(),
        }))
    }

    async fn get_codelab_progress(
        &self,
        request: Request<GetCodelabProgressRequest>,
    ) -> Result<Response<GetCodelabProgressResponse>, Status> {
        let claims = self.authenticate_request(&request)?;
        let player_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Status::internal("Invalid player_id in token"))?;
        let solved_ids = self
            .codelab_service
            .get_solved_ids(player_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetCodelabProgressResponse {
            solved_challenge_ids: solved_ids,
            error_message: String::new(),
        }))
    }
}

fn wallet_error_to_status(e: WalletError) -> Status {
    match e {
        WalletError::InsufficientBalance => Status::failed_precondition("Insufficient balance"),
        WalletError::CardLimitExceeded => Status::failed_precondition("Card credit limit exceeded"),
        WalletError::InvalidCurrency => Status::invalid_argument("Invalid currency"),
        WalletError::ConvertedAmountTooSmall => {
            Status::invalid_argument("Converted amount is zero or too small")
        }
        WalletError::RecipientNotFound => Status::invalid_argument("Recipient not found"),
        WalletError::InvoiceNotFound => Status::not_found("Invoice not found"),
        WalletError::InvoiceAlreadyPaid => Status::failed_precondition("Invoice already paid"),
        WalletError::CardNotFound => Status::not_found("Card not found"),
        WalletError::CardInvalid => Status::invalid_argument("Card invalid (wrong CVV, expired, or holder)"),
        WalletError::Db(db_err) => Status::internal(db_err.to_string()),
    }
}

fn card_to_grpc(c: super::db::wallet_card_service::WalletCard) -> GrpcWalletCard {
    GrpcWalletCard {
        id: c.id.to_string(),
        label: c.label.unwrap_or_default(),
        number_full: c.number_full,
        last4: c.last4,
        expiry_month: c.expiry_month,
        expiry_year: c.expiry_year,
        cvv: c.cvv,
        holder_name: c.holder_name,
        credit_limit: c.credit_limit,
        current_debt: c.current_debt,
        is_virtual: c.is_virtual,
        is_active: c.is_active,
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
        self, codelab_service::CodelabService, email_account_service::EmailAccountService,
        email_service::EmailService,         faction_invite_service::FactionInviteService,
        faction_member_service::FactionMemberService,
        faction_service::FactionService, feed_service::FeedService,
        hackerboard_dm_service::HackerboardDmService,
        hackerboard_faction_chat_service::HackerboardFactionChatService,
        fs_service::FsService, player_block_service::PlayerBlockService,
        player_service::{Player, PlayerService}, shortcuts_service::ShortcutsService,
        user_service::UserService, vm_service::{VmConfig, VmService}, wallet_service::WalletService,
        wallet_card_service::WalletCardService,
    };
    use super::super::pixel_art_binary::{PIXEL_ART_MAGIC, PIXEL_ART_MIME};
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
            Arc::new(FactionInviteService::new(pool.clone())),
            Arc::new(FactionMemberService::new(pool.clone())),
            Arc::new(PlayerBlockService::new(pool.clone())),
            Arc::new(HackerboardDmService::new(pool.clone())),
            Arc::new(HackerboardFactionChatService::new(pool.clone())),
            Arc::new(ShortcutsService::new(pool.clone())),
            Arc::new(EmailService::new(pool.clone())),
            Arc::new(EmailAccountService::new(pool.clone())),
            Arc::new(WalletService::new(pool.clone())),
            Arc::new(WalletCardService::new(pool.clone())),
            Arc::new(CodelabService::new(pool.clone())),
            Arc::new(FeedService::new(pool.clone())),
            mailbox_hub::new_hub(),
            new_hub(),
            new_process_spy_hub(),
            new_process_run_hub(),
            Arc::new(DashMap::new()),
            Arc::new(DashMap::new()),
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
            Arc::new(FactionInviteService::new(pool.clone())),
            Arc::new(FactionMemberService::new(pool.clone())),
            Arc::new(PlayerBlockService::new(pool.clone())),
            Arc::new(HackerboardDmService::new(pool.clone())),
            Arc::new(HackerboardFactionChatService::new(pool.clone())),
            Arc::new(ShortcutsService::new(pool.clone())),
            Arc::new(EmailService::new(pool.clone())),
            Arc::new(EmailAccountService::new(pool.clone())),
            Arc::new(WalletService::new(pool.clone())),
            Arc::new(WalletCardService::new(pool.clone())),
            Arc::new(CodelabService::new(pool.clone())),
            Arc::new(FeedService::new(pool.clone())),
            mailbox_hub::new_hub(),
            new_hub(),
            new_process_spy_hub(),
            new_process_run_hub(),
            process_snapshot_store,
            Arc::new(DashMap::new()),
            Arc::new(DashMap::new()),
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

        let request = Request::new(GetProcessListRequest {
            omit_resource_metrics: false,
        });
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
                cpu_utilization_percent: 40,
                args: vec!["lua".to_string(), "/tmp/script.lua".to_string()],
            },
            ProcessSnapshot {
                pid: 2,
                name: "init".to_string(),
                username: "root".to_string(),
                status: "finished".to_string(),
                memory_bytes: 32_768,
                cpu_utilization_percent: 0,
                args: vec!["init".to_string()],
            },
        ];
        let process_snapshot_store = Arc::new(DashMap::new());
        process_snapshot_store.insert(vm_id, snapshot.clone());

        let svc = test_cluster_game_service_with_store(&pool, process_snapshot_store);

        let token = auth::generate_token(player_id, &name, &auth::get_jwt_secret())
            .expect("generate token");
        let mut request = Request::new(GetProcessListRequest {
            omit_resource_metrics: false,
        });
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
        assert_eq!(out.processes[0].cpu_utilization_percent, 40);
        assert_eq!(out.processes[1].pid, 2);
        assert_eq!(out.processes[1].name, "init");
        assert_eq!(out.processes[1].status, "finished");
        assert_eq!(out.processes[1].memory_bytes, 32_768);
        assert_eq!(out.processes[1].cpu_utilization_percent, 0);

        assert_eq!(out.disk_used_bytes, 0, "new VM has no files");
        assert_eq!(out.disk_total_bytes, (vm.disk_mb as i64) * 1024 * 1024);
        assert!(out.error_message.is_empty());

        vm_service.delete_vm(vm_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_grpc_get_wallet_cards_returns_credit_limit() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let wallet_service = Arc::new(WalletService::new(pool.clone()));
        let wallet_card_service = Arc::new(WalletCardService::new(pool.clone()));
        let name = format!("carduser_{}", Uuid::new_v4());
        let player = player_service.create_player(&name, "secret").await.unwrap();
        wallet_service.create_wallet_for_player(player.id).await.unwrap();

        let card = wallet_card_service
            .create_card(player.id, Some("Test Card"), &name)
            .await
            .unwrap();
        assert_eq!(card.credit_limit, 20_000, "card should have shared $200 limit");

        let svc = test_cluster_game_service(&pool);
        let token = auth::generate_token(player.id, &name, &auth::get_jwt_secret())
            .expect("generate token");
        let mut request = Request::new(GetWalletCardsRequest {});
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", token).parse().unwrap());

        let res = svc.get_wallet_cards(request).await.unwrap();
        let out = res.into_inner();

        assert!(
            out.error_message.is_empty(),
            "error_message should be empty: {}",
            out.error_message
        );
        assert_eq!(out.cards.len(), 1, "should return one card");
        assert_eq!(
            out.cards[0].credit_limit, 20_000,
            "get_wallet_cards must return credit_limit from DB"
        );
    }

    #[tokio::test]
    async fn test_grpc_create_wallet_card_uses_default_limit_and_returns_it() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let wallet_service = Arc::new(WalletService::new(pool.clone()));
        let name = format!("createcard_{}", Uuid::new_v4());
        let player = player_service.create_player(&name, "secret").await.unwrap();
        wallet_service.create_wallet_for_player(player.id).await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let token = auth::generate_token(player.id, &name, &auth::get_jwt_secret())
            .expect("generate token");

        let mut create_req = Request::new(CreateWalletCardRequest {
            label: "My Card".to_string(),
            credit_limit: 0,
        });
        create_req
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", token).parse().unwrap());

        let create_res = svc.create_wallet_card(create_req).await.unwrap();
        let create_out = create_res.into_inner();
        assert!(
            create_out.error_message.is_empty(),
            "create error: {}",
            create_out.error_message
        );
        let created = create_out.card.expect("card should be returned");
        assert_eq!(
            created.credit_limit, 20_000,
            "create with credit_limit=0 must use default $200 (20000 cents)"
        );

        let mut get_req = Request::new(GetWalletCardsRequest {});
        get_req
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", token).parse().unwrap());
        let get_res = svc.get_wallet_cards(get_req).await.unwrap();
        let get_out = get_res.into_inner();
        assert_eq!(get_out.cards.len(), 1);
        assert_eq!(
            get_out.cards[0].credit_limit, 20_000,
            "get_wallet_cards must return the same credit_limit"
        );
    }

    #[tokio::test]
    async fn test_grpc_create_wallet_card_returns_shared_limit() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let wallet_service = Arc::new(WalletService::new(pool.clone()));
        let name = format!("explimit_{}", Uuid::new_v4());
        let player = player_service.create_player(&name, "secret").await.unwrap();
        wallet_service.create_wallet_for_player(player.id).await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let token = auth::generate_token(player.id, &name, &auth::get_jwt_secret())
            .expect("generate token");

        let mut create_req = Request::new(CreateWalletCardRequest {
            label: "Premium".to_string(),
            credit_limit: 50_000, // ignored; account uses default $200
        });
        create_req
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", token).parse().unwrap());

        let create_res = svc.create_wallet_card(create_req).await.unwrap();
        let created = create_res.into_inner().card.unwrap();
        assert_eq!(created.credit_limit, 20_000, "shared account limit $200");

        let mut get_req = Request::new(GetWalletCardsRequest {});
        get_req
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", token).parse().unwrap());
        let get_res = svc.get_wallet_cards(get_req).await.unwrap();
        assert_eq!(get_res.into_inner().cards[0].credit_limit, 20_000);
    }

    #[tokio::test]
    async fn test_grpc_faction_invite_send_list_accept() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let name_a = format!("invgrpc_a_{}", Uuid::new_v4());
        let name_b = format!("invgrpc_b_{}", Uuid::new_v4());
        let pa = players.create_player(&name_a, "pw").await.unwrap();
        let pb = players.create_player(&name_b, "pw").await.unwrap();
        let fac = factions.create("Grpc Fac", pa.id).await.unwrap();
        players
            .set_faction_id(pa.id, Some(fac.id))
            .await
            .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token_a = auth::generate_token(pa.id, &name_a, &auth::get_jwt_secret()).unwrap();
        let token_b = auth::generate_token(pb.id, &name_b, &auth::get_jwt_secret()).unwrap();

        let mut send_req = Request::new(SendFactionInviteRequest {
            target_username: name_b.clone(),
        });
        send_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_a).parse().unwrap(),
        );
        let send_out = svc
            .send_faction_invite(send_req)
            .await
            .unwrap()
            .into_inner();
        assert!(
            send_out.error_message.is_empty(),
            "send invite: {}",
            send_out.error_message
        );
        assert!(!send_out.invite_id.is_empty());

        let mut list_req = Request::new(ListFactionInvitesRequest {});
        list_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_b).parse().unwrap(),
        );
        let list_out = svc
            .list_faction_invites(list_req)
            .await
            .unwrap()
            .into_inner();
        assert!(list_out.error_message.is_empty(), "{}", list_out.error_message);
        assert_eq!(list_out.invites.len(), 1);
        assert_eq!(list_out.invites[0].faction_name, "Grpc Fac");

        let invite_id = list_out.invites[0].invite_id.clone();
        let mut acc_req = Request::new(AcceptFactionInviteRequest { invite_id });
        acc_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_b).parse().unwrap(),
        );
        let acc_out = svc
            .accept_faction_invite(acc_req)
            .await
            .unwrap()
            .into_inner();
        assert!(acc_out.success, "accept: {}", acc_out.error_message);

        let pb_after = players.get_by_id(pb.id).await.unwrap().unwrap();
        assert_eq!(pb_after.faction_id, Some(fac.id));
    }

    #[tokio::test]
    async fn test_grpc_hackerboard_dm_send_threads_messages() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let name_a = format!("dmgrpc_a_{}", Uuid::new_v4());
        let name_b = format!("dmgrpc_b_{}", Uuid::new_v4());
        let pa = players.create_player(&name_a, "pw").await.unwrap();
        let _pb = players.create_player(&name_b, "pw").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let token_a = auth::generate_token(pa.id, &name_a, &auth::get_jwt_secret()).unwrap();

        let mut dm_req = Request::new(SendHackerboardDmRequest {
            target_username: name_b.clone(),
            body: "hello grpc".to_string(),
        });
        dm_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_a).parse().unwrap(),
        );
        let dm_out = svc.send_hackerboard_dm(dm_req).await.unwrap().into_inner();
        assert!(dm_out.error_message.is_empty(), "{}", dm_out.error_message);
        assert!(!dm_out.message_id.is_empty());

        let mut threads_req = Request::new(ListHackerboardDmThreadsRequest { limit: 20 });
        threads_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_a).parse().unwrap(),
        );
        let threads_out = svc
            .list_hackerboard_dm_threads(threads_req)
            .await
            .unwrap()
            .into_inner();
        assert!(threads_out.error_message.is_empty());
        assert_eq!(threads_out.threads.len(), 1);
        assert_eq!(threads_out.threads[0].peer_username, name_b);

        let mut msgs_req = Request::new(ListHackerboardDmMessagesRequest {
            peer_username: name_b.clone(),
            before_message_id: String::new(),
            limit: 50,
        });
        msgs_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_a).parse().unwrap(),
        );
        let msgs_out = svc
            .list_hackerboard_dm_messages(msgs_req)
            .await
            .unwrap()
            .into_inner();
        assert!(msgs_out.error_message.is_empty(), "{}", msgs_out.error_message);
        assert_eq!(msgs_out.messages.len(), 1);
        assert_eq!(msgs_out.messages[0].body, "hello grpc");
    }

    #[tokio::test]
    async fn test_grpc_hackerboard_faction_chat_send_list() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let name_a = format!("fcgrpc_a_{}", Uuid::new_v4());
        let name_b = format!("fcgrpc_b_{}", Uuid::new_v4());
        let pa = players.create_player(&name_a, "pw").await.unwrap();
        let pb = players.create_player(&name_b, "pw").await.unwrap();
        let fac = factions.create("Grpc FC", pa.id).await.unwrap();
        players
            .set_faction_id(pa.id, Some(fac.id))
            .await
            .unwrap();
        players
            .set_faction_id(pb.id, Some(fac.id))
            .await
            .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token_a = auth::generate_token(pa.id, &name_a, &auth::get_jwt_secret()).unwrap();
        let token_b = auth::generate_token(pb.id, &name_b, &auth::get_jwt_secret()).unwrap();

        let mut send_req = Request::new(SendHackerboardFactionMessageRequest {
            body: "room hello".to_string(),
        });
        send_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_a).parse().unwrap(),
        );
        let send_out = svc
            .send_hackerboard_faction_message(send_req)
            .await
            .unwrap()
            .into_inner();
        assert!(send_out.error_message.is_empty(), "{}", send_out.error_message);

        let mut list_req = Request::new(ListHackerboardFactionMessagesRequest {
            before_message_id: String::new(),
            limit: 50,
        });
        list_req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_b).parse().unwrap(),
        );
        let list_out = svc
            .list_hackerboard_faction_messages(list_req)
            .await
            .unwrap()
            .into_inner();
        assert!(list_out.error_message.is_empty(), "{}", list_out.error_message);
        assert_eq!(list_out.messages.len(), 1);
        assert_eq!(list_out.messages[0].body, "room hello");
        assert_eq!(list_out.messages[0].from_username, name_a);
    }

    #[tokio::test]
    async fn test_grpc_login_creates_default_card() {
        let pool = db::test_pool().await;
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let _wallet_service = Arc::new(WalletService::new(pool.clone()));
        let wallet_card_service = Arc::new(WalletCardService::new(pool.clone()));
        let name = format!("defaultcard_{}", Uuid::new_v4());
        let player = player_service.create_player(&name, "secret").await.unwrap();

        let svc = test_cluster_game_service(&pool);
        let login_req = Request::new(LoginRequest {
            username: name.clone(),
            password: "secret".to_string(),
        });
        let login_res = svc.login(login_req).await.unwrap();
        let login_out = login_res.into_inner();
        assert!(login_out.success);

        let cards = wallet_card_service.get_cards(player.id).await.unwrap();
        assert_eq!(cards.len(), 1, "default card must be created on first login");
        assert_eq!(cards[0].label.as_deref(), Some("Default"));
    }

    fn sample_ntpx_16() -> Vec<u8> {
        let mut v = Vec::with_capacity(8 + 16 * 16 * 3);
        v.extend_from_slice(PIXEL_ART_MAGIC);
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.resize(8 + 16 * 16 * 3, 0);
        v[8] = 0xAB;
        v[9] = 0xCD;
        v[10] = 0xEF;
        v
    }

    async fn setup_player_owned_vm(pool: &sqlx::PgPool) -> (Player, Uuid, FsService) {
        let players = PlayerService::new(pool.clone());
        let vm_service = VmService::new(pool.clone());
        let fs_service = FsService::new(pool.clone());
        let name = format!("pxvm_{}", Uuid::new_v4());
        let player = players.create_player(&name, "pw").await.unwrap();
        let vm_id = Uuid::new_v4();
        vm_service
            .create_vm(
                vm_id,
                VmConfig {
                    hostname: "px-test-vm".to_string(),
                    dns_name: None,
                    cpu_cores: 1,
                    memory_mb: 512,
                    disk_mb: 10240,
                    ip: None,
                    subnet: None,
                    gateway: None,
                    mac: None,
                    owner_id: Some(player.id),
                    create_email_account: true,
                },
            )
            .await
            .unwrap();
        fs_service.bootstrap_fs(vm_id).await.unwrap();
        let home = format!("/home/{}", player.username);
        fs_service.mkdir(vm_id, &home, "root").await.unwrap();
        fs_service
            .ensure_standard_home_subdirs(vm_id, &home, &player.username)
            .await
            .unwrap();
        (player, vm_id, fs_service)
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_avatar_from_vm_path_success() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let (player, vm_id, fs) = setup_player_owned_vm(&pool).await;
        let pixel_path = format!("/home/{}/avatar.ntpixels", player.username);
        let bytes = sample_ntpx_16();
        fs.write_file(
            vm_id,
            &pixel_path,
            &bytes,
            Some(PIXEL_ART_MIME),
            &player.username,
        )
        .await
        .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token =
            auth::generate_token(player.id, &player.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardAvatarFromVmPathRequest {
            vm_path: pixel_path,
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_avatar_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(out.success, "{}", out.error_message);

        let p = players.get_by_id(player.id).await.unwrap().unwrap();
        assert_eq!(p.hackerboard_avatar_pixel.as_ref(), Some(&bytes));
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_avatar_from_vm_path_rejects_outside_home() {
        let pool = db::test_pool().await;
        let (player, _vm_id, _fs) = setup_player_owned_vm(&pool).await;
        let svc = test_cluster_game_service(&pool);
        let token =
            auth::generate_token(player.id, &player.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardAvatarFromVmPathRequest {
            vm_path: "/etc/not-under-home.bin".to_string(),
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_avatar_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(!out.success);
        assert_eq!(out.error_message, "Path must be under home");
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_avatar_from_vm_path_rejects_invalid_ntpx() {
        let pool = db::test_pool().await;
        let (player, vm_id, fs) = setup_player_owned_vm(&pool).await;
        let pixel_path = format!("/home/{}/bad.ntpixels", player.username);
        fs.write_file(
            vm_id,
            &pixel_path,
            b"not-ntpx",
            Some("application/octet-stream"),
            &player.username,
        )
        .await
        .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token =
            auth::generate_token(player.id, &player.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardAvatarFromVmPathRequest {
            vm_path: pixel_path,
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_avatar_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(!out.success);
        assert!(
            out.error_message.contains("magic") || out.error_message.contains("short"),
            "unexpected message: {}",
            out.error_message
        );
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_faction_emblem_from_vm_path_success() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let (player, vm_id, fs) = setup_player_owned_vm(&pool).await;
        let fac = factions.create("Px Fac", player.id).await.unwrap();
        players
            .set_faction_id(player.id, Some(fac.id))
            .await
            .unwrap();

        let pixel_path = format!("/home/{}/emblem.ntpixels", player.username);
        let bytes = sample_ntpx_16();
        fs.write_file(
            vm_id,
            &pixel_path,
            &bytes,
            Some(PIXEL_ART_MIME),
            &player.username,
        )
        .await
        .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token =
            auth::generate_token(player.id, &player.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardFactionEmblemFromVmPathRequest {
            vm_path: pixel_path,
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_faction_emblem_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(out.success, "{}", out.error_message);

        let f = factions.get_by_id(fac.id).await.unwrap().unwrap();
        assert_eq!(f.hackerboard_emblem_pixel.as_ref(), Some(&bytes));
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_faction_emblem_from_vm_path_not_creator() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let (creator, _, _) = setup_player_owned_vm(&pool).await;
        let name_m = format!("pxmem_{}", Uuid::new_v4());
        let member = players.create_player(&name_m, "pw").await.unwrap();
        let fac = factions.create("Px Fac M", creator.id).await.unwrap();
        players
            .set_faction_id(creator.id, Some(fac.id))
            .await
            .unwrap();
        players
            .set_faction_id(member.id, Some(fac.id))
            .await
            .unwrap();

        let svc = test_cluster_game_service(&pool);
        let token_m =
            auth::generate_token(member.id, &member.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardFactionEmblemFromVmPathRequest {
            vm_path: "/home/x/emblem.ntpixels".to_string(),
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token_m).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_faction_emblem_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(!out.success);
        assert_eq!(
            out.error_message,
            "Only the faction creator can set the emblem"
        );
    }

    #[tokio::test]
    async fn test_grpc_set_hackerboard_faction_emblem_from_vm_path_not_in_faction() {
        let pool = db::test_pool().await;
        let (player, _, _) = setup_player_owned_vm(&pool).await;
        let svc = test_cluster_game_service(&pool);
        let token =
            auth::generate_token(player.id, &player.username, &auth::get_jwt_secret()).unwrap();
        let mut req = Request::new(SetHackerboardFactionEmblemFromVmPathRequest {
            vm_path: format!("/home/{}/e.ntpixels", player.username),
        });
        req.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let out = svc
            .set_hackerboard_faction_emblem_from_vm_path(req)
            .await
            .unwrap()
            .into_inner();
        assert!(!out.success);
        assert_eq!(out.error_message, "Not in a faction");
    }
}
