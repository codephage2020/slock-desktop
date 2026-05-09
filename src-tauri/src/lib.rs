mod agent_card_inject;
mod agent_env_import;
mod config;
mod theme;
mod workspace;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use config::{
    load_settings, save_settings, AppSettings, CustomThemeSettings, SavedAccountSettings,
    ServiceMachineBinding, ServiceSettings,
};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::{HashSet, VecDeque},
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    thread::{self, sleep},
    time::{Duration, Instant},
};
use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    webview::PageLoadEvent,
    window::Color,
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, RunEvent, State, Theme, Url,
    WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Updater, UpdaterExt};
use theme::{
    color_catalog, resolve_style, resolve_theme_with_style, sanitize_hex,
    style_catalog, CustomStyleSet, CustomThemeItem, CustomThemeSet, ThemeStyleConfig,
};

const MAIN_LABEL: &str = "main";
const AUTH_LABEL: &str = "auth";
const WORKSPACE_URL: &str = "https://app.slock.ai";
const LOGIN_URL: &str = "https://app.slock.ai/login";
const DESKTOP_AUTH_CALLBACK_EVENT: &str = "desktop-auth-complete";
const DESKTOP_AUTH_CANCELLED_EVENT: &str = "desktop-auth-cancelled";
const MESSAGE_REMINDER_EVENT: &str = "slock-message-reminder";
const DEFAULT_SERVER_URL: &str = "https://api.slock.ai";
const DAEMON_PACKAGE: &str = "@slock-ai/daemon@latest";
const DAEMON_MACHINE_NAME: &str = "Slock Desktop";
const LAUNCHER_WINDOW_WIDTH: f64 = 800.0;
const LAUNCHER_WINDOW_HEIGHT: f64 = 460.0;
const LAUNCHER_WINDOW_MIN_WIDTH: f64 = 720.0;
const LAUNCHER_WINDOW_MIN_HEIGHT: f64 = 420.0;
const AUTH_WINDOW_WIDTH: f64 = 520.0;
const AUTH_WINDOW_HEIGHT: f64 = 720.0;
const AUTH_WINDOW_MIN_WIDTH: f64 = 420.0;
const AUTH_WINDOW_MIN_HEIGHT: f64 = 560.0;
const WORKSPACE_WINDOW_WIDTH: f64 = 1480.0;
const WORKSPACE_WINDOW_HEIGHT: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_WIDTH: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_HEIGHT: f64 = 760.0;
const WORKSPACE_WINDOW_MARGIN: f64 = 24.0;
const DAEMON_SERVER_SLUG_ARG: &str = "--slock-desktop-server-slug";
const DAEMON_MACHINE_ID_ARG: &str = "--slock-desktop-machine-id";
const DAEMON_DESKTOP_MANAGED_ARG: &str = "--slock-desktop-managed";
const RUNTIME_WRAPPER_DIR: &str = "runtime-wrappers";
const CLAUDE_WRAPPER_NAME: &str = "claude";
const DESKTOP_UPDATER_ENDPOINT: &str =
    "https://github.com/codephage2020/slock-desktop/releases/latest/download/latest.json";
const DESKTOP_UPDATE_CHECK_TIMEOUT: u64 = 8;
const SERVICE_MACHINE_FETCH_CONCURRENCY_LIMIT: usize = 8;
const SERVICE_LOG_MAX_BYTES: u64 = 2 * 1024 * 1024;
const SERVICE_LOG_DEFAULT_WINDOW_MS: i64 = 30 * 60 * 1000;
const MESSAGE_REMINDER_RECENT_LIMIT: usize = 200;
const MESSAGE_REMINDER_RETRY_AFTER_MS: u64 = 30_000;
const MESSAGE_REMINDER_MUTED_REFRESH_MS: u64 = 30_000;
const MESSAGE_REMINDER_MUTED_FAILURE_RETRY_MS: u64 = 5 * 60_000;
#[cfg(debug_assertions)]
const WORKSPACE_LAUNCH_LOG_PATH: &str = "/tmp/slock-desktop-launch.log";

pub struct DesktopState {
    settings: Mutex<AppSettings>,
    service: Mutex<ServiceRuntime>,
    auth: Mutex<AuthRuntime>,
    app_close: Mutex<AppCloseRuntime>,
    launch_metrics: Mutex<WorkspaceLaunchMetrics>,
    update_cache: Mutex<Option<DesktopUpdateCheck>>,
    message_reminders: Mutex<MessageReminderRuntime>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    app_name: String,
    workspace_url: String,
    color_scheme: String,
    style_scheme: String,
    appearance_mode: String,
    custom_themes: Vec<CustomThemeSettings>,
    custom_styles: Vec<ThemeStyleConfig>,
    language: String,
    resolved_language: String,
    workspace_open: bool,
    themes: Vec<theme::ThemeMeta>,
    theme_styles: Vec<theme::ThemeStyleMeta>,
    service: ServiceSnapshot,
    updates: UpdateSnapshot,
}

struct ServiceRuntime {
    child: Option<Child>,
    last_error: Option<String>,
    active_server_slug: Option<String>,
    active_machine_id: Option<String>,
    active_pid: Option<u32>,
    cached_servers: Vec<ServiceServerSnapshot>,
    cached_sync_error: Option<String>,
}

struct MessageReminderRuntime {
    desired_key: Option<String>,
    status: MessageReminderStatus,
    muted_channel_ids: HashSet<String>,
    muted_channels_checked_at: Option<Instant>,
    muted_channels_retry_after: Option<Instant>,
    recent_message_ids: VecDeque<String>,
    context: Option<MessageReminderContext>,
    connection: Option<MessageReminderConnection>,
    last_attempt: Option<Instant>,
}

impl Default for MessageReminderRuntime {
    fn default() -> Self {
        Self {
            desired_key: None,
            status: MessageReminderStatus::Idle,
            muted_channel_ids: HashSet::new(),
            muted_channels_checked_at: None,
            muted_channels_retry_after: None,
            recent_message_ids: VecDeque::new(),
            context: None,
            connection: None,
            last_attempt: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageReminderStatus {
    Idle,
    Connecting,
    Connected,
    Failed,
}

#[derive(Default)]
struct AuthRuntime {
    clear_login_session_storage: bool,
}

#[derive(Debug, Clone)]
struct ServiceCommand {
    executable: PathBuf,
    path_env: String,
}

#[derive(Default)]
struct WorkspaceLaunchMetrics {
    next_id: u64,
    active: Option<WorkspaceLaunchTrace>,
}

struct WorkspaceLaunchTrace {
    id: u64,
    target_url: String,
    command_started: Instant,
    navigate_called: Option<Instant>,
    page_started: Option<Instant>,
}

#[derive(Debug, Clone)]
struct ServiceDaemonProcess {
    pid: u32,
    server_slug: String,
    machine_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedServiceMachine {
    binding: ServiceMachineBinding,
    api_key_prefix: Option<String>,
    machine_status: String,
}

#[derive(Debug, Clone)]
struct ServiceStartTarget {
    binding: ServiceMachineBinding,
    api_key: String,
    api_key_prefix: Option<String>,
    machine_status: String,
}

#[derive(Default)]
struct AppCloseRuntime {
    prompt_visible: bool,
    confirmed_exit: bool,
    service_stop_completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseAppServiceBehavior {
    Ask,
    Keep,
    Stop,
}

#[derive(Debug, Clone)]
struct WorkspaceSessionSeed {
    access_token: String,
    refresh_token: String,
    target_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceSnapshot {
    server_url: String,
    selected_server_slug: String,
    active_server_slug: String,
    auto_start_with_workspace: bool,
    close_app_behavior: String,
    authenticated: bool,
    account: Option<ServiceAccountSnapshot>,
    accounts: Vec<ServiceAccountSnapshot>,
    configured: bool,
    running: bool,
    pid: Option<u32>,
    last_error: Option<String>,
    sync_error: Option<String>,
    servers: Vec<ServiceServerSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountSnapshot {
    id: String,
    display_name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
    initials: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceServerSnapshot {
    id: String,
    name: String,
    slug: String,
    selected: bool,
    machine_id: Option<String>,
    machine_name: Option<String>,
    machine_status: String,
    api_key_ready: bool,
    /// How the local binding was established: "desktop_created", "pid_scan", "user_bound", or empty
    binding_source: String,
    #[serde(skip_serializing)]
    api_key_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSnapshot {
    current_version: String,
    latest: Option<DesktopUpdateCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopUpdateCheck {
    current_version: String,
    available: bool,
    version: Option<String>,
    body: Option<String>,
    date: Option<String>,
    download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceLogSnapshot {
    server_slug: String,
    path: String,
    content: String,
    truncated: bool,
    total_bytes: u64,
    from_epoch_ms: i64,
    to_epoch_ms: i64,
    timestamp_filtered: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseAppPromptCopy {
    title: &'static str,
    description: String,
    server_label: String,
    keep_server: &'static str,
    close_server: &'static str,
    cancel: &'static str,
    remember: &'static str,
    processing_keep_server: &'static str,
    processing_close_server: &'static str,
    error: &'static str,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiServer {
    id: String,
    name: String,
    slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMachine {
    id: String,
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    api_key_prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMachinesEnvelope {
    #[serde(default)]
    machines: Vec<ApiMachine>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMachineRegistration {
    machine: ApiMachine,
    api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMachineKeyRotation {
    api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRefreshSession {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Default, Clone)]
struct SessionAccountProfile {
    display_name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

// Dashboard types

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardData {
    channels: Vec<DashboardChannel>,
    unread: Vec<DashboardChannelUnread>,
    tasks: Vec<DashboardTask>,
    agents: Vec<DashboardAgent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardChannel {
    id: String,
    name: String,
    #[serde(alias = "type", rename(serialize = "type"))]
    channel_type: String,
    #[serde(default, alias = "is_archived")]
    is_archived: bool,
    #[serde(default)]
    joined: bool,
    #[serde(alias = "last_message_at")]
    last_message_at: Option<String>,
    #[serde(default, alias = "member_count")]
    member_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardChannelUnread {
    channel_id: String,
    #[serde(default)]
    unread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardTask {
    id: String,
    title: String,
    status: String,
    #[serde(alias = "assignee_id")]
    assignee: Option<String>,
    #[serde(alias = "channel_id")]
    channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardAgent {
    id: String,
    name: String,
    status: String,
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(alias = "updated_at")]
    updated_at: Option<String>,
}

#[derive(Debug, Clone)]
struct MessageReminderConnection {
    key: String,
    socket_url: String,
    server_url: String,
    server_id: String,
    server_slug: String,
    server_name: String,
    access_token: String,
    identity: MessageReminderIdentity,
}

#[derive(Debug, Clone)]
struct MessageReminderContext {
    key: String,
    server_url: String,
    server_id: String,
    server_slug: String,
    server_name: String,
    identity: MessageReminderIdentity,
}

#[derive(Debug, Clone, Default)]
struct MessageReminderIdentity {
    values: HashSet<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageReminderPayload {
    id: String,
    channel_id: String,
    server_id: String,
    server_slug: String,
    server_name: String,
    sender_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sender_type: Option<String>,
    content_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentActivityEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    activity: String,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default, alias = "launch_id")]
    launch_id: Option<String>,
    #[serde(default, alias = "created_at")]
    created_at: Option<String>,
    // Additional fields the API may return (ignored by frontend but needed for deserialization)
    #[serde(default, alias = "agent_id")]
    agent_id: Option<String>,
    // Catch-all: preserve any extra API fields so JS fallback chain can access them
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Envelope wrapper in case API wraps activity entries
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentActivityEnvelope {
    #[serde(default)]
    entries: Vec<AgentActivityEntry>,
    #[serde(default)]
    data: Vec<AgentActivityEntry>,
    #[serde(default, alias = "activity_log")]
    activity_log: Vec<AgentActivityEntry>,
}

// ── Inbox types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxThread {
    id: String,
    name: Option<String>,
    #[serde(alias = "parent_channel_id")]
    parent_channel_id: Option<String>,
    #[serde(alias = "parent_channel_name")]
    parent_channel_name: Option<String>,
    #[serde(default, alias = "parent_message_id")]
    parent_message_id: Option<String>,
    #[serde(default, alias = "is_done")]
    is_done: bool,
    #[serde(alias = "last_message_at")]
    last_message_at: Option<String>,
    #[serde(default, alias = "unread_count")]
    unread_count: u32,
}

/// Envelope returned by GET /api/channels/threads/followed
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FollowedThreadsEnvelope {
    #[serde(default)]
    threads: Vec<InboxThread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxDmChannel {
    id: String,
    name: String,
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(alias = "last_message_at")]
    last_message_at: Option<String>,
    #[serde(default, alias = "unread_count")]
    unread_count: u32,
    #[serde(default)]
    members: Vec<InboxDmMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxDmMember {
    id: String,
    name: String,
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(default, alias = "avatar_url")]
    avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxMessage {
    id: String,
    #[serde(default)]
    seq: Option<u64>,
    #[serde(alias = "channel_id")]
    channel_id: String,
    content: String,
    #[serde(alias = "sender_id")]
    sender_id: Option<String>,
    #[serde(alias = "sender_name")]
    sender_name: Option<String>,
    #[serde(alias = "sender_type")]
    sender_type: Option<String>,
    #[serde(alias = "sender_display_name")]
    sender_display_name: Option<String>,
    #[serde(alias = "sender_avatar_url")]
    sender_avatar_url: Option<String>,
    #[serde(alias = "created_at")]
    created_at: String,
    #[serde(default, alias = "updated_at")]
    updated_at: Option<String>,
}

/// Response returned to the frontend from fetch_thread_messages
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxMessagesResponse {
    messages: Vec<InboxMessage>,
    has_more: bool,
}

/// Envelope returned by GET /api/messages/channel/{channelId}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesEnvelope {
    #[serde(default)]
    messages: Vec<InboxMessage>,
    #[serde(default, alias = "has_more")]
    has_more: bool,
}

// --- Unified inbox types ---

/// Item returned by GET /channels/inbox
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxFeedItem {
    #[serde(default)]
    kind: String,
    #[serde(default, alias = "channel_id")]
    channel_id: Option<String>,
    #[serde(default, alias = "thread_channel_id")]
    thread_channel_id: Option<String>,
    #[serde(default, alias = "parent_channel_id")]
    parent_channel_id: Option<String>,
    #[serde(default, alias = "channel_name")]
    channel_name: Option<String>,
    #[serde(default, alias = "parent_channel_name")]
    parent_channel_name: Option<String>,
    #[serde(default, alias = "parent_channel_type")]
    parent_channel_type: Option<String>,
    #[serde(default, alias = "unread_count")]
    unread_count: u32,
    #[serde(default, alias = "first_unread_message_id")]
    first_unread_message_id: Option<String>,
    #[serde(default, alias = "last_message_at")]
    last_message_at: Option<String>,
    #[serde(default, alias = "last_activity_at")]
    last_activity_at: Option<String>,
    #[serde(default, alias = "last_message_sender_id")]
    last_message_sender_id: Option<String>,
    #[serde(default, alias = "last_message_sender_name")]
    last_message_sender_name: Option<String>,
    #[serde(default, alias = "last_message_sender_type")]
    last_message_sender_type: Option<String>,
    #[serde(default, alias = "last_message_preview")]
    last_message_preview: Option<String>,
    #[serde(default, alias = "last_message_id")]
    last_message_id: Option<String>,
    #[serde(default, alias = "latest_activity_sender_id")]
    latest_activity_sender_id: Option<String>,
    #[serde(default, alias = "latest_activity_preview")]
    latest_activity_preview: Option<String>,
    #[serde(default, alias = "latest_activity_message_id")]
    latest_activity_message_id: Option<String>,
    #[serde(default, alias = "parent_message_id")]
    parent_message_id: Option<String>,
    #[serde(default, alias = "parent_message_preview")]
    parent_message_preview: Option<String>,
    #[serde(default, alias = "reply_count")]
    reply_count: Option<u32>,
    #[serde(default, alias = "task_number")]
    task_number: Option<u32>,
    #[serde(default, alias = "task_status")]
    task_status: Option<String>,
    #[serde(default, alias = "task_claimed_by_name")]
    task_claimed_by_name: Option<String>,
}

/// Response from GET /channels/inbox
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxFeedResponse {
    #[serde(default)]
    items: Vec<InboxFeedItem>,
    #[serde(default, alias = "has_more")]
    has_more: bool,
    #[serde(default, alias = "total_count")]
    total_count: u32,
    #[serde(default, alias = "total_unread_count")]
    total_unread_count: u32,
}

// --- Unified inbox types ---

/// Server member returned by GET /api/servers/{serverId}/members
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerMember {
    id: String,
    name: String,
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(default, alias = "avatar_url")]
    avatar_url: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

/// Per-server unread count returned by GET /api/servers/unread-summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerUnreadEntry {
    #[serde(alias = "server_id")]
    server_id: String,
    #[serde(default, alias = "unread_count")]
    unread_count: u32,
}

/// Envelope returned by POST /api/messages
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendMessageEnvelope {
    #[serde(default, alias = "message_id")]
    _message_id: Option<String>,
    message: InboxMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InboxUnreadEntry {
    #[serde(alias = "channel_id")]
    channel_id: String,
    #[serde(default, alias = "unread_count")]
    unread_count: u32,
}

#[tauri::command]
fn bootstrap(
    app: AppHandle,
    state: State<'_, DesktopState>,
    refresh: Option<bool>,
) -> Result<BootstrapPayload, String> {
    build_bootstrap(&app, &state, refresh.unwrap_or(true))
}

#[tauri::command]
fn set_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_id: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let custom = custom_theme_set(&settings.custom_themes);
        let styles = custom_style_set(&settings.custom_styles);
        if theme_id == "original" {
            settings.color_scheme = theme::default_color_scheme().to_string();
            settings.style_scheme = "original".to_string();
        } else {
            let theme = resolve_theme_with_style(
                &theme_id,
                &settings.style_scheme,
                &settings.appearance_mode,
                &custom,
                &styles,
            );
            settings.color_scheme = theme.id;
        }
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_theme_style(
    app: AppHandle,
    state: State<'_, DesktopState>,
    style_id: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let styles = custom_style_set(&settings.custom_styles);
        let style = resolve_style(&style_id, &styles);
        settings.style_scheme = style.id;
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn import_theme_style(
    app: AppHandle,
    state: State<'_, DesktopState>,
    config: ThemeStyleConfig,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let style = theme::sanitize_style_config(config);
        settings.custom_styles.retain(|item| item.id != style.id);
        settings.custom_styles.push(style.clone());
        settings.style_scheme = style.id;
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_theme_mode(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_mode: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.appearance_mode = theme::normalize_mode(&theme_mode).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn create_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    name: String,
    accent: String,
) -> Result<BootstrapPayload, String> {
    let (style_scheme, appearance_mode, language, custom_themes, custom_styles, new_id) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let id = format!("custom-{}", uuid::Uuid::new_v4());
        let theme = CustomThemeSettings {
            id: id.clone(),
            name: sanitize_theme_name(&name),
            accent: sanitize_hex(&accent).unwrap_or_else(|| "#10a37f".to_string()),
        };
        settings.custom_themes.push(theme);
        settings.color_scheme = id.clone();
        save_settings(&app, &settings)?;
        (
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            id,
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(&new_id, &style_scheme, &appearance_mode, &custom, &styles);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn rename_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    id: String,
    name: String,
) -> Result<BootstrapPayload, String> {
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let trimmed = sanitize_theme_name(&name);
        if let Some(item) = settings.custom_themes.iter_mut().find(|item| item.id == id) {
            item.name = trimmed;
        }
        save_settings(&app, &settings)?;
    }

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn update_custom_theme_accent(
    app: AppHandle,
    state: State<'_, DesktopState>,
    id: String,
    accent: String,
) -> Result<BootstrapPayload, String> {
    let (style_scheme, appearance_mode, language, custom_themes, custom_styles, color_scheme) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let cleaned = sanitize_hex(&accent).unwrap_or_else(|| "#10a37f".to_string());
        if let Some(item) = settings.custom_themes.iter_mut().find(|item| item.id == id) {
            item.accent = cleaned;
        }
        save_settings(&app, &settings)?;
        (
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.color_scheme.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn delete_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    id: String,
) -> Result<BootstrapPayload, String> {
    let (style_scheme, appearance_mode, language, custom_themes, custom_styles, color_scheme) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.custom_themes.retain(|item| item.id != id);
        if settings.color_scheme == id {
            settings.color_scheme = theme::default_color_scheme().to_string();
        }
        save_settings(&app, &settings)?;
        (
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.color_scheme.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_language(
    app: AppHandle,
    state: State<'_, DesktopState>,
    language: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.language = sanitize_language(&language).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let styles = custom_style_set(&custom_styles);
    let theme = resolve_theme_with_style(
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom,
        &styles,
    );
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom, &styles)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn save_session_tokens(
    app: AppHandle,
    state: State<'_, DesktopState>,
    access_token: String,
    refresh_token: String,
    display_name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
) -> Result<(), String> {
    let access_token = access_token.trim().to_string();
    let refresh_token = refresh_token.trim().to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Ok(());
    }

    save_session_tokens_to_settings(
        &app,
        &state,
        access_token,
        refresh_token,
        display_name,
        email,
        avatar_url,
    )
}

fn save_session_tokens_to_settings(
    app: &AppHandle,
    state: &DesktopState,
    access_token: String,
    refresh_token: String,
    display_name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
) -> Result<(), String> {
    let (same_tokens, previous_display_name, previous_email, previous_avatar_url, server_url) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        (
            settings.session.access_token == access_token
                && settings.session.refresh_token == refresh_token,
            settings.session.display_name.clone(),
            settings.session.email.clone(),
            settings.session.avatar_url.clone(),
            settings.service.server_url.clone(),
        )
    };

    let display_name = display_name
        .and_then(|value| clean_optional_account_text(&value))
        .or_else(|| same_tokens.then_some(previous_display_name));
    let email = email
        .and_then(|value| clean_optional_account_text(&value))
        .or_else(|| same_tokens.then_some(previous_email));
    let avatar_url = avatar_url
        .and_then(|value| sanitize_account_avatar_url(&value))
        .or_else(|| same_tokens.then_some(previous_avatar_url));

    let needs_profile_fetch =
        !same_tokens && (display_name.is_none() || email.is_none() || avatar_url.is_none());
    let profile_access_token = needs_profile_fetch.then(|| access_token.clone());
    let display_name = display_name.unwrap_or_default();
    let email = email.unwrap_or_default();
    let avatar_url = avatar_url.unwrap_or_default();

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    let same_tokens = settings.session.access_token == access_token
        && settings.session.refresh_token == refresh_token;
    if same_tokens
        && settings.session.display_name == display_name
        && settings.session.email == email
        && settings.session.avatar_url == avatar_url
    {
        return Ok(());
    }

    settings.session.access_token = access_token;
    settings.session.refresh_token = refresh_token;
    settings.session.display_name = display_name;
    settings.session.email = email;
    settings.session.avatar_url = avatar_url;
    upsert_saved_session_account(&mut settings.session);
    save_settings(app, &settings)?;
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.cached_servers.clear();
    runtime.cached_sync_error = None;
    let _ = app.emit(DESKTOP_AUTH_CALLBACK_EVENT, ());
    if let Some(profile_access_token) = profile_access_token {
        spawn_session_account_profile_fetch(app.clone(), server_url, profile_access_token);
    }
    Ok(())
}

fn spawn_session_account_profile_fetch(app: AppHandle, server_url: String, access_token: String) {
    thread::spawn(move || {
        let profile = match fetch_session_account_profile(&server_url, &access_token) {
            Ok(Some(profile)) => profile,
            Ok(None) => return,
            Err(err) => {
                log::debug!("failed to fetch account profile: {err}");
                return;
            }
        };

        let state = app.state::<DesktopState>();
        let mut settings = match state.settings.lock() {
            Ok(settings) => settings,
            Err(_) => return,
        };

        if settings.session.access_token != access_token {
            return;
        }

        let mut changed = false;
        if settings.session.display_name.trim().is_empty() {
            if let Some(value) = profile.display_name {
                settings.session.display_name = value;
                changed = true;
            }
        }
        if settings.session.email.trim().is_empty() {
            if let Some(value) = profile.email {
                settings.session.email = value;
                changed = true;
            }
        }
        if settings.session.avatar_url.trim().is_empty() {
            if let Some(value) = profile.avatar_url {
                settings.session.avatar_url = value;
                changed = true;
            }
        }
        if !changed {
            return;
        }

        upsert_saved_session_account(&mut settings.session);
        if let Err(err) = save_settings(&app, &settings) {
            log::debug!("failed to save account profile: {err}");
            return;
        }
        drop(settings);

        let _ = app.emit(DESKTOP_AUTH_CALLBACK_EVENT, ());
    });
}

#[tauri::command]
fn open_workspace(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: Option<String>,
) -> Result<BootstrapPayload, String> {
    let command_started = Instant::now();
    persist_service_target_slug(&app, &state, selected_server_slug, false)?;
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };

    let (
        color_scheme,
        style_scheme,
        appearance_mode,
        custom_themes,
        custom_styles,
        language,
        selected_server_slug,
    ) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        (
            settings.color_scheme.clone(),
            settings.style_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.custom_styles.clone(),
            settings.language.clone(),
            settings.service.selected_server_slug.clone(),
        )
    };
    let target_url = workspace_url_for_slug(&selected_server_slug);
    begin_workspace_launch_trace(&state, command_started, &target_url);
    let service_bound = selected_service_has_local_binding(&service_settings);

    enter_workspace_in_main_window(
        &app,
        &state,
        &color_scheme,
        &style_scheme,
        &appearance_mode,
        &language,
        &custom_theme_set(&custom_themes),
        &custom_style_set(&custom_styles),
        &selected_server_slug,
    )?;
    build_bootstrap(&app, &state, service_bound)
}

#[tauri::command]
fn exit_workspace(app: AppHandle, state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())?;

    if !window_is_workspace(&window) {
        return build_bootstrap(&app, &state, false);
    }

    let (appearance_mode, language) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        (settings.appearance_mode.clone(), settings.language.clone())
    };

    apply_window_language(&app, &window, &language, false);
    apply_launcher_window_theme(&window, &appearance_mode);
    apply_launcher_titlebar_style(&window);
    apply_launcher_window_size(&window);

    // Navigate back to the launcher frontend.
    // In dev mode, use the Vite dev server URL; in production, use the embedded asset URL.
    #[cfg(debug_assertions)]
    let launcher_url = "http://localhost:1420"
        .parse::<Url>()
        .map_err(|err| err.to_string())?;
    #[cfg(not(debug_assertions))]
    let launcher_url = "tauri://localhost"
        .parse::<Url>()
        .map_err(|err| err.to_string())?;
    window.navigate(launcher_url).map_err(|err| err.to_string())?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn open_login(app: AppHandle, state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    let command_started = Instant::now();
    open_login_window(&app, &state, command_started, false)?;
    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn open_login_browser(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let url = format!("{LOGIN_URL}?desktop=1");
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|err| format!("Failed to open browser: {err}"))?;
    log::info!("[login] opened system browser: {url}");
    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn switch_account(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let command_started = Instant::now();
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        upsert_saved_session_account(&mut settings.session);
        save_settings(&app, &settings)?;
    }
    clear_desktop_session(&app, &state)?;
    open_login_window(&app, &state, command_started, true)?;
    let bootstrap = build_bootstrap(&app, &state, true)?;
    Ok(bootstrap)
}

#[tauri::command]
fn switch_account_browser(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        upsert_saved_session_account(&mut settings.session);
        save_settings(&app, &settings)?;
    }
    clear_desktop_session(&app, &state)?;
    let url = format!("{LOGIN_URL}?desktop=1");
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|err| format!("Failed to open browser: {err}"))?;
    log::info!("[login] opened system browser for switch account: {url}");
    build_bootstrap(&app, &state, true)
}

fn handle_desktop_deep_link(app: &AppHandle, url: &Url) -> Result<(), String> {
    // Expected: slock://auth/callback#access_token=xxx&refresh_token=xxx
    if url.host_str() != Some("auth") || url.path() != "/callback" {
        log::info!("[deep-link] ignoring non-auth URL: {url}");
        return Ok(());
    }

    // Tokens are in the fragment (hash): #access_token=xxx&refresh_token=xxx&email=xxx&name=xxx
    let fragment = url.fragment().unwrap_or("");
    let params: std::collections::HashMap<String, String> = fragment
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key.to_string(), value.to_string()))
        })
        .collect();

    let access_token = params
        .get("access_token")
        .cloned()
        .ok_or_else(|| "missing access_token in deep link".to_string())?;
    let refresh_token = params
        .get("refresh_token")
        .cloned()
        .ok_or_else(|| "missing refresh_token in deep link".to_string())?;

    let display_name = params.get("name").cloned();
    let email = params.get("email").cloned();
    let avatar_url = params.get("avatar").cloned();

    log::info!(
        "[deep-link] received auth callback for {:?}",
        email.as_deref().unwrap_or("unknown")
    );

    let state = app.state::<DesktopState>();
    save_session_tokens_to_settings(
        app,
        &state,
        access_token,
        refresh_token,
        display_name,
        email,
        avatar_url,
    )?;

    Ok(())
}

fn clear_webview_cookies(window: &tauri::WebviewWindow) {
    match window.cookies() {
        Ok(cookies) => {
            let mut cleared = 0;
            for cookie in cookies {
                let is_slock = cookie
                    .domain()
                    .map(|d| d == "slock.ai" || d.ends_with(".slock.ai"))
                    .unwrap_or(false);
                if is_slock {
                    log::info!(
                        "[login] deleting cookie: domain={:?} name={:?}",
                        cookie.domain(),
                        cookie.name()
                    );
                    let _ = window.delete_cookie(cookie);
                    cleared += 1;
                }
            }
            log::info!("[login] cleared {cleared} slock.ai cookies");
        }
        Err(err) => {
            log::warn!("[login] failed to get cookies: {err}");
        }
    }
}

fn open_login_window(
    app: &AppHandle,
    state: &DesktopState,
    command_started: Instant,
    clear_login_session_storage: bool,
) -> Result<(), String> {
    begin_workspace_launch_trace(state, command_started, LOGIN_URL);
    {
        let mut auth = state
            .auth
            .lock()
            .map_err(|_| "Unable to lock auth runtime".to_string())?;
        auth.clear_login_session_storage = clear_login_session_storage;
    }
    let url = LOGIN_URL.parse::<Url>().map_err(|err| err.to_string())?;

    if let Some(window) = app.get_webview_window(AUTH_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        if clear_login_session_storage {
            clear_webview_cookies(&window);
        }
        mark_workspace_launch_navigate_called(state, LOGIN_URL);
        window.navigate(url).map_err(|err| err.to_string())?;
        return Ok(());
    }

    mark_workspace_launch_navigate_called(state, LOGIN_URL);
    let window = WebviewWindowBuilder::new(
        app,
        AUTH_LABEL,
        WebviewUrl::External("about:blank".parse().unwrap()),
    )
    .title("Slock Sign In")
    .inner_size(AUTH_WINDOW_WIDTH, AUTH_WINDOW_HEIGHT)
    .min_inner_size(AUTH_WINDOW_MIN_WIDTH, AUTH_WINDOW_MIN_HEIGHT)
    .resizable(true)
    .focused(true)
    .build()
    .map_err(|err| err.to_string())?;
    let _ = window.center();
    if clear_login_session_storage {
        clear_webview_cookies(&window);
    }
    window.navigate(url).map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn close_login_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window(AUTH_LABEL) {
        let _ = window.close();
    }
}

#[tauri::command]
fn activate_account(
    app: AppHandle,
    state: State<'_, DesktopState>,
    account_id: String,
) -> Result<BootstrapPayload, String> {
    let account_id = account_id.trim().to_string();
    if account_id.is_empty() {
        return build_bootstrap(&app, &state, true);
    }

    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        upsert_saved_session_account(&mut settings.session);
        let account = settings
            .session
            .accounts
            .iter()
            .find(|account| account.id == account_id)
            .cloned()
            .ok_or_else(|| "Account is unavailable".to_string())?;

        settings.session.access_token = account.access_token;
        settings.session.refresh_token = account.refresh_token;
        settings.session.display_name = account.display_name;
        settings.session.email = account.email;
        settings.session.avatar_url = account.avatar_url;
        save_settings(&app, &settings)?;
    }

    {
        let mut runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        clear_desktop_session_service_cache(&mut runtime);
    }

    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        if window_is_workspace(&window) {
            apply_workspace_session_seed_to_window(&window, &state)?;
        }
    }

    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn forget_account(
    app: AppHandle,
    state: State<'_, DesktopState>,
    account_id: String,
) -> Result<BootstrapPayload, String> {
    let account_id = account_id.trim().to_string();
    if account_id.is_empty() {
        return build_bootstrap(&app, &state, true);
    }

    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let was_active = settings.session.access_token
            == settings
                .session
                .accounts
                .iter()
                .find(|a| a.id == account_id)
                .map(|a| a.access_token.as_str())
                .unwrap_or("");
        settings.session.accounts.retain(|a| a.id != account_id);
        if was_active {
            if let Some(fallback) = settings.session.accounts.first().cloned() {
                settings.session.access_token = fallback.access_token;
                settings.session.refresh_token = fallback.refresh_token;
                settings.session.display_name = fallback.display_name;
                settings.session.email = fallback.email;
                settings.session.avatar_url = fallback.avatar_url;
            } else {
                settings.session.access_token.clear();
                settings.session.refresh_token.clear();
                settings.session.display_name.clear();
                settings.session.email.clear();
                settings.session.avatar_url.clear();
            }
        }
        save_settings(&app, &settings)?;
    }

    {
        let mut runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime.cached_servers.clear();
        runtime.cached_sync_error = None;
    }

    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        if window_is_workspace(&window) {
            let _ = apply_workspace_session_seed_to_window(&window, &state);
        }
    }

    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn select_service_server(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: String,
) -> Result<BootstrapPayload, String> {
    persist_service_target_slug(&app, &state, Some(selected_server_slug), false)?;
    build_bootstrap_with_service_options(&app, &state, false, true)
}

#[tauri::command]
fn save_service_settings(
    app: AppHandle,
    state: State<'_, DesktopState>,
    service: ServiceSettings,
) -> Result<BootstrapPayload, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    settings.service = sanitize_service_settings(service);
    save_settings(&app, &settings)?;
    drop(settings);

    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn start_service(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: Option<String>,
) -> Result<BootstrapPayload, String> {
    persist_service_target_slug(&app, &state, selected_server_slug, false)?;
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };

    force_start_service(&app, &state, &service_settings)?;
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn stop_service(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: Option<String>,
) -> Result<BootstrapPayload, String> {
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };

    stop_service_process(
        &app,
        &state,
        Some(&service_settings),
        selected_server_slug.as_deref(),
    )?;
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn resolve_app_close_request(
    app: AppHandle,
    state: State<'_, DesktopState>,
    action: String,
    remember: bool,
) -> Result<(), String> {
    let Some(behavior) = close_app_behavior_from_action(&action) else {
        mark_app_close_prompt_visible(&state, false);
        return Ok(());
    };

    let service_settings = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        if remember {
            settings.service.close_app_behavior = close_app_behavior_id(behavior).to_string();
            save_settings(&app, &settings)?;
        }
        settings.service.clone()
    };

    finish_app_close_async(app, behavior, Some(service_settings));
    Ok(())
}

#[tauri::command]
fn refresh_service_servers(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn refresh_service_server_status(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();
    let servers = fetch_cached_service_server_status(&app, &state, &settings);
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    match servers {
        Ok(servers) => {
            runtime.cached_servers = servers;
            runtime.cached_sync_error = None;
        }
        Err(err) => {
            runtime.cached_sync_error = Some(err);
        }
    }
    drop(runtime);
    build_bootstrap_with_service_options(&app, &state, false, true)
}

#[tauri::command]
fn refresh_service_server_catalog(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();
    let servers = fetch_service_server_catalog(&app, &state, &settings)?;
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.cached_servers = servers;
    runtime.cached_sync_error = None;
    drop(runtime);
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn update_service(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: Option<String>,
) -> Result<BootstrapPayload, String> {
    persist_service_target_slug(&app, &state, selected_server_slug, true)?;
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };

    if !selected_service_running_on_current_computer(&state, &service_settings)? {
        return build_bootstrap_with_service_options(&app, &state, false, true);
    }

    stop_service_process(&app, &state, Some(&service_settings), None)?;
    force_start_service(&app, &state, &service_settings)?;
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn fetch_dashboard(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
) -> Result<DashboardData, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    // Find the server ID from cached servers; refresh if cache is empty
    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
    };
    let server_id = match server_id {
        Some(id) => id,
        None => {
            // Cache miss — try refreshing server catalog
            let refreshed = fetch_service_server_catalog(&app, &state, &settings)
                .unwrap_or_default();
            let mut runtime = state
                .service
                .lock()
                .map_err(|_| "Unable to lock service runtime".to_string())?;
            runtime.cached_servers = refreshed;
            runtime
                .cached_servers
                .iter()
                .find(|s| s.slug == slug)
                .map(|s| s.id.clone())
                .ok_or_else(|| format!("Server '{slug}' not found"))?
        }
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let mut warnings: Vec<String> = Vec::new();

    // Fetch channels (GET /channels with X-Server-Id header)
    let channels = match load_authenticated_json::<Vec<DashboardChannel>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    ) {
        Ok(data) => data,
        Err(err) => {
            warnings.push(format!("channels: {err}"));
            Vec::new()
        }
    };

    // Fetch unread counts (GET /channels/unread with X-Server-Id header)
    // Returns { channelId: count } object, convert to Vec<DashboardChannelUnread>
    let unread = match load_authenticated_json::<std::collections::HashMap<String, u32>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels/unread"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    ) {
        Ok(map) => map
            .into_iter()
            .map(|(channel_id, unread_count)| DashboardChannelUnread {
                channel_id,
                unread_count,
            })
            .collect(),
        Err(err) => {
            warnings.push(format!("unread: {err}"));
            Vec::new()
        }
    };

    // Fetch tasks (GET /tasks/server with X-Server-Id header)
    let tasks = match load_authenticated_json::<Vec<DashboardTask>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/tasks/server"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    ) {
        Ok(data) => data,
        Err(err) => {
            warnings.push(format!("tasks: {err}"));
            Vec::new()
        }
    };

    // Fetch agents (GET /agents with X-Server-Id header)
    let agents = match load_authenticated_json::<Vec<DashboardAgent>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/agents"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    ) {
        Ok(data) => data,
        Err(err) => {
            warnings.push(format!("agents: {err}"));
            Vec::new()
        }
    };

    Ok(DashboardData {
        channels,
        unread,
        tasks,
        agents,
        warnings,
    })
}

fn sync_message_reminders(
    app: &AppHandle,
    state: &DesktopState,
    settings: &AppSettings,
    service: &ServiceSnapshot,
) {
    let Some(connection) = build_message_reminder_connection(settings, service) else {
        stop_message_reminders(state);
        return;
    };

    let action = {
        let Ok(mut runtime) = state.message_reminders.lock() else {
            log::warn!("[message-reminders] failed to lock runtime");
            return;
        };

        if runtime.desired_key.as_deref() == Some(&connection.key) {
            runtime.connection = Some(connection.clone());
            if runtime.status == MessageReminderStatus::Connected {
                Some(MessageReminderSyncAction::Inject(connection))
            } else if runtime.status == MessageReminderStatus::Connecting {
                None
            } else if runtime
                .last_attempt
                .map(|attempt| {
                    attempt.elapsed() < Duration::from_millis(MESSAGE_REMINDER_RETRY_AFTER_MS)
                })
                .unwrap_or(false)
            {
                None
            } else {
                runtime.status = MessageReminderStatus::Connecting;
                runtime.last_attempt = Some(Instant::now());
                Some(MessageReminderSyncAction::Start(connection))
            }
        } else {
            runtime.desired_key = Some(connection.key.clone());
            runtime.status = MessageReminderStatus::Connecting;
            runtime.muted_channel_ids.clear();
            runtime.muted_channels_checked_at = None;
            runtime.muted_channels_retry_after = None;
            runtime.recent_message_ids.clear();
            runtime.context = None;
            runtime.connection = Some(connection.clone());
            runtime.last_attempt = Some(Instant::now());
            Some(MessageReminderSyncAction::Start(connection))
        }
    };

    match action {
        Some(MessageReminderSyncAction::Start(connection)) => {
            let app = app.clone();
            thread::spawn(move || connect_message_reminder_socket(app, connection));
        }
        Some(MessageReminderSyncAction::Inject(connection)) => {
            if let Err(err) = inject_message_reminder_bridge(app, &connection) {
                mark_message_reminders_failed(app, &connection.key, &err);
                log::warn!("[message-reminders] bridge re-injection failed: {err}");
            }
        }
        None => {}
    }
}

enum MessageReminderSyncAction {
    Start(MessageReminderConnection),
    Inject(MessageReminderConnection),
}

fn stop_message_reminders(state: &DesktopState) {
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return;
    };
    runtime.desired_key = None;
    runtime.status = MessageReminderStatus::Idle;
    runtime.muted_channel_ids.clear();
    runtime.muted_channels_checked_at = None;
    runtime.muted_channels_retry_after = None;
    runtime.recent_message_ids.clear();
    runtime.context = None;
    runtime.connection = None;
    runtime.last_attempt = None;
}

fn build_message_reminder_connection(
    settings: &AppSettings,
    service: &ServiceSnapshot,
) -> Option<MessageReminderConnection> {
    if !service.authenticated {
        return None;
    }

    let selected_slug = service.selected_server_slug.trim();
    if selected_slug.is_empty() {
        return None;
    }

    let selected_server = service
        .servers
        .iter()
        .find(|server| server.slug == selected_slug)
        .or_else(|| service.servers.iter().find(|server| server.selected))?;
    let access_token = settings.session.access_token.trim();
    if access_token.is_empty() {
        return None;
    }

    let server_url = sanitize_service_server_url(&settings.service.server_url);
    let key = format!(
        "{}|{}|{}",
        server_url,
        selected_server.id,
        token_account_id(access_token)
    );

    Some(MessageReminderConnection {
        key,
        socket_url: message_reminder_socket_url(&server_url),
        server_url,
        server_id: selected_server.id.clone(),
        server_slug: selected_server.slug.clone(),
        server_name: selected_server.name.clone(),
        access_token: access_token.to_string(),
        identity: message_reminder_identity(settings),
    })
}

fn message_reminder_socket_url(server_url: &str) -> String {
    let sanitized = sanitize_service_server_url(server_url);
    if let Some(rest) = sanitized.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = sanitized.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        sanitized
    }
}

fn message_reminder_identity(settings: &AppSettings) -> MessageReminderIdentity {
    let mut values = HashSet::new();
    insert_identity_value(&mut values, &settings.session.display_name);
    insert_identity_value(&mut values, &settings.session.email);
    insert_identity_value(
        &mut values,
        session_account_id(
            &settings.session.access_token,
            Some(&settings.session.display_name),
            Some(&settings.session.email),
        ),
    );

    if let Some(claims) = jwt_claims(&settings.session.access_token) {
        for key in [
            "sub",
            "id",
            "userId",
            "user_id",
            "email",
            "emailAddress",
            "email_address",
            "displayName",
            "display_name",
            "name",
            "username",
        ] {
            if let Some(value) = first_claim_string(&claims, &[key]) {
                insert_identity_value(&mut values, value);
            }
        }
    }

    MessageReminderIdentity { values }
}

fn jwt_claims(access_token: &str) -> Option<serde_json::Value> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = decode_base64_url(payload)?;
    serde_json::from_slice::<serde_json::Value>(&decoded).ok()
}

fn insert_identity_value(values: &mut HashSet<String>, value: impl AsRef<str>) {
    let normalized = normalize_identity_value(value.as_ref());
    if !normalized.is_empty() {
        values.insert(normalized);
    }
}

fn normalize_identity_value(value: &str) -> String {
    value.trim().to_lowercase()
}

fn connect_message_reminder_socket(app: AppHandle, connection: MessageReminderConnection) {
    match fetch_message_reminder_muted_channels(
        &app,
        &connection.server_url,
        &connection.server_id,
    ) {
        Ok(channels) => {
            if !set_message_reminder_muted_channels(&app, &connection.key, channels) {
                return;
            }
        }
        Err(err) => {
            mark_message_reminder_muted_channels_retry(&app, &connection.key);
            log::warn!(
                "[message-reminders] muted channel sync failed; continuing without muted filter: {err}"
            );
        }
    }

    let context = MessageReminderContext {
        key: connection.key.clone(),
        server_url: connection.server_url.clone(),
        server_id: connection.server_id.clone(),
        server_slug: connection.server_slug.clone(),
        server_name: connection.server_name.clone(),
        identity: connection.identity.clone(),
    };

    set_message_reminder_context(&app, context);

    if let Err(err) = inject_message_reminder_bridge(&app, &connection) {
        mark_message_reminders_failed(&app, &connection.key, &err);
        log::warn!("[message-reminders] bridge injection failed: {err}");
        return;
    }

    mark_message_reminders_connected(&app, &connection.key);
}

fn inject_message_reminder_bridge(
    app: &AppHandle,
    connection: &MessageReminderConnection,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return Err("main window unavailable".to_string());
    };
    let script = message_reminder_bridge_script(connection);
    window.eval(&script).map_err(|err| err.to_string())
}

fn inject_message_reminder_bridge_to_webview(
    webview: &tauri::Webview,
    connection: &MessageReminderConnection,
) -> Result<(), String> {
    let script = message_reminder_bridge_script(connection);
    webview.eval(&script).map_err(|err| err.to_string())
}

fn reinject_current_message_reminder_bridge(webview: &tauri::Webview) {
    let state = webview.state::<DesktopState>();
    let connection = {
        let Ok(runtime) = state.message_reminders.lock() else {
            log::warn!("[message-reminders] failed to lock runtime");
            return;
        };
        if runtime.status != MessageReminderStatus::Connected {
            return;
        }
        runtime.connection.clone()
    };

    let Some(connection) = connection else {
        return;
    };

    if let Err(err) = inject_message_reminder_bridge_to_webview(webview, &connection) {
        mark_message_reminders_failed_state(&state, &connection.key, &err);
        log::warn!("[message-reminders] bridge page-load re-injection failed: {err}");
    }
}

fn message_reminder_bridge_script(connection: &MessageReminderConnection) -> String {
    let key = serde_json::to_string(&connection.key).unwrap_or_else(|_| "null".to_string());
    let socket_url =
        serde_json::to_string(&connection.socket_url).unwrap_or_else(|_| "null".to_string());
    let access_token =
        serde_json::to_string(&connection.access_token).unwrap_or_else(|_| "null".to_string());
    let server_id =
        serde_json::to_string(&connection.server_id).unwrap_or_else(|_| "null".to_string());

    format!(
        r#"(() => {{
  const KEY = {key};
  const SOCKET_URL = {socket_url};
  const TOKEN = {access_token};
  const SERVER_ID = {server_id};
  const STATE_KEY = "__slockDesktopMessageReminderBridge";
  const pushEvent = (event) => {{
    if (!event || typeof event !== "object") return;
    try {{
      window.__TAURI__?.core?.invoke("enqueue_message_reminder_event", {{ key: KEY, event }});
    }} catch (_) {{}}
  }};
  const previous = window[STATE_KEY];
  if (previous && previous.key === KEY && previous.socket && previous.socket.connected) {{
    return;
  }}
  if (previous && previous.socket && typeof previous.socket.disconnect === "function") {{
    try {{ previous.socket.disconnect(); }} catch (_) {{}}
  }}
  window[STATE_KEY] = {{ key: KEY, socket: null }};

  const bindSocket = (ioFactory) => {{
    if (window[STATE_KEY]?.key !== KEY) return;
    const socket = ioFactory(SOCKET_URL, {{
      transports: ["websocket"],
      auth: {{ token: TOKEN }},
      forceNew: true,
      reconnection: true,
    }});
    window[STATE_KEY] = {{ key: KEY, socket }};
    const joinServer = () => {{
      try {{ socket.emit("join:channel", {{ serverId: SERVER_ID }}); }} catch (_) {{}}
    }};
    socket.on("connect", joinServer);
    socket.on("reconnect", joinServer);
    socket.on("message:new", pushEvent);
  }};

  if (typeof window.io === "function") {{
    bindSocket(window.io);
    return;
  }}

  const existing = document.querySelector('script[data-slock-desktop-socketio="1"]');
  const script = existing || document.createElement("script");
  script.dataset.slockDesktopSocketio = "1";
  script.async = true;
  script.src = "https://cdn.socket.io/4.8.1/socket.io.min.js";
  script.onload = () => {{
    if (typeof window.io === "function") bindSocket(window.io);
  }};
  script.onerror = () => console.warn("[Slock Desktop] Socket.IO client failed to load");
  if (!existing) document.head.appendChild(script);
}})();"#,
    )
}

fn set_message_reminder_context(app: &AppHandle, context: MessageReminderContext) {
    let state = app.state::<DesktopState>();
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return;
    };
    if runtime.desired_key.as_deref() == Some(&context.key) {
        runtime.context = Some(context);
    }
}

