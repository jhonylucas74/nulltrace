use game::game_service_server::{GameService, GameServiceServer};
use game::{
    CopyPathRequest, CopyPathResponse, CreateFolderRequest, CreateFolderResponse, CreateFactionRequest, CreateFactionResponse,
    GetDiskUsageRequest, GetDiskUsageResponse, GetHomePathRequest, GetHomePathResponse,
    GetPlayerProfileRequest, GetPlayerProfileResponse, GetProcessListRequest, GetProcessListResponse,
    GetRankingRequest, GetRankingResponse, HelloRequest, HelloResponse, LeaveFactionRequest,
    LeaveFactionResponse, ListFsRequest, ListFsResponse, LoginRequest, LoginResponse,
    MovePathRequest, MovePathResponse, PingRequest, PingResponse, ProcessSpyClientMessage,
    ProcessSpyServerMessage,     RefreshTokenRequest, RefreshTokenResponse, RenamePathRequest,
    RenamePathResponse, RunProcessRequest, RunProcessResponse, RestoreDiskRequest, RestoreDiskResponse, SetPreferredThemeRequest,
    SetPreferredThemeResponse, SetShortcutsRequest, SetShortcutsResponse, TerminalClientMessage,
    TerminalServerMessage, WriteFileRequest, WriteFileResponse,
    ReadFileRequest, ReadFileResponse,
    EmptyTrashRequest, EmptyTrashResponse,
    GetInstalledStoreAppsRequest, GetInstalledStoreAppsResponse,
    InstallStoreAppRequest, InstallStoreAppResponse,
    UninstallStoreAppRequest, UninstallStoreAppResponse,
    GetEmailsRequest, GetEmailsResponse,
    SendEmailRequest, SendEmailResponse,
    MarkEmailReadRequest, MarkEmailReadResponse,
    MoveEmailRequest, MoveEmailResponse,
    DeleteEmailRequest, DeleteEmailResponse,
    MailboxStreamRequest, MailboxStreamMessage,
    GetWalletBalancesRequest, GetWalletBalancesResponse,
    GetWalletTransactionsRequest, GetWalletTransactionsResponse,
    GetWalletKeysRequest, GetWalletKeysResponse,
    TransferFundsRequest, TransferFundsResponse,
    ResolveTransferKeyRequest, ResolveTransferKeyResponse,
    ConvertFundsRequest, ConvertFundsResponse,
    GetWalletCardsRequest, GetWalletCardsResponse,
    CreateWalletCardRequest, CreateWalletCardResponse,
    DeleteWalletCardRequest, DeleteWalletCardResponse,
    GetCardTransactionsRequest, GetCardTransactionsResponse,
    GetCardStatementRequest, GetCardStatementResponse,
    PayCardBillRequest, PayCardBillResponse,
    PayAccountBillRequest, PayAccountBillResponse,
};
use tonic::{Request, Response, Status, transport::Server};
use tokio_stream::wrappers::ReceiverStream;

pub mod game {
    tonic::include_proto!("game");
}

use game::terminal_server_message::Msg as TerminalServerMsg;
use game::TerminalError;

#[derive(Default)]
pub struct MyGameService {}

type TerminalStreamStream = ReceiverStream<Result<TerminalServerMessage, Status>>;
type ProcessSpyStreamStream = ReceiverStream<Result<ProcessSpyServerMessage, Status>>;
type RunProcessStream = ReceiverStream<Result<RunProcessResponse, Status>>;
type MailboxStreamStream = ReceiverStream<Result<MailboxStreamMessage, Status>>;

#[tonic::async_trait]
impl GameService for MyGameService {
    type TerminalStreamStream = TerminalStreamStream;
    type ProcessSpyStreamStream = ProcessSpyStreamStream;
    type RunProcessStream = RunProcessStream;
    type MailboxStreamStream = MailboxStreamStream;

