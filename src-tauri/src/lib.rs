mod config;
mod theme;
mod workspace;

use config::{
    load_settings, save_settings, AppSettings, CustomThemeSettings, ServiceMachineBinding,
    ServiceSettings,
};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
#[cfg(debug_assertions)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
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
    AppHandle, LogicalSize, Manager, RunEvent, State, Theme, Url, WindowEvent,
};
use tauri_plugin_updater::UpdaterExt;
use theme::{meta_catalog, resolve_theme, sanitize_hex, CustomThemeItem, CustomThemeSet};

const MAIN_LABEL: &str = "main";
const WORKSPACE_URL: &str = "https://app.slock.ai";
const DEFAULT_SERVER_URL: &str = "https://api.slock.ai";
const DAEMON_PACKAGE: &str = "@slock-ai/daemon@latest";
const DAEMON_MACHINE_NAME: &str = "Slock Desktop";
const LAUNCHER_WINDOW_WIDTH: f64 = 800.0;
const LAUNCHER_WINDOW_HEIGHT: f64 = 460.0;
const LAUNCHER_WINDOW_MIN_WIDTH: f64 = 720.0;
const LAUNCHER_WINDOW_MIN_HEIGHT: f64 = 420.0;
const WORKSPACE_WINDOW_WIDTH: f64 = 1480.0;
const WORKSPACE_WINDOW_HEIGHT: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_WIDTH: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_HEIGHT: f64 = 760.0;
const DAEMON_SERVER_SLUG_ARG: &str = "--slock-desktop-server-slug";
const DAEMON_MACHINE_ID_ARG: &str = "--slock-desktop-machine-id";
const DAEMON_DESKTOP_MANAGED_ARG: &str = "--slock-desktop-managed";
const DESKTOP_RELEASE_REPOSITORY: &str = "codephage2020/slock-desktop";
const DESKTOP_UPDATER_ENDPOINT: &str =
    "https://github.com/codephage2020/slock-desktop/releases/latest/download/latest.json";
const WORKSPACE_SERVICE_START_DELAY_MS: u64 = 750;
#[cfg(debug_assertions)]
const WORKSPACE_LAUNCH_LOG_PATH: &str = "/tmp/slock-desktop-launch.log";

pub struct DesktopState {
    settings: Mutex<AppSettings>,
    service: Mutex<ServiceRuntime>,
    app_close: Mutex<AppCloseRuntime>,
    launch_metrics: Mutex<WorkspaceLaunchMetrics>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    app_name: String,
    workspace_url: String,
    color_scheme: String,
    appearance_mode: String,
    custom_themes: Vec<CustomThemeSettings>,
    language: String,
    resolved_language: String,
    workspace_open: bool,
    themes: Vec<theme::ThemeMeta>,
    service: ServiceSnapshot,
    updates: UpdateSnapshot,
}