fn mark_message_reminders_connected(app: &AppHandle, key: &str) {
    let state = app.state::<DesktopState>();
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return;
    };
    if runtime.desired_key.as_deref() == Some(key) {
        runtime.status = MessageReminderStatus::Connected;
    }
}

#[tauri::command]
fn enqueue_message_reminder_event(
    app: AppHandle,
    state: State<'_, DesktopState>,
    key: String,
    event: serde_json::Value,
) -> Result<(), String> {
    let context = {
        let runtime = state
            .message_reminders
            .lock()
            .map_err(|_| "Unable to lock message reminders".to_string())?;
        if runtime.desired_key.as_deref() != Some(key.as_str()) {
            return Ok(());
        }
        runtime
            .context
            .clone()
            .ok_or_else(|| "Message reminder context is unavailable".to_string())?
    };

    handle_message_reminder_value(&app, &context, event);
    Ok(())
}

fn fetch_message_reminder_muted_channels(
    app: &AppHandle,
    server_url: &str,
    server_id: &str,
) -> Result<HashSet<String>, String> {
    let state = app.state::<DesktopState>();
    let api_root = api_base_url(server_url);
    let payload = load_authenticated_json::<serde_json::Value>(
        app,
        &state,
        server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels/muted"))
                .header("X-Server-Id", server_id)
                .bearer_auth(access_token)
        },
    )?;
    Ok(parse_muted_channel_ids(&payload))
}

fn parse_muted_channel_ids(payload: &serde_json::Value) -> HashSet<String> {
    let mut ids = HashSet::new();
    collect_muted_channel_ids(payload, &mut ids);
    ids
}

fn collect_muted_channel_ids(payload: &serde_json::Value, ids: &mut HashSet<String>) {
    match payload {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_muted_channel_ids(item, ids);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(channel_id) =
                first_json_string(payload, &["channelId", "channel_id", "id", "slug"])
            {
                insert_channel_id(ids, &channel_id);
            }
            if let Some(channel) = map.get("channel") {
                collect_muted_channel_ids(channel, ids);
            }
            for key in [
                "channels",
                "muted",
                "mutedChannels",
                "data",
                "items",
                "results",
            ] {
                if let Some(value) = map.get(key) {
                    collect_muted_channel_ids(value, ids);
                }
            }
            for (key, value) in map {
                if value.as_bool() == Some(true) {
                    insert_channel_id(ids, key);
                }
            }
        }
        serde_json::Value::String(channel_id) => insert_channel_id(ids, channel_id),
        _ => {}
    }
}

fn insert_channel_id(ids: &mut HashSet<String>, channel_id: &str) {
    let channel_id = channel_id.trim();
    if !channel_id.is_empty() {
        ids.insert(channel_id.to_string());
    }
}

fn set_message_reminder_muted_channels(
    app: &AppHandle,
    key: &str,
    muted_channels: HashSet<String>,
) -> bool {
    let state = app.state::<DesktopState>();
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return false;
    };
    if runtime.desired_key.as_deref() != Some(key) {
        return false;
    }
    runtime.muted_channel_ids = muted_channels;
    runtime.muted_channels_checked_at = Some(Instant::now());
    runtime.muted_channels_retry_after = None;
    true
}

fn mark_message_reminder_muted_channels_retry(app: &AppHandle, key: &str) {
    let state = app.state::<DesktopState>();
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return;
    };
    if runtime.desired_key.as_deref() == Some(key) {
        let now = Instant::now();
        runtime.muted_channels_checked_at = Some(now);
        runtime.muted_channels_retry_after =
            Some(now + Duration::from_millis(MESSAGE_REMINDER_MUTED_FAILURE_RETRY_MS));
    }
}

fn refresh_message_reminder_muted_channels_if_stale(
    app: &AppHandle,
    context: &MessageReminderContext,
) {
    let should_refresh = {
        let state = app.state::<DesktopState>();
        let Ok(runtime) = state.message_reminders.lock() else {
            return;
        };
        if runtime.desired_key.as_deref() != Some(&context.key) {
            return;
        }
        if let Some(retry_after) = runtime.muted_channels_retry_after {
            if Instant::now() < retry_after {
                return;
            }
        }
        runtime
            .muted_channels_checked_at
            .map(|checked_at| {
                checked_at.elapsed() >= Duration::from_millis(MESSAGE_REMINDER_MUTED_REFRESH_MS)
            })
            .unwrap_or(true)
    };

    if !should_refresh {
        return;
    }

    match fetch_message_reminder_muted_channels(app, &context.server_url, &context.server_id) {
        Ok(channels) => {
            set_message_reminder_muted_channels(app, &context.key, channels);
        }
        Err(err) => {
            mark_message_reminder_muted_channels_retry(app, &context.key);
            log::warn!("[message-reminders] muted channel refresh failed: {err}");
        }
    }
}

fn mark_message_reminders_failed(app: &AppHandle, key: &str, error: &str) {
    let state = app.state::<DesktopState>();
    mark_message_reminders_failed_state(&state, key, error);
}

fn mark_message_reminders_failed_state(state: &DesktopState, key: &str, error: &str) {
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return;
    };
    if runtime.desired_key.as_deref() == Some(key) {
        runtime.status = MessageReminderStatus::Failed;
    }
    if !error.trim().is_empty() {
        log::warn!("[message-reminders] connection failed: {error}");
    }
}

fn handle_message_reminder_value(
    app: &AppHandle,
    context: &MessageReminderContext,
    value: serde_json::Value,
) {
    let Some(reminder) = message_reminder_from_value(value, context) else {
        return;
    };
    if should_suppress_message_reminder(app, context, &reminder) {
        return;
    }

    if let Err(err) = show_message_reminder_notification(app, &reminder) {
        log::warn!("[message-reminders] failed to show notification: {err}");
    }
    if let Err(err) = app.emit(MESSAGE_REMINDER_EVENT, reminder) {
        log::debug!("[message-reminders] failed to emit in-app reminder: {err}");
    }
}

fn message_reminder_from_value(
    value: serde_json::Value,
    context: &MessageReminderContext,
) -> Option<MessageReminderPayload> {
    let event_server_id = first_json_string(&value, &["serverId", "server_id"])
        .or_else(|| nested_json_string(&value, "server", &["id"]));
    if event_server_id
        .as_deref()
        .map(|server_id| server_id != context.server_id)
        .unwrap_or(false)
    {
        return None;
    }

    let channel_id = first_json_string(
        &value,
        &[
            "channelId",
            "channel_id",
            "targetChannelId",
            "target_channel_id",
        ],
    )
    .or_else(|| nested_json_string(&value, "channel", &["id", "channelId", "channel_id"]))?;
    let id = first_json_string(&value, &["id", "messageId", "message_id"])
        .unwrap_or_else(|| fallback_message_id(&value, &channel_id));
    let sender_id = first_json_string(
        &value,
        &[
            "senderId",
            "sender_id",
            "userId",
            "user_id",
            "authorId",
            "author_id",
        ],
    )
    .or_else(|| nested_json_string(&value, "sender", &["id", "userId", "user_id"]))
    .or_else(|| nested_json_string(&value, "author", &["id", "userId", "user_id"]));
    let sender_name = first_json_string(
        &value,
        &[
            "senderName",
            "sender_name",
            "authorName",
            "author_name",
            "userName",
            "user_name",
            "name",
        ],
    )
    .or_else(|| nested_json_string(&value, "sender", &["name", "displayName", "display_name"]))
    .or_else(|| nested_json_string(&value, "author", &["name", "displayName", "display_name"]))
    .unwrap_or_else(|| "Slock".to_string());
    let sender_type = first_json_string(&value, &["senderType", "sender_type", "type"]);
    let content_preview =
        message_content_preview(&value).unwrap_or_else(|| "New message".to_string());

    Some(MessageReminderPayload {
        id,
        channel_id,
        server_id: context.server_id.clone(),
        server_slug: context.server_slug.clone(),
        server_name: context.server_name.clone(),
        sender_name,
        sender_id,
        sender_type,
        content_preview,
    })
}

fn fallback_message_id(payload: &serde_json::Value, channel_id: &str) -> String {
    let timestamp = first_json_string(
        payload,
        &[
            "createdAt",
            "created_at",
            "timestamp",
            "time",
            "updatedAt",
            "updated_at",
        ],
    )
    .unwrap_or_default();
    let sender = first_json_string(
        payload,
        &["senderId", "sender_id", "senderName", "sender_name"],
    )
    .unwrap_or_default();
    let content = message_content_preview(payload).unwrap_or_default();
    format!("{channel_id}:{sender}:{timestamp}:{content}")
}

fn first_json_string(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = payload.get(*key).and_then(json_value_to_string) {
            return Some(value);
        }
    }
    None
}

fn nested_json_string(
    payload: &serde_json::Value,
    object_key: &str,
    keys: &[&str],
) -> Option<String> {
    payload
        .get(object_key)
        .and_then(|value| first_json_string(value, keys))
}

fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
    let text = match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn message_content_preview(payload: &serde_json::Value) -> Option<String> {
    let content = first_json_string(payload, &["content", "text", "body", "message", "preview"])
        .or_else(|| {
            nested_json_string(
                payload,
                "content",
                &["text", "plainText", "plain_text", "markdown", "html"],
            )
        })
        .or_else(|| nested_json_string(payload, "message", &["content", "text", "body"]));
    content.and_then(|content| {
        let preview = truncate_message_preview(&plain_message_text(&content), 100);
        if preview.is_empty() {
            None
        } else {
            Some(preview)
        }
    })
}

fn plain_message_text(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut in_tag = false;
    for character in content.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_message_preview(content: &str, max_chars: usize) -> String {
    let mut output = String::new();
    let mut truncated = false;
    for (index, character) in content.chars().enumerate() {
        if index >= max_chars {
            truncated = true;
            break;
        }
        output.push(character);
    }
    if truncated {
        output.push('…');
    }
    output
}

fn should_suppress_message_reminder(
    app: &AppHandle,
    context: &MessageReminderContext,
    reminder: &MessageReminderPayload,
) -> bool {
    if sender_matches_identity(reminder, &context.identity) {
        return true;
    }

    refresh_message_reminder_muted_channels_if_stale(app, context);

    let state = app.state::<DesktopState>();
    let Ok(mut runtime) = state.message_reminders.lock() else {
        return true;
    };
    if runtime.desired_key.as_deref() != Some(&context.key) {
        return true;
    }
    if runtime.muted_channel_ids.contains(&reminder.channel_id) {
        return true;
    }
    if runtime
        .recent_message_ids
        .iter()
        .any(|id| id == &reminder.id)
    {
        return true;
    }
    runtime.recent_message_ids.push_back(reminder.id.clone());
    while runtime.recent_message_ids.len() > MESSAGE_REMINDER_RECENT_LIMIT {
        runtime.recent_message_ids.pop_front();
    }
    false
}

fn sender_matches_identity(
    reminder: &MessageReminderPayload,
    identity: &MessageReminderIdentity,
) -> bool {
    reminder
        .sender_id
        .as_deref()
        .map(|value| identity.values.contains(&normalize_identity_value(value)))
        .unwrap_or(false)
        || identity
            .values
            .contains(&normalize_identity_value(&reminder.sender_name))
}

fn show_message_reminder_notification(
    app: &AppHandle,
    reminder: &MessageReminderPayload,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return Ok(());
    };
    let title = if reminder.sender_name.trim().is_empty() {
        reminder.server_name.clone()
    } else {
        format!("{} · {}", reminder.sender_name, reminder.server_name)
    };
    let title = serde_json::to_string(&title).unwrap_or_else(|_| "\"Slock\"".to_string());
    let body = serde_json::to_string(&reminder.content_preview)
        .unwrap_or_else(|_| "\"New message\"".to_string());
    let script = format!(
        r#"(() => {{
  try {{
    if (!("Notification" in window)) return;
    const show = () => new Notification({title}, {{ body: {body}, tag: "slock-message" }});
    if (Notification.permission === "granted") {{
      show();
    }} else if (Notification.permission !== "denied") {{
      Notification.requestPermission().then((permission) => {{
        if (permission === "granted") show();
      }});
    }}
  }} catch (error) {{
    console.warn("[Slock Desktop] notification failed", error);
  }}
}})();"#
    );
    window.eval(script).map_err(|err| err.to_string())
}