    async fn run_process(
        &self,
        _request: Request<RunProcessRequest>,
    ) -> Result<Response<Self::RunProcessStream>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = _tx.try_send(Ok(RunProcessResponse {
            msg: Some(game::run_process_response::Msg::Finished(game::RunProcessFinished {
                exit_code: 1,
            })),
        }));
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
        _request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        // Standalone server has no DB; always reject login.
        Ok(Response::new(LoginResponse {
            success: false,
            player_id: String::new(),
            token: String::new(),
            error_message: "Use the unified cluster binary for login".to_string(),
            preferred_theme: String::new(),
            shortcuts_overrides: String::new(),
        }))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        Ok(Response::new(RefreshTokenResponse {
            success: false,
            token: String::new(),
            error_message: "Use the unified cluster binary for token refresh".to_string(),
        }))
    }

    async fn terminal_stream(
        &self,
        _request: Request<tonic::Streaming<TerminalClientMessage>>,
    ) -> Result<Response<TerminalStreamStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(TerminalServerMessage {
                msg: Some(TerminalServerMsg::TerminalError(TerminalError {
                    message: "Use the unified cluster binary for terminal".to_string(),
                })),
            }))
            .await;
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn process_spy_stream(
        &self,
        _request: Request<tonic::Streaming<ProcessSpyClientMessage>>,
    ) -> Result<Response<ProcessSpyStreamStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(Ok(ProcessSpyServerMessage {
                msg: Some(game::process_spy_server_message::Msg::ProcessSpyError(
                    game::ProcessSpyError {
                        message: "Use the unified cluster binary for process spy".to_string(),
                    },
                )),
            }))
            .await;
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_disk_usage(
        &self,
        _request: Request<GetDiskUsageRequest>,
    ) -> Result<Response<GetDiskUsageResponse>, Status> {
        Ok(Response::new(GetDiskUsageResponse {
            used_bytes: 0,
            total_bytes: 0,
            error_message: "Use the unified cluster binary for disk operations".to_string(),
        }))
    }

    async fn restore_disk(
        &self,
        _request: Request<RestoreDiskRequest>,
    ) -> Result<Response<RestoreDiskResponse>, Status> {
        Ok(Response::new(RestoreDiskResponse {
            success: false,
            error_message: "Use the unified cluster binary for disk operations".to_string(),
        }))
    }

    async fn get_home_path(
        &self,
        _request: Request<GetHomePathRequest>,
    ) -> Result<Response<GetHomePathResponse>, Status> {
        Ok(Response::new(GetHomePathResponse {
            home_path: String::new(),
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn list_fs(
        &self,
        _request: Request<ListFsRequest>,
    ) -> Result<Response<ListFsResponse>, Status> {
        Ok(Response::new(ListFsResponse {
            entries: vec![],
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn copy_path(
        &self,
        _request: Request<CopyPathRequest>,
    ) -> Result<Response<CopyPathResponse>, Status> {
        Ok(Response::new(CopyPathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn move_path(
        &self,
        _request: Request<MovePathRequest>,
    ) -> Result<Response<MovePathResponse>, Status> {
        Ok(Response::new(MovePathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn rename_path(
        &self,
        _request: Request<RenamePathRequest>,
    ) -> Result<Response<RenamePathResponse>, Status> {
        Ok(Response::new(RenamePathResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn create_folder(
        &self,
        _request: Request<CreateFolderRequest>,
    ) -> Result<Response<CreateFolderResponse>, Status> {
        Ok(Response::new(CreateFolderResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn write_file(
        &self,
        _request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        Ok(Response::new(WriteFileResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn read_file(
        &self,
        _request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        Ok(Response::new(ReadFileResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
            content: vec![],
        }))
    }

    async fn empty_trash(
        &self,
        _request: Request<EmptyTrashRequest>,
    ) -> Result<Response<EmptyTrashResponse>, Status> {
        Ok(Response::new(EmptyTrashResponse {
            success: false,
            error_message: "Use the unified cluster binary for file operations".to_string(),
        }))
    }

    async fn get_installed_store_apps(
        &self,
        _request: Request<GetInstalledStoreAppsRequest>,
    ) -> Result<Response<GetInstalledStoreAppsResponse>, Status> {
        Ok(Response::new(GetInstalledStoreAppsResponse {
            app_types: vec![],
            error_message: "Use the unified cluster binary for store apps".to_string(),
        }))
    }

    async fn install_store_app(
        &self,
        _request: Request<InstallStoreAppRequest>,
    ) -> Result<Response<InstallStoreAppResponse>, Status> {
        Ok(Response::new(InstallStoreAppResponse {
            success: false,
            error_message: "Use the unified cluster binary for store apps".to_string(),
        }))
    }

    async fn uninstall_store_app(
        &self,
        _request: Request<UninstallStoreAppRequest>,
    ) -> Result<Response<UninstallStoreAppResponse>, Status> {
        Ok(Response::new(UninstallStoreAppResponse {
            success: false,
            error_message: "Use the unified cluster binary for store apps".to_string(),
        }))
    }

    async fn get_process_list(
        &self,
        _request: Request<GetProcessListRequest>,
    ) -> Result<Response<GetProcessListResponse>, Status> {
        Ok(Response::new(GetProcessListResponse {
            processes: vec![],
            disk_used_bytes: 0,
            disk_total_bytes: 0,
            error_message: "Use the unified cluster binary for process list".to_string(),
            vm_lua_memory_bytes: 0,
        }))
    }

    async fn get_sysinfo(
        &self,
        _request: Request<game::GetSysinfoRequest>,
    ) -> Result<Response<game::GetSysinfoResponse>, Status> {
        Ok(Response::new(game::GetSysinfoResponse {
            cpu_cores: 0,
            memory_mb: 0,
            disk_mb: 0,
            error_message: "Use the unified cluster binary for sysinfo".to_string(),
        }))
    }

    async fn get_ranking(
        &self,
        _request: Request<GetRankingRequest>,
    ) -> Result<Response<GetRankingResponse>, Status> {
        Ok(Response::new(GetRankingResponse {
            entries: vec![],
            error_message: "Use the unified cluster binary for ranking".to_string(),
        }))
    }

    async fn get_player_profile(
        &self,
        _request: Request<GetPlayerProfileRequest>,
    ) -> Result<Response<GetPlayerProfileResponse>, Status> {
        Ok(Response::new(GetPlayerProfileResponse {
            rank: 0,
            points: 0,
            faction_id: String::new(),
            faction_name: String::new(),
            error_message: "Use the unified cluster binary for profile".to_string(),
            preferred_theme: String::new(),
            shortcuts_overrides: String::new(),
        }))
    }

    async fn set_shortcuts(
        &self,
        _request: Request<SetShortcutsRequest>,
    ) -> Result<Response<SetShortcutsResponse>, Status> {
        Ok(Response::new(SetShortcutsResponse {
            success: false,
            error_message: "Use the unified cluster binary for shortcuts".to_string(),
        }))
    }

    async fn set_preferred_theme(
        &self,
        _request: Request<SetPreferredThemeRequest>,
    ) -> Result<Response<SetPreferredThemeResponse>, Status> {
        Ok(Response::new(SetPreferredThemeResponse {
            success: false,
            error_message: "Use the unified cluster binary for theme preference".to_string(),
        }))
    }

    async fn create_faction(
        &self,
        _request: Request<CreateFactionRequest>,
    ) -> Result<Response<CreateFactionResponse>, Status> {
        Ok(Response::new(CreateFactionResponse {
            faction_id: String::new(),
            name: String::new(),
            error_message: "Use the unified cluster binary for factions".to_string(),
        }))
    }

    async fn leave_faction(
        &self,
        _request: Request<LeaveFactionRequest>,
    ) -> Result<Response<LeaveFactionResponse>, Status> {
        Ok(Response::new(LeaveFactionResponse {
            success: false,
            error_message: "Use the unified cluster binary for factions".to_string(),
        }))
    }

    async fn get_emails(
        &self,
        _request: Request<GetEmailsRequest>,
    ) -> Result<Response<GetEmailsResponse>, Status> {
        Ok(Response::new(GetEmailsResponse {
            emails: vec![],
            has_more: false,
        }))
    }

    async fn send_email(
        &self,
        _request: Request<SendEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        Ok(Response::new(SendEmailResponse {
            success: false,
            error_message: "Use the unified cluster binary".to_string(),
        }))
    }

    async fn mark_email_read(
        &self,
        _request: Request<MarkEmailReadRequest>,
    ) -> Result<Response<MarkEmailReadResponse>, Status> {
        Ok(Response::new(MarkEmailReadResponse { success: false }))
    }

    async fn move_email(
        &self,
        _request: Request<MoveEmailRequest>,
    ) -> Result<Response<MoveEmailResponse>, Status> {
        Ok(Response::new(MoveEmailResponse { success: false }))
    }

    async fn delete_email(
        &self,
        _request: Request<DeleteEmailRequest>,
    ) -> Result<Response<DeleteEmailResponse>, Status> {
        Ok(Response::new(DeleteEmailResponse { success: false }))
    }

    async fn mailbox_stream(
        &self,
        _request: Request<MailboxStreamRequest>,
    ) -> Result<Response<Self::MailboxStreamStream>, Status> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_wallet_balances(
        &self,
        _request: Request<GetWalletBalancesRequest>,
    ) -> Result<Response<GetWalletBalancesResponse>, Status> {
        Ok(Response::new(GetWalletBalancesResponse {
            balances: vec![],
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn get_wallet_transactions(
        &self,
        _request: Request<GetWalletTransactionsRequest>,
    ) -> Result<Response<GetWalletTransactionsResponse>, Status> {
        Ok(Response::new(GetWalletTransactionsResponse {
            transactions: vec![],
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn get_wallet_keys(
        &self,
        _request: Request<GetWalletKeysRequest>,
    ) -> Result<Response<GetWalletKeysResponse>, Status> {
        Ok(Response::new(GetWalletKeysResponse {
            keys: vec![],
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn transfer_funds(
        &self,
        _request: Request<TransferFundsRequest>,
    ) -> Result<Response<TransferFundsResponse>, Status> {
        Ok(Response::new(TransferFundsResponse {
            success: false,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn resolve_transfer_key(
        &self,
        _request: Request<ResolveTransferKeyRequest>,
    ) -> Result<Response<ResolveTransferKeyResponse>, Status> {
        Ok(Response::new(ResolveTransferKeyResponse {
            is_valid: false,
            is_usd: false,
            account_holder_name: String::new(),
            target_currency: String::new(),
        }))
    }

    async fn convert_funds(
        &self,
        _request: Request<ConvertFundsRequest>,
    ) -> Result<Response<ConvertFundsResponse>, Status> {
        Ok(Response::new(ConvertFundsResponse {
            success: false,
            converted_amount: 0,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn get_wallet_cards(
        &self,
        _request: Request<GetWalletCardsRequest>,
    ) -> Result<Response<GetWalletCardsResponse>, Status> {
        Ok(Response::new(GetWalletCardsResponse {
            cards: vec![],
            account_credit_limit: 0,
            account_total_debt: 0,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn create_wallet_card(
        &self,
        _request: Request<CreateWalletCardRequest>,
    ) -> Result<Response<CreateWalletCardResponse>, Status> {
        Ok(Response::new(CreateWalletCardResponse {
            card: None,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn delete_wallet_card(
        &self,
        _request: Request<DeleteWalletCardRequest>,
    ) -> Result<Response<DeleteWalletCardResponse>, Status> {
        Ok(Response::new(DeleteWalletCardResponse {
            success: false,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn get_card_transactions(
        &self,
        _request: Request<GetCardTransactionsRequest>,
    ) -> Result<Response<GetCardTransactionsResponse>, Status> {
        Ok(Response::new(GetCardTransactionsResponse {
            transactions: vec![],
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn get_card_statement(
        &self,
        _request: Request<GetCardStatementRequest>,
    ) -> Result<Response<GetCardStatementResponse>, Status> {
        Ok(Response::new(GetCardStatementResponse {
            statement: None,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn pay_card_bill(
        &self,
        _request: Request<PayCardBillRequest>,
    ) -> Result<Response<PayCardBillResponse>, Status> {
        Ok(Response::new(PayCardBillResponse {
            success: false,
            amount_paid: 0,
            error_message: "Use the unified cluster binary for wallet".to_string(),
        }))
    }

    async fn pay_account_bill(
        &self,
        _request: Request<PayAccountBillRequest>,
    ) -> Result<Response<PayAccountBillResponse>, Status> {
        Ok(Response::new(PayAccountBillResponse {
            success: false,
            amount_paid: 0,
            error_message: "Use the unified cluster binary for wallet".to_string(),
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