struct ServiceRuntime {
    child: Option<Child>,
    last_error: Option<String>,
    active_server_slug: Option<String>,
    active_machine_id: Option<String>,
    cached_servers: Vec<ServiceServerSnapshot>,
    cached_sync_error: Option<String>,
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
    configured: bool,
    running: bool,
    pid: Option<u32>,
    last_error: Option<String>,
    sync_error: Option<String>,
    servers: Vec<ServiceServerSnapshot>,
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
    #[serde(skip_serializing)]
    api_key_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSnapshot {
    current_version: String,
    latest_release_api_url: String,
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
    let (color_scheme, appearance_mode, custom_themes, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let theme = resolve_theme(
            &theme_id,
            &settings.appearance_mode,
            &custom_theme_set(&settings.custom_themes),
        );
        settings.color_scheme = theme.id.clone();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_theme_mode(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_mode: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, appearance_mode, custom_themes, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.appearance_mode = theme::normalize_mode(&theme_mode).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn create_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    name: String,
    accent: String,
) -> Result<BootstrapPayload, String> {
    let (appearance_mode, language, custom_themes, new_id) = {
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
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            id,
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&new_id, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

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
    let (appearance_mode, language, custom_themes, color_scheme) = {
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
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            settings.color_scheme.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn delete_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    id: String,
) -> Result<BootstrapPayload, String> {
    let (appearance_mode, language, custom_themes, color_scheme) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.custom_themes.retain(|item| item.id != id);
        if settings.color_scheme == id {
            settings.color_scheme = "original".to_string();
        }
        save_settings(&app, &settings)?;
        (
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_themes.clone(),
            settings.color_scheme.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_language(
    app: AppHandle,
    state: State<'_, DesktopState>,
    language: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, appearance_mode, custom_themes, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.language = sanitize_language(&language).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_set(&custom_themes);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn save_session_tokens(
    app: AppHandle,
    state: State<'_, DesktopState>,
    access_token: String,
    refresh_token: String,
) -> Result<(), String> {
    let access_token = access_token.trim().to_string();
    let refresh_token = refresh_token.trim().to_string();
    if access_token.is_empty() || refresh_token.is_empty() {
        return Ok(());
    }

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    if settings.session.access_token == access_token
        && settings.session.refresh_token == refresh_token
    {
        return Ok(());
    }

    settings.session.access_token = access_token;
    settings.session.refresh_token = refresh_token;
    save_settings(&app, &settings)?;
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.cached_servers.clear();
    runtime.cached_sync_error = None;
    Ok(())
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

    let (color_scheme, appearance_mode, custom_themes, language, selected_server_slug) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_themes.clone(),
            settings.language.clone(),
            settings.service.selected_server_slug.clone(),
        )
    };
    let target_url = workspace_url_for_slug(&selected_server_slug);
    begin_workspace_launch_trace(&state, command_started, &target_url);

    enter_workspace_in_main_window(
        &app,
        &state,
        &color_scheme,
        &appearance_mode,
        &language,
        &custom_theme_set(&custom_themes),
        &selected_server_slug,
    )?;
    start_workspace_service_in_background(app.clone(), service_settings);
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn select_service_server(
    app: AppHandle,
    state: State<'_, DesktopState>,
    selected_server_slug: String,
) -> Result<BootstrapPayload, String> {
    persist_service_target_slug(&app, &state, Some(selected_server_slug), false)?;
    build_bootstrap(&app, &state, false)
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
    build_bootstrap(&app, &state, true)
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
    build_bootstrap(&app, &state, true)
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

    if behavior == CloseAppServiceBehavior::Stop {
        stop_service_process(&app, &state, Some(&service_settings), None)?;
    }

    mark_app_close_confirmed(&state);
    app.exit(0);
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

    stop_service_process(&app, &state, Some(&service_settings), None)?;
    force_start_service(&app, &state, &service_settings)?;
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
async fn install_desktop_update(app: AppHandle) -> Result<(), String> {
    let pubkey = desktop_updater_public_key()?;
    let endpoint = Url::parse(DESKTOP_UPDATER_ENDPOINT).map_err(|err| err.to_string())?;
    let updater = app
        .updater_builder()
        .pubkey(pubkey)
        .timeout(Duration::from_secs(30))
        .endpoints(vec![endpoint])
        .map_err(|err| err.to_string())?
        .build()
        .map_err(|err| err.to_string())?;

    let Some(update) = updater.check().await.map_err(|err| err.to_string())? else {
        return Err("Slock Desktop is already up to date.".to_string());
    };

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|err| err.to_string())?;
    app.restart()
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
            apply_remembered_close_behavior(app, state, behavior);
            mark_app_close_confirmed(state);
            app.exit(0);
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
        behavior => {
            apply_remembered_close_behavior(app, state, behavior);
            true
        }
    }
}

fn handle_app_exit(app: &AppHandle, state: &DesktopState) {
    if current_close_app_behavior(state) == CloseAppServiceBehavior::Stop {
        let service_settings = state
            .settings
            .lock()
            .ok()
            .map(|settings| settings.service.clone());
        let _ = stop_service_process(app, state, service_settings.as_ref(), None);
    }
}

fn apply_remembered_close_behavior(
    app: &AppHandle,
    state: &DesktopState,
    behavior: CloseAppServiceBehavior,
) {
    if behavior != CloseAppServiceBehavior::Stop {
        return;
    }
    let service_settings = state
        .settings
        .lock()
        .ok()
        .map(|settings| settings.service.clone());
    let _ = stop_service_process(app, state, service_settings.as_ref(), None);
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

fn desktop_updater_public_key() -> Result<&'static str, String> {
    option_env!("SLOCK_DESKTOP_UPDATER_PUBKEY")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Slock Desktop updater public key is not configured. Set SLOCK_DESKTOP_UPDATER_PUBKEY when building the app.".to_string()
        })
}

fn build_bootstrap(
    app: &AppHandle,
    state: &State<'_, DesktopState>,
    refresh_service: bool,
) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .clone();

    let service = collect_service_snapshot(app, state, &settings.service, refresh_service)?;
    let appearance_mode = theme::normalize_mode(&settings.appearance_mode).to_string();
    let updates = UpdateSnapshot {
        current_version: app.package_info().version.to_string(),
        latest_release_api_url: format!(
            "https://api.github.com/repos/{}/releases/latest",
            DESKTOP_RELEASE_REPOSITORY
        ),
    };

    Ok(BootstrapPayload {
        app_name: "slock-desktop".to_string(),
        workspace_url: workspace_url_for_slug(&settings.service.selected_server_slug),
        color_scheme: settings.color_scheme.clone(),
        appearance_mode: appearance_mode.clone(),
        custom_themes: settings.custom_themes.clone(),
        language: sanitize_language(&settings.language).to_string(),
        resolved_language: resolve_desktop_language(&settings.language).to_string(),
        workspace_open: main_window_is_workspace(app),
        themes: meta_catalog(&appearance_mode, &custom_theme_set(&settings.custom_themes)),
        service,
        updates,
    })
}

fn enter_workspace_in_main_window(
    app: &AppHandle,
    state: &DesktopState,
    theme_id: &str,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeSet,
    selected_server_slug: &str,
) -> Result<(), String> {
    let theme = resolve_theme(theme_id, theme_mode, custom_theme);
    let resolved_language = resolve_desktop_language(language);
    let target_url = workspace_url_for_slug(selected_server_slug)
        .parse::<Url>()
        .map_err(|err| err.to_string())?;
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())?;

    if window_is_workspace(&window) {
        let _ = window.unminimize();
        let _ = window.show();
        apply_workspace_window_size(&window);
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
            theme_mode,
            language,
            resolved_language,
            custom_theme,
        );
    }

    apply_window_language(app, &window, language, true);
    apply_window_theme(&window, theme_mode);
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

fn apply_workspace_window_size(window: &tauri::WebviewWindow) {
    let _ = window.set_min_size(Some(LogicalSize::new(
        WORKSPACE_WINDOW_MIN_WIDTH,
        WORKSPACE_WINDOW_MIN_HEIGHT,
    )));
    let _ = window.set_size(LogicalSize::new(
        WORKSPACE_WINDOW_WIDTH,
        WORKSPACE_WINDOW_HEIGHT,
    ));
    let _ = window.center();
}

fn apply_workspace_titlebar_style(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Visible);
    }
}