#[tauri::command]
fn fetch_agent_activity(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    agent_id: String,
) -> Result<Vec<AgentActivityEntry>, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    // Try parsing response with fallback: direct array first, then envelope wrappers
    let url = format!("{api_root}/agents/{agent_id}/activity-log?limit=50");
    let response = send_authenticated(&app, &state, &server_url, |client, access_token| {
        client
            .get(&url)
            .header("X-Server-Id", &server_id)
            .bearer_auth(access_token)
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(format!("Desktop API returned {status}: {body}"));
    }

    let body = response
        .text()
        .map_err(|err| format!("Failed to read agent activity response: {err}"))?;

    let body_len = body.len();

    // Try 1: direct array
    if let Ok(entries) = serde_json::from_str::<Vec<AgentActivityEntry>>(&body) {
        eprintln!(
            "[agent-activity] agent={agent_id} status={status} body_len={body_len} parsed=direct_array entries={}",
            entries.len()
        );
        return Ok(entries);
    }

    // Try 2: envelope wrapper (entries/data/activityLog)
    if let Ok(envelope) = serde_json::from_str::<AgentActivityEnvelope>(&body) {
        // Return whichever field has data
        if !envelope.entries.is_empty() {
            eprintln!(
                "[agent-activity] agent={agent_id} status={status} body_len={body_len} parsed=envelope.entries entries={}",
                envelope.entries.len()
            );
            return Ok(envelope.entries);
        }
        if !envelope.data.is_empty() {
            eprintln!(
                "[agent-activity] agent={agent_id} status={status} body_len={body_len} parsed=envelope.data entries={}",
                envelope.data.len()
            );
            return Ok(envelope.data);
        }
        if !envelope.activity_log.is_empty() {
            eprintln!(
                "[agent-activity] agent={agent_id} status={status} body_len={body_len} parsed=envelope.activity_log entries={}",
                envelope.activity_log.len()
            );
            return Ok(envelope.activity_log);
        }
        // All empty — return empty vec (API returned valid JSON but no entries)
        eprintln!(
            "[agent-activity] agent={agent_id} status={status} body_len={body_len} parsed=envelope_all_empty entries=0"
        );
        return Ok(Vec::new());
    }

    // Both failed — log raw body prefix for debugging
    let preview: String = body.chars().take(200).collect();
    eprintln!(
        "[agent-activity] agent={agent_id} status={status} body_len={body_len} parse_failed preview={preview}"
    );
    Err(format!(
        "Failed to parse agent activity: body preview: {preview}"
    ))
}

#[tauri::command]
fn stop_agent(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    agent_id: String,
) -> Result<(), String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    let response = send_authenticated(&app, &state, &server_url, |client, access_token| {
        client
            .post(format!("{api_root}/agents/{agent_id}/stop"))
            .header("X-Server-Id", &server_id)
            .bearer_auth(access_token)
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to stop agent: {status}: {body}"));
    }

    Ok(())
}

#[tauri::command]
fn start_agent(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    agent_id: String,
) -> Result<(), String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    let response = send_authenticated(&app, &state, &server_url, |client, access_token| {
        client
            .post(format!("{api_root}/agents/{agent_id}/start"))
            .header("X-Server-Id", &server_id)
            .bearer_auth(access_token)
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to start agent: {status}: {body}"));
    }

    Ok(())
}

// ── Inbox commands ───────────────────────────────────────────────────

#[tauri::command]
fn fetch_inbox(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    filter: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<InboxFeedResponse, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let filter_val = match filter.as_deref() {
        Some("unread") => "unread",
        _ => "all",
    };
    let limit_val = limit.unwrap_or(30).min(100);
    let offset_val = offset.unwrap_or(0);

    load_authenticated_json::<InboxFeedResponse>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!(
                    "{api_root}/channels/inbox?filter={filter_val}&limit={limit_val}&offset={offset_val}"
                ))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    )
}

#[tauri::command]
fn fetch_followed_threads(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
) -> Result<Vec<InboxThread>, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    let envelope = load_authenticated_json::<FollowedThreadsEnvelope>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels/threads/followed"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    )?;

    Ok(envelope.threads)
}

#[tauri::command]
fn fetch_dm_channels(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
) -> Result<Vec<InboxDmChannel>, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    load_authenticated_json::<Vec<InboxDmChannel>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels/dm"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    )
}

#[tauri::command]
fn fetch_unread_channels(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
) -> Result<Vec<InboxUnreadEntry>, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    // The API returns { channelId: count } object — convert to Vec<InboxUnreadEntry>
    let map = load_authenticated_json::<std::collections::HashMap<String, u32>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/channels/unread"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
        },
    )?;

    Ok(map
        .into_iter()
        .map(|(channel_id, unread_count)| InboxUnreadEntry {
            channel_id,
            unread_count,
        })
        .collect())
}

#[tauri::command]
fn fetch_thread_messages(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    channel_id: String,
    limit: Option<u32>,
    before: Option<String>,
    after: Option<String>,
) -> Result<InboxMessagesResponse, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }
    if channel_id.trim().is_empty() {
        return Err("Channel ID is required".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let msg_limit = limit.unwrap_or(50);

    let envelope = load_authenticated_json::<MessagesEnvelope>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            let mut req = client
                .get(format!("{api_root}/messages/channel/{channel_id}"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
                .query(&[("limit", msg_limit.to_string())]);

            if let Some(ref b) = before {
                req = req.query(&[("before", b.as_str())]);
            }
            if let Some(ref a) = after {
                req = req.query(&[("after", a.as_str())]);
            }

            req
        },
    )?;

    Ok(InboxMessagesResponse {
        messages: envelope.messages,
        has_more: envelope.has_more,
    })
}

/// Fetch messages for any channel (regular or thread).
/// Same API endpoint as fetch_thread_messages — provided as a semantic alias
/// for the unified inbox "channel messages" use case.
#[tauri::command]
fn fetch_channel_messages(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    channel_id: String,
    limit: Option<u32>,
    before: Option<String>,
    after: Option<String>,
) -> Result<InboxMessagesResponse, String> {
    fetch_thread_messages(app, state, server_slug, channel_id, limit, before, after)
}

/// Fetch all members of a server.
/// GET /api/servers/{serverId}/members
#[tauri::command]
fn fetch_server_members(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
) -> Result<Vec<ServerMember>, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    load_authenticated_json::<Vec<ServerMember>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers/{server_id}/members"))
                .bearer_auth(access_token)
        },
    )
}

/// Fetch unread summary across all servers.
/// GET /api/servers/unread-summary (no X-Server-Id needed)
#[tauri::command]
fn fetch_server_unread_summary(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<Vec<ServerUnreadEntry>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    load_authenticated_json::<Vec<ServerUnreadEntry>>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers/unread-summary"))
                .bearer_auth(access_token)
        },
    )
}

#[tauri::command]
fn send_message(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    channel_id: String,
    content: String,
) -> Result<InboxMessage, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }
    if channel_id.trim().is_empty() {
        return Err("Channel ID is required".to_string());
    }
    if content.trim().is_empty() {
        return Err("Message content cannot be empty".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    let envelope = load_authenticated_json::<SendMessageEnvelope>(
        &app,
        &state,
        &server_url,
        |client, access_token| {
            client
                .post(format!("{api_root}/messages"))
                .header("X-Server-Id", &server_id)
                .bearer_auth(access_token)
                .json(&serde_json::json!({
                    "channelId": channel_id,
                    "content": content
                }))
        },
    )?;

    Ok(envelope.message)
}

#[tauri::command]
fn mark_channel_read(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    channel_id: String,
) -> Result<(), String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }
    if channel_id.trim().is_empty() {
        return Err("Channel ID is required".to_string());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .service
        .clone();

    let server_id = {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .map(|s| s.id.clone())
            .ok_or_else(|| format!("Server '{slug}' not found"))?
    };

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);

    let response = send_authenticated(&app, &state, &server_url, |client, access_token| {
        client
            .post(format!("{api_root}/channels/{channel_id}/read"))
            .header("X-Server-Id", &server_id)
            .bearer_auth(access_token)
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to mark channel as read: {status}: {body}"));
    }

    Ok(())
}

/// Manually bind a server's machine to this local desktop.
/// Persists the binding to settings.json with source="user_bound".
#[tauri::command]
fn bind_local_machine(
    app: AppHandle,
    state: State<'_, DesktopState>,
    server_slug: String,
    machine_id: String,
) -> Result<BootstrapPayload, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("No server selected".to_string());
    }
    let machine_id_trimmed = machine_id.trim();
    if machine_id_trimmed.is_empty() {
        return Err("Machine ID is required".to_string());
    }

    // Find the server and machine from cached data
    // Extract server info from cache, then release locks before network call
    let (server_id, server_url) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;

        let server = runtime
            .cached_servers
            .iter()
            .find(|s| s.slug == slug)
            .ok_or_else(|| format!("Server '{slug}' not found in cache"))?;

        (server.id.clone(), settings.service.server_url.clone())
    };
    // Locks released — safe to make network calls

    // Try to find the machine name from the API
    let machines = fetch_server_machines(
        &app,
        &state,
        &server_url,
        &server_id,
    )
    .unwrap_or_default();
    let machine_name = machines
        .iter()
        .find(|m| m.id == machine_id_trimmed)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let binding = ServiceMachineBinding {
        server_id,
        server_slug: slug.to_string(),
        machine_id: machine_id_trimmed.to_string(),
        machine_name,
        api_key: String::new(),
        source: "user_bound".to_string(),
    };
    upsert_service_binding(&app, &state, binding)?;

    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn open_service_log(
    app: AppHandle,
    server_slug: String,
    from_epoch_ms: Option<i64>,
    to_epoch_ms: Option<i64>,
) -> Result<ServiceLogSnapshot, String> {
    let slug = server_slug.trim();
    if slug.is_empty() {
        return Err("Choose a server before opening logs.".to_string());
    }

    let now = current_epoch_ms();
    let to_epoch_ms = to_epoch_ms.unwrap_or(now);
    let from_epoch_ms = from_epoch_ms.unwrap_or(to_epoch_ms - SERVICE_LOG_DEFAULT_WINDOW_MS);
    let (from_epoch_ms, to_epoch_ms) = if from_epoch_ms <= to_epoch_ms {
        (from_epoch_ms, to_epoch_ms)
    } else {
        (to_epoch_ms, from_epoch_ms)
    };

    let log_path = service_log_path(&app, slug)?;
    if !log_path.exists() {
        fs::write(&log_path, format!("Slock daemon log for {slug}\n"))
            .map_err(|err| format!("Unable to create server log: {err}"))?;
    }

    read_service_log_range(slug, &log_path, from_epoch_ms, to_epoch_ms)
}

#[tauri::command]
fn start_window_drag(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())?;

    window.start_dragging().map_err(|err| err.to_string())
}

#[tauri::command]
async fn check_desktop_update(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<DesktopUpdateCheck, String> {
    let current_version = app.package_info().version.to_string();
    let updater = desktop_updater(&app)?;
    let update = updater.check().await.map_err(|err| err.to_string())?;

    let result = match update {
        Some(update) => DesktopUpdateCheck {
            current_version,
            available: true,
            version: Some(update.version),
            body: update.body,
            date: update.date.map(|date| date.to_string()),
            download_url: Some(update.download_url.to_string()),
        },
        None => DesktopUpdateCheck {
            current_version,
            available: false,
            version: None,
            body: None,
            date: None,
            download_url: None,
        },
    };

    if let Ok(mut cache) = state.update_cache.lock() {
        *cache = Some(result.clone());
    }
    let _ = app.emit("desktop_update_checked", result.clone());

    Ok(result)
}

#[tauri::command]
async fn install_desktop_update(app: AppHandle) -> Result<(), String> {
    let updater = desktop_updater(&app)?;

    let Some(update) = updater.check().await.map_err(|err| err.to_string())? else {
        return Err("Slock Desktop is already up to date.".to_string());
    };

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|err| err.to_string())?;
    app.restart()
}

fn desktop_updater(app: &AppHandle) -> Result<Updater, String> {
    let endpoint = Url::parse(DESKTOP_UPDATER_ENDPOINT).map_err(|err| err.to_string())?;
    app.updater_builder()
        .timeout(Duration::from_secs(DESKTOP_UPDATE_CHECK_TIMEOUT))
        .endpoints(vec![endpoint])
        .map_err(|err| err.to_string())?
        .build()
        .map_err(|err| err.to_string())
}

fn handle_window_close_requested(app: &AppHandle, state: &DesktopState) {
    if close_request_confirmed(state) {
        app.exit(0);
        return;
    }

    match current_close_app_behavior(state) {
        CloseAppServiceBehavior::Ask => {
            if service_may_be_running(app, state) {
                request_app_close_prompt(app, state);
            } else {
                mark_app_close_confirmed(state);
                app.exit(0);
            }
        }
        behavior => {
            if request_app_close_progress(app, state, behavior) {
                let service_settings = state
                    .settings
                    .lock()
                    .ok()
                    .map(|settings| settings.service.clone());
                finish_app_close_async(app.clone(), behavior, service_settings);
            }
        }
    }
}

fn handle_app_exit_requested(app: &AppHandle, state: &DesktopState) -> bool {
    if close_request_confirmed(state) {
        return true;
    }

    match current_close_app_behavior(state) {
        CloseAppServiceBehavior::Ask => {
            if service_may_be_running(app, state) {
                request_app_close_prompt(app, state);
                false
            } else {
                true
            }
        }
        CloseAppServiceBehavior::Keep => true,
        CloseAppServiceBehavior::Stop => {
            if request_app_close_progress(app, state, CloseAppServiceBehavior::Stop) {
                let service_settings = state
                    .settings
                    .lock()
                    .ok()
                    .map(|settings| settings.service.clone());
                finish_app_close_async(
                    app.clone(),
                    CloseAppServiceBehavior::Stop,
                    service_settings,
                );
            }
            false
        }
    }
}

fn handle_app_exit(app: &AppHandle, state: &DesktopState) {
    stop_message_reminders(state);
    if current_close_app_behavior(state) == CloseAppServiceBehavior::Stop {
        if take_app_close_service_stop_completed(state) {
            return;
        }
        let service_settings = state
            .settings
            .lock()
            .ok()
            .map(|settings| settings.service.clone());
        let _ = stop_service_process(app, state, service_settings.as_ref(), None);
    }
}

fn finish_app_close_async(
    app: AppHandle,
    behavior: CloseAppServiceBehavior,
    service_settings: Option<ServiceSettings>,
) {
    thread::spawn(move || {
        let state = app.state::<DesktopState>();
        let result = if behavior == CloseAppServiceBehavior::Stop {
            stop_service_process(&app, &state, service_settings.as_ref(), None)
        } else {
            Ok(())
        };

        match result {
            Ok(()) => {
                if behavior == CloseAppServiceBehavior::Stop {
                    mark_app_close_service_stop_completed(&state);
                }
                mark_app_close_confirmed(&state);
                app.exit(0);
            }
            Err(err) => {
                log::warn!("failed to close app with selected server behavior: {err}");
                mark_app_close_prompt_visible(&state, false);
                show_app_close_error(&app, &err);
            }
        }
    });
}

fn current_close_app_behavior(state: &DesktopState) -> CloseAppServiceBehavior {
    state
        .settings
        .lock()
        .ok()
        .map(|settings| close_app_behavior_from_id(&settings.service.close_app_behavior))
        .unwrap_or(CloseAppServiceBehavior::Ask)
}

fn request_app_close_prompt(app: &AppHandle, state: &DesktopState) {
    if !mark_app_close_prompt_requested(state) {
        return;
    }

    let copy = app_close_prompt_copy(state);
    let script = app_close_prompt_script(&copy);
    let result = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())
        .and_then(|window| {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            window.eval(&script).map_err(|err| err.to_string())
        });

    if let Err(err) = result {
        log::warn!("failed to show app close prompt: {err}");
        mark_app_close_prompt_visible(state, false);
    }
}

fn request_app_close_progress(
    app: &AppHandle,
    state: &DesktopState,
    behavior: CloseAppServiceBehavior,
) -> bool {
    if !mark_app_close_prompt_requested(state) {
        return false;
    }

    let copy = app_close_prompt_copy(state);
    let prompt_script = app_close_prompt_script(&copy);
    let progress_script = app_close_progress_script(behavior);
    let result = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())
        .and_then(|window| {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            window.eval(&prompt_script).map_err(|err| err.to_string())?;
            window.eval(&progress_script).map_err(|err| err.to_string())
        });

    if let Err(err) = result {
        log::warn!("failed to show app close progress: {err}");
        mark_app_close_prompt_visible(state, false);
        return false;
    }

    true
}

fn close_app_action_for_behavior(behavior: CloseAppServiceBehavior) -> &'static str {
    match behavior {
        CloseAppServiceBehavior::Stop => "closeServer",
        _ => "keepServer",
    }
}

fn app_close_progress_script(behavior: CloseAppServiceBehavior) -> String {
    let action = close_app_action_for_behavior(behavior);
    let payload = serde_json::to_string(action).unwrap_or_else(|_| "\"keepServer\"".to_string());
    format!("window.__slockDesktopCloseSetBusy?.({payload});")
}

fn show_app_close_error(app: &AppHandle, message: &str) {
    let payload = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!("window.__slockDesktopCloseSetError?.({payload});");
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.eval(&script);
    }
}

fn app_close_prompt_copy(state: &DesktopState) -> CloseAppPromptCopy {
    let (language, selected_server_slug) = state
        .settings
        .lock()
        .map(|settings| {
            (
                resolve_desktop_language(&settings.language).to_string(),
                settings.service.selected_server_slug.clone(),
            )
        })
        .unwrap_or_else(|_| ("en-US".to_string(), String::new()));
    let server_label = if selected_server_slug.trim().is_empty() {
        "Slock daemon".to_string()
    } else {
        selected_server_slug
    };

    if language == "zh-CN" {
        CloseAppPromptCopy {
            title: "退出 Slock",
            description: "退出 Slock 后，Server 可以继续运行，也可以随应用一起关闭。".to_string(),
            server_label: format!("当前 Server：{server_label}"),
            keep_server: "保留 Server 并退出",
            close_server: "关闭 Server 并退出",
            cancel: "取消",
            remember: "记住这次选择",
            processing_keep_server: "正在保留 Server 并退出…",
            processing_close_server: "正在关闭 Server 并退出…",
            error: "关闭处理失败，请重试。",
        }
    } else {
        CloseAppPromptCopy {
            title: "Quit Slock",
            description: "After Slock quits, the server can stay running or close with the app."
                .to_string(),
            server_label: format!("Current server: {server_label}"),
            keep_server: "Keep server running and quit",
            close_server: "Close server and quit",
            cancel: "Cancel",
            remember: "Remember this choice",
            processing_keep_server: "Keeping server running and quitting…",
            processing_close_server: "Closing server and quitting…",
            error: "Close handling failed. Try again.",
        }
    }
}

fn app_close_prompt_script(copy: &CloseAppPromptCopy) -> String {
    let payload = serde_json::to_string(copy).unwrap_or_else(|_| "{}".to_string());
    format!(
        r##"(function () {{
  const copy = {payload};
  const hostId = "slock-desktop-close-host";
  document.getElementById(hostId)?.remove();
  const host = document.createElement("div");
  host.id = hostId;
  const surfaceLooksDark = (element) => {{
    if (!element) return false;
    const color = window.getComputedStyle(element).backgroundColor;
    const match = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    if (!match) return false;
    const [, red, green, blue] = match.map(Number);
    return red * 0.299 + green * 0.587 + blue * 0.114 < 128;
  }};
  const shell = document.querySelector(".studio-shell");
  const dark =
    shell?.getAttribute("data-mode") === "dark" ||
    surfaceLooksDark(shell) ||
    surfaceLooksDark(document.body) ||
    document.querySelector('[data-mode="dark"]') ||
    document.documentElement.classList.contains("dark") ||
    window.matchMedia?.("(prefers-color-scheme: dark)")?.matches;
  const tone = dark
    ? {{
        scrim: "rgba(3,7,18,.72)",
        panel: "#1f241f",
        panelBorder: "rgba(148,163,184,.22)",
        title: "#f8fafc",
        body: "#94a3b8",
        muted: "#8190a8",
        label: "#94a3b8",
        error: "#fca5a5",
        secondaryBg: "#252b25",
        secondaryText: "#f8fafc",
        secondaryBorder: "rgba(148,163,184,.42)",
        dangerBg: "#302525",
        dangerText: "#fecaca",
        dangerBorder: "rgba(248,113,113,.56)",
        primaryBg: "#10a37f",
        primaryText: "#fff",
        primaryBorder: "#10a37f",
      }}
    : {{
        scrim: "rgba(15,23,42,.32)",
        panel: "#fff",
        panelBorder: "rgba(15,23,42,.14)",
        title: "#111827",
        body: "#4b5563",
        muted: "#6b7280",
        label: "#374151",
        error: "#b42318",
        secondaryBg: "#fff",
        secondaryText: "#374151",
        secondaryBorder: "#d1d5db",
        dangerBg: "#fff",
        dangerText: "#b42318",
        dangerBorder: "#d92d20",
        primaryBg: "#10a37f",
        primaryText: "#fff",
        primaryBorder: "#10a37f",
      }};
  host.style.cssText = `position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;background:${{tone.scrim}};font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:${{tone.title}};cursor:default;`;
  host.innerHTML = `
    <div role="dialog" aria-modal="true" aria-labelledby="slock-close-title" style="width:min(420px,calc(100vw - 32px));border:1px solid ${{tone.panelBorder}};border-radius:18px;background:${{tone.panel}};box-shadow:0 24px 80px rgba(2,6,23,.38);padding:22px;">
      <h2 id="slock-close-title" data-close-copy="title" style="margin:0;font-size:18px;line-height:1.3;font-weight:700;color:${{tone.title}};"></h2>
      <p data-close-copy="description" style="margin:10px 0 0;font-size:14px;line-height:1.55;color:${{tone.body}};"></p>
      <p data-close-copy="serverLabel" style="margin:12px 0 0;font-size:12px;line-height:1.4;color:${{tone.muted}};"></p>
      <label style="display:flex;align-items:center;gap:8px;margin:18px 0 0;font-size:13px;color:${{tone.label}};">
        <input data-close-remember type="checkbox" style="width:16px;height:16px;accent-color:#10a37f;" />
        <span data-close-copy="remember"></span>
      </label>
      <p data-close-busy role="status" style="display:none;margin:14px 0 0;font-size:13px;line-height:1.4;color:${{tone.body}};"></p>
      <p data-close-error style="display:none;margin:14px 0 0;font-size:13px;line-height:1.4;color:${{tone.error}};"></p>
      <div style="display:flex;gap:10px;justify-content:flex-end;margin-top:20px;flex-wrap:wrap;">
        <button type="button" data-close-action="cancel" data-close-copy="cancel" style="appearance:none;-webkit-appearance:none;border:1px solid ${{tone.secondaryBorder}};border-radius:10px;background:${{tone.secondaryBg}};color:${{tone.secondaryText}};font-size:13px;font-weight:650;padding:9px 12px;cursor:pointer;"></button>
        <button type="button" data-close-action="closeServer" data-close-copy="closeServer" style="appearance:none;-webkit-appearance:none;border:1px solid ${{tone.dangerBorder}};border-radius:10px;background:${{tone.dangerBg}};color:${{tone.dangerText}};font-size:13px;font-weight:650;padding:9px 12px;cursor:pointer;"></button>
        <button type="button" data-close-action="keepServer" data-close-copy="keepServer" style="appearance:none;-webkit-appearance:none;border:1px solid ${{tone.primaryBorder}};border-radius:10px;background:${{tone.primaryBg}};color:${{tone.primaryText}};font-size:13px;font-weight:700;padding:9px 12px;cursor:pointer;"></button>
      </div>
    </div>`;
  document.body.appendChild(host);
  host.querySelectorAll("[data-close-copy]").forEach((element) => {{
    const key = element.getAttribute("data-close-copy");
    element.textContent = copy[key] || "";
  }});
  const invoke = window.__TAURI__?.core?.invoke;
  const busyMessage = host.querySelector("[data-close-busy]");
  const error = host.querySelector("[data-close-error]");
  const remember = host.querySelector("[data-close-remember]");
  const setBusy = (busy, action) => {{
    if (busyMessage) {{
      busyMessage.textContent = action === "closeServer" ? copy.processingCloseServer : copy.processingKeepServer;
      busyMessage.style.display = busy ? "block" : "none";
    }}
    host.querySelectorAll("button").forEach((button) => {{
      button.disabled = busy;
      button.style.opacity = busy ? ".65" : "1";
      button.style.cursor = busy ? "default" : "pointer";
    }});
  }};
  window.__slockDesktopCloseSetBusy = (action) => {{
    if (error) error.style.display = "none";
    setBusy(true, action);
  }};
  window.__slockDesktopCloseSetError = (message) => {{
    setBusy(false, "closeServer");
    if (error) {{
      error.textContent = message || copy.error;
      error.style.display = "block";
    }}
  }};
  host.addEventListener("click", async (event) => {{
    const target = event.target instanceof Element ? event.target : event.target?.parentElement;
    const button = target?.closest("[data-close-action]");
    if (!button) return;
    const action = button.getAttribute("data-close-action");
    if (action === "cancel") {{
      host.remove();
      if (invoke) await invoke("resolve_app_close_request", {{ action, remember: false }});
      return;
    }}
    if (!invoke) {{
      error.textContent = copy.error;
      error.style.display = "block";
      return;
    }}
    try {{
      setBusy(true, action);
      error.style.display = "none";
      await invoke("resolve_app_close_request", {{ action, remember: !!remember?.checked }});
    }} catch (err) {{
      setBusy(false, action);
      error.textContent = err && typeof err === "object" && "message" in err ? err.message : String(err || copy.error);
      error.style.display = "block";
    }}
  }});
  host.addEventListener("keydown", async (event) => {{
    if (event.key !== "Escape") return;
    host.remove();
    if (invoke) await invoke("resolve_app_close_request", {{ action: "cancel", remember: false }});
  }});
  host.tabIndex = -1;
  host.focus();
}})();"##
    )
}

fn mark_app_close_prompt_requested(state: &DesktopState) -> bool {
    let mut runtime = match state.app_close.lock() {
        Ok(runtime) => runtime,
        Err(_) => return true,
    };
    if runtime.prompt_visible {
        return false;
    }
    runtime.prompt_visible = true;
    true
}

fn mark_app_close_prompt_visible(state: &DesktopState, visible: bool) {
    if let Ok(mut runtime) = state.app_close.lock() {
        runtime.prompt_visible = visible;
    }
}

fn mark_app_close_confirmed(state: &DesktopState) {
    if let Ok(mut runtime) = state.app_close.lock() {
        runtime.prompt_visible = false;
        runtime.confirmed_exit = true;
    }
}

fn mark_app_close_service_stop_completed(state: &DesktopState) {
    if let Ok(mut runtime) = state.app_close.lock() {
        runtime.service_stop_completed = true;
    }
}

fn take_app_close_service_stop_completed(state: &DesktopState) -> bool {
    let mut runtime = match state.app_close.lock() {
        Ok(runtime) => runtime,
        Err(_) => return false,
    };
    let completed = runtime.service_stop_completed;
    runtime.service_stop_completed = false;
    completed
}

fn close_request_confirmed(state: &DesktopState) -> bool {
    let mut runtime = match state.app_close.lock() {
        Ok(runtime) => runtime,
        Err(_) => return false,
    };
    if runtime.confirmed_exit {
        runtime.confirmed_exit = false;
        runtime.prompt_visible = false;
        return true;
    }
    false
}

fn service_may_be_running(_app: &AppHandle, state: &DesktopState) -> bool {
    let service_settings = state
        .settings
        .lock()
        .ok()
        .map(|settings| settings.service.clone());
    let Some(service_settings) = service_settings else {
        return false;
    };

    let target_slug = service_settings.selected_server_slug.trim().to_string();
    let mut runtime_active_slug = None;
    let mut runtime_active_machine_id = None;
    let mut cached_server = None;
    if let Ok(mut runtime) = state.service.lock() {
        if let Some(child) = runtime.child.as_mut() {
            if child.try_wait().ok().flatten().is_none() {
                return true;
            }
            runtime.child = None;
        }
        runtime_active_slug = runtime.active_server_slug.clone();
        runtime_active_machine_id = runtime.active_machine_id.clone();
        cached_server = runtime
            .cached_servers
            .iter()
            .find(|server| server.slug == target_slug)
            .cloned();
    }
    if target_slug.is_empty() {
        return false;
    }

    if runtime_active_slug.as_deref() == Some(target_slug.as_str())
        || cached_server
            .as_ref()
            .map(|server| machine_counts_as_started(&server.machine_status))
            .unwrap_or(false)
    {
        return true;
    }

    let binding = service_settings
        .machines
        .iter()
        .find(|binding| binding.server_slug == target_slug)
        .cloned();
    let machine_id = cached_server
        .as_ref()
        .and_then(|server| server.machine_id.as_deref())
        .or_else(|| binding.as_ref().map(|binding| binding.machine_id.as_str()))
        .or(runtime_active_machine_id.as_deref())
        .filter(|machine_id| !machine_id.trim().is_empty());
    let api_key_prefix = cached_server
        .as_ref()
        .and_then(|server| server.api_key_prefix.as_deref())
        .filter(|prefix| !prefix.trim().is_empty());
    let api_key = binding
        .as_ref()
        .map(|binding| binding.api_key.as_str())
        .filter(|api_key| !api_key.trim().is_empty());

    find_daemon_process_ids(
        &service_settings.server_url,
        Some(target_slug.as_str()),
        machine_id,
        api_key_prefix,
        api_key,
        false,
    )
    .map(|pids| !pids.is_empty())
    .unwrap_or(false)
}

fn close_app_behavior_from_action(action: &str) -> Option<CloseAppServiceBehavior> {
    match action {
        "keepServer" => Some(CloseAppServiceBehavior::Keep),
        "closeServer" => Some(CloseAppServiceBehavior::Stop),
        _ => None,
    }
}

fn close_app_behavior_from_id(value: &str) -> CloseAppServiceBehavior {
    match value {
        "keep" => CloseAppServiceBehavior::Keep,
        "stop" => CloseAppServiceBehavior::Stop,
        _ => CloseAppServiceBehavior::Ask,
    }
}

fn close_app_behavior_id(behavior: CloseAppServiceBehavior) -> &'static str {
    match behavior {
        CloseAppServiceBehavior::Ask => "ask",
        CloseAppServiceBehavior::Keep => "keep",
        CloseAppServiceBehavior::Stop => "stop",
    }
}

fn build_bootstrap(
    app: &AppHandle,
    state: &State<'_, DesktopState>,
    refresh_service: bool,
) -> Result<BootstrapPayload, String> {
    build_bootstrap_with_service_options(app, state, refresh_service, refresh_service)
}

fn build_bootstrap_with_service_options(
    app: &AppHandle,
    state: &State<'_, DesktopState>,
    refresh_service: bool,
    detect_service_process: bool,
) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .clone();

    let service = collect_service_snapshot(
        app,
        state,
        &settings.service,
        refresh_service,
        detect_service_process,
    )?;
    sync_message_reminders(app, state, &settings, &service);
    let appearance_mode = theme::normalize_mode(&settings.appearance_mode).to_string();
    let latest = state
        .update_cache
        .lock()
        .map_err(|_| "Unable to lock desktop update cache".to_string())?
        .clone();
    let updates = UpdateSnapshot {
        current_version: app.package_info().version.to_string(),
        latest,
    };

    Ok(BootstrapPayload {
        app_name: "slock-desktop".to_string(),
        workspace_url: workspace_url_for_slug(&settings.service.selected_server_slug),
        color_scheme: settings.color_scheme.clone(),
        style_scheme: settings.style_scheme.clone(),
        appearance_mode: appearance_mode.clone(),
        custom_themes: settings.custom_themes.clone(),
        custom_styles: settings.custom_styles.clone(),
        language: sanitize_language(&settings.language).to_string(),
        resolved_language: resolve_desktop_language(&settings.language).to_string(),
        workspace_open: main_window_is_workspace(app),
        themes: color_catalog(
            &appearance_mode,
            &settings.style_scheme,
            &custom_theme_set(&settings.custom_themes),
            &custom_style_set(&settings.custom_styles),
        ),
        theme_styles: style_catalog(
            &appearance_mode,
            &settings.color_scheme,
            &custom_theme_set(&settings.custom_themes),
            &custom_style_set(&settings.custom_styles),
        ),
        service,
        updates,
    })
}

fn enter_workspace_in_main_window(
    app: &AppHandle,
    state: &DesktopState,
    theme_id: &str,
    style_id: &str,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeSet,
    custom_style: &CustomStyleSet,
    selected_server_slug: &str,
) -> Result<(), String> {
    let target_url = workspace_url_for_slug(selected_server_slug);
    enter_workspace_url_in_main_window(
        app,
        state,
        theme_id,
        style_id,
        theme_mode,
        language,
        custom_theme,
        custom_style,
        selected_server_slug,
        &target_url,
    )
}

fn enter_workspace_url_in_main_window(
    app: &AppHandle,
    state: &DesktopState,
    theme_id: &str,
    style_id: &str,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeSet,
    custom_style: &CustomStyleSet,
    server_slug: &str,
    target_url: &str,
) -> Result<(), String> {
    let theme = resolve_theme_with_style(theme_id, style_id, theme_mode, custom_theme, custom_style);
    let resolved_language = resolve_desktop_language(language);
    let target_url = target_url.parse::<Url>().map_err(|err| err.to_string())?;
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())?;

    if window_is_workspace(&window) {
        let _ = window.unminimize();
        let _ = window.show();
        apply_workspace_window_size(&window, false);
        apply_workspace_titlebar_style(&window);
        let _ = window.set_focus();
        apply_window_theme(&window, theme_mode);
        apply_window_language(app, &window, language, true);
        apply_workspace_session_seed_to_window(&window, state)?;
        if window.url().ok().as_ref() != Some(&target_url) {
            mark_workspace_launch_navigate_called(state, target_url.as_str());
            return window.navigate(target_url).map_err(|err| err.to_string());
        }
        return apply_workspace_scripts_to_window(
            &window,
            theme,
            theme_id,
            style_id,
            theme_mode,
            language,
            resolved_language,
            server_slug,
            custom_theme,
            custom_style,
        );
    }

    apply_window_language(app, &window, language, true);
    apply_window_theme(&window, theme_mode);
    apply_workspace_window_size(&window, true);
    apply_workspace_titlebar_style(&window);
    let _ = window.set_focus();
    mark_workspace_launch_navigate_called(state, target_url.as_str());
    window.navigate(target_url).map_err(|err| err.to_string())
}

fn apply_launcher_window_size(window: &tauri::WebviewWindow) {
    let _ = window.set_min_size(Some(LogicalSize::new(
        LAUNCHER_WINDOW_MIN_WIDTH,
        LAUNCHER_WINDOW_MIN_HEIGHT,
    )));
    let _ = window.set_size(LogicalSize::new(
        LAUNCHER_WINDOW_WIDTH,
        LAUNCHER_WINDOW_HEIGHT,
    ));
    let _ = window.center();
}

fn apply_launcher_titlebar_style(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
    }
}

fn apply_workspace_window_size(window: &tauri::WebviewWindow, reposition: bool) {
    let _ = window.set_min_size(Some(LogicalSize::new(
        WORKSPACE_WINDOW_MIN_WIDTH,
        WORKSPACE_WINDOW_MIN_HEIGHT,
    )));
    let _ = window.set_size(LogicalSize::new(
        WORKSPACE_WINDOW_WIDTH,
        WORKSPACE_WINDOW_HEIGHT,
    ));
    if reposition {
        place_workspace_webview_window(window);
    }
}

fn apply_workspace_titlebar_style(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
    }
}

fn apply_workspace_window_size_to_window(window: &tauri::Window, reposition: bool) {
    let _ = window.set_min_size(Some(LogicalSize::new(
        WORKSPACE_WINDOW_MIN_WIDTH,
        WORKSPACE_WINDOW_MIN_HEIGHT,
    )));
    let _ = window.set_size(LogicalSize::new(
        WORKSPACE_WINDOW_WIDTH,
        WORKSPACE_WINDOW_HEIGHT,
    ));
    if reposition {
        place_workspace_window(window);
    }
}

fn place_workspace_webview_window(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let _ = window.set_position(workspace_position_from_monitor(&monitor));
    }
}

fn place_workspace_window(window: &tauri::Window) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let _ = window.set_position(workspace_position_from_monitor(&monitor));
    }
}

fn workspace_position_from_monitor(monitor: &tauri::window::Monitor) -> LogicalPosition<f64> {
    let work_area = monitor.work_area();
    workspace_window_logical_position(
        work_area.position.x,
        work_area.position.y,
        monitor.scale_factor(),
    )
}

fn workspace_window_logical_position(
    work_area_x: i32,
    work_area_y: i32,
    scale_factor: f64,
) -> LogicalPosition<f64> {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };

    LogicalPosition::new(
        f64::from(work_area_x) / scale + WORKSPACE_WINDOW_MARGIN,
        f64::from(work_area_y) / scale + WORKSPACE_WINDOW_MARGIN,
    )
}

fn apply_workspace_titlebar_style_to_window(window: &tauri::Window) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
    }
}

fn apply_theme_to_workspace(
    app: &AppHandle,
    theme: theme::ThemeDefinition,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeSet,
    custom_style: &CustomStyleSet,
) -> Result<(), String> {
    let server_slug = app
        .state::<DesktopState>()
        .settings
        .lock()
        .map(|s| s.service.selected_server_slug.clone())
        .unwrap_or_default();
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        apply_window_language(app, &window, language, window_is_workspace(&window));
        if window_is_workspace(&window) {
            apply_window_theme(&window, theme_mode);
            let active_theme_id = theme.id.clone();
            let active_style_id = theme.style_id.clone();
            apply_workspace_scripts_to_window(
                &window,
                theme,
                &active_theme_id,
                &active_style_id,
                theme_mode,
                language,
                resolve_desktop_language(language),
                &server_slug,
                custom_theme,
                custom_style,
            )?;
        } else {
            apply_launcher_window_theme(&window, theme_mode);
        }
    }

    Ok(())
}

fn apply_window_theme(window: &tauri::WebviewWindow, theme_mode: &str) {
    apply_native_window_theme(window, theme_mode);

    let background = if effective_window_dark(window, theme_mode) {
        Color(37, 38, 35, 255)
    } else {
        Color(255, 255, 255, 255)
    };
    let _ = window.set_background_color(Some(background));
}

fn apply_launcher_window_theme(window: &tauri::WebviewWindow, theme_mode: &str) {
    apply_native_window_theme(window, theme_mode);
    let background = if effective_window_dark(window, theme_mode) {
        Color(31, 31, 28, 255)
    } else {
        Color(247, 247, 245, 255)
    };
    let _ = window.set_background_color(Some(background));
}

fn effective_window_dark(window: &tauri::WebviewWindow, theme_mode: &str) -> bool {
    let normalized_mode = theme::normalize_mode(theme_mode);
    normalized_mode == "dark"
        || (normalized_mode == "system" && matches!(window.theme(), Ok(Theme::Dark)))
}

fn apply_native_window_theme(window: &tauri::WebviewWindow, theme_mode: &str) {
    let normalized_mode = theme::normalize_mode(theme_mode);
    let native_theme = match normalized_mode {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };
    let _ = window.set_theme(native_theme);
}

fn apply_window_language(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    language: &str,
    workspace: bool,
) {
    let resolved_language = resolve_desktop_language(language);
    let title = match (resolved_language, workspace) {
        ("zh-CN", true) => "Slock 工作区",
        ("zh-CN", false) => "slock-desktop",
        (_, true) => "Slock Workspace",
        (_, false) => "slock-desktop",
    };
    let _ = window.set_title(title);
    if let Err(err) = apply_native_menu(app, resolved_language) {
        log::warn!("failed to apply localized native menu: {err}");
    }
}

struct NativeMenuCopy {
    app: &'static str,
    about: &'static str,
    services: &'static str,
    hide: &'static str,
    hide_others: &'static str,
    quit: &'static str,
    edit: &'static str,
    undo: &'static str,
    redo: &'static str,
    cut: &'static str,
    copy: &'static str,
    paste: &'static str,
    select_all: &'static str,
    view: &'static str,
    fullscreen: &'static str,
    window: &'static str,
    minimize: &'static str,
    zoom: &'static str,
    close: &'static str,
}

fn apply_native_menu(app: &AppHandle, language: &str) -> tauri::Result<()> {
    static APPLIED_MENU_LANGUAGE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    let menu_language = sanitize_language(language);
    let cache = APPLIED_MENU_LANGUAGE.get_or_init(|| Mutex::new(None));
    if cache
        .lock()
        .map(|current| current.as_deref() == Some(menu_language))
        .unwrap_or(false)
    {
        return Ok(());
    }

    let copy = native_menu_copy(menu_language);
    let app_menu = SubmenuBuilder::new(app, copy.app)
        .about_with_text(copy.about, None)
        .separator()
        .services_with_text(copy.services)
        .separator()
        .hide_with_text(copy.hide)
        .hide_others_with_text(copy.hide_others)
        .separator()
        .quit_with_text(copy.quit)
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, copy.edit)
        .undo_with_text(copy.undo)
        .redo_with_text(copy.redo)
        .separator()
        .cut_with_text(copy.cut)
        .copy_with_text(copy.copy)
        .paste_with_text(copy.paste)
        .select_all_with_text(copy.select_all)
        .build()?;
    let view_menu = SubmenuBuilder::new(app, copy.view)
        .fullscreen_with_text(copy.fullscreen)
        .build()?;
    let window_menu = SubmenuBuilder::new(app, copy.window)
        .minimize_with_text(copy.minimize)
        .maximize_with_text(copy.zoom)
        .separator()
        .close_window_with_text(copy.close)
        .build()?;
    let menu = MenuBuilder::new(app)
        .item(&app_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .build()?;

    app.set_menu(menu)?;
    if let Ok(mut current) = cache.lock() {
        *current = Some(menu_language.to_string());
    }
    Ok(())
}

fn native_menu_copy(language: &str) -> NativeMenuCopy {
    if language == "zh-CN" {
        NativeMenuCopy {
            app: "Slock",
            about: "关于 Slock",
            services: "服务",
            hide: "隐藏 Slock",
            hide_others: "隐藏其他",
            quit: "退出 Slock",
            edit: "编辑",
            undo: "撤销",
            redo: "重做",
            cut: "剪切",
            copy: "复制",
            paste: "粘贴",
            select_all: "全选",
            view: "显示",
            fullscreen: "进入全屏",
            window: "窗口",
            minimize: "最小化",
            zoom: "缩放",
            close: "关闭窗口",
        }
    } else {
        NativeMenuCopy {
            app: "Slock",
            about: "About Slock",
            services: "Services",
            hide: "Hide Slock",
            hide_others: "Hide Others",
            quit: "Quit Slock",
            edit: "Edit",
            undo: "Undo",
            redo: "Redo",
            cut: "Cut",
            copy: "Copy",
            paste: "Paste",
            select_all: "Select All",
            view: "View",
            fullscreen: "Enter Full Screen",
            window: "Window",
            minimize: "Minimize",
            zoom: "Zoom",
            close: "Close Window",
        }
    }
}

fn apply_workspace_scripts_to_window(
    window: &tauri::WebviewWindow,
    theme: theme::ThemeDefinition,
    active_theme_id: &str,
    active_style_id: &str,
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    server_slug: &str,
    custom_theme: &CustomThemeSet,
    custom_style: &CustomStyleSet,
) -> Result<(), String> {
    window
        .eval(theme::injected_script(theme))
        .map_err(|err| err.to_string())?;
    window
        .eval(workspace::settings_overlay_script(
            active_theme_id,
            active_style_id,
            active_theme_mode,
            active_language,
            resolved_language,
            &color_catalog(active_theme_mode, active_style_id, custom_theme, custom_style),
            &style_catalog(active_theme_mode, active_theme_id, custom_theme, custom_style),
        ))
        .map_err(|err| err.to_string())?;
    window
        .eval(agent_env_import::agent_env_import_script(resolved_language))
        .map_err(|err| err.to_string())?;
    window
        .eval(agent_card_inject::agent_card_inject_script(server_slug, resolved_language))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn apply_workspace_session_seed_to_window(
    window: &tauri::WebviewWindow,
    state: &DesktopState,
) -> Result<(), String> {
    let Some(seed) = current_workspace_session_seed(state)? else {
        return Ok(());
    };
    window
        .eval(workspace_session_seed_script(&seed))
        .map_err(|err| err.to_string())
}

fn apply_workspace_session_seed_to_webview(
    webview: &tauri::Webview,
    state: &DesktopState,
) -> Result<(), String> {
    let Some(seed) = current_workspace_session_seed(state)? else {
        return Ok(());
    };
    webview
        .eval(workspace_session_seed_script(&seed))
        .map_err(|err| err.to_string())
}

fn apply_workspace_scripts_to_webview(
    webview: &tauri::Webview,
    theme: theme::ThemeDefinition,
    active_theme_id: &str,
    active_style_id: &str,
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    server_slug: &str,
    custom_theme: &CustomThemeSet,
    custom_style: &CustomStyleSet,
) -> Result<(), String> {
    webview
        .eval(theme::injected_script(theme))
        .map_err(|err| err.to_string())?;
    webview
        .eval(workspace::settings_overlay_script(
            active_theme_id,
            active_style_id,
            active_theme_mode,
            active_language,
            resolved_language,
            &color_catalog(active_theme_mode, active_style_id, custom_theme, custom_style),
            &style_catalog(active_theme_mode, active_theme_id, custom_theme, custom_style),
        ))
        .map_err(|err| err.to_string())?;
    webview
        .eval(agent_env_import::agent_env_import_script(resolved_language))
        .map_err(|err| err.to_string())?;
    webview
        .eval(agent_card_inject::agent_card_inject_script(server_slug, resolved_language))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn main_window_is_workspace(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN_LABEL)
        .map(|window| window_is_workspace(&window))
        .unwrap_or(false)
}

fn window_is_workspace(window: &tauri::WebviewWindow) -> bool {
    window
        .url()
        .map(|url| is_workspace_url(&url))
        .unwrap_or(false)
}

fn is_workspace_url(url: &Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some("app.slock.ai")
}

fn workspace_url_for_slug(server_slug: &str) -> String {
    let slug = server_slug.trim();
    if slug.is_empty() {
        WORKSPACE_URL.to_string()
    } else {
        format!("{WORKSPACE_URL}/s/{slug}")
    }
}

fn begin_workspace_launch_trace(state: &DesktopState, command_started: Instant, target_url: &str) {
    let Ok(mut metrics) = state.launch_metrics.lock() else {
        return;
    };
    metrics.next_id = metrics.next_id.saturating_add(1);
    let id = metrics.next_id;
    let trace = WorkspaceLaunchTrace {
        id,
        target_url: target_url.to_string(),
        command_started,
        navigate_called: None,
        page_started: None,
    };
    log_workspace_launch_step(
        &trace,
        "command_received",
        command_started,
        Some(target_url),
    );
    metrics.active = Some(trace);
}

fn mark_workspace_launch_navigate_called(state: &DesktopState, target_url: &str) {
    let now = Instant::now();
    let Ok(mut metrics) = state.launch_metrics.lock() else {
        return;
    };
    let Some(trace) = metrics.active.as_mut() else {
        return;
    };

    trace.navigate_called = Some(now);
    log_workspace_launch_step(trace, "navigate_called", now, Some(target_url));
}

fn mark_workspace_launch_page_started(state: &DesktopState, url: &Url) {
    let now = Instant::now();
    let Ok(mut metrics) = state.launch_metrics.lock() else {
        return;
    };
    let Some(trace) = metrics.active.as_mut() else {
        return;
    };
    if trace.page_started.is_some() || !workspace_launch_url_is_relevant(&trace.target_url, url) {
        return;
    }

    trace.page_started = Some(now);
    log_workspace_launch_step(trace, "page_started", now, Some(url.as_str()));
}

fn mark_workspace_launch_page_finished(state: &DesktopState, url: &Url) {
    let now = Instant::now();
    let Ok(mut metrics) = state.launch_metrics.lock() else {
        return;
    };
    let mut finished = false;
    if let Some(trace) = metrics.active.as_mut() {
        if workspace_launch_url_is_relevant(&trace.target_url, url) {
            log_workspace_launch_step(trace, "page_finished", now, Some(url.as_str()));
            finished = true;
        }
    }
    if finished {
        metrics.active = None;
    }
}

fn workspace_launch_url_is_relevant(target_url: &str, url: &Url) -> bool {
    if !is_workspace_url(url) {
        return false;
    }

    let Ok(target) = target_url.parse::<Url>() else {
        return true;
    };
    let target_path = target.path().trim_end_matches('/');
    if target_path.is_empty() || target_path == "/" {
        return true;
    }

    let url_path = url.path();
    url_path == target_path
        || url_path
            .strip_prefix(target_path)
            .map(|suffix| suffix.starts_with('/'))
            .unwrap_or(false)
}

fn log_workspace_launch_step(
    trace: &WorkspaceLaunchTrace,
    step: &str,
    at: Instant,
    url: Option<&str>,
) {
    let total_ms = at.duration_since(trace.command_started).as_millis();
    let since_navigate_ms = trace
        .navigate_called
        .map(|started| at.duration_since(started).as_millis());
    let since_page_start_ms = trace
        .page_started
        .map(|started| at.duration_since(started).as_millis());
    let url = url.unwrap_or(trace.target_url.as_str());
    let message = format!(
        "[slock-launch:{}] step={} total={}ms since_navigate={} since_page_start={} url={}",
        trace.id,
        step,
        total_ms,
        format_duration_ms(since_navigate_ms),
        format_duration_ms(since_page_start_ms),
        url
    );
    eprintln!("{message}");
    log::info!("{message}");
    #[cfg(debug_assertions)]
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(WORKSPACE_LAUNCH_LOG_PATH)
    {
        let _ = writeln!(file, "{message}");
    }
}

fn format_duration_ms(value: Option<u128>) -> String {
    value
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "-".to_string())
}

fn collect_service_snapshot(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    refresh_service: bool,
    detect_service_process: bool,
) -> Result<ServiceSnapshot, String> {
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;

    let mut running = false;
    let mut pid = None;

    if let Some(child) = runtime.child.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                runtime.last_error = Some(format!("Service exited with status {status}"));
                runtime.child = None;
                runtime.active_pid = None;
            }
            Ok(None) => {
                running = true;
                pid = Some(child.id());
                runtime.active_pid = pid;
            }
            Err(err) => {
                runtime.last_error = Some(format!("Service state check failed: {err}"));
                runtime.child = None;
                runtime.active_pid = None;
            }
        }
    } else if let Some(active_pid) = runtime.active_pid {
        match process_is_alive(active_pid) {
            Ok(true) => {
                running = true;
                pid = Some(active_pid);
            }
            Ok(false) => {
                runtime.active_pid = None;
            }
            Err(err) => {
                runtime.last_error = Some(format!("Service state check failed: {err}"));
                runtime.active_pid = None;
            }
        }
    }

    let last_error = runtime.last_error.clone();
    let mut active_server_slug = runtime.active_server_slug.clone().unwrap_or_default();
    let cached_servers = runtime.cached_servers.clone();
    let cached_sync_error = runtime.cached_sync_error.clone();
    drop(runtime);

    let authenticated = current_session_tokens(state)?.is_some();
    if !authenticated {
        let mut runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        runtime.cached_servers.clear();
        runtime.cached_sync_error = None;

        return Ok(ServiceSnapshot {
            server_url: settings.server_url.clone(),
            selected_server_slug: settings.selected_server_slug.clone(),
            active_server_slug,
            auto_start_with_workspace: settings.auto_start_with_workspace,
            close_app_behavior: close_app_behavior_id(close_app_behavior_from_id(
                &settings.close_app_behavior,
            ))
            .to_string(),
            authenticated,
            account: None,
            accounts: current_saved_session_accounts(state)?,
            configured: false,
            running,
            pid,
            last_error,
            sync_error: None,
            servers: Vec::new(),
        });
    }

    let refresh_needed = should_refresh_service_servers(refresh_service, cached_servers.is_empty());
    let (mut servers, sync_error) = if refresh_needed {
        match fetch_service_servers(app, state, settings) {
            Ok(servers) => {
                let mut runtime = state
                    .service
                    .lock()
                    .map_err(|_| "Unable to lock service runtime".to_string())?;
                runtime.cached_servers = servers.clone();
                runtime.cached_sync_error = None;
                (servers, None)
            }
            Err(err) => {
                let mut runtime = state
                    .service
                    .lock()
                    .map_err(|_| "Unable to lock service runtime".to_string())?;
                runtime.cached_sync_error = Some(err.clone());
                (cached_servers, Some(err))
            }
        }
    } else {
        (cached_servers, cached_sync_error)
    };
    let account = current_session_account(state)?;
    let accounts = current_saved_session_accounts(state)?;

    for server in &mut servers {
        server.selected = server.slug == settings.selected_server_slug;
    }

    if should_detect_selected_service_process(
        detect_service_process,
        &settings.selected_server_slug,
        &active_server_slug,
        running,
    ) {
        if let Ok(Some(process)) = selected_service_daemon_process_from_servers(settings, &servers)
        {
            running = true;
            pid = Some(process.pid);
            active_server_slug = process.server_slug.clone();
            let _ = mark_service_daemon_process_running(state, &process);
        }
    }

    let configured = servers
        .iter()
        .find(|server| server.selected)
        .map(|server| server.api_key_ready || machine_counts_as_started(&server.machine_status))
        .unwrap_or(false);

    Ok(ServiceSnapshot {
        server_url: settings.server_url.clone(),
        selected_server_slug: settings.selected_server_slug.clone(),
        active_server_slug,
        auto_start_with_workspace: settings.auto_start_with_workspace,
        close_app_behavior: close_app_behavior_id(close_app_behavior_from_id(
            &settings.close_app_behavior,
        ))
        .to_string(),
        authenticated,
        account,
        accounts,
        configured,
        running,
        pid,
        last_error,
        sync_error,
        servers,
    })
}

fn should_refresh_service_servers(refresh_service: bool, _cached_servers_empty: bool) -> bool {
    refresh_service
}

fn should_detect_selected_service_process(
    detect_service_process: bool,
    selected_server_slug: &str,
    active_server_slug: &str,
    running: bool,
) -> bool {
    let selected_server_slug = selected_server_slug.trim();
    detect_service_process
        && !selected_server_slug.is_empty()
        && (!running || active_server_slug.trim() != selected_server_slug)
}

fn selected_service_has_local_binding(settings: &ServiceSettings) -> bool {
    let selected_slug = settings.selected_server_slug.trim();
    !selected_slug.is_empty()
        && settings.machines.iter().any(|binding| {
            binding.server_slug == selected_slug && !binding.machine_id.trim().is_empty()
        })
}

fn selected_service_running_on_current_computer(
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<bool, String> {
    let selected_slug = settings.selected_server_slug.trim();
    if selected_slug.is_empty() {
        return Ok(false);
    }

    let (cached_servers, runtime_active_machine_id) = {
        let mut runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;

        if runtime.active_server_slug.as_deref() == Some(selected_slug) {
            if let Some(child) = runtime.child.as_mut() {
                let still_running = child
                    .try_wait()
                    .map_err(|err| format!("Unable to inspect service state: {err}"))?
                    .is_none();
                if still_running {
                    return Ok(true);
                }
                runtime.child = None;
                runtime.active_pid = None;
            } else if let Some(active_pid) = runtime.active_pid {
                if process_is_alive(active_pid)? {
                    return Ok(true);
                }
                runtime.active_pid = None;
            }
        }

        (
            runtime.cached_servers.clone(),
            runtime.active_machine_id.clone(),
        )
    };

    let Some(process) = selected_service_daemon_process_from_cached_state(
        settings,
        &cached_servers,
        runtime_active_machine_id.as_deref(),
    )?
    else {
        return Ok(false);
    };

    mark_service_daemon_process_running(state, &process)?;
    Ok(true)
}

fn selected_service_daemon_process_from_cached_state(
    settings: &ServiceSettings,
    cached_servers: &[ServiceServerSnapshot],
    runtime_active_machine_id: Option<&str>,
) -> Result<Option<ServiceDaemonProcess>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(selected_service_daemon_process_from_cached_output(
            settings,
            cached_servers,
            runtime_active_machine_id,
            &listing,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = settings;
        let _ = cached_servers;
        let _ = runtime_active_machine_id;
        Ok(None)
    }
}

fn selected_service_daemon_process_from_cached_output(
    settings: &ServiceSettings,
    cached_servers: &[ServiceServerSnapshot],
    runtime_active_machine_id: Option<&str>,
    output: &str,
) -> Option<ServiceDaemonProcess> {
    let selected_slug = settings.selected_server_slug.trim();
    if selected_slug.is_empty() {
        return None;
    }

    let cached_server = cached_servers
        .iter()
        .find(|server| server.slug == selected_slug);
    let binding = find_service_binding(settings, "", selected_slug);
    let machine_id = binding
        .as_ref()
        .map(|binding| binding.machine_id.as_str())
        .or_else(|| cached_server.and_then(|server| server.machine_id.as_deref()))
        .or(runtime_active_machine_id)
        .filter(|machine_id| !machine_id.trim().is_empty());
    let api_key_prefix = cached_server
        .and_then(|server| server.api_key_prefix.as_deref())
        .filter(|prefix| !prefix.trim().is_empty());
    let legacy_api_key = binding
        .as_ref()
        .map(|binding| binding.api_key.as_str())
        .filter(|api_key| !api_key.trim().is_empty());

    service_daemon_process_from_target(
        settings,
        selected_slug,
        machine_id,
        api_key_prefix,
        legacy_api_key,
        output,
    )
}

fn cached_service_start_target(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<Option<ServiceStartTarget>, String> {
    let selected_slug = settings.selected_server_slug.trim();
    if selected_slug.is_empty() {
        return Ok(None);
    }

    let Some(binding) = find_service_binding(settings, "", selected_slug) else {
        return Ok(None);
    };
    if binding.server_id.trim().is_empty() || binding.machine_id.trim().is_empty() {
        return Ok(None);
    }

    let api_key = rotate_machine_api_key(
        app,
        state,
        &settings.server_url,
        &binding.server_id,
        &binding.machine_id,
    )?;
    let machine_status = state
        .service
        .lock()
        .ok()
        .and_then(|runtime| {
            runtime
                .cached_servers
                .iter()
                .find(|server| server.slug == selected_slug || server.id == binding.server_id)
                .map(|server| server.machine_status.clone())
        })
        .unwrap_or_else(|| "offline".to_string());

    Ok(Some(ServiceStartTarget {
        binding,
        api_key,
        api_key_prefix: None,
        machine_status,
    }))
}

fn force_start_service(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<(), String> {
    let cached_target = match cached_service_start_target(app, state, settings) {
        Ok(target) => target,
        Err(err) => {
            log::warn!(
                "cached service start target failed, falling back to full resolution: {err}"
            );
            None
        }
    };
    if let Some(target) = cached_target {
        if prepare_runtime_for_service_target(
            state,
            &target.binding.server_slug,
            Some(target.binding.machine_id.as_str()),
            true,
        )? {
            return Ok(());
        }

        if let Some(process) = service_daemon_process_for_start_target(settings, &target)? {
            adopt_service_daemon_process(state, &process)?;
            return Ok(());
        }

        let api_key = target.api_key.trim();
        if !api_key.is_empty() {
            return spawn_service_daemon(app, state, settings, target);
        }
    }

    let selected_server = resolve_selected_server(app, state, settings)?;

    if let Some(target) = resolve_existing_service_machine(app, state, settings, &selected_server)?
    {
        if let Some(process) = service_daemon_process_for_resolved_target(settings, &target)? {
            adopt_service_daemon_process(state, &process)?;
            return Ok(());
        }

        if machine_counts_as_started(&target.machine_status) {
            if let Some(process) = unique_untagged_service_daemon_process(
                &settings.server_url,
                &selected_server.slug,
                Some(target.binding.machine_id.as_str()),
            )? {
                adopt_service_daemon_process(state, &process)?;
                return Ok(());
            }
        }
    }

    let target = ensure_machine_binding(app, state, settings, &selected_server)?;
    let binding = target.binding.clone();

    if prepare_runtime_for_service_target(
        state,
        &selected_server.slug,
        Some(binding.machine_id.as_str()),
        true,
    )? {
        return Ok(());
    }

    if let Some(process) = service_daemon_process_for_start_target(settings, &target)? {
        adopt_service_daemon_process(state, &process)?;
        return Ok(());
    }

    if machine_counts_as_started(&target.machine_status) {
        if let Some(process) = unique_untagged_service_daemon_process(
            &settings.server_url,
            &selected_server.slug,
            Some(binding.machine_id.as_str()),
        )? {
            adopt_service_daemon_process(state, &process)?;
            return Ok(());
        }
    }

    let api_key = target.api_key.trim();
    if api_key.is_empty() {
        return Err("Selected server did not return a daemon API key.".to_string());
    }

    spawn_service_daemon(app, state, settings, target)
}

fn spawn_service_daemon(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    target: ServiceStartTarget,
) -> Result<(), String> {
    let binding = target.binding;
    let api_key = target.api_key.trim();
    if api_key.is_empty() {
        return Err("Selected server did not return a daemon API key.".to_string());
    }

    let service_command = resolve_service_command()?;
    let service_path_env = prepare_service_path_env(app, &service_command.path_env)?;
    let mut log_file = open_service_log_file(app, &binding.server_slug)?;
    writeln!(
        &mut log_file,
        "\n[slock-desktop ts={} stream=desktop] starting daemon for {}",
        current_epoch_ms(),
        binding.server_slug,
    )
    .map_err(|err| format!("Unable to write service log header: {err}"))?;
    let log_file_for_stdout = log_file
        .try_clone()
        .map_err(|err| format!("Unable to prepare service log: {err}"))?;
    let log_file_for_stderr = log_file
        .try_clone()
        .map_err(|err| format!("Unable to prepare service log: {err}"))?;
    let mut command = Command::new(&service_command.executable);
    command
        .args([
            "--yes",
            DAEMON_PACKAGE,
            "--server-url",
            settings.server_url.as_str(),
            "--api-key",
            api_key,
            DAEMON_SERVER_SLUG_ARG,
            binding.server_slug.as_str(),
            DAEMON_MACHINE_ID_ARG,
            binding.machine_id.as_str(),
            DAEMON_DESKTOP_MANAGED_ARG,
        ])
        .env("PATH", &service_path_env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|err| {
        format!(
            "Failed to start service with {}: {err}",
            service_command.executable.display()
        )
    })?;
    if let Some(stdout) = child.stdout.take() {
        pipe_service_output_to_log(stdout, log_file_for_stdout, "stdout");
    }
    if let Some(stderr) = child.stderr.take() {
        pipe_service_output_to_log(stderr, log_file_for_stderr, "stderr");
    }
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.last_error = None;
    runtime.active_server_slug = Some(binding.server_slug);
    runtime.active_machine_id = Some(binding.machine_id);
    runtime.active_pid = Some(child.id());
    runtime.child = Some(child);
    Ok(())
}

fn open_service_log_file(app: &AppHandle, server_slug: &str) -> Result<fs::File, String> {
    let log_path = service_log_path(app, server_slug)?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|err| format!("Unable to open service log: {err}"))
}

fn pipe_service_output_to_log<R>(reader: R, mut log_file: fs::File, stream: &'static str)
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            let read = match reader.read_until(b'\n', &mut bytes) {
                Ok(read) => read,
                Err(err) => {
                    let _ = writeln!(
                        log_file,
                        "[slock-desktop ts={} stream=desktop] failed to read {stream}: {err}",
                        current_epoch_ms()
                    );
                    break;
                }
            };
            if read == 0 {
                break;
            }

            while matches!(bytes.last(), Some(b'\n' | b'\r')) {
                bytes.pop();
            }
            let line = String::from_utf8_lossy(&bytes);
            let _ = writeln!(
                log_file,
                "[slock-desktop ts={} stream={stream}] {line}",
                current_epoch_ms()
            );
            let _ = log_file.flush();
        }
    });
}