fn apply_workspace_window_size_to_window(window: &tauri::Window) {
    let _ = window.set_min_size(Some(LogicalSize::new(
        WORKSPACE_WINDOW_MIN_WIDTH,
        WORKSPACE_WINDOW_MIN_HEIGHT,
    )));
    let _ = window.set_size(LogicalSize::new(
        WORKSPACE_WINDOW_WIDTH,
        WORKSPACE_WINDOW_HEIGHT,
    ));
    let _ = window.center();
}

fn apply_workspace_titlebar_style_to_window(window: &tauri::Window) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_title_bar_style(tauri::TitleBarStyle::Visible);
    }
}

fn apply_theme_to_workspace(
    app: &AppHandle,
    theme: theme::ThemeDefinition,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeSet,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        apply_window_theme(&window, theme_mode);
        apply_window_language(app, &window, language, window_is_workspace(&window));
        if window_is_workspace(&window) {
            let active_theme_id = theme.id.clone();
            apply_workspace_scripts_to_window(
                &window,
                theme,
                &active_theme_id,
                theme_mode,
                language,
                resolve_desktop_language(language),
                custom_theme,
            )?;
        }
    }

    Ok(())
}

fn apply_window_theme(window: &tauri::WebviewWindow, theme_mode: &str) {
    let normalized_mode = theme::normalize_mode(theme_mode);
    let native_theme = match normalized_mode {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };
    let _ = window.set_theme(native_theme);

    let effective_dark = normalized_mode == "dark"
        || (normalized_mode == "system" && matches!(window.theme(), Ok(Theme::Dark)));
    let background = if effective_dark {
        Color(37, 38, 35, 255)
    } else {
        Color(255, 255, 255, 255)
    };
    let _ = window.set_background_color(Some(background));
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
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    custom_theme: &CustomThemeSet,
) -> Result<(), String> {
    window
        .eval(theme::injected_script(theme))
        .map_err(|err| err.to_string())?;
    window
        .eval(workspace::settings_overlay_script(
            active_theme_id,
            active_theme_mode,
            active_language,
            resolved_language,
            &meta_catalog(active_theme_mode, custom_theme),
        ))
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
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    custom_theme: &CustomThemeSet,
) -> Result<(), String> {
    webview
        .eval(theme::injected_script(theme))
        .map_err(|err| err.to_string())?;
    webview
        .eval(workspace::settings_overlay_script(
            active_theme_id,
            active_theme_mode,
            active_language,
            resolved_language,
            &meta_catalog(active_theme_mode, custom_theme),
        ))
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
            }
            Ok(None) => {
                running = true;
                pid = Some(child.id());
            }
            Err(err) => {
                runtime.last_error = Some(format!("Service state check failed: {err}"));
                runtime.child = None;
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

    for server in &mut servers {
        server.selected = server.slug == settings.selected_server_slug;
    }

    if refresh_service && !running {
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

fn maybe_start_service(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    force_for_workspace: bool,
) -> Result<(), String> {
    if should_start_service_for_workspace(settings, force_for_workspace) {
        force_start_service(app, state, settings)?;
    }

    Ok(())
}

fn should_start_service_for_workspace(
    settings: &ServiceSettings,
    force_for_workspace: bool,
) -> bool {
    !settings.selected_server_slug.trim().is_empty()
        && (force_for_workspace || settings.auto_start_with_workspace)
}

fn start_workspace_service_in_background(app: AppHandle, settings: ServiceSettings) {
    if settings.selected_server_slug.trim().is_empty() {
        return;
    }

    thread::spawn(move || {
        sleep(Duration::from_millis(WORKSPACE_SERVICE_START_DELAY_MS));
        let state = app.state::<DesktopState>();
        if let Err(err) = maybe_start_service(&app, &state, &settings, true) {
            log::warn!("failed to start workspace service in background: {err}");
            if let Ok(mut runtime) = state.service.lock() {
                runtime.last_error = Some(err);
            }
        }
    });
}

fn force_start_service(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<(), String> {
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

    let service_command = resolve_service_command()?;
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
        .env("PATH", &service_command.path_env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command.spawn().map_err(|err| {
        format!(
            "Failed to start service with {}: {err}",
            service_command.executable.display()
        )
    })?;
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    runtime.last_error = None;
    runtime.active_server_slug = Some(selected_server.slug);
    runtime.active_machine_id = Some(binding.machine_id);
    runtime.child = Some(child);
    Ok(())
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
    Ok(())
}

fn stop_service_process(
    app: &AppHandle,
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
        }
    }

    let resolved_target = if stopped_daemon_process {
        None
    } else {
        match (service_settings, target_slug) {
            (Some(settings), Some(slug)) => {
                resolve_service_machine_for_slug(app, state, settings, slug)?
            }
            _ => None,
        }
    };
    let target_binding = resolved_target
        .as_ref()
        .map(|target| target.binding.clone())
        .or_else(|| {
            service_settings.and_then(|settings| {
                target_slug.and_then(|slug| find_service_binding(settings, "", slug))
            })
        });
    daemon_pids = if stopped_daemon_process {
        Vec::new()
    } else {
        find_daemon_process_ids(
            target_server_url,
            target_slug,
            target_binding
                .as_ref()
                .map(|binding| binding.machine_id.as_str())
                .filter(|machine_id| !machine_id.trim().is_empty()),
            resolved_target
                .as_ref()
                .and_then(|target| target.api_key_prefix.as_deref()),
            target_binding
                .as_ref()
                .map(|binding| binding.api_key.as_str())
                .filter(|api_key| !api_key.trim().is_empty()),
            false,
        )?
    };
    if daemon_pids.is_empty()
        && resolved_target
            .as_ref()
            .map(|target| machine_counts_as_started(&target.machine_status))
            .unwrap_or(false)
    {
        daemon_pids = unique_untagged_daemon_process_ids(target_server_url)?;
    }
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
        }
        return Err("Selected server service is not running.".to_string());
    }

    if should_clear_runtime {
        runtime.last_error = None;
        runtime.active_server_slug = None;
        runtime.active_machine_id = None;
    }
    Ok(())
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
        if daemon_command_is_desktop_managed(command)
            && target_server_slug
                .filter(|server_slug| !server_slug.trim().is_empty())
                .map(|server_slug| {
                    command_arg_value_matches(command, DAEMON_SERVER_SLUG_ARG, server_slug)
                })
                .unwrap_or(false)
        {
            return true;
        }

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

fn api_base_url(server_url: &str) -> String {
    format!("{}/api", sanitize_service_server_url(server_url))
}

static API_CLIENT: OnceLock<Client> = OnceLock::new();

fn api_client() -> Result<Client, String> {
    if let Some(client) = API_CLIENT.get() {
        return Ok(client.clone());
    }
    let client = Client::builder()
        .user_agent("Slock Desktop")
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Unable to create desktop API client: {err}"))?;
    let _ = API_CLIENT.set(client.clone());
    Ok(client)
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
        return Err(
            "Open Slock once and sign in, then the desktop launcher can load your server list."
                .to_string(),
        );
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

    let machines_by_server: Vec<Vec<ApiMachine>> = std::thread::scope(|scope| {
        let handles: Vec<_> = servers
            .iter()
            .map(|server| {
                let server_id = server.id.clone();
                let server_url = server_url.clone();
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

    let mut snapshots = Vec::with_capacity(servers.len());
    for (server, machines) in servers.into_iter().zip(machines_by_server) {
        let binding = find_service_binding(settings, &server.id, &server.slug);
        let bound_machine = binding.as_ref().and_then(|binding| {
            machines
                .iter()
                .find(|machine| machine.id == binding.machine_id)
        });
        let active_machine = bound_machine
            .or_else(|| {
                machines
                    .iter()
                    .find(|machine| machine_counts_as_started(&machine.status))
            })
            .or_else(|| machines.first());
        let machine_status = active_machine
            .map(|machine| normalize_machine_status(&machine.status))
            .unwrap_or_else(|| {
                if binding.is_some() {
                    "offline".to_string()
                } else {
                    "not linked".to_string()
                }
            });
        let api_key_prefix = active_machine
            .map(|machine| machine.api_key_prefix.trim().to_string())
            .filter(|prefix| !prefix.is_empty());

        snapshots.push(ServiceServerSnapshot {
            id: server.id.clone(),
            name: server.name,
            slug: server.slug.clone(),
            selected: server.slug == settings.selected_server_slug,
            machine_id: binding
                .as_ref()
                .map(|item| item.machine_id.clone())
                .or_else(|| active_machine.map(|machine| machine.id.clone())),
            machine_name: bound_machine
                .map(|machine| machine.name.clone())
                .or_else(|| {
                    binding
                        .as_ref()
                        .map(|item| item.machine_name.clone())
                        .filter(|name| !name.is_empty())
                })
                .or_else(|| active_machine.map(|machine| machine.name.clone())),
            machine_status,
            api_key_ready: api_key_prefix.is_some()
                || binding
                    .as_ref()
                    .map(|item| !item.machine_id.trim().is_empty())
                    .unwrap_or(false),
            api_key_prefix,
        });
    }

    Ok(snapshots)
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
        });
    }

    Ok(snapshots)
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

fn resolve_service_machine_for_slug(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    server_slug: &str,
) -> Result<Option<ResolvedServiceMachine>, String> {
    let server = resolve_service_server(app, state, settings, server_slug)?;
    resolve_existing_service_machine(app, state, settings, &server)
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
    if let Some(binding) = binding {
        if let Some(machine) = machines
            .iter()
            .find(|machine| machine.id == binding.machine_id)
        {
            return Some(machine.clone());
        }
        let binding_name = binding.machine_name.trim();
        if !binding_name.is_empty() {
            if let Some(machine) = machines.iter().find(|machine| machine.name == binding_name) {
                return Some(machine.clone());
            }
        }
    }

    machines
        .iter()
        .find(|machine| machine_counts_as_started(&machine.status))
        .or_else(|| machines.first())
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

fn normalize_app_settings(settings: AppSettings) -> AppSettings {
    let appearance_mode = theme::normalize_mode(&settings.appearance_mode).to_string();
    let custom_themes = sanitize_custom_themes(settings.custom_themes);
    let color_scheme = resolve_theme(
        &settings.color_scheme,
        &appearance_mode,
        &custom_theme_set(&custom_themes),
    )
    .id;

    AppSettings {
        color_scheme,
        appearance_mode,
        custom_themes,
        language: sanitize_language(&settings.language).to_string(),
        session: config::SessionSettings {
            access_token: settings.session.access_token.trim().to_string(),
            refresh_token: settings.session.refresh_token.trim().to_string(),
        },
        service: sanitize_service_settings(settings.service),
    }
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(DesktopState {
            settings: Mutex::new(AppSettings::default()),
            service: Mutex::new(ServiceRuntime {
                child: None,
                last_error: None,
                active_server_slug: None,
                active_machine_id: None,
                cached_servers: Vec::new(),
                cached_sync_error: None,
            }),
            app_close: Mutex::new(AppCloseRuntime::default()),
            launch_metrics: Mutex::new(WorkspaceLaunchMetrics::default()),
        })
        .on_page_load(|webview, payload| {
            if webview.label() != MAIN_LABEL || !is_workspace_url(payload.url()) {
                return;
            }

            if matches!(payload.event(), PageLoadEvent::Started) {
                let state = webview.state::<DesktopState>();
                mark_workspace_launch_page_started(&state, payload.url());
                if let Err(err) = apply_workspace_session_seed_to_webview(webview, &state) {
                    log::warn!("failed to seed workspace session: {err}");
                }
                apply_workspace_titlebar_style_to_window(&webview.window());
                apply_workspace_window_size_to_window(&webview.window());
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

            let (color_scheme, appearance_mode, custom_themes, language) = webview
                .state::<DesktopState>()
                .settings
                .lock()
                .map(|settings| {
                    (
                        settings.color_scheme.clone(),
                        settings.appearance_mode.clone(),
                        settings.custom_themes.clone(),
                        settings.language.clone(),
                    )
                })
                .unwrap_or_else(|_| {
                    (
                        "original".to_string(),
                        "system".to_string(),
                        Vec::<CustomThemeSettings>::new(),
                        "system".to_string(),
                    )
                });
            let custom = custom_theme_set(&custom_themes);
            let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);

            if let Err(err) = apply_workspace_scripts_to_webview(
                webview,
                theme,
                &color_scheme,
                &appearance_mode,
                &language,
                resolve_desktop_language(&language),
                &custom,
            ) {
                log::error!("failed to apply workspace desktop scripts: {err}");
            }
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
                apply_window_theme(&window, &appearance_mode);
                apply_launcher_titlebar_style(&window);
                apply_launcher_window_size(&window);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_theme,
            set_theme_mode,
            create_custom_theme,
            rename_custom_theme,
            update_custom_theme_accent,
            delete_custom_theme,
            set_language,
            save_session_tokens,
            open_workspace,
            save_service_settings,
            refresh_service_servers,
            refresh_service_server_catalog,
            select_service_server,
            start_service,
            stop_service,
            resolve_app_close_request,
            update_service,
            install_desktop_update
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
        app_close_prompt_script, close_app_behavior_from_action, close_app_behavior_from_id,
        close_app_behavior_id, daemon_command_matches, daemon_pids_from_ps_output,
        prepare_runtime_for_service_target, process_entries_from_ps_output,
        process_tree_pids_from_entries, resolve_service_command_from_dirs,
        sanitize_service_settings, select_existing_machine,
        selected_service_daemon_process_from_server_snapshots,
        service_daemon_process_from_resolved_target, should_refresh_service_servers,
        should_start_service_for_workspace, terminate_daemon_process,
        untagged_daemon_pids_from_ps_output, workspace_session_seed_script, ApiMachine,
        AppCloseRuntime, CloseAppPromptCopy, CloseAppServiceBehavior, DesktopState,
        ResolvedServiceMachine, ServiceRuntime, ServiceServerSnapshot, WorkspaceLaunchMetrics,
        WorkspaceSessionSeed,
    };
    use crate::config::{AppSettings, ServiceMachineBinding, ServiceSettings};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::Command,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
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
    fn desktop_managed_daemon_parser_matches_slug_when_machine_binding_is_stale() {
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
                cached_servers: Vec::new(),
                cached_sync_error: None,
            }),
            app_close: Mutex::new(AppCloseRuntime::default()),
            launch_metrics: Mutex::new(WorkspaceLaunchMetrics::default()),
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
    fn workspace_launch_starts_selected_service_even_when_machine_appears_online() {
        let settings = ServiceSettings {
            selected_server_slug: "open-have".to_string(),
            auto_start_with_workspace: false,
            ..ServiceSettings::default()
        };

        assert!(should_start_service_for_workspace(&settings, true));
    }

    #[test]
    fn initial_bootstrap_can_skip_service_network_refresh() {
        assert!(!should_refresh_service_servers(false, true));
        assert!(!should_refresh_service_servers(false, false));
        assert!(should_refresh_service_servers(true, true));
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
                },
                ServiceMachineBinding {
                    server_id: "server-tyan".to_string(),
                    server_slug: "tyan-dyun".to_string(),
                    machine_id: "machine-tyan".to_string(),
                    machine_name: "Tyan machine".to_string(),
                    api_key: "sk_machine_tyan".to_string(),
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
    fn existing_machine_selection_reuses_any_existing_machine_without_binding() {
        let machines = vec![ApiMachine {
            id: "existing".to_string(),
            name: "Existing machine".to_string(),
            status: "offline".to_string(),
            api_key_prefix: String::new(),
        }];

        let selected = select_existing_machine(None, &machines);

        assert_eq!(
            selected.map(|machine| machine.id),
            Some("existing".to_string())
        );
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
            }],
            ..ServiceSettings::default()
        });
        assert_eq!(sanitized.close_app_behavior, "ask");
        assert_eq!(sanitized.machines.len(), 1);
        assert_eq!(sanitized.machines[0].api_key, "");
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
}