fn service_log_path(app: &AppHandle, server_slug: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| err.to_string())?
        .join("server-logs");
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Unable to create server log directory: {err}"))?;
    Ok(dir.join(format!("{}.log", safe_log_slug(server_slug))))
}

fn read_service_log_range(
    server_slug: &str,
    log_path: &Path,
    from_epoch_ms: i64,
    to_epoch_ms: i64,
) -> Result<ServiceLogSnapshot, String> {
    let file =
        fs::File::open(log_path).map_err(|err| format!("Unable to read service log: {err}"))?;
    let total_bytes = file
        .metadata()
        .map_err(|err| format!("Unable to inspect service log: {err}"))?
        .len();
    let mut reader = BufReader::new(file);
    let mut raw_line = Vec::new();
    let mut content = String::new();
    let mut active_timestamp = None;
    let mut saw_timestamp = false;
    let mut truncated = false;

    loop {
        raw_line.clear();
        let read = reader
            .read_until(b'\n', &mut raw_line)
            .map_err(|err| format!("Unable to read service log: {err}"))?;
        if read == 0 {
            break;
        }

        let line = String::from_utf8_lossy(&raw_line);
        if let Some(timestamp) = parse_log_line_epoch_ms(&line) {
            active_timestamp = Some(timestamp);
            saw_timestamp = true;
        }

        let include = active_timestamp
            .map(|timestamp| timestamp >= from_epoch_ms && timestamp <= to_epoch_ms)
            .unwrap_or(false);
        if include {
            content.push_str(&line);
            while content.len() as u64 > SERVICE_LOG_MAX_BYTES {
                truncated = true;
                if let Some(next_line) = content.find('\n') {
                    content.drain(..=next_line);
                } else {
                    content.clear();
                    break;
                }
            }
        }
    }

    if !saw_timestamp {
        return read_service_log_tail(server_slug, log_path, from_epoch_ms, to_epoch_ms);
    }

    Ok(ServiceLogSnapshot {
        server_slug: server_slug.to_string(),
        path: log_path.to_string_lossy().to_string(),
        content,
        truncated,
        total_bytes,
        from_epoch_ms,
        to_epoch_ms,
        timestamp_filtered: true,
    })
}

fn read_service_log_tail(
    server_slug: &str,
    log_path: &Path,
    from_epoch_ms: i64,
    to_epoch_ms: i64,
) -> Result<ServiceLogSnapshot, String> {
    let mut file =
        fs::File::open(log_path).map_err(|err| format!("Unable to read service log: {err}"))?;
    let total_bytes = file
        .metadata()
        .map_err(|err| format!("Unable to inspect service log: {err}"))?
        .len();
    let offset = total_bytes.saturating_sub(SERVICE_LOG_MAX_BYTES);
    if offset > 0 {
        file.seek(SeekFrom::Start(offset))
            .map_err(|err| format!("Unable to read service log: {err}"))?;
    }

    let mut bytes = Vec::with_capacity((total_bytes - offset) as usize);
    file.read_to_end(&mut bytes)
        .map_err(|err| format!("Unable to read service log: {err}"))?;
    let mut content = String::from_utf8_lossy(&bytes).into_owned();
    if offset > 0 {
        if let Some(line_start) = content.find('\n') {
            content = content[line_start + 1..].to_string();
        }
    }

    Ok(ServiceLogSnapshot {
        server_slug: server_slug.to_string(),
        path: log_path.to_string_lossy().to_string(),
        content,
        truncated: offset > 0,
        total_bytes,
        from_epoch_ms,
        to_epoch_ms,
        timestamp_filtered: false,
    })
}

fn current_epoch_ms() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().min(i64::MAX as u128) as i64,
        Err(_) => 0,
    }
}

fn parse_log_line_epoch_ms(line: &str) -> Option<i64> {
    parse_log_marker_epoch_ms(line)
        .or_else(|| parse_system_time_debug_epoch_ms(line))
        .or_else(|| parse_common_datetime_epoch_ms(line))
}

fn parse_log_marker_epoch_ms(line: &str) -> Option<i64> {
    for marker in ["ts=", "timestamp="] {
        let Some(start) = line.find(marker).map(|index| index + marker.len()) else {
            continue;
        };
        let digits: String = line[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if digits.len() >= 10 {
            return digits.parse::<i64>().ok();
        }
    }
    None
}

fn parse_system_time_debug_epoch_ms(line: &str) -> Option<i64> {
    let seconds = parse_i64_after(line, "tv_sec:")?;
    let nanos = parse_i64_after(line, "tv_nsec:").unwrap_or(0);
    Some(seconds.saturating_mul(1000).saturating_add(nanos / 1_000_000))
}

fn parse_i64_after(line: &str, marker: &str) -> Option<i64> {
    let start = line.find(marker)? + marker.len();
    let digits: String = line[start..]
        .chars()
        .skip_while(|ch| ch.is_ascii_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse::<i64>().ok()
}

fn parse_common_datetime_epoch_ms(line: &str) -> Option<i64> {
    let trimmed = line
        .trim_start()
        .trim_start_matches('[')
        .trim_start_matches('(');
    for len in [35usize, 30, 29, 25, 24, 23, 20, 19] {
        if trimmed.len() < len {
            continue;
        }
        let candidate = trimmed[..len]
            .trim_end_matches(|ch: char| matches!(ch, ']' | ')' | ',' | ' '))
            .trim();
        if let Ok(parsed) = DateTime::parse_from_rfc3339(candidate) {
            return Some(parsed.timestamp_millis());
        }
        for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
            if let Ok(naive) = NaiveDateTime::parse_from_str(candidate, format) {
                if let Some(local) = Local.from_local_datetime(&naive).earliest() {
                    return Some(local.timestamp_millis());
                }
            }
        }
    }
    None
}

fn safe_log_slug(value: &str) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "server".to_string()
    } else {
        sanitized
    }
}

fn resolve_service_command() -> Result<ServiceCommand, String> {
    let mut search_dirs = service_command_search_dirs();
    if let Some(command) = resolve_service_command_from_dirs(search_dirs.clone()) {
        return Ok(command);
    }

    append_login_shell_path_dirs(&mut search_dirs);
    resolve_service_command_from_dirs(search_dirs).ok_or_else(|| {
        "Unable to find npx and Node.js. Install Node.js/npm, then restart Slock Desktop."
            .to_string()
    })
}

fn prepare_service_path_env(app: &AppHandle, path_env: &str) -> Result<String, String> {
    #[cfg(unix)]
    {
        let wrapper_dir = install_runtime_command_wrappers(app)?;
        return Ok(prepend_path_env_dir(&wrapper_dir, path_env));
    }

    #[cfg(not(unix))]
    {
        let _ = app;
        Ok(path_env.to_string())
    }
}

#[cfg(unix)]
fn install_runtime_command_wrappers(app: &AppHandle) -> Result<PathBuf, String> {
    let wrapper_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| err.to_string())?
        .join(RUNTIME_WRAPPER_DIR);
    fs::create_dir_all(&wrapper_dir)
        .map_err(|err| format!("Unable to create runtime wrapper directory: {err}"))?;

    let claude_wrapper_path = wrapper_dir.join(CLAUDE_WRAPPER_NAME);
    fs::write(&claude_wrapper_path, claude_wrapper_script())
        .map_err(|err| format!("Unable to write Claude runtime wrapper: {err}"))?;
    let mut permissions = fs::metadata(&claude_wrapper_path)
        .map_err(|err| format!("Unable to inspect Claude runtime wrapper: {err}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&claude_wrapper_path, permissions)
        .map_err(|err| format!("Unable to prepare Claude runtime wrapper: {err}"))?;

    Ok(wrapper_dir)
}

#[cfg(unix)]
fn prepend_path_env_dir(dir: &Path, path_env: &str) -> String {
    let mut path_dirs = vec![dir.to_path_buf()];
    push_path_env_dirs(&mut path_dirs, std::ffi::OsStr::new(path_env));
    env::join_paths(&path_dirs)
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            path_dirs
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(":")
        })
}

#[cfg(unix)]
fn claude_wrapper_script() -> &'static str {
    r#"#!/usr/bin/env node
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const wrapperPath = safeRealpath(__filename);
const wrapperDir = safeRealpath(path.dirname(__filename));
const rawPath = process.env.PATH || "";
const searchDirs = rawPath
  .split(path.delimiter)
  .filter(Boolean)
  .filter((dir) => safeRealpath(dir) !== wrapperDir);

function safeRealpath(value) {
  try {
    return fs.realpathSync(value);
  } catch {
    return path.resolve(value);
  }
}

function isExecutable(filePath) {
  try {
    const stat = fs.statSync(filePath);
    if (!stat.isFile()) return false;
    if (process.platform === "win32") return true;
    return (stat.mode & 0o111) !== 0;
  } catch {
    return false;
  }
}

function findRealClaude() {
  for (const dir of searchDirs) {
    const candidate = path.join(dir, "claude");
    if (safeRealpath(candidate) !== wrapperPath && isExecutable(candidate)) {
      return candidate;
    }
  }

  if (process.platform === "darwin") {
    const home = process.env.HOME || "";
    const fallbacks = [
      path.join(home, "Applications", "Claude Code URL Handler.app", "Contents", "MacOS", "claude"),
      "/Applications/Claude Code URL Handler.app/Contents/MacOS/claude",
    ];
    for (const candidate of fallbacks) {
      if (safeRealpath(candidate) !== wrapperPath && isExecutable(candidate)) {
        return candidate;
      }
    }
  }

  return null;
}

function overrideModelArgs(args, model) {
  const cleaned = (model || "").trim();
  if (!cleaned) return args;

  const next = [];
  let replaced = false;
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--model") {
      next.push(arg, cleaned);
      if (index + 1 < args.length) index += 1;
      replaced = true;
      continue;
    }
    if (arg.startsWith("--model=")) {
      next.push(`--model=${cleaned}`);
      replaced = true;
      continue;
    }
    next.push(arg);
  }
  if (!replaced) {
    next.push("--model", cleaned);
  }
  return next;
}

const command = findRealClaude();
if (!command) {
  console.error("Slock Claude wrapper: unable to find the real claude command.");
  process.exit(127);
}

const requestedModel =
  process.env.SLOCK_CLAUDE_MODEL ||
  process.env.CLAUDE_MODEL ||
  process.env.ANTHROPIC_MODEL ||
  "";
const args = overrideModelArgs(process.argv.slice(2), requestedModel);
const env = { ...process.env, PATH: searchDirs.join(path.delimiter) };
const result = spawnSync(command, args, { stdio: "inherit", env });

if (result.error) {
  console.error(result.error.message);
  process.exit(126);
}
process.exit(typeof result.status === "number" ? result.status : 1);
"#
}

fn resolve_service_command_from_dirs(search_dirs: Vec<PathBuf>) -> Option<ServiceCommand> {
    let executable = find_executable_in_dirs("npx", &search_dirs)?;
    let mut path_dirs = Vec::new();
    if let Some(parent) = executable.parent() {
        push_unique_path(&mut path_dirs, parent.to_path_buf());
    }
    for dir in search_dirs {
        push_unique_path(&mut path_dirs, dir);
    }

    find_executable_in_dirs("node", &path_dirs)?;
    let path_env = env::join_paths(&path_dirs)
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            path_dirs
                .iter()
                .map(|dir| dir.to_string_lossy())
                .collect::<Vec<_>>()
                .join(":")
        });

    Some(ServiceCommand {
        executable,
        path_env,
    })
}

fn service_command_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(path_env) = env::var_os("PATH") {
        push_path_env_dirs(&mut dirs, &path_env);
    }

    append_node_install_dirs(&mut dirs);

    for dir in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/opt/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        push_unique_path(&mut dirs, PathBuf::from(dir));
    }

    dirs
}

fn append_login_shell_path_dirs(dirs: &mut Vec<PathBuf>) {
    let shell = env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_string());
    let Ok(output) = Command::new(shell)
        .args(["-lc", "printf '%s' \"$PATH\""])
        .output()
    else {
        return;
    };
    if output.status.success() {
        let path_env =
            std::ffi::OsString::from(String::from_utf8_lossy(&output.stdout).into_owned());
        push_path_env_dirs(dirs, &path_env);
    }
}

fn append_node_install_dirs(dirs: &mut Vec<PathBuf>) {
    append_homebrew_node_dirs(dirs, Path::new("/opt/homebrew/opt"));
    append_homebrew_node_dirs(dirs, Path::new("/usr/local/opt"));

    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return;
    };

    push_existing_dir(dirs, home.join(".volta/bin"));
    push_existing_dir(dirs, home.join(".asdf/shims"));
    append_versioned_node_bins(dirs, &home.join(".nvm/versions/node"), &["bin"]);
    append_versioned_node_bins(
        dirs,
        &home.join(".fnm/node-versions"),
        &["installation", "bin"],
    );
    append_versioned_node_bins(
        dirs,
        &home.join(".local/share/mise/installs/node"),
        &["bin"],
    );
}

fn append_homebrew_node_dirs(dirs: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut node_bins = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "node" || name.starts_with("node@") {
                Some(entry.path().join("bin"))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    node_bins.sort();
    node_bins.reverse();
    for dir in node_bins {
        push_existing_dir(dirs, dir);
    }
}

fn append_versioned_node_bins(dirs: &mut Vec<PathBuf>, root: &Path, suffix: &[&str]) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut bins = entries
        .flatten()
        .map(|entry| {
            suffix
                .iter()
                .fold(entry.path(), |path, segment| path.join(segment))
        })
        .collect::<Vec<_>>();
    bins.sort();
    bins.reverse();
    for dir in bins {
        push_existing_dir(dirs, dir);
    }
}

fn find_executable_in_dirs(command_name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(command_name))
        .find(|path| executable_exists(path))
}

fn executable_exists(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn push_path_env_dirs(dirs: &mut Vec<PathBuf>, path_env: &std::ffi::OsStr) {
    for dir in env::split_paths(path_env) {
        push_unique_path(dirs, dir);
    }
}

fn push_existing_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if dir.is_dir() {
        push_unique_path(dirs, dir);
    }
}

fn push_unique_path(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dirs.iter().any(|existing| existing == &dir) {
        dirs.push(dir);
    }
}

fn prepare_runtime_for_service_target(
    state: &DesktopState,
    server_slug: &str,
    machine_id: Option<&str>,
    keep_matching_child: bool,
) -> Result<bool, String> {
    let target_machine_id = machine_id
        .map(str::trim)
        .filter(|machine_id| !machine_id.is_empty());
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    let same_target = runtime.active_server_slug.as_deref() == Some(server_slug)
        && runtime.active_machine_id.as_deref() == target_machine_id;
    let mut matching_child_running = false;
    let mut clear_child = false;

    if let Some(child) = runtime.child.as_mut() {
        let still_running = child
            .try_wait()
            .map_err(|err| format!("Unable to inspect service state: {err}"))?
            .is_none();

        if still_running && same_target {
            matching_child_running = true;
            clear_child = !keep_matching_child;
        } else {
            // A running child for another server remains alive so multiple server daemons can coexist.
            clear_child = true;
        }
    }

    if clear_child {
        runtime.child = None;
    }

    if matching_child_running {
        runtime.last_error = None;
        runtime.active_server_slug = Some(server_slug.to_string());
        runtime.active_machine_id = target_machine_id.map(str::to_string);
        runtime.active_pid = runtime.child.as_ref().map(|child| child.id());
    }

    Ok(matching_child_running)
}

fn adopt_service_daemon_process(
    state: &DesktopState,
    process: &ServiceDaemonProcess,
) -> Result<(), String> {
    prepare_runtime_for_service_target(
        state,
        &process.server_slug,
        process.machine_id.as_deref(),
        false,
    )?;

    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.last_error = None;
    runtime.active_server_slug = Some(process.server_slug.clone());
    runtime.active_machine_id = process.machine_id.clone();
    runtime.active_pid = Some(process.pid);
    runtime.child = None;
    Ok(())
}

fn service_daemon_process_for_resolved_target(
    settings: &ServiceSettings,
    target: &ResolvedServiceMachine,
) -> Result<Option<ServiceDaemonProcess>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(service_daemon_process_from_resolved_target(
            settings, target, &listing,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = settings;
        let _ = target;
        Ok(None)
    }
}

fn service_daemon_process_for_start_target(
    settings: &ServiceSettings,
    target: &ServiceStartTarget,
) -> Result<Option<ServiceDaemonProcess>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(service_daemon_process_from_start_target(
            settings, target, &listing,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = settings;
        let _ = target;
        Ok(None)
    }
}

fn service_daemon_process_from_resolved_target(
    settings: &ServiceSettings,
    target: &ResolvedServiceMachine,
    output: &str,
) -> Option<ServiceDaemonProcess> {
    service_daemon_process_from_target(
        settings,
        &target.binding.server_slug,
        Some(target.binding.machine_id.as_str()),
        target.api_key_prefix.as_deref(),
        Some(target.binding.api_key.as_str()).filter(|api_key| !api_key.trim().is_empty()),
        output,
    )
}

fn service_daemon_process_from_start_target(
    settings: &ServiceSettings,
    target: &ServiceStartTarget,
    output: &str,
) -> Option<ServiceDaemonProcess> {
    service_daemon_process_from_target(
        settings,
        &target.binding.server_slug,
        Some(target.binding.machine_id.as_str()),
        target.api_key_prefix.as_deref(),
        Some(target.api_key.as_str()).filter(|api_key| !api_key.trim().is_empty()),
        output,
    )
}

fn unique_untagged_service_daemon_process(
    target_server_url: &str,
    server_slug: &str,
    machine_id: Option<&str>,
) -> Result<Option<ServiceDaemonProcess>, String> {
    Ok(unique_untagged_daemon_process_ids(target_server_url)?
        .into_iter()
        .next()
        .map(|pid| ServiceDaemonProcess {
            pid,
            server_slug: server_slug.to_string(),
            machine_id: machine_id
                .map(str::trim)
                .filter(|machine_id| !machine_id.is_empty())
                .map(str::to_string),
        }))
}

fn mark_service_daemon_process_running(
    state: &DesktopState,
    process: &ServiceDaemonProcess,
) -> Result<(), String> {
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.last_error = None;
    runtime.active_server_slug = Some(process.server_slug.clone());
    runtime.active_machine_id = process.machine_id.clone();
    runtime.active_pid = Some(process.pid);
    runtime.child = None;
    Ok(())
}

fn stop_service_process(
    _app: &AppHandle,
    state: &DesktopState,
    service_settings: Option<&ServiceSettings>,
    target_server_slug: Option<&str>,
) -> Result<(), String> {
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    let active_server_slug = runtime.active_server_slug.clone();
    let requested_server_slug = target_server_slug
        .map(|slug| slug.trim().to_string())
        .filter(|slug| !slug.is_empty());
    let should_stop_tracked_child = requested_server_slug
        .as_deref()
        .map(|slug| active_server_slug.as_deref() == Some(slug))
        .unwrap_or(true);
    let mut stopped_tracked_child = false;
    let target_slug = service_settings
        .map(|settings| {
            requested_server_slug
                .as_deref()
                .filter(|slug| !slug.trim().is_empty())
                .or_else(|| {
                    active_server_slug
                        .as_deref()
                        .filter(|slug| !slug.trim().is_empty())
                })
                .unwrap_or(settings.selected_server_slug.as_str())
        })
        .map(str::trim)
        .filter(|slug| !slug.is_empty());
    let target_server_url = service_settings
        .map(|settings| settings.server_url.as_str())
        .unwrap_or(DEFAULT_SERVER_URL);
    let mut daemon_pids = match target_slug {
        Some(slug) => {
            find_daemon_process_ids(target_server_url, Some(slug), None, None, None, false)?
        }
        None => Vec::new(),
    };
    let mut stopped_daemon_process = !daemon_pids.is_empty();
    terminate_daemon_processes(daemon_pids)?;

    if should_stop_tracked_child {
        if let Some(child) = runtime.child.as_mut() {
            let still_running = child
                .try_wait()
                .map_err(|err| format!("Unable to inspect service state: {err}"))?
                .is_none();
            if still_running {
                terminate_process_tree(child.id())?;
                stopped_tracked_child = true;
            }
            let _ = child.wait();
        }
        runtime.child = None;
    } else if let Some(child) = runtime.child.as_mut() {
        let still_running = child
            .try_wait()
            .map_err(|err| format!("Unable to inspect service state: {err}"))?
            .is_none();
        if !still_running {
            runtime.child = None;
            runtime.active_pid = None;
        }
    }

    let target_binding = service_settings
        .and_then(|settings| target_slug.and_then(|slug| find_service_binding(settings, "", slug)));
    daemon_pids = if !should_resolve_remote_daemon_after_local_stop(
        stopped_daemon_process,
        stopped_tracked_child,
    ) {
        Vec::new()
    } else {
        find_daemon_process_ids(
            target_server_url,
            target_slug,
            target_binding
                .as_ref()
                .map(|binding| binding.machine_id.as_str())
                .filter(|machine_id| !machine_id.trim().is_empty()),
            None,
            target_binding
                .as_ref()
                .map(|binding| binding.api_key.as_str())
                .filter(|api_key| !api_key.trim().is_empty()),
            false,
        )?
    };
    stopped_daemon_process = stopped_daemon_process || !daemon_pids.is_empty();
    terminate_daemon_processes(daemon_pids)?;

    let should_clear_runtime = requested_server_slug
        .as_deref()
        .map(|slug| active_server_slug.as_deref() == Some(slug))
        .unwrap_or(true);
    if requested_server_slug.is_some() && !stopped_tracked_child && !stopped_daemon_process {
        if should_clear_runtime {
            runtime.last_error = Some("Selected server service is not running.".to_string());
            runtime.active_server_slug = None;
            runtime.active_machine_id = None;
            runtime.active_pid = None;
        }
        return Err("Selected server service is not running.".to_string());
    }

    if should_clear_runtime {
        runtime.last_error = None;
        runtime.active_server_slug = None;
        runtime.active_machine_id = None;
        runtime.active_pid = None;
    }
    Ok(())
}

fn should_resolve_remote_daemon_after_local_stop(
    stopped_daemon_process: bool,
    stopped_tracked_child: bool,
) -> bool {
    !stopped_daemon_process && !stopped_tracked_child
}

fn find_daemon_process_ids(
    target_server_url: &str,
    target_server_slug: Option<&str>,
    target_machine_id: Option<&str>,
    api_key_prefix: Option<&str>,
    legacy_api_key: Option<&str>,
    include_untagged: bool,
) -> Result<Vec<u32>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(daemon_pids_from_ps_output(
            &listing,
            target_server_url,
            target_server_slug,
            target_machine_id,
            api_key_prefix,
            legacy_api_key,
            include_untagged,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = target_server_url;
        let _ = target_server_slug;
        let _ = target_machine_id;
        let _ = api_key_prefix;
        let _ = legacy_api_key;
        let _ = include_untagged;
        Ok(Vec::new())
    }
}

fn unique_untagged_daemon_process_ids(target_server_url: &str) -> Result<Vec<u32>, String> {
    let pids = find_untagged_daemon_process_ids(target_server_url)?;
    if pids.len() == 1 {
        Ok(pids)
    } else {
        Ok(Vec::new())
    }
}

fn find_untagged_daemon_process_ids(target_server_url: &str) -> Result<Vec<u32>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(untagged_daemon_pids_from_ps_output(
            &listing,
            target_server_url,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = target_server_url;
        Ok(Vec::new())
    }
}

fn daemon_pids_from_ps_output(
    output: &str,
    target_server_url: &str,
    target_server_slug: Option<&str>,
    target_machine_id: Option<&str>,
    api_key_prefix: Option<&str>,
    legacy_api_key: Option<&str>,
    include_untagged: bool,
) -> Vec<u32> {
    let entries = process_entries_from_ps_output(output);
    entries
        .iter()
        .filter_map(|entry| {
            if process_entry_has_agent_descendant(entry.pid, &entries)
                && !daemon_command_is_desktop_managed(&entry.command)
            {
                return None;
            }
            if daemon_command_matches(
                &entry.command,
                target_server_url,
                target_server_slug,
                target_machine_id,
                api_key_prefix,
                legacy_api_key,
                include_untagged,
            ) {
                Some(entry.pid)
            } else {
                None
            }
        })
        .collect()
}

fn untagged_daemon_pids_from_ps_output(output: &str, target_server_url: &str) -> Vec<u32> {
    let entries = process_entries_from_ps_output(output);
    entries
        .iter()
        .filter_map(|entry| {
            if process_entry_has_agent_descendant(entry.pid, &entries) {
                return None;
            }
            if daemon_command_is_untagged(&entry.command, target_server_url) {
                Some(entry.pid)
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug)]
struct ProcessEntry {
    pid: u32,
    ppid: Option<u32>,
    command: String,
}

fn process_entries_from_ps_output(output: &str) -> Vec<ProcessEntry> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let mut parts = trimmed.splitn(3, char::is_whitespace);
            let pid = parts.next()?.parse::<u32>().ok()?;
            let second = parts.next()?.trim();
            let third = parts.next().map(str::trim);
            if let (Ok(ppid), Some(command)) = (second.parse::<u32>(), third) {
                return Some(ProcessEntry {
                    pid,
                    ppid: Some(ppid),
                    command: command.to_string(),
                });
            }

            let command = if let Some(command_tail) = third {
                format!("{second} {command_tail}")
            } else {
                second.to_string()
            };
            Some(ProcessEntry {
                pid,
                ppid: None,
                command,
            })
        })
        .collect()
}

fn process_tree_pids_from_entries(root_pid: u32, entries: &[ProcessEntry]) -> Vec<u32> {
    let mut pids = vec![root_pid];
    let mut stack = vec![root_pid];

    while let Some(parent_pid) = stack.pop() {
        for child in entries
            .iter()
            .filter(|entry| entry.ppid == Some(parent_pid))
        {
            if pids.contains(&child.pid) {
                continue;
            }
            pids.push(child.pid);
            stack.push(child.pid);
        }
    }

    pids
}

fn process_entry_has_agent_descendant(pid: u32, entries: &[ProcessEntry]) -> bool {
    let mut stack: Vec<u32> = entries
        .iter()
        .filter(|entry| entry.ppid == Some(pid))
        .map(|entry| entry.pid)
        .collect();

    while let Some(child_pid) = stack.pop() {
        if let Some(child) = entries.iter().find(|entry| entry.pid == child_pid) {
            if process_command_is_agent_runtime(&child.command) {
                return true;
            }
            stack.extend(
                entries
                    .iter()
                    .filter(|entry| entry.ppid == Some(child_pid))
                    .map(|entry| entry.pid),
            );
        }
    }

    false
}

fn process_command_is_agent_runtime(command: &str) -> bool {
    command.contains("codex app-server")
        || command.contains("/codex app-server")
        || (command.contains("/claude ") && command.contains("You are \""))
        || (command.contains(" claude ") && command.contains("You are \""))
}

fn daemon_command_is_desktop_managed(command: &str) -> bool {
    command
        .split_whitespace()
        .any(|part| part == DAEMON_DESKTOP_MANAGED_ARG)
}

fn daemon_command_matches(
    command: &str,
    target_server_url: &str,
    target_server_slug: Option<&str>,
    target_machine_id: Option<&str>,
    api_key_prefix: Option<&str>,
    legacy_api_key: Option<&str>,
    include_untagged: bool,
) -> bool {
    if !daemon_command_has_marker(command)
        || !command.contains("--server-url")
        || !command.contains(target_server_url)
    {
        return false;
    }

    if command.contains(DAEMON_MACHINE_ID_ARG) {
        if target_machine_id
            .filter(|machine_id| !machine_id.trim().is_empty())
            .map(|machine_id| command_arg_value_matches(command, DAEMON_MACHINE_ID_ARG, machine_id))
            .unwrap_or(false)
        {
            return true;
        }

        if command_arg_value_matches(command, DAEMON_MACHINE_ID_ARG, "***") {
            return target_server_slug
                .filter(|server_slug| !server_slug.trim().is_empty())
                .map(|server_slug| {
                    command_arg_value_matches(command, DAEMON_SERVER_SLUG_ARG, server_slug)
                })
                .unwrap_or(false);
        }

        if daemon_command_is_desktop_managed(command)
            && target_machine_id
                .filter(|machine_id| !machine_id.trim().is_empty())
                .is_none()
        {
            return target_server_slug
                .filter(|server_slug| !server_slug.trim().is_empty())
                .map(|server_slug| {
                    command_arg_value_matches(command, DAEMON_SERVER_SLUG_ARG, server_slug)
                })
                .unwrap_or(false);
        }

        return false;
    }

    if command.contains(DAEMON_SERVER_SLUG_ARG) {
        return target_server_slug
            .filter(|server_slug| !server_slug.trim().is_empty())
            .map(|server_slug| {
                command_arg_value_matches(command, DAEMON_SERVER_SLUG_ARG, server_slug)
            })
            .unwrap_or(false);
    }

    if let Some(prefix) = api_key_prefix.filter(|prefix| !prefix.trim().is_empty()) {
        return command_arg_value_starts_with(command, "--api-key", prefix);
    }

    if let Some(api_key) = legacy_api_key.filter(|api_key| !api_key.trim().is_empty()) {
        return command_arg_value_matches(command, "--api-key", api_key);
    }

    include_untagged
}

fn daemon_command_is_untagged(command: &str, target_server_url: &str) -> bool {
    daemon_command_has_marker(command)
        && command.contains("--server-url")
        && command.contains(target_server_url)
        && !command.contains(DAEMON_MACHINE_ID_ARG)
        && !command.contains(DAEMON_SERVER_SLUG_ARG)
}

fn daemon_command_has_marker(command: &str) -> bool {
    command.contains("@slock-ai/daemon")
        || command.contains("slock-ai/daemon")
        || command.split_whitespace().any(|part| {
            part.rsplit(['/', '\\'])
                .next()
                .map(|name| name == "slock-daemon")
                .unwrap_or(false)
        })
}

fn command_arg_value_starts_with(command: &str, arg: &str, expected_prefix: &str) -> bool {
    let expected_prefix = expected_prefix.trim();
    if expected_prefix.is_empty() {
        return false;
    }

    let equals_prefix = format!("{arg}=");
    let mut parts = command.split_whitespace();
    while let Some(part) = parts.next() {
        if part == arg {
            return parts
                .next()
                .map(|value| value.starts_with(expected_prefix))
                .unwrap_or(false);
        }
        if part
            .strip_prefix(equals_prefix.as_str())
            .map(|value| value.starts_with(expected_prefix))
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

fn command_arg_value_matches(command: &str, arg: &str, expected: &str) -> bool {
    let expected = expected.trim();
    if expected.is_empty() {
        return false;
    }

    let equals_prefix = format!("{arg}=");
    let mut parts = command.split_whitespace();
    while let Some(part) = parts.next() {
        if part == arg {
            return parts.next().map(|value| value == expected).unwrap_or(false);
        }
        if part
            .strip_prefix(equals_prefix.as_str())
            .map(|value| value == expected)
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

/// Extract the value of a CLI argument from a command string.
/// Supports both `--arg value` and `--arg=value` forms.
fn extract_arg_value_from_command<'a>(command: &'a str, arg: &str) -> Option<&'a str> {
    let equals_prefix = format!("{arg}=");
    let mut parts = command.split_whitespace();
    while let Some(part) = parts.next() {
        if part == arg {
            return parts.next();
        }
        if let Some(value) = part.strip_prefix(equals_prefix.as_str()) {
            return Some(value);
        }
    }
    None
}

/// Scan local daemon processes for a machine_id matching the given server URL.
/// Returns (machine_id, machine_name_from_process) if found.
fn detect_local_machine_by_pid(
    target_server_url: &str,
    target_server_slug: &str,
) -> Option<String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let listing = String::from_utf8_lossy(&output.stdout);
        for entry in process_entries_from_ps_output(&listing) {
            if !daemon_command_has_marker(&entry.command) {
                continue;
            }
            if !entry.command.contains("--server-url") || !entry.command.contains(target_server_url) {
                continue;
            }
            // Match server slug if the daemon has one tagged
            if entry.command.contains(DAEMON_SERVER_SLUG_ARG) {
                if !command_arg_value_matches(&entry.command, DAEMON_SERVER_SLUG_ARG, target_server_slug) {
                    continue;
                }
            }
            // Try to extract machine_id from command line
            if let Some(machine_id) = extract_arg_value_from_command(&entry.command, "--machine-id")
                .or_else(|| extract_arg_value_from_command(&entry.command, DAEMON_MACHINE_ID_ARG))
            {
                let machine_id = machine_id.trim().to_string();
                if !machine_id.is_empty() && machine_id != "***" {
                    return Some(machine_id);
                }
            }
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = target_server_url;
        let _ = target_server_slug;
        None
    }
}

fn selected_service_daemon_process_from_servers(
    settings: &ServiceSettings,
    servers: &[ServiceServerSnapshot],
) -> Result<Option<ServiceDaemonProcess>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(selected_service_daemon_process_from_server_snapshots(
            settings, servers, &listing,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = settings;
        let _ = servers;
        Ok(None)
    }
}

fn selected_service_daemon_process_from_server_snapshots(
    settings: &ServiceSettings,
    servers: &[ServiceServerSnapshot],
    output: &str,
) -> Option<ServiceDaemonProcess> {
    let target = servers.iter().find(|server| server.selected)?;
    let legacy_api_key = find_service_binding(settings, &target.id, &target.slug)
        .map(|binding| binding.api_key)
        .unwrap_or_default();
    service_daemon_process_from_target(
        settings,
        &target.slug,
        target.machine_id.as_deref(),
        target.api_key_prefix.as_deref(),
        Some(legacy_api_key.as_str()).filter(|api_key| !api_key.trim().is_empty()),
        output,
    )
}

fn service_daemon_process_from_target(
    settings: &ServiceSettings,
    server_slug: &str,
    machine_id: Option<&str>,
    api_key_prefix: Option<&str>,
    legacy_api_key: Option<&str>,
    output: &str,
) -> Option<ServiceDaemonProcess> {
    daemon_pids_from_ps_output(
        output,
        &settings.server_url,
        Some(server_slug),
        machine_id,
        api_key_prefix,
        legacy_api_key,
        false,
    )
    .into_iter()
    .next()
    .map(|pid| ServiceDaemonProcess {
        pid,
        server_slug: server_slug.to_string(),
        machine_id: machine_id
            .map(|machine_id| machine_id.trim().to_string())
            .filter(|machine_id| !machine_id.is_empty()),
    })
}

fn terminate_daemon_processes(mut pids: Vec<u32>) -> Result<(), String> {
    pids.sort_unstable_by(|left, right| right.cmp(left));
    pids.dedup();

    for pid in pids {
        terminate_daemon_process(pid)?;
    }

    Ok(())
}

fn terminate_process_tree(root_pid: u32) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect process tree {root_pid}: {err}"))?;
        if !output.status.success() {
            return Err(format!("Failed to inspect process tree {root_pid}"));
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        let entries = process_entries_from_ps_output(&listing);
        terminate_daemon_processes(process_tree_pids_from_entries(root_pid, &entries))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        terminate_daemon_process(root_pid)?;
    }

    Ok(())
}

fn terminate_daemon_process(pid: u32) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let pid_text = pid.to_string();
        let status = Command::new("kill")
            .args(["-TERM", pid_text.as_str()])
            .status()
            .map_err(|err| format!("Failed to stop daemon process {pid}: {err}"))?;
        if !status.success() {
            if !process_is_alive(pid)? {
                return Ok(());
            }
            return Err(format!("Failed to stop daemon process {pid}"));
        }

        sleep(Duration::from_millis(250));
        if process_is_alive(pid)? {
            let kill_status = Command::new("kill")
                .args(["-KILL", pid_text.as_str()])
                .status()
                .map_err(|err| format!("Failed to force-stop daemon process {pid}: {err}"))?;
            if !kill_status.success() {
                if !process_is_alive(pid)? {
                    return Ok(());
                }
                return Err(format!("Failed to force-stop daemon process {pid}"));
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = pid;
    }

    Ok(())
}

fn process_is_alive(pid: u32) -> Result<bool, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let pid_text = pid.to_string();
        let status = Command::new("kill")
            .args(["-0", pid_text.as_str()])
            .status()
            .map_err(|err| format!("Failed to inspect daemon process {pid}: {err}"))?;
        Ok(status.success())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = pid;
        Ok(false)
    }
}

fn sanitize_service_settings(service: ServiceSettings) -> ServiceSettings {
    let mut machines = Vec::new();
    for binding in service.machines {
        let normalized = ServiceMachineBinding {
            server_id: binding.server_id.trim().to_string(),
            server_slug: binding.server_slug.trim().to_string(),
            machine_id: binding.machine_id.trim().to_string(),
            machine_name: binding.machine_name.trim().to_string(),
            api_key: String::new(),
            source: binding.source.clone(),
        };
        if normalized.server_id.is_empty()
            || normalized.server_slug.is_empty()
            || normalized.machine_id.is_empty()
        {
            continue;
        }
        let duplicate = machines.iter().any(|existing: &ServiceMachineBinding| {
            existing.server_id == normalized.server_id
                || existing.server_slug == normalized.server_slug
        });
        if !duplicate {
            machines.push(normalized);
        }
    }

    ServiceSettings {
        server_url: sanitize_service_server_url(&service.server_url),
        selected_server_slug: service.selected_server_slug.trim().to_string(),
        auto_start_with_workspace: service.auto_start_with_workspace,
        close_app_behavior: close_app_behavior_id(close_app_behavior_from_id(
            &service.close_app_behavior,
        ))
        .to_string(),
        machines,
    }
}

fn sanitize_service_server_url(server_url: &str) -> String {
    let trimmed = server_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return DEFAULT_SERVER_URL.to_string();
    }
    trimmed.strip_suffix("/api").unwrap_or(trimmed).to_string()
}

fn current_session_tokens(state: &DesktopState) -> Result<Option<(String, String)>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    let access_token = settings.session.access_token.trim().to_string();
    let refresh_token = settings.session.refresh_token.trim().to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        Ok(None)
    } else {
        Ok(Some((access_token, refresh_token)))
    }
}

fn desktop_session_has_tokens(state: &DesktopState) -> bool {
    current_session_tokens(state).ok().flatten().is_some()
}

fn current_session_account(state: &DesktopState) -> Result<Option<ServiceAccountSnapshot>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    let display_name = clean_optional_account_text(&settings.session.display_name);
    let email = clean_optional_account_text(&settings.session.email);
    let avatar_url = sanitize_account_avatar_url(&settings.session.avatar_url);

    if display_name.is_some() || email.is_some() || avatar_url.is_some() {
        let initials = account_initials(display_name.as_deref().or(email.as_deref()));
        let id = session_account_id(
            &settings.session.access_token,
            display_name.as_deref(),
            email.as_deref(),
        );
        return Ok(Some(ServiceAccountSnapshot {
            id,
            display_name,
            email,
            avatar_url,
            initials,
        }));
    }

    Ok(session_account_from_token(&settings.session.access_token))
}

fn current_saved_session_accounts(
    state: &DesktopState,
) -> Result<Vec<ServiceAccountSnapshot>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    Ok(session_account_snapshots(&settings.session.accounts))
}

fn session_account_from_token(access_token: &str) -> Option<ServiceAccountSnapshot> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = decode_base64_url(payload)?;
    let claims = serde_json::from_slice::<serde_json::Value>(&decoded).ok()?;
    let display_name = first_claim_string(
        &claims,
        &[
            "displayName",
            "display_name",
            "fullName",
            "name",
            "username",
        ],
    );
    let email = first_claim_string(&claims, &["email", "emailAddress", "email_address"]);
    let avatar_url = first_claim_string(
        &claims,
        &[
            "avatarUrl",
            "avatar_url",
            "picture",
            "image",
            "profileImage",
        ],
    )
    .and_then(|url| sanitize_account_avatar_url(&url));
    let initials = account_initials(display_name.as_deref().or(email.as_deref()));
    let id = first_claim_string(&claims, &["sub", "id", "userId", "user_id"])
        .or_else(|| email.clone())
        .unwrap_or_else(|| token_account_id(access_token));

    if display_name.is_none() && email.is_none() && avatar_url.is_none() {
        return None;
    }

    Some(ServiceAccountSnapshot {
        id,
        display_name,
        email,
        avatar_url,
        initials,
    })
}

fn session_account_snapshots(accounts: &[SavedAccountSettings]) -> Vec<ServiceAccountSnapshot> {
    accounts.iter().filter_map(saved_account_snapshot).collect()
}

fn saved_account_snapshot(account: &SavedAccountSettings) -> Option<ServiceAccountSnapshot> {
    if account.id.trim().is_empty() {
        return None;
    }

    let display_name = clean_optional_account_text(&account.display_name);
    let email = clean_optional_account_text(&account.email);
    let avatar_url = sanitize_account_avatar_url(&account.avatar_url);
    if display_name.is_none() && email.is_none() {
        return None;
    }

    let initials = account_initials(display_name.as_deref().or(email.as_deref()));

    Some(ServiceAccountSnapshot {
        id: account.id.trim().to_string(),
        display_name,
        email,
        avatar_url,
        initials,
    })
}

fn session_account_id(
    access_token: &str,
    display_name: Option<&str>,
    email: Option<&str>,
) -> String {
    email
        .and_then(clean_optional_account_text)
        .or_else(|| display_name.and_then(clean_optional_account_text))
        .unwrap_or_else(|| token_account_id(access_token))
}

fn token_account_id(access_token: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in access_token.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("token-{hash:016x}")
}

fn decode_base64_url(input: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;

    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' => break,
            _ => return None,
        } as u32;

        buffer = (buffer << 6) | value;
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Some(output)
}

fn first_claim_string(claims: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = clean_claim_string(claims.get(*key)) {
            return Some(value);
        }
    }

    for parent in [
        "data",
        "result",
        "user",
        "profile",
        "account",
        "currentUser",
        "me",
    ] {
        if let Some(value) = claims.get(parent) {
            for key in keys {
                if let Some(value) = clean_claim_string(value.get(*key)) {
                    return Some(value);
                }
            }
        }
    }

    None
}

fn clean_claim_string(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    clean_optional_account_text(value)
}

fn clean_optional_account_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.chars().take(160).collect())
    }
}

fn sanitize_account_avatar_url(value: &str) -> Option<String> {
    let url = value.trim();
    let parsed = Url::parse(url).ok()?;
    if matches!(parsed.scheme(), "https" | "http") {
        Some(url.to_string())
    } else {
        None
    }
}

fn account_initials(source: Option<&str>) -> String {
    let source = source.unwrap_or("").trim();
    let mut initials = source
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '@' | '.' | '_' | '-')
        })
        .filter_map(|part| part.chars().find(|character| character.is_alphanumeric()))
        .take(2)
        .collect::<String>();

    if initials.is_empty() {
        initials.push('S');
    }

    initials.to_uppercase()
}

fn current_workspace_session_seed(
    state: &DesktopState,
) -> Result<Option<WorkspaceSessionSeed>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    let access_token = settings.session.access_token.trim().to_string();
    let refresh_token = settings.session.refresh_token.trim().to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Ok(None);
    }

    Ok(Some(WorkspaceSessionSeed {
        access_token,
        refresh_token,
        target_url: workspace_url_for_slug(&settings.service.selected_server_slug),
    }))
}

fn desktop_session_required_message() -> String {
    "Open Slock once and sign in, then the desktop launcher can load your server list.".to_string()
}

fn desktop_session_expired_message() -> String {
    "Your Slock session expired. Open Slock and sign in again, then Desktop will sync your server list.".to_string()
}

fn clear_desktop_session(app: &AppHandle, state: &DesktopState) -> Result<(), String> {
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        clear_desktop_session_settings(&mut settings);
        save_settings(app, &settings)?;
    }

    {
        let mut runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        clear_desktop_session_service_cache(&mut runtime);
    }

    clear_workspace_session_storage(app);
    Ok(())
}

fn clear_desktop_session_settings(settings: &mut AppSettings) {
    settings.session.access_token.clear();
    settings.session.refresh_token.clear();
    settings.session.display_name.clear();
    settings.session.email.clear();
    settings.session.avatar_url.clear();
}

fn upsert_saved_session_account(session: &mut config::SessionSettings) {
    let access_token = session.access_token.trim().to_string();
    let refresh_token = session.refresh_token.trim().to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return;
    }

    let mut display_name = clean_optional_account_text(&session.display_name).unwrap_or_default();
    let mut email = clean_optional_account_text(&session.email).unwrap_or_default();
    let mut avatar_url = sanitize_account_avatar_url(&session.avatar_url).unwrap_or_default();
    fill_account_fields_from_token(
        &access_token,
        &mut display_name,
        &mut email,
        &mut avatar_url,
    );
    if display_name.is_empty() && email.is_empty() {
        return;
    }

    let id = session_account_id(&access_token, Some(&display_name), Some(&email));
    let saved = SavedAccountSettings {
        id: id.clone(),
        access_token,
        refresh_token,
        display_name,
        email,
        avatar_url,
    };

    if let Some(existing) = session.accounts.iter_mut().find(|account| {
        account.id == id
            || (!saved.email.is_empty()
                && clean_optional_account_text(&account.email) == Some(saved.email.clone()))
    }) {
        *existing = saved;
    } else {
        session.accounts.push(saved);
    }
}

fn fill_account_fields_from_token(
    access_token: &str,
    display_name: &mut String,
    email: &mut String,
    avatar_url: &mut String,
) {
    if !display_name.is_empty() && !email.is_empty() && !avatar_url.is_empty() {
        return;
    }

    let Some(token_account) = session_account_from_token(access_token) else {
        return;
    };
    if display_name.is_empty() {
        if let Some(value) = token_account.display_name {
            *display_name = value;
        }
    }
    if email.is_empty() {
        if let Some(value) = token_account.email {
            *email = value;
        }
    }
    if avatar_url.is_empty() {
        if let Some(value) = token_account.avatar_url {
            *avatar_url = value;
        }
    }
}

fn clear_desktop_session_service_cache(runtime: &mut ServiceRuntime) {
    runtime.cached_servers.clear();
    runtime.cached_sync_error = None;
}

fn clear_workspace_session_storage(app: &AppHandle) {
    let script = workspace_session_clear_script();
    for window in app.webview_windows().values() {
        let _ = window.eval(&script);
    }
}

fn login_window_session_sync_script(clear_login_session_storage: bool) -> String {
    let clear_login_session_storage = if clear_login_session_storage {
        "true"
    } else {
        "false"
    };

    r#"(function() {
  try {
    if (window.location.origin !== "https://app.slock.ai") return;
    const shouldClearLoginSession = __SLOCK_DESKTOP_CLEAR_LOGIN_SESSION__;
    const clearKey = "slock_desktop_login_session_cleared";
    const ignoredSignatureKey = "slock_desktop_login_ignored_signature";
    if (shouldClearLoginSession && sessionStorage.getItem(clearKey) !== "1") {
      const previousAccessToken = localStorage.getItem("slock_access_token") || "";
      const previousRefreshToken = localStorage.getItem("slock_refresh_token") || "";
      const hadTokens =
        !!previousAccessToken ||
        !!previousRefreshToken;
      if (previousAccessToken && previousRefreshToken) {
        sessionStorage.setItem(ignoredSignatureKey, `${previousAccessToken}::${previousRefreshToken}`);
      }
      localStorage.removeItem("slock_access_token");
      localStorage.removeItem("slock_refresh_token");
      try {
        const cookies = document.cookie.split(";");
        for (let i = 0; i < cookies.length; i++) {
          const cookie = cookies[i];
          const eqPos = cookie.indexOf("=");
          const name = (eqPos > -1 ? cookie.substring(0, eqPos) : cookie).trim();
          if (name) {
            document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/";
            document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=.slock.ai";
            document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/;domain=slock.ai";
          }
        }
      } catch (_) {}
      sessionStorage.setItem(clearKey, "1");
      delete window.__slockDesktopLoginSignature;
      try {
        window.dispatchEvent(
          new StorageEvent("storage", {
            key: "slock_refresh_token",
            newValue: null,
            storageArea: localStorage,
            url: window.location.href,
          })
        );
      } catch (_) {}
      try {
        const channel = new BroadcastChannel("slock-auth-tokens");
        channel.postMessage({
          type: "tokens-cleared",
          sourceId: "slock-desktop-login-reset",
        });
        window.setTimeout(() => channel.close(), 1000);
      } catch (_) {}

      if (hadTokens || !/^\/(?:login|signin|sign-in|auth)(?:\/|$)/i.test(window.location.pathname || "/")) {
        window.location.replace("https://app.slock.ai/login");
        return;
      }
    }

    if (window.__slockDesktopLoginSyncBound) return;
    window.__slockDesktopLoginSyncBound = true;

    const accountText = (value) => typeof value === "string" ? value.trim() : "";
    const accountField = (source, keys) => {
      if (!source || typeof source !== "object") return "";
      for (const key of keys) {
        const value = accountText(source[key]);
        if (value) return value;
      }
      return "";
    };
    const collectSessionAccount = () => {
      const account = { displayName: "", email: "", avatarUrl: "" };
      const readCandidate = (candidate) => {
        if (!candidate || typeof candidate !== "object") return;
        const sources = [
          candidate,
          candidate.data,
          candidate.result,
          candidate.user,
          candidate.profile,
          candidate.account,
          candidate.currentUser,
          candidate.me,
        ];
        for (const source of sources) {
          if (!source || typeof source !== "object") continue;
          account.displayName ||= accountField(source, ["displayName", "display_name", "fullName", "name", "username"]);
          account.email ||= accountField(source, ["email", "emailAddress", "email_address"]);
          account.avatarUrl ||= accountField(source, ["avatarUrl", "avatar_url", "picture", "image", "profileImage"]);
        }
      };

      for (let index = 0; index < localStorage.length; index += 1) {
        const key = localStorage.key(index) || "";
        const raw = localStorage.getItem(key) || "";
        if (!raw || raw.length > 50000) continue;
        if (!/(user|profile|account|auth|session|slock)/i.test(`${key} ${raw.slice(0, 200)}`)) continue;
        try {
          readCandidate(JSON.parse(raw));
        } catch (_) {}
      }

      return account;
    };

    const syncSessionTokens = async () => {
      try {
        const accessToken = localStorage.getItem("slock_access_token");
        const refreshToken = localStorage.getItem("slock_refresh_token");
        if (!accessToken || !refreshToken) return;

        const nextSignature = `${accessToken}::${refreshToken}`;
        if (sessionStorage.getItem(ignoredSignatureKey) === nextSignature) return;
        if (window.__slockDesktopLoginSignature === nextSignature) return;

        const invoke = window.__TAURI__?.core?.invoke;
        if (typeof invoke !== "function") return;

        const account = collectSessionAccount();
        await invoke("save_session_tokens", {
          accessToken,
          refreshToken,
          displayName: account.displayName || null,
          email: account.email || null,
          avatarUrl: account.avatarUrl || null,
        });
        window.__slockDesktopLoginSignature = nextSignature;
        window.setTimeout(() => {
          invoke("close_login_window").catch(() => {});
        }, 450);
      } catch (error) {
        console.warn("[Slock Desktop] login session sync failed", error);
      }
    };

    window.addEventListener("focus", syncSessionTokens);
    window.addEventListener("storage", syncSessionTokens);
    window.addEventListener("visibilitychange", syncSessionTokens);
    window.__slockDesktopLoginSyncTimer = window.setInterval(syncSessionTokens, 1000);
    syncSessionTokens();
  } catch (error) {
    console.warn("[Slock Desktop] login sync setup failed", error);
  }
})();"#
        .replace(
            "__SLOCK_DESKTOP_CLEAR_LOGIN_SESSION__",
            clear_login_session_storage,
        )
}

fn workspace_session_seed_script(seed: &WorkspaceSessionSeed) -> String {
    let access_token =
        serde_json::to_string(&seed.access_token).unwrap_or_else(|_| "\"\"".to_string());
    let refresh_token =
        serde_json::to_string(&seed.refresh_token).unwrap_or_else(|_| "\"\"".to_string());
    let target_url = serde_json::to_string(&seed.target_url).unwrap_or_else(|_| "\"\"".to_string());

    format!(
        r#"(function() {{
  try {{
    if (window.location.origin !== "https://app.slock.ai") return;
    const accessToken = {access_token};
    const refreshToken = {refresh_token};
    const targetUrl = {target_url};
    if (!accessToken || !refreshToken) return;
    localStorage.setItem("slock_access_token", accessToken);
    localStorage.setItem("slock_refresh_token", refreshToken);
    window.__slockDesktopSessionSignature = accessToken + "::" + refreshToken;
    const notifyTokenListeners = () => {{
      try {{
        window.dispatchEvent(
          new StorageEvent("storage", {{
            key: "slock_refresh_token",
            newValue: refreshToken,
            storageArea: localStorage,
            url: window.location.href,
          }})
        );
      }} catch (_) {{}}

      try {{
        const channel = new BroadcastChannel("slock-auth-tokens");
        channel.postMessage({{
          type: "tokens-updated",
          sourceId: "slock-desktop-session-seed",
          accessToken,
          refreshToken,
        }});
        window.setTimeout(() => channel.close(), 1000);
      }} catch (_) {{}}
    }};
    notifyTokenListeners();
    window.setTimeout(notifyTokenListeners, 100);
    window.setTimeout(notifyTokenListeners, 800);
    const path = window.location.pathname || "/";
    const isAuthPath = /^\/(?:login|signin|sign-in|auth)(?:\/|$)/i.test(path);
    const isRootPath = path === "/";
    const target = targetUrl ? new URL(targetUrl, window.location.origin) : null;
    const targetPath = target?.pathname || "/";
    const targetDiffers =
      target &&
      target.origin === window.location.origin &&
      (window.location.pathname !== target.pathname ||
        window.location.search !== target.search ||
        window.location.hash !== target.hash);
    if ((isAuthPath || (isRootPath && targetPath !== "/")) && targetDiffers) {{
      window.location.replace(target.href);
    }}
    window.setTimeout(() => {{
      const loginFormVisible =
        /^\/s\//.test(window.location.pathname) &&
        !!document.querySelector("input[type='password']");
      const reloadKey = "slock_desktop_session_seed_reload";
      if (loginFormVisible && !sessionStorage.getItem(reloadKey)) {{
        sessionStorage.setItem(reloadKey, "1");
        window.location.replace(target?.href || window.location.href);
      }}
    }}, 1200);
  }} catch (error) {{
    console.warn("[Slock Desktop] session restore failed", error);
  }}
}})();"#
    )
}

fn workspace_session_clear_script() -> String {
    r#"(function() {
  try {
    if (window.location.origin !== "https://app.slock.ai") return;
    localStorage.removeItem("slock_access_token");
    localStorage.removeItem("slock_refresh_token");
    sessionStorage.removeItem("slock_desktop_session_seed_reload");
    delete window.__slockDesktopSessionSignature;
    try {
      window.dispatchEvent(
        new StorageEvent("storage", {
          key: "slock_refresh_token",
          newValue: null,
          storageArea: localStorage,
          url: window.location.href,
        })
      );
    } catch (_) {}
    try {
      const channel = new BroadcastChannel("slock-auth-tokens");
      channel.postMessage({
        type: "tokens-cleared",
        sourceId: "slock-desktop-session-clear",
      });
      window.setTimeout(() => channel.close(), 1000);
    } catch (_) {}
  } catch (error) {
    console.warn("[Slock Desktop] session clear failed", error);
  }
})();"#
        .to_string()
}

fn api_base_url(server_url: &str) -> String {
    format!("{}/api", sanitize_service_server_url(server_url))
}

static API_CLIENT: OnceLock<Client> = OnceLock::new();

fn api_client_builder() -> reqwest::blocking::ClientBuilder {
    Client::builder()
        .no_proxy()
        .user_agent("Slock Desktop")
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
}

fn api_client() -> Result<Client, String> {
    if let Some(client) = API_CLIENT.get() {
        return Ok(client.clone());
    }
    let client = api_client_builder()
        .build()
        .map_err(|err| format!("Unable to create desktop API client: {err}"))?;
    let _ = API_CLIENT.set(client.clone());
    Ok(client)
}

fn fetch_session_account_profile(
    server_url: &str,
    access_token: &str,
) -> Result<Option<SessionAccountProfile>, String> {
    let client = api_client_builder()
        .connect_timeout(Duration::from_millis(900))
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|err| format!("Unable to create account profile API client: {err}"))?;
    let mut api_roots = vec![api_base_url(server_url)];
    let workspace_api_root = format!("{WORKSPACE_URL}/api");
    if !api_roots.iter().any(|root| root == &workspace_api_root) {
        api_roots.push(workspace_api_root);
    }

    for api_root in api_roots {
        for path in [
            "auth/me",
            "auth/session",
            "users/me",
            "user/me",
            "user",
            "me",
            "profile",
            "account",
        ] {
            let response = client
                .get(format!("{api_root}/{path}"))
                .bearer_auth(access_token)
                .send();
            let Ok(response) = response else {
                continue;
            };

            if response.status() == reqwest::StatusCode::NOT_FOUND
                || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
                || response.status() == reqwest::StatusCode::UNAUTHORIZED
                || response.status() == reqwest::StatusCode::FORBIDDEN
            {
                continue;
            }

            if !response.status().is_success() {
                continue;
            }

            let payload = response
                .json::<serde_json::Value>()
                .map_err(|err| format!("Failed to parse account profile: {err}"))?;
            let profile = session_account_profile_from_json(&payload);
            if profile.display_name.is_some()
                || profile.email.is_some()
                || profile.avatar_url.is_some()
            {
                return Ok(Some(profile));
            }
        }
    }

    Ok(None)
}

fn session_account_profile_from_json(payload: &serde_json::Value) -> SessionAccountProfile {
    SessionAccountProfile {
        display_name: first_claim_string(
            payload,
            &[
                "displayName",
                "display_name",
                "fullName",
                "full_name",
                "name",
                "username",
            ],
        ),
        email: first_claim_string(
            payload,
            &["email", "emailAddress", "email_address", "primaryEmail"],
        ),
        avatar_url: first_claim_string(
            payload,
            &[
                "avatarUrl",
                "avatar_url",
                "picture",
                "image",
                "profileImage",
                "profile_image",
            ],
        )
        .and_then(|url| sanitize_account_avatar_url(&url)),
    }
}

fn refresh_session_tokens(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    refresh_token: &str,
) -> Result<(String, String), String> {
    let client = api_client()?;
    let response = client
        .post(format!("{}/auth/refresh", api_base_url(server_url)))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .map_err(|err| format!("Failed to refresh desktop session: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            clear_desktop_session(app, state)?;
            return Err(desktop_session_expired_message());
        }
        return Err(format!(
            "Desktop session refresh failed with {status}: {body}"
        ));
    }

    let payload = response
        .json::<ApiRefreshSession>()
        .map_err(|err| format!("Failed to parse refreshed desktop session: {err}"))?;

    let access_token = payload.access_token.trim().to_string();
    let refresh_token = payload.refresh_token.trim().to_string();

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    settings.session.access_token = access_token.clone();
    settings.session.refresh_token = refresh_token.clone();
    save_settings(app, &settings)?;

    Ok((access_token, refresh_token))
}

fn send_authenticated(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    request: impl Fn(&Client, &str) -> RequestBuilder,
) -> Result<reqwest::blocking::Response, String> {
    let client = api_client()?;
    let Some((access_token, refresh_token)) = current_session_tokens(state)? else {
        return Err(desktop_session_required_message());
    };

    let mut response = request(&client, &access_token)
        .send()
        .map_err(|err| format!("Desktop API request failed: {err}"))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let (next_access_token, _) =
            refresh_session_tokens(app, state, server_url, &refresh_token)?;
        response = request(&client, &next_access_token)
            .send()
            .map_err(|err| format!("Desktop API retry failed: {err}"))?;
    }

    Ok(response)
}

fn load_authenticated_json<T: DeserializeOwned>(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    request: impl Fn(&Client, &str) -> RequestBuilder,
) -> Result<T, String> {
    let response = send_authenticated(app, state, server_url, request)?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Desktop API returned {status}: {body}"));
    }

    response
        .json::<T>()
        .map_err(|err| format!("Failed to parse desktop API payload: {err}"))
}

fn fetch_service_servers(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<Vec<ServiceServerSnapshot>, String> {
    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let servers = load_authenticated_json::<Vec<ApiServer>>(
        app,
        state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers"))
                .bearer_auth(access_token)
        },
    )?;

    let server_ids = servers
        .iter()
        .map(|server| server.id.clone())
        .collect::<Vec<_>>();
    let machines_by_server = fetch_machines_for_servers(app, state, &server_url, &server_ids)?;

    let mut snapshots = Vec::with_capacity(servers.len());
    for (server, machines) in servers.into_iter().zip(machines_by_server) {
        let mut binding = find_service_binding(settings, &server.id, &server.slug);

        // PID scan: if no binding exists, try to detect a local daemon for this server
        if binding.is_none() {
            if let Some(detected_machine_id) = detect_local_machine_by_pid(&server_url, &server.slug) {
                // Verify this machine_id exists in the server's machine list
                if let Some(matched_machine) = machines.iter().find(|m| m.id == detected_machine_id) {
                    let new_binding = ServiceMachineBinding {
                        server_id: server.id.clone(),
                        server_slug: server.slug.clone(),
                        machine_id: detected_machine_id,
                        machine_name: matched_machine.name.clone(),
                        api_key: String::new(),
                        source: "pid_scan".to_string(),
                    };
                    if let Ok(persisted) = upsert_service_binding(app, state, new_binding) {
                        binding = Some(persisted);
                    }
                }
            }
        }

        let (machine_id, machine_name, machine_status, api_key_ready, api_key_prefix, binding_source) =
            service_server_machine_fields(binding.as_ref(), &machines);

        snapshots.push(ServiceServerSnapshot {
            id: server.id.clone(),
            name: server.name,
            slug: server.slug.clone(),
            selected: server.slug == settings.selected_server_slug,
            machine_id,
            machine_name,
            machine_status,
            api_key_ready,
            api_key_prefix,
            binding_source,
        });
    }

    Ok(snapshots)
}

fn fetch_cached_service_server_status(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<Vec<ServiceServerSnapshot>, String> {
    let cached_servers = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?
        .cached_servers
        .clone();

    if cached_servers.is_empty() {
        return fetch_service_servers(app, state, settings);
    }

    let server_ids = cached_servers
        .iter()
        .map(|server| server.id.clone())
        .collect::<Vec<_>>();
    let machines_by_server =
        fetch_machines_for_servers(app, state, &settings.server_url, &server_ids)?;

    let mut snapshots = Vec::with_capacity(cached_servers.len());
    for (server, machines) in cached_servers.into_iter().zip(machines_by_server) {
        let binding = find_service_binding(settings, &server.id, &server.slug);
        let (machine_id, machine_name, machine_status, api_key_ready, api_key_prefix, binding_source) =
            service_server_machine_fields(binding.as_ref(), &machines);

        snapshots.push(ServiceServerSnapshot {
            id: server.id.clone(),
            name: server.name,
            slug: server.slug.clone(),
            selected: server.slug == settings.selected_server_slug,
            machine_id,
            machine_name,
            machine_status,
            api_key_ready,
            api_key_prefix,
            binding_source,
        });
    }

    Ok(snapshots)
}

fn fetch_machines_for_servers(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    server_ids: &[String],
) -> Result<Vec<Vec<ApiMachine>>, String> {
    let concurrency = service_machine_fetch_concurrency(server_ids.len());
    let mut machines_by_server = Vec::with_capacity(server_ids.len());

    for chunk in server_ids.chunks(concurrency) {
        let chunk_results: Vec<Vec<ApiMachine>> = thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|server_id| {
                    let server_id = server_id.clone();
                    let server_url = server_url.to_string();
                    scope.spawn(move || fetch_server_machines(app, state, &server_url, &server_id))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| "Machine fetch thread panicked".to_string())?
                })
                .collect::<Result<Vec<_>, String>>()
        })?;
        machines_by_server.extend(chunk_results);
    }

    Ok(machines_by_server)
}

fn service_machine_fetch_concurrency(server_count: usize) -> usize {
    server_count.clamp(1, SERVICE_MACHINE_FETCH_CONCURRENCY_LIMIT)
}

fn fetch_service_server_catalog(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<Vec<ServiceServerSnapshot>, String> {
    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let servers = load_authenticated_json::<Vec<ApiServer>>(
        app,
        state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers"))
                .bearer_auth(access_token)
        },
    )?;
    let cached_servers = state
        .service
        .lock()
        .ok()
        .map(|runtime| runtime.cached_servers.clone())
        .unwrap_or_default();

    let mut snapshots = Vec::with_capacity(servers.len());
    for server in servers {
        let binding = find_service_binding(settings, &server.id, &server.slug);
        let cached = cached_servers
            .iter()
            .find(|item| item.id == server.id || item.slug == server.slug);
        let machine_id = binding
            .as_ref()
            .map(|item| item.machine_id.clone())
            .filter(|machine_id| !machine_id.trim().is_empty())
            .or_else(|| cached.and_then(|item| item.machine_id.clone()));
        let machine_name = binding
            .as_ref()
            .map(|item| item.machine_name.clone())
            .filter(|machine_name| !machine_name.trim().is_empty())
            .or_else(|| cached.and_then(|item| item.machine_name.clone()));
        let machine_status = cached
            .map(|item| item.machine_status.clone())
            .unwrap_or_else(|| {
                if binding.is_some() {
                    "offline".to_string()
                } else {
                    "not linked".to_string()
                }
            });
        let api_key_prefix = cached.and_then(|item| item.api_key_prefix.clone());
        let api_key_ready = api_key_prefix.is_some() || machine_id.is_some();
        let binding_source = binding
            .as_ref()
            .map(|b| b.source.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                cached.map(|item| item.binding_source.clone()).filter(|s| !s.is_empty())
            })
            .unwrap_or_default();

        snapshots.push(ServiceServerSnapshot {
            id: server.id.clone(),
            name: server.name,
            slug: server.slug.clone(),
            selected: server.slug == settings.selected_server_slug,
            machine_id,
            machine_name,
            machine_status,
            api_key_ready,
            api_key_prefix,
            binding_source,
        });
    }

    Ok(snapshots)
}

fn service_server_machine_fields(
    binding: Option<&ServiceMachineBinding>,
    machines: &[ApiMachine],
) -> (Option<String>, Option<String>, String, bool, Option<String>, String) {
    let bound_machine = binding.and_then(|binding| {
        machines
            .iter()
            .find(|machine| machine.id == binding.machine_id)
    });
    let machine_id = bound_machine.map(|machine| machine.id.clone()).or_else(|| {
        binding
            .map(|item| item.machine_id.clone())
            .filter(|machine_id| !machine_id.trim().is_empty())
    });
    let machine_name = bound_machine
        .map(|machine| machine.name.clone())
        .or_else(|| {
            binding
                .map(|item| item.machine_name.clone())
                .filter(|machine_name| !machine_name.trim().is_empty())
        });
    let machine_status = bound_machine
        .map(|machine| normalize_machine_status(&machine.status))
        .unwrap_or_else(|| {
            if binding.is_some() {
                "offline".to_string()
            } else {
                "not linked".to_string()
            }
        });
    let api_key_prefix = bound_machine
        .map(|machine| machine.api_key_prefix.trim().to_string())
        .filter(|prefix| !prefix.is_empty());
    let api_key_ready = bound_machine.is_some()
        && (api_key_prefix.is_some()
            || machine_id
                .as_ref()
                .map(|machine_id| !machine_id.trim().is_empty())
                .unwrap_or(false));
    let binding_source = binding
        .map(|b| b.source.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    (
        machine_id,
        machine_name,
        machine_status,
        api_key_ready,
        api_key_prefix,
        binding_source,
    )
}

fn fetch_server_machines(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    server_id: &str,
) -> Result<Vec<ApiMachine>, String> {
    let api_root = api_base_url(server_url);
    let payload = load_authenticated_json::<serde_json::Value>(
        app,
        state,
        server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers/{server_id}/machines"))
                .header("X-Server-Id", server_id)
                .bearer_auth(access_token)
        },
    )?;

    let machines = if payload.is_array() {
        serde_json::from_value::<Vec<ApiMachine>>(payload)
            .map_err(|err| format!("Failed to parse machine list: {err}"))?
    } else {
        serde_json::from_value::<ApiMachinesEnvelope>(payload)
            .map_err(|err| format!("Failed to parse machine list envelope: {err}"))?
            .machines
    };

    Ok(machines)
}

fn resolve_selected_server(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<ApiServer, String> {
    let selected_slug = settings.selected_server_slug.trim();
    if !selected_slug.is_empty() {
        return resolve_service_server(app, state, settings, selected_slug);
    }

    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let servers = load_authenticated_json::<Vec<ApiServer>>(
        app,
        state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers"))
                .bearer_auth(access_token)
        },
    )?;

    if servers.len() == 1 {
        let server = servers[0].clone();
        persist_selected_server_slug(app, state, &server.slug)?;
        return Ok(server);
    }

    Err("Pick a server in the launcher before starting Slock.".to_string())
}

fn resolve_service_server(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    server_slug: &str,
) -> Result<ApiServer, String> {
    let server_url = settings.server_url.clone();
    let api_root = api_base_url(&server_url);
    let servers = load_authenticated_json::<Vec<ApiServer>>(
        app,
        state,
        &server_url,
        |client, access_token| {
            client
                .get(format!("{api_root}/servers"))
                .bearer_auth(access_token)
        },
    )?;

    if let Some(server) = servers
        .iter()
        .find(|server| server.slug == server_slug)
        .cloned()
    {
        return Ok(server);
    }

    Err("Pick a server in the launcher before starting Slock.".to_string())
}

fn resolve_existing_service_machine(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    server: &ApiServer,
) -> Result<Option<ResolvedServiceMachine>, String> {
    let existing_binding = find_service_binding(settings, &server.id, &server.slug);
    let machines = fetch_server_machines(app, state, &settings.server_url, &server.id)?;
    let Some(machine) = select_existing_machine(existing_binding.as_ref(), &machines) else {
        return Ok(None);
    };

    let api_key_prefix =
        Some(machine.api_key_prefix.trim().to_string()).filter(|prefix| !prefix.is_empty());
    let machine_status = normalize_machine_status(&machine.status);
    let legacy_api_key = existing_binding
        .as_ref()
        .map(|binding| binding.api_key.trim().to_string())
        .filter(|api_key| !api_key.is_empty())
        .unwrap_or_default();

    Ok(Some(ResolvedServiceMachine {
        binding: ServiceMachineBinding {
            server_id: server.id.clone(),
            server_slug: server.slug.clone(),
            machine_id: machine.id,
            machine_name: machine.name,
            api_key: legacy_api_key,
            source: existing_binding
                .as_ref()
                .map(|b| b.source.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_default(),
        },
        api_key_prefix,
        machine_status,
    }))
}

fn ensure_machine_binding(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    server: &ApiServer,
) -> Result<ServiceStartTarget, String> {
    let server_url = settings.server_url.clone();
    if let Some(target) = resolve_existing_service_machine(app, state, settings, server)? {
        let api_key = rotate_machine_api_key(
            app,
            state,
            &server_url,
            &server.id,
            &target.binding.machine_id,
        )?;
        let mut binding = target.binding;
        binding.api_key = String::new();
        let binding = upsert_service_binding(app, state, binding)?;
        return Ok(ServiceStartTarget {
            binding,
            api_key,
            api_key_prefix: target.api_key_prefix,
            machine_status: target.machine_status,
        });
    }

    let api_root = api_base_url(&server_url);
    let payload = load_authenticated_json::<ApiMachineRegistration>(
        app,
        state,
        &server_url,
        |client, access_token| {
            client
                .post(format!("{api_root}/servers/{}/machines", server.id))
                .header("X-Server-Id", server.id.as_str())
                .bearer_auth(access_token)
                .json(&serde_json::json!({ "name": DAEMON_MACHINE_NAME }))
        },
    )?;

    let binding = ServiceMachineBinding {
        server_id: server.id.clone(),
        server_slug: server.slug.clone(),
        machine_id: payload.machine.id,
        machine_name: payload.machine.name,
        api_key: String::new(),
        source: "desktop_created".to_string(),
    };
    let binding = upsert_service_binding(app, state, binding)?;
    Ok(ServiceStartTarget {
        binding,
        api_key: payload.api_key,
        api_key_prefix: None,
        machine_status: normalize_machine_status(&payload.machine.status),
    })
}

fn select_existing_machine(
    binding: Option<&ServiceMachineBinding>,
    machines: &[ApiMachine],
) -> Option<ApiMachine> {
    let binding = binding?;
    machines
        .iter()
        .find(|machine| machine.id == binding.machine_id)
        .cloned()
}

fn rotate_machine_api_key(
    app: &AppHandle,
    state: &DesktopState,
    server_url: &str,
    server_id: &str,
    machine_id: &str,
) -> Result<String, String> {
    let api_root = api_base_url(server_url);
    let payload = load_authenticated_json::<ApiMachineKeyRotation>(
        app,
        state,
        server_url,
        |client, access_token| {
            client
                .post(format!(
                    "{api_root}/servers/{server_id}/machines/{machine_id}/rotate-key"
                ))
                .header("X-Server-Id", server_id)
                .bearer_auth(access_token)
        },
    )?;

    Ok(payload.api_key)
}

fn find_service_binding(
    settings: &ServiceSettings,
    server_id: &str,
    server_slug: &str,
) -> Option<ServiceMachineBinding> {
    settings
        .machines
        .iter()
        .find(|binding| binding.server_id == server_id || binding.server_slug == server_slug)
        .cloned()
}

fn upsert_service_binding(
    app: &AppHandle,
    state: &DesktopState,
    binding: ServiceMachineBinding,
) -> Result<ServiceMachineBinding, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    settings.service.machines.retain(|item| {
        item.server_id != binding.server_id && item.server_slug != binding.server_slug
    });
    settings.service.machines.push(binding.clone());
    if settings.service.selected_server_slug.trim().is_empty() {
        settings.service.selected_server_slug = binding.server_slug.clone();
    }
    save_settings(app, &settings)?;
    Ok(binding)
}

fn persist_selected_server_slug(
    app: &AppHandle,
    state: &DesktopState,
    server_slug: &str,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    settings.service.selected_server_slug = server_slug.trim().to_string();
    save_settings(app, &settings)
}

fn normalize_machine_status(status: &str) -> String {
    let status = status.trim();
    if status.is_empty() {
        "offline".to_string()
    } else {
        status.to_string()
    }
}

fn machine_counts_as_started(status: &str) -> bool {
    matches!(
        normalize_machine_status(status).as_str(),
        "online" | "running" | "idle" | "healthy"
    )
}

fn persist_service_target_slug(
    app: &AppHandle,
    state: &DesktopState,
    selected_server_slug: Option<String>,
    prefer_active_runtime: bool,
) -> Result<(), String> {
    let mut candidate = selected_server_slug
        .map(|slug| slug.trim().to_string())
        .filter(|slug| !slug.is_empty());

    if candidate.is_none() && prefer_active_runtime {
        let runtime = state
            .service
            .lock()
            .map_err(|_| "Unable to lock service runtime".to_string())?;
        candidate = runtime
            .active_server_slug
            .as_ref()
            .map(|slug| slug.trim().to_string())
            .filter(|slug| !slug.is_empty());
    }

    let Some(candidate) = candidate else {
        return Ok(());
    };

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    if settings.service.selected_server_slug == candidate {
        return Ok(());
    }

    settings.service.selected_server_slug = candidate;
    save_settings(app, &settings)
}

fn sanitize_theme_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Custom".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_custom_theme(mut custom_theme: CustomThemeSettings) -> CustomThemeSettings {
    if custom_theme.id.trim().is_empty() {
        custom_theme.id = format!("custom-{}", uuid::Uuid::new_v4());
    }
    CustomThemeSettings {
        id: custom_theme.id,
        name: sanitize_theme_name(&custom_theme.name),
        accent: sanitize_hex(&custom_theme.accent).unwrap_or_else(|| "#10a37f".to_string()),
    }
}

fn sanitize_custom_themes(items: Vec<CustomThemeSettings>) -> Vec<CustomThemeSettings> {
    items.into_iter().map(sanitize_custom_theme).collect()
}

fn sanitize_custom_styles(items: Vec<ThemeStyleConfig>) -> Vec<ThemeStyleConfig> {
    items.into_iter().map(theme::sanitize_style_config).collect()
}

fn normalize_app_settings(settings: AppSettings) -> AppSettings {
    let appearance_mode = theme::normalize_mode(&settings.appearance_mode).to_string();
    let custom_themes = sanitize_custom_themes(settings.custom_themes);
    let custom_styles = sanitize_custom_styles(settings.custom_styles);
    let legacy_original_theme = settings.color_scheme == "original";
    let inferred_style_scheme = if legacy_original_theme {
        "original".to_string()
    } else {
        settings.style_scheme.clone()
    };
    let style_scheme = resolve_style(&inferred_style_scheme, &custom_style_set(&custom_styles)).id;
    let requested_color_scheme = if legacy_original_theme {
        theme::default_color_scheme()
    } else {
        settings.color_scheme.as_str()
    };
    let color_scheme = resolve_theme_with_style(
        requested_color_scheme,
        &style_scheme,
        &appearance_mode,
        &custom_theme_set(&custom_themes),
        &custom_style_set(&custom_styles),
    )
    .id;

    AppSettings {
        color_scheme,
        style_scheme,
        appearance_mode,
        custom_themes,
        custom_styles,
        language: sanitize_language(&settings.language).to_string(),
        session: config::SessionSettings {
            access_token: settings.session.access_token.trim().to_string(),
            refresh_token: settings.session.refresh_token.trim().to_string(),
            display_name: clean_optional_account_text(&settings.session.display_name)
                .unwrap_or_default(),
            email: clean_optional_account_text(&settings.session.email).unwrap_or_default(),
            avatar_url: sanitize_account_avatar_url(&settings.session.avatar_url)
                .unwrap_or_default(),
            accounts: sanitize_saved_accounts(settings.session.accounts),
        },
        service: sanitize_service_settings(settings.service),
    }
}

fn sanitize_saved_accounts(accounts: Vec<SavedAccountSettings>) -> Vec<SavedAccountSettings> {
    let mut sanitized = Vec::new();
    for account in accounts {
        let access_token = account.access_token.trim().to_string();
        let refresh_token = account.refresh_token.trim().to_string();
        if access_token.is_empty() || refresh_token.is_empty() {
            continue;
        }

        let mut display_name =
            clean_optional_account_text(&account.display_name).unwrap_or_default();
        let mut email = clean_optional_account_text(&account.email).unwrap_or_default();
        let mut avatar_url = sanitize_account_avatar_url(&account.avatar_url).unwrap_or_default();
        fill_account_fields_from_token(
            &access_token,
            &mut display_name,
            &mut email,
            &mut avatar_url,
        );
        if display_name.is_empty() && email.is_empty() {
            continue;
        }

        let id = clean_optional_account_text(&account.id).unwrap_or_else(|| {
            session_account_id(&access_token, Some(&display_name), Some(&email))
        });
        let duplicate = sanitized.iter().any(|existing: &SavedAccountSettings| {
            existing.id == id || (!email.is_empty() && existing.email == email)
        });
        if duplicate {
            continue;
        }

        sanitized.push(SavedAccountSettings {
            id,
            access_token,
            refresh_token,
            display_name,
            email,
            avatar_url,
        });
    }

    sanitized
}

fn sanitize_language(language: &str) -> &'static str {
    match language {
        "zh-CN" => "zh-CN",
        "en-US" => "en-US",
        "system" => "system",
        _ => "system",
    }
}

fn resolve_desktop_language(language: &str) -> &'static str {
    match sanitize_language(language) {
        "zh-CN" => "zh-CN",
        "en-US" => "en-US",
        _ => resolve_system_language(),
    }
}

fn resolve_system_language() -> &'static str {
    static SYSTEM_LANGUAGE: OnceLock<&'static str> = OnceLock::new();
    *SYSTEM_LANGUAGE.get_or_init(|| {
        if let Some(lang) = read_system_language() {
            if lang.to_ascii_lowercase().starts_with("zh") {
                return "zh-CN";
            }
            if !lang.is_empty() {
                return "en-US";
            }
        }
        "en-US"
    })
}

#[cfg(target_os = "macos")]
fn read_system_language() -> Option<String> {
    if let Ok(output) = Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
    {
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    if let Ok(output) = Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
    {
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout);
            for line in raw.lines() {
                let trimmed = line.trim_start().trim_end_matches(',');
                let stripped = trimmed.trim_matches('"');
                if !stripped.is_empty()
                    && stripped != "("
                    && stripped != ")"
                    && !stripped.starts_with('(')
                {
                    return Some(stripped.to_string());
                }
            }
        }
    }
    env_locale()
}

#[cfg(not(target_os = "macos"))]
fn read_system_language() -> Option<String> {
    env_locale()
}

fn env_locale() -> Option<String> {
    env::var("LC_ALL")
        .or_else(|_| env::var("LC_MESSAGES"))
        .or_else(|_| env::var("LANG"))
        .ok()
        .filter(|value| !value.is_empty())
}

fn custom_theme_set(custom_themes: &[CustomThemeSettings]) -> CustomThemeSet {
    CustomThemeSet {
        items: custom_themes
            .iter()
            .map(|item| CustomThemeItem {
                id: item.id.clone(),
                name: item.name.clone(),
                accent: item.accent.clone(),
            })
            .collect(),
    }
}

fn custom_style_set(custom_styles: &[ThemeStyleConfig]) -> CustomStyleSet {
    CustomStyleSet {
        items: custom_styles.to_vec(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .manage(DesktopState {
            settings: Mutex::new(AppSettings::default()),
            service: Mutex::new(ServiceRuntime {
                child: None,
                last_error: None,
                active_server_slug: None,
                active_machine_id: None,
                active_pid: None,
                cached_servers: Vec::new(),
                cached_sync_error: None,
            }),
            auth: Mutex::new(AuthRuntime::default()),
            app_close: Mutex::new(AppCloseRuntime::default()),
            launch_metrics: Mutex::new(WorkspaceLaunchMetrics::default()),
            update_cache: Mutex::new(None),
            message_reminders: Mutex::new(MessageReminderRuntime::default()),
        })
        .on_page_load(|webview, payload| {
            // Only handle workspace URLs (https://app.slock.ai).
            // Launcher (tauri://localhost) and other URLs exit early here,
            // so workspace titlebar/size/scripts are never applied to non-workspace pages.
            if !is_workspace_url(payload.url()) {
                if webview.label() == MAIN_LABEL
                    && matches!(payload.event(), PageLoadEvent::Finished)
                {
                    reinject_current_message_reminder_bridge(webview);
                }
                return;
            }

            if webview.label() == AUTH_LABEL {
                if matches!(payload.event(), PageLoadEvent::Finished) {
                    let clear_login_session_storage = webview
                        .state::<DesktopState>()
                        .auth
                        .lock()
                        .map(|mut auth| {
                            let clear = auth.clear_login_session_storage;
                            auth.clear_login_session_storage = false;
                            clear
                        })
                        .unwrap_or(false);
                    if let Err(err) = webview.eval(login_window_session_sync_script(
                        clear_login_session_storage,
                    )) {
                        log::warn!("failed to apply login session sync: {err}");
                    }
                }
                return;
            }

            if webview.label() != MAIN_LABEL {
                return;
            }

            if matches!(payload.event(), PageLoadEvent::Started) {
                let state = webview.state::<DesktopState>();
                mark_workspace_launch_page_started(&state, payload.url());
                if let Err(err) = apply_workspace_session_seed_to_webview(webview, &state) {
                    log::warn!("failed to seed workspace session: {err}");
                }
                apply_workspace_titlebar_style_to_window(&webview.window());
                apply_workspace_window_size_to_window(&webview.window(), false);
                return;
            }

            if !matches!(payload.event(), PageLoadEvent::Finished) {
                return;
            }

            let state = webview.state::<DesktopState>();
            mark_workspace_launch_page_finished(&state, payload.url());
            if let Err(err) = apply_workspace_session_seed_to_webview(webview, &state) {
                log::warn!("failed to seed workspace session: {err}");
            }

            let (color_scheme, style_scheme, appearance_mode, custom_themes, custom_styles, language, server_slug) = webview
                .state::<DesktopState>()
                .settings
                .lock()
                .map(|settings| {
                    (
                        settings.color_scheme.clone(),
                        settings.style_scheme.clone(),
                        settings.appearance_mode.clone(),
                        settings.custom_themes.clone(),
                        settings.custom_styles.clone(),
                        settings.language.clone(),
                        settings.service.selected_server_slug.clone(),
                    )
                })
                .unwrap_or_else(|_| {
                    (
                        theme::default_color_scheme().to_string(),
                        theme::default_style_scheme().to_string(),
                        "system".to_string(),
                        Vec::<CustomThemeSettings>::new(),
                        Vec::<ThemeStyleConfig>::new(),
                        "system".to_string(),
                        String::new(),
                    )
                });
            let custom = custom_theme_set(&custom_themes);
            let styles = custom_style_set(&custom_styles);
            let theme = resolve_theme_with_style(
                &color_scheme,
                &style_scheme,
                &appearance_mode,
                &custom,
                &styles,
            );

            if let Err(err) = apply_workspace_scripts_to_webview(
                webview,
                theme,
                &color_scheme,
                &style_scheme,
                &appearance_mode,
                &language,
                resolve_desktop_language(&language),
                &server_slug,
                &custom,
                &styles,
            ) {
                log::error!("failed to apply workspace desktop scripts: {err}");
            }

            reinject_current_message_reminder_bridge(webview);
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            {
                let loaded_settings = load_settings(app.handle());
                let settings = normalize_app_settings(loaded_settings.clone());
                if settings != loaded_settings {
                    save_settings(app.handle(), &settings).map_err(std::io::Error::other)?;
                }
                let state = app.state::<DesktopState>();
                let mut current = state
                    .settings
                    .lock()
                    .map_err(|_| std::io::Error::other("settings-lock"))?;
                *current = settings;
            }

            if let Some(window) = app.get_webview_window(MAIN_LABEL) {
                let (appearance_mode, language) = app
                    .state::<DesktopState>()
                    .settings
                    .lock()
                    .map(|settings| (settings.appearance_mode.clone(), settings.language.clone()))
                    .unwrap_or_else(|_| ("system".to_string(), "system".to_string()));
                apply_window_language(app.handle(), &window, &language, false);
                apply_launcher_window_theme(&window, &appearance_mode);
                apply_launcher_titlebar_style(&window);
                apply_launcher_window_size(&window);
            }

            // Register deep link scheme for desktop auth callback
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                let _ = app.deep_link().register_all();
            }

            // Handle deep link URLs (slock://auth/callback#access_token=...&refresh_token=...)
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    if let Err(err) = handle_desktop_deep_link(&handle, &url) {
                        log::warn!("[deep-link] failed to handle {url}: {err}");
                    }
                }
            });

            // Check if app was launched via deep link
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    if let Err(err) = handle_desktop_deep_link(app.handle(), &url) {
                        log::warn!("[deep-link] failed to handle launch URL {url}: {err}");
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_theme,
            set_theme_style,
            import_theme_style,
            set_theme_mode,
            create_custom_theme,
            rename_custom_theme,
            update_custom_theme_accent,
            delete_custom_theme,
            set_language,
            save_session_tokens,
            open_workspace,
            exit_workspace,
            save_service_settings,
            refresh_service_servers,
            refresh_service_server_status,
            refresh_service_server_catalog,
            select_service_server,
            start_service,
            stop_service,
            resolve_app_close_request,
            update_service,
            open_service_log,
            start_window_drag,
            check_desktop_update,
            install_desktop_update,
            enqueue_message_reminder_event,
            open_login,
            open_login_browser,
            switch_account,
            switch_account_browser,
            close_login_window,
            activate_account,
            forget_account,
            fetch_dashboard,
            fetch_agent_activity,
            stop_agent,
            start_agent,
            fetch_inbox,
            fetch_followed_threads,
            fetch_dm_channels,
            fetch_unread_channels,
            fetch_thread_messages,
            fetch_channel_messages,
            fetch_server_members,
            fetch_server_unread_summary,
            send_message,
            mark_channel_read,
            bind_local_machine
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app: &AppHandle, event: RunEvent| {
        let state = app.state::<DesktopState>();
        match event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } if label == MAIN_LABEL => {
                api.prevent_close();
                handle_window_close_requested(app, &state);
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { .. },
                ..
            } if label == AUTH_LABEL => {
                if !desktop_session_has_tokens(&state) {
                    let _ = app.emit(DESKTOP_AUTH_CANCELLED_EVENT, ());
                }
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::ThemeChanged(_),
                ..
            } if label == MAIN_LABEL => {
                if let Some(window) = app.get_webview_window(MAIN_LABEL) {
                    let appearance_mode = state
                        .settings
                        .lock()
                        .map(|settings| settings.appearance_mode.clone())
                        .unwrap_or_else(|_| "system".to_string());
                    if window_is_workspace(&window) {
                        apply_window_theme(&window, &appearance_mode);
                    } else {
                        apply_launcher_window_theme(&window, &appearance_mode);
                    }
                }
            }
            RunEvent::ExitRequested { api, .. } if !handle_app_exit_requested(app, &state) => {
                api.prevent_exit();
            }
            RunEvent::Exit => handle_app_exit(app, &state),
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        app_close_prompt_script, clear_desktop_session_service_cache,
        clear_desktop_session_settings, close_app_behavior_from_action, close_app_behavior_from_id,
        close_app_behavior_id, daemon_command_matches, daemon_pids_from_ps_output,
        desktop_session_expired_message, mark_app_close_service_stop_completed,
        normalize_app_settings, prepare_runtime_for_service_target, process_entries_from_ps_output,
        process_tree_pids_from_entries, resolve_service_command_from_dirs, sanitize_saved_accounts,
        sanitize_service_settings, select_existing_machine,
        selected_service_daemon_process_from_cached_output,
        selected_service_daemon_process_from_server_snapshots,
        service_daemon_process_from_resolved_target, service_machine_fetch_concurrency,
        service_server_machine_fields, session_account_snapshots,
        should_detect_selected_service_process,
        should_refresh_service_servers, should_resolve_remote_daemon_after_local_stop,
        take_app_close_service_stop_completed,
        terminate_daemon_process, untagged_daemon_pids_from_ps_output,
        upsert_saved_session_account, workspace_session_clear_script,
        workspace_session_seed_script, ApiMachine, AppCloseRuntime, AuthRuntime,
        CloseAppPromptCopy, CloseAppServiceBehavior, DesktopState, ResolvedServiceMachine,
        ServiceRuntime, ServiceServerSnapshot, WorkspaceLaunchMetrics, WorkspaceSessionSeed,
    };
    #[cfg(unix)]
    use super::{claude_wrapper_script, prepend_path_env_dir};
    use crate::config::{
        AppSettings, SavedAccountSettings, ServiceMachineBinding, ServiceSettings, SessionSettings,
    };
    use crate::theme;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        env, fs,
        io::{Read, Write},
        net::TcpListener,
        path::{Path, PathBuf},
        process::Command,
        sync::Mutex,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    fn make_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "slock-desktop-{name}-{}-{suffix}",
            std::process::id()
        ))
    }

    #[test]
    fn missing_style_scheme_defaults_to_desktop_default_style() {
        let settings: AppSettings = serde_json::from_str(r#"{"colorScheme":"default"}"#).unwrap();
        let normalized = normalize_app_settings(settings);

        assert_eq!(theme::default_style_scheme(), "default");
        assert_eq!(normalized.color_scheme, "default");
        assert_eq!(normalized.style_scheme, "default");
    }

    #[test]
    fn legacy_original_color_scheme_migrates_to_original_style() {
        let settings: AppSettings = serde_json::from_str(r#"{"activeTheme":"original"}"#).unwrap();
        let normalized = normalize_app_settings(settings);

        assert_eq!(normalized.color_scheme, "default");
        assert_eq!(normalized.style_scheme, "original");
    }

    #[test]
    fn explicit_original_style_scheme_is_preserved() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"colorScheme":"default","styleScheme":"original"}"#,
        )
        .unwrap();
        let normalized = normalize_app_settings(settings);

        assert_eq!(normalized.color_scheme, "default");
        assert_eq!(normalized.style_scheme, "original");
    }

    #[test]
    fn desktop_api_client_ignores_system_proxy_configuration() {
        let _env_guard = EnvGuard::capture(&[
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
            "NO_PROXY",
            "no_proxy",
        ]);
        env::set_var("HTTP_PROXY", "http://127.0.0.1:9");
        env::set_var("http_proxy", "http://127.0.0.1:9");
        env::set_var("HTTPS_PROXY", "http://127.0.0.1:9");
        env::set_var("https_proxy", "http://127.0.0.1:9");
        env::set_var("ALL_PROXY", "http://127.0.0.1:9");
        env::set_var("all_proxy", "http://127.0.0.1:9");
        env::remove_var("NO_PROXY");
        env::remove_var("no_proxy");

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin_addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buffer = [0; 512];
                        let _ = stream.read(&mut buffer);
                        stream
                            .write_all(
                                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                            )
                            .unwrap();
                        return true;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return false;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(err) => panic!("test server failed: {err}"),
                }
            }
        });

        let client = super::api_client_builder()
            .resolve("api.slock.ai", origin_addr)
            .build()
            .expect("desktop API client should ignore system proxy settings");
        let response = client
            .get(format!("http://api.slock.ai:{}/health", origin_addr.port()))
            .send();
        let accepted = server.join().unwrap();

        let response = response.expect("desktop API request should bypass system proxy");
        assert!(accepted);
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().unwrap(), "ok");
    }

    #[test]
    fn expired_desktop_session_clears_cached_service_state() {
        let mut settings = AppSettings::default();
        settings.session.access_token = "stale-access-token".to_string();
        settings.session.refresh_token = "stale-refresh-token".to_string();
        let mut runtime = ServiceRuntime {
            child: None,
            last_error: None,
            active_server_slug: Some("open-have".to_string()),
            active_machine_id: Some("machine-open".to_string()),
            active_pid: None,
            cached_servers: vec![ServiceServerSnapshot {
                id: "server-open".to_string(),
                name: "Open".to_string(),
                slug: "open-have".to_string(),
                selected: true,
                machine_id: Some("machine-open".to_string()),
                machine_name: Some("Open machine".to_string()),
                machine_status: "running".to_string(),
                api_key_ready: true,
                api_key_prefix: Some("sk_live".to_string()),
            }],
            cached_sync_error: Some("Desktop session refresh failed".to_string()),
        };

        clear_desktop_session_settings(&mut settings);
        clear_desktop_session_service_cache(&mut runtime);

        assert!(settings.session.access_token.is_empty());
        assert!(settings.session.refresh_token.is_empty());
        assert!(runtime.cached_servers.is_empty());
        assert_eq!(runtime.cached_sync_error, None);
        assert_eq!(
            desktop_session_expired_message(),
            "Your Slock session expired. Open Slock and sign in again, then Desktop will sync your server list."
        );
    }

    struct EnvGuard {
        values: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn capture(keys: &[&'static str]) -> Self {
            Self {
                values: keys.iter().map(|key| (*key, env::var_os(key))).collect(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.values {
                if let Some(value) = value {
                    env::set_var(key, value);
                } else {
                    env::remove_var(key);
                }
            }
        }
    }

    #[test]
    fn tauri_config_uses_opaque_startup_window_background() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri config should parse");
        let app = config
            .get("app")
            .and_then(|value| value.as_object())
            .expect("tauri config should include app settings");
        let window = app
            .get("windows")
            .and_then(|value| value.as_array())
            .and_then(|windows| windows.first())
            .and_then(|value| value.as_object())
            .expect("tauri config should include the main window");

        assert_eq!(
            app.get("macOSPrivateApi").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            window.get("transparent").and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            window.get("backgroundColor"),
            Some(&serde_json::json!([247, 247, 245, 255]))
        );
    }

    #[test]
    fn workspace_position_uses_monitor_work_area_margin() {
        let position = super::workspace_window_logical_position(0, 50, 2.0);

        assert_eq!(position.x, 24.0);
        assert_eq!(position.y, 49.0);
    }

    #[test]
    fn service_command_resolution_adds_node_dir_for_npx_shebang() {
        let root = temp_test_dir("service-command-path");
        let node_bin = root.join("node-v24/bin");
        fs::create_dir_all(&node_bin).unwrap();
        let npx = node_bin.join("npx");
        make_executable(&npx);
        make_executable(&node_bin.join("node"));

        let command =
            resolve_service_command_from_dirs(vec![PathBuf::from("/usr/bin"), node_bin.clone()])
                .unwrap();
        let path_dirs = env::split_paths(&command.path_env).collect::<Vec<_>>();

        assert_eq!(command.executable, npx);
        assert_eq!(path_dirs.first(), Some(&node_bin));
        assert!(path_dirs.contains(&node_bin));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn service_command_resolution_requires_node_for_npx() {
        let root = temp_test_dir("service-command-missing-node");
        let node_bin = root.join("node-v24/bin");
        fs::create_dir_all(&node_bin).unwrap();
        make_executable(&node_bin.join("npx"));

        assert!(resolve_service_command_from_dirs(vec![node_bin]).is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn runtime_wrapper_path_is_prepended_to_service_path() {
        let root = temp_test_dir("runtime-wrapper-path");
        let wrapper_dir = root.join("wrappers");
        let node_bin = root.join("node-v24/bin");
        fs::create_dir_all(&wrapper_dir).unwrap();
        fs::create_dir_all(&node_bin).unwrap();
        let path_env = env::join_paths([node_bin.as_path()]).unwrap();

        let next_path = prepend_path_env_dir(&wrapper_dir, &path_env.to_string_lossy());
        let path_dirs = env::split_paths(&next_path).collect::<Vec<_>>();

        assert_eq!(path_dirs.first(), Some(&wrapper_dir));
        assert!(path_dirs.contains(&node_bin));

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn claude_wrapper_supports_model_override_env_vars() {
        let script = claude_wrapper_script();

        assert!(script.contains("SLOCK_CLAUDE_MODEL"));
        assert!(script.contains("CLAUDE_MODEL"));
        assert!(script.contains("ANTHROPIC_MODEL"));
        assert!(script.contains("arg === \"--model\""));
        assert!(script.contains("arg.startsWith(\"--model=\")"));
    }

    #[cfg(unix)]
    #[test]
    fn claude_wrapper_rewrites_model_argument_from_env() {
        if Command::new("node").arg("--version").output().is_err() {
            return;
        }

        let root = temp_test_dir("claude-wrapper-rewrite");
        let wrapper_dir = root.join("wrappers");
        let real_dir = root.join("real-bin");
        fs::create_dir_all(&wrapper_dir).unwrap();
        fs::create_dir_all(&real_dir).unwrap();

        let wrapper_path = wrapper_dir.join("claude");
        fs::write(&wrapper_path, claude_wrapper_script()).unwrap();
        let mut wrapper_permissions = fs::metadata(&wrapper_path).unwrap().permissions();
        wrapper_permissions.set_mode(0o755);
        fs::set_permissions(&wrapper_path, wrapper_permissions).unwrap();

        let real_path = real_dir.join("claude");
        fs::write(
            &real_path,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SLOCK_TEST_ARG_FILE\"\n",
        )
        .unwrap();
        let mut real_permissions = fs::metadata(&real_path).unwrap().permissions();
        real_permissions.set_mode(0o755);
        fs::set_permissions(&real_path, real_permissions).unwrap();

        let args_path = root.join("args.txt");
        let original_path = env::var_os("PATH").unwrap_or_default();
        let mut path_dirs = vec![wrapper_dir.clone(), real_dir.clone()];
        path_dirs.extend(env::split_paths(&original_path));
        let path_env = env::join_paths(&path_dirs).unwrap();
        let status = Command::new(&wrapper_path)
            .args(["--model", "sonnet", "--output-format", "stream-json"])
            .env("PATH", path_env)
            .env("SLOCK_CLAUDE_MODEL", "claude-sonnet-4-5")
            .env("SLOCK_TEST_ARG_FILE", &args_path)
            .status()
            .unwrap();

        assert!(status.success());
        let args = fs::read_to_string(args_path).unwrap();
        assert!(args.contains("--model\nclaude-sonnet-4-5\n"));
        assert!(!args.contains("sonnet\n"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_command_matching_respects_target_api_key() {
        let command = "node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_current";

        assert!(daemon_command_matches(
            command,
            "https://api.slock.ai",
            None,
            None,
            None,
            Some("sk_machine_current"),
            false,
        ));
        assert!(!daemon_command_matches(
            command,
            "https://api.slock.ai",
            None,
            None,
            None,
            Some("sk_machine_other"),
            false,
        ));
        assert!(!daemon_command_matches(
            command,
            "https://other.slock.ai",
            None,
            None,
            None,
            Some("sk_machine_current"),
            false,
        ));
    }

    #[test]
    fn daemon_command_matching_prefers_stable_desktop_markers() {
        let command = "node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_rotating --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open";

        assert!(daemon_command_matches(
            command,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-open"),
            None,
            Some("old-key"),
            false,
        ));
        assert!(!daemon_command_matches(
            command,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-other"),
            None,
            Some("sk_rotating"),
            false,
        ));
    }

    #[test]
    fn daemon_pid_parser_keeps_only_matching_target_processes() {
        let output = r#"
  101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_current --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
  102 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_other --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id machine-tyan
  103 node /tmp/another-command --server-url https://api.slock.ai --api-key sk_machine_current --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-open"),
            None,
            None,
            false,
        );

        assert_eq!(pids, vec![101]);
    }

    #[test]
    fn daemon_pid_parser_includes_masked_npm_parent_for_marked_daemon() {
        let output = r#"
  101   1 npm exec @slock-ai/daemon@latest --server-url https://api.slock.ai --api-key sk_machine_current --slock-desktop-server-slug open-have --slock-desktop-machine-id ***
  102 101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_current --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
  103   1 npm exec @slock-ai/daemon@latest --server-url https://api.slock.ai --api-key sk_machine_other --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id ***
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-open"),
            None,
            None,
            false,
        );

        assert_eq!(pids, vec![101, 102]);
    }

    #[test]
    fn daemon_pid_parser_excludes_agent_hosted_daemons() {
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_agent --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
  201 101 node /opt/homebrew/bin/codex app-server --listen stdio://
  102   1 npm exec @slock-ai/daemon@latest --server-url https://api.slock.ai --api-key sk_service --slock-desktop-server-slug open-have --slock-desktop-machine-id ***
  103 102 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_service --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open --slock-desktop-managed
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-open"),
            None,
            None,
            false,
        );

        assert_eq!(pids, vec![102, 103]);
    }

    #[test]
    fn daemon_pid_parser_keeps_desktop_managed_daemon_with_agent_descendants() {
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_service --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open --slock-desktop-managed
  201 101 node /opt/homebrew/bin/codex app-server --listen stdio://
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("open-have"),
            Some("machine-open"),
            None,
            None,
            false,
        );

        assert_eq!(pids, vec![101]);
    }

    #[test]
    fn desktop_managed_daemon_parser_requires_machine_match_when_binding_exists() {
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_service --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id actual-machine --slock-desktop-managed
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("tyan-dyun"),
            Some("stale-machine"),
            None,
            None,
            false,
        );

        assert!(pids.is_empty());
    }

    #[test]
    fn desktop_managed_daemon_parser_matches_slug_without_machine_target() {
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_service --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id actual-machine --slock-desktop-managed
"#;

        let pids = daemon_pids_from_ps_output(
            output,
            "https://api.slock.ai",
            Some("tyan-dyun"),
            None,
            None,
            None,
            false,
        );

        assert_eq!(pids, vec![101]);
    }

    #[test]
    fn resolved_service_target_detects_running_daemon_before_key_rotation() {
        let settings = ServiceSettings::default();
        let target = ResolvedServiceMachine {
            binding: ServiceMachineBinding {
                server_id: "server-open".to_string(),
                server_slug: "open-have".to_string(),
                machine_id: "machine-open".to_string(),
                machine_name: "Open machine".to_string(),
                api_key: String::new(),
                source: String::new(),
            },
            api_key_prefix: None,
            machine_status: "running".to_string(),
        };
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_previous --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open --slock-desktop-managed
"#;

        let process = service_daemon_process_from_resolved_target(&settings, &target, output);

        assert_eq!(process.as_ref().map(|process| process.pid), Some(101));
        assert_eq!(
            process.as_ref().map(|process| process.server_slug.as_str()),
            Some("open-have")
        );
        assert_eq!(
            process.and_then(|process| process.machine_id),
            Some("machine-open".to_string())
        );
    }

    #[test]
    fn tracked_service_process_tree_includes_npx_wrapper_and_daemon_child() {
        let output = r#"
  101   1 npm exec @slock-ai/daemon@latest --server-url https://api.slock.ai --api-key sk_current --slock-desktop-server-slug open-have --slock-desktop-machine-id ***
  102 101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_current --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open --slock-desktop-managed
  103 102 node /opt/homebrew/bin/codex app-server --listen stdio://
  104   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_other --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id machine-tyan --slock-desktop-managed
"#;

        let entries = process_entries_from_ps_output(output);
        let pids = process_tree_pids_from_entries(101, &entries);

        assert_eq!(pids, vec![101, 102, 103]);
    }

    #[test]
    fn preparing_runtime_for_another_server_detaches_without_stopping_existing_child() {
        let child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep child");
        let pid = child.id();
        let state = DesktopState {
            settings: Mutex::new(AppSettings::default()),
            service: Mutex::new(ServiceRuntime {
                child: Some(child),
                last_error: None,
                active_server_slug: Some("tyan-dyun".to_string()),
                active_machine_id: Some("machine-tyan".to_string()),
                active_pid: Some(pid),
                cached_servers: Vec::new(),
                cached_sync_error: None,
            }),
            auth: Mutex::new(AuthRuntime::default()),
            app_close: Mutex::new(AppCloseRuntime::default()),
            launch_metrics: Mutex::new(WorkspaceLaunchMetrics::default()),
            update_cache: Mutex::new(None),
        };

        let matched =
            prepare_runtime_for_service_target(&state, "open-have", Some("machine-open"), true)
                .expect("prepare runtime");
        let child_still_running = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        let _ = terminate_daemon_process(pid);

        assert!(!matched);
        assert!(child_still_running);
        assert!(state.service.lock().unwrap().child.is_none());
    }

    #[test]
    fn completed_close_stop_is_consumed_once_for_exit_hook() {
        let state = DesktopState {
            settings: Mutex::new(AppSettings::default()),
            service: Mutex::new(ServiceRuntime {
                child: None,
                last_error: None,
                active_server_slug: None,
                active_machine_id: None,
                active_pid: None,
                cached_servers: Vec::new(),
                cached_sync_error: None,
            }),
            auth: Mutex::new(AuthRuntime::default()),
            app_close: Mutex::new(AppCloseRuntime::default()),
            launch_metrics: Mutex::new(WorkspaceLaunchMetrics::default()),
            update_cache: Mutex::new(None),
        };

        assert!(!take_app_close_service_stop_completed(&state));
        mark_app_close_service_stop_completed(&state);
        assert!(take_app_close_service_stop_completed(&state));
        assert!(!take_app_close_service_stop_completed(&state));
    }

    #[test]
    fn local_stop_skips_remote_daemon_resolution() {
        assert!(should_resolve_remote_daemon_after_local_stop(false, false));
        assert!(!should_resolve_remote_daemon_after_local_stop(true, false));
        assert!(!should_resolve_remote_daemon_after_local_stop(false, true));
        assert!(!should_resolve_remote_daemon_after_local_stop(true, true));
    }

    #[test]
    fn untagged_daemon_parser_ignores_desktop_marked_processes() {
        let output = r#"
  101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_old
  102 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_new --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
  103 node /tmp/npx/@slock-ai/daemon --server-url https://other.slock.ai --api-key sk_other
"#;

        let pids = untagged_daemon_pids_from_ps_output(output, "https://api.slock.ai");

        assert_eq!(pids, vec![101]);
    }

    #[test]
    fn daemon_update_requires_selected_process_on_current_computer() {
        let settings = ServiceSettings {
            selected_server_slug: "open-have".to_string(),
            machines: vec![ServiceMachineBinding {
                server_id: "server-open".to_string(),
                server_slug: "open-have".to_string(),
                machine_id: "machine-open".to_string(),
                machine_name: "Open machine".to_string(),
                api_key: String::new(),
                source: String::new(),
            }],
            ..ServiceSettings::default()
        };
        let servers = vec![ServiceServerSnapshot {
            id: "server-open".to_string(),
            name: "Open Have".to_string(),
            slug: "open-have".to_string(),
            selected: true,
            machine_id: Some("machine-open".to_string()),
            machine_name: Some("Open machine".to_string()),
            machine_status: "online".to_string(),
            api_key_ready: true,
            api_key_prefix: Some("sk_live".to_string()),
        }];
        let output = "";

        let process =
            selected_service_daemon_process_from_cached_output(&settings, &servers, None, output);

        assert!(process.is_none());
    }

    #[test]
    fn daemon_update_detects_selected_process_on_current_computer() {
        let settings = ServiceSettings {
            selected_server_slug: "open-have".to_string(),
            machines: vec![ServiceMachineBinding {
                server_id: "server-open".to_string(),
                server_slug: "open-have".to_string(),
                machine_id: "machine-open".to_string(),
                machine_name: "Open machine".to_string(),
                api_key: String::new(),
                source: String::new(),
            }],
            ..ServiceSettings::default()
        };
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_current --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open --slock-desktop-managed
"#;

        let process =
            selected_service_daemon_process_from_cached_output(&settings, &[], None, output);

        assert_eq!(process.as_ref().map(|process| process.pid), Some(101));
        assert_eq!(
            process.as_ref().map(|process| process.server_slug.as_str()),
            Some("open-have")
        );
    }

    #[test]
    fn initial_bootstrap_can_skip_service_network_refresh() {
        assert!(!should_refresh_service_servers(false, true));
        assert!(!should_refresh_service_servers(false, false));
        assert!(should_refresh_service_servers(true, true));
    }

    #[test]
    fn switching_service_server_detects_selected_daemon_process() {
        assert!(should_detect_selected_service_process(
            true,
            "open-have",
            "",
            false
        ));
        assert!(should_detect_selected_service_process(
            true,
            "tyan-dyun",
            "open-have",
            true
        ));
        assert!(!should_detect_selected_service_process(
            true,
            "open-have",
            "open-have",
            true
        ));
        assert!(!should_detect_selected_service_process(
            false,
            "tyan-dyun",
            "open-have",
            true
        ));
    }

    #[test]
    fn machine_status_refresh_uses_bounded_concurrency() {
        assert_eq!(service_machine_fetch_concurrency(0), 1);
        assert_eq!(service_machine_fetch_concurrency(1), 1);
        assert_eq!(service_machine_fetch_concurrency(3), 3);
        assert_eq!(service_machine_fetch_concurrency(64), 8);
    }

    #[test]
    fn selected_service_process_detection_uses_desktop_markers() {
        let settings = ServiceSettings {
            selected_server_slug: "open-have".to_string(),
            machines: vec![
                ServiceMachineBinding {
                    server_id: "server-open".to_string(),
                    server_slug: "open-have".to_string(),
                    machine_id: "machine-open".to_string(),
                    machine_name: "Open machine".to_string(),
                    api_key: "sk_machine_open".to_string(),
                    source: String::new(),
                },
                ServiceMachineBinding {
                    server_id: "server-tyan".to_string(),
                    server_slug: "tyan-dyun".to_string(),
                    machine_id: "machine-tyan".to_string(),
                    machine_name: "Tyan machine".to_string(),
                    api_key: "sk_machine_tyan".to_string(),
                    source: String::new(),
                },
            ],
            ..ServiceSettings::default()
        };
        let output = r#"
  101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_tyan --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id machine-tyan
  102 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_rotated --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
"#;

        let servers = vec![
            ServiceServerSnapshot {
                id: "server-open".to_string(),
                name: "Open".to_string(),
                slug: "open-have".to_string(),
                selected: true,
                machine_id: Some("machine-open".to_string()),
                machine_name: Some("Open machine".to_string()),
                machine_status: "offline".to_string(),
                api_key_ready: true,
                api_key_prefix: None,
            },
            ServiceServerSnapshot {
                id: "server-tyan".to_string(),
                name: "Tyan".to_string(),
                slug: "tyan-dyun".to_string(),
                selected: false,
                machine_id: Some("machine-tyan".to_string()),
                machine_name: Some("Tyan machine".to_string()),
                machine_status: "offline".to_string(),
                api_key_ready: true,
                api_key_prefix: None,
            },
        ];

        let process =
            selected_service_daemon_process_from_server_snapshots(&settings, &servers, output);

        assert_eq!(process.as_ref().map(|process| process.pid), Some(102));
        assert_eq!(
            process.as_ref().map(|process| process.server_slug.as_str()),
            Some("open-have")
        );
        assert_eq!(
            process.and_then(|process| process.machine_id),
            Some("machine-open".to_string())
        );
    }

    #[test]
    fn selected_service_process_detection_matches_daemon_bin_name() {
        let settings = ServiceSettings {
            selected_server_slug: "tyan-dyun".to_string(),
            machines: vec![ServiceMachineBinding {
                server_id: "server-tyan".to_string(),
                server_slug: "tyan-dyun".to_string(),
                machine_id: "machine-tyan".to_string(),
                machine_name: "Tyan machine".to_string(),
                api_key: "sk_machine_tyan".to_string(),
                source: String::new(),
            }],
            ..ServiceSettings::default()
        };
        let output = r#"
  101   1 node /Users/example/.npm/_npx/277f35d2ed0078b9/node_modules/.bin/slock-daemon --server-url https://api.slock.ai --api-key sk_machine_tyan --slock-desktop-server-slug tyan-dyun --slock-desktop-machine-id machine-tyan --slock-desktop-managed
"#;

        let servers = vec![ServiceServerSnapshot {
            id: "server-tyan".to_string(),
            name: "Tyan".to_string(),
            slug: "tyan-dyun".to_string(),
            selected: true,
            machine_id: Some("machine-tyan".to_string()),
            machine_name: Some("Tyan machine".to_string()),
            machine_status: "running".to_string(),
            api_key_ready: true,
            api_key_prefix: None,
        }];

        let process =
            selected_service_daemon_process_from_server_snapshots(&settings, &servers, output);

        assert_eq!(process.as_ref().map(|process| process.pid), Some(101));
        assert_eq!(
            process.as_ref().map(|process| process.server_slug.as_str()),
            Some("tyan-dyun")
        );
        assert_eq!(
            process.and_then(|process| process.machine_id),
            Some("machine-tyan".to_string())
        );
    }

    #[test]
    fn selected_service_process_detection_excludes_unmanaged_agent_host() {
        let settings = ServiceSettings {
            selected_server_slug: "open-have".to_string(),
            machines: vec![ServiceMachineBinding {
                server_id: "server-open".to_string(),
                server_slug: "open-have".to_string(),
                machine_id: "machine-open".to_string(),
                machine_name: "Open machine".to_string(),
                api_key: "sk_machine_open".to_string(),
                source: String::new(),
            }],
            ..ServiceSettings::default()
        };
        let output = r#"
  101   1 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_agent --slock-desktop-server-slug open-have --slock-desktop-machine-id machine-open
  201 101 node /opt/homebrew/bin/codex app-server --listen stdio://
"#;

        let servers = vec![ServiceServerSnapshot {
            id: "server-open".to_string(),
            name: "Open".to_string(),
            slug: "open-have".to_string(),
            selected: true,
            machine_id: Some("machine-open".to_string()),
            machine_name: Some("Open machine".to_string()),
            machine_status: "offline".to_string(),
            api_key_ready: true,
            api_key_prefix: None,
        }];

        let process =
            selected_service_daemon_process_from_server_snapshots(&settings, &servers, output);

        assert!(process.is_none());
    }

    #[test]
    fn existing_machine_selection_prefers_bound_machine_before_creating_new_one() {
        let binding = ServiceMachineBinding {
            server_id: "server".to_string(),
            server_slug: "slug".to_string(),
            machine_id: "bound".to_string(),
            machine_name: "Bound machine".to_string(),
            api_key: String::new(),
            source: String::new(),
        };
        let machines = vec![
            ApiMachine {
                id: "other".to_string(),
                name: "Other machine".to_string(),
                status: "online".to_string(),
                api_key_prefix: String::new(),
            },
            ApiMachine {
                id: "bound".to_string(),
                name: "Bound machine".to_string(),
                status: "offline".to_string(),
                api_key_prefix: String::new(),
            },
        ];

        let selected = select_existing_machine(Some(&binding), &machines);

        assert_eq!(
            selected.map(|machine| machine.id),
            Some("bound".to_string())
        );
    }

    #[test]
    fn existing_machine_selection_ignores_existing_machine_without_binding() {
        let machines = vec![ApiMachine {
            id: "existing".to_string(),
            name: "Existing machine".to_string(),
            status: "offline".to_string(),
            api_key_prefix: String::new(),
        }];

        let selected = select_existing_machine(None, &machines);

        assert!(selected.is_none());
    }

    #[test]
    fn existing_machine_selection_ignores_stale_binding_name_match() {
        let binding = ServiceMachineBinding {
            server_id: "server".to_string(),
            server_slug: "slug".to_string(),
            machine_id: "missing".to_string(),
            machine_name: "Slock Desktop".to_string(),
            api_key: String::new(),
            source: String::new(),
        };
        let machines = vec![ApiMachine {
            id: "other".to_string(),
            name: "Slock Desktop".to_string(),
            status: "online".to_string(),
            api_key_prefix: "sk_other".to_string(),
        }];

        let selected = select_existing_machine(Some(&binding), &machines);

        assert!(selected.is_none());
    }

    #[test]
    fn machine_snapshot_leaves_unbound_servers_unlinked() {
        let machines = vec![ApiMachine {
            id: "existing".to_string(),
            name: "Existing machine".to_string(),
            status: "online".to_string(),
            api_key_prefix: "sk_existing".to_string(),
        }];

        let (machine_id, machine_name, machine_status, api_key_ready, api_key_prefix, binding_source) =
            service_server_machine_fields(None, &machines);

        assert_eq!(machine_id, None);
        assert_eq!(machine_name, None);
        assert_eq!(machine_status, "not linked");
        assert!(!api_key_ready);
        assert_eq!(api_key_prefix, None);
        assert_eq!(binding_source, "");
    }

    #[test]
    fn close_app_behavior_sanitizes_actions_and_settings() {
        assert_eq!(
            close_app_behavior_from_action("keepServer"),
            Some(CloseAppServiceBehavior::Keep)
        );
        assert_eq!(
            close_app_behavior_from_action("closeServer"),
            Some(CloseAppServiceBehavior::Stop)
        );
        assert_eq!(close_app_behavior_from_action("cancel"), None);
        assert_eq!(
            close_app_behavior_id(close_app_behavior_from_id("keep")),
            "keep"
        );
        assert_eq!(
            close_app_behavior_id(close_app_behavior_from_id("stop")),
            "stop"
        );
        assert_eq!(
            close_app_behavior_id(close_app_behavior_from_id("later")),
            "ask"
        );

        let sanitized = sanitize_service_settings(ServiceSettings {
            close_app_behavior: "later".to_string(),
            machines: vec![ServiceMachineBinding {
                server_id: "server".to_string(),
                server_slug: "open-have".to_string(),
                machine_id: "machine".to_string(),
                machine_name: "Machine".to_string(),
                api_key: "sk_rotating".to_string(),
                source: String::new(),
            }],
            ..ServiceSettings::default()
        });
        assert_eq!(sanitized.close_app_behavior, "ask");
        assert_eq!(sanitized.machines.len(), 1);
        assert_eq!(sanitized.machines[0].api_key, "");
    }

    #[test]
    fn saved_account_sanitizer_drops_token_only_entries() {
        let sanitized = sanitize_saved_accounts(vec![
            SavedAccountSettings {
                id: "token-only-a".to_string(),
                access_token: "opaque-token-a".to_string(),
                refresh_token: "refresh-a".to_string(),
                ..SavedAccountSettings::default()
            },
            SavedAccountSettings {
                id: "known".to_string(),
                access_token: "opaque-token-known".to_string(),
                refresh_token: "refresh-known".to_string(),
                email: " user@example.com ".to_string(),
                ..SavedAccountSettings::default()
            },
            SavedAccountSettings {
                id: "token-only-b".to_string(),
                access_token: "opaque-token-b".to_string(),
                refresh_token: "refresh-b".to_string(),
                ..SavedAccountSettings::default()
            },
        ]);

        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].id, "known");
        assert_eq!(sanitized[0].email, "user@example.com");

        let snapshots = session_account_snapshots(&sanitized);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn saved_account_upsert_waits_for_account_identity() {
        let mut session = SessionSettings {
            access_token: "opaque-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            ..SessionSettings::default()
        };

        upsert_saved_session_account(&mut session);
        assert!(session.accounts.is_empty());

        session.email = "user@example.com".to_string();
        upsert_saved_session_account(&mut session);
        assert_eq!(session.accounts.len(), 1);
        assert_eq!(session.accounts[0].email, "user@example.com");
    }

    #[test]
    fn close_app_prompt_script_wires_decision_actions() {
        let script = app_close_prompt_script(&CloseAppPromptCopy {
            title: "Quit Slock",
            description: "After Slock quits, the server can stay running or close with the app."
                .to_string(),
            server_label: "Current server: open-have".to_string(),
            keep_server: "Keep server running and quit",
            close_server: "Close server and quit",
            cancel: "Cancel",
            remember: "Remember this choice",
            processing_keep_server: "Keeping server running and quitting…",
            processing_close_server: "Closing server and quitting…",
            error: "Close handling failed. Try again.",
        });

        assert!(script.contains("slock-desktop-close-host"));
        assert!(script.contains("data-close-remember"));
        assert!(script.contains("data-close-action=\"keepServer\""));
        assert!(script.contains("data-close-action=\"closeServer\""));
        assert!(script.contains("resolve_app_close_request"));
        assert!(script.contains("data-close-busy"));
        assert!(script.contains("__slockDesktopCloseSetBusy"));
        assert!(script.contains("__slockDesktopCloseSetError"));
        assert!(script.contains("surfaceLooksDark"));
        assert!(script.contains("document.querySelector(\".studio-shell\")"));
        assert!(script.contains("prefers-color-scheme: dark"));
        assert!(script.contains("secondaryBg: \"#252b25\""));
        assert!(script.contains("dangerText: \"#fecaca\""));
        assert!(script.contains("appearance:none;-webkit-appearance:none"));
    }

    #[test]
    fn workspace_session_seed_script_restores_tokens_and_target_route() {
        let script = workspace_session_seed_script(&WorkspaceSessionSeed {
            access_token: "access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            target_url: "https://app.slock.ai/s/open-have".to_string(),
        });

        assert!(script.contains("localStorage.setItem(\"slock_access_token\", accessToken)"));
        assert!(script.contains("localStorage.setItem(\"slock_refresh_token\", refreshToken)"));
        assert!(script.contains("new StorageEvent(\"storage\""));
        assert!(script.contains("new BroadcastChannel(\"slock-auth-tokens\")"));
        assert!(script.contains("slock_desktop_session_seed_reload"));
        assert!(script.contains("window.location.replace(target.href)"));
        assert!(script.contains("\"https://app.slock.ai/s/open-have\""));
    }

    #[test]
    fn workspace_session_clear_script_removes_stored_tokens() {
        let script = workspace_session_clear_script();

        assert!(script.contains("localStorage.removeItem(\"slock_access_token\")"));
        assert!(script.contains("localStorage.removeItem(\"slock_refresh_token\")"));
        assert!(script.contains("delete window.__slockDesktopSessionSignature"));
        assert!(script.contains("slock_desktop_session_seed_reload"));
        assert!(script.contains("tokens-cleared"));
    }
}
