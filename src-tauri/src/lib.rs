mod config;
mod theme;
mod workspace;

use config::{
    load_settings, save_settings, AppSettings, CustomThemeSettings, ServiceMachineBinding,
    ServiceSettings, UpdateSettings,
};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    env,
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread::sleep,
    time::Duration,
};
use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    webview::PageLoadEvent,
    window::Color,
    AppHandle, LogicalSize, Manager, RunEvent, State, Theme, Url,
};
use theme::{meta_catalog, resolve_theme, CustomThemeInput};

const MAIN_LABEL: &str = "main";
const WORKSPACE_URL: &str = "https://app.slock.ai";
const DEFAULT_SERVER_URL: &str = "https://api.slock.ai";
const DAEMON_PACKAGE: &str = "@slock-ai/daemon@latest";
const DAEMON_MACHINE_NAME: &str = "Slock Desktop";
const LAUNCHER_WINDOW_WIDTH: f64 = 980.0;
const LAUNCHER_WINDOW_HEIGHT: f64 = 720.0;
const LAUNCHER_WINDOW_MIN_WIDTH: f64 = 920.0;
const LAUNCHER_WINDOW_MIN_HEIGHT: f64 = 680.0;
const WORKSPACE_WINDOW_WIDTH: f64 = 1480.0;
const WORKSPACE_WINDOW_HEIGHT: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_WIDTH: f64 = 980.0;
const WORKSPACE_WINDOW_MIN_HEIGHT: f64 = 760.0;

pub struct DesktopState {
    settings: Mutex<AppSettings>,
    service: Mutex<ServiceRuntime>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    app_name: String,
    workspace_url: String,
    color_scheme: String,
    appearance_mode: String,
    custom_theme: CustomThemeSettings,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceSnapshot {
    server_url: String,
    selected_server_slug: String,
    active_server_slug: String,
    auto_start_with_workspace: bool,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSnapshot {
    current_version: String,
    repository_slug: String,
    releases_url: String,
    latest_release_api_url: String,
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
fn bootstrap(app: AppHandle, state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn set_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_id: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, appearance_mode, custom_theme, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let theme = resolve_theme(
            &theme_id,
            &settings.appearance_mode,
            &custom_theme_input(&settings.custom_theme),
        );
        settings.color_scheme = theme.id.clone();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
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
    let (color_scheme, appearance_mode, custom_theme, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.appearance_mode = theme::normalize_mode(&theme_mode).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
    let theme = resolve_theme(&color_scheme, &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn save_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    custom_theme: CustomThemeSettings,
) -> Result<BootstrapPayload, String> {
    let (appearance_mode, language, saved_custom_theme) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.custom_theme = sanitize_custom_theme(custom_theme);
        settings.color_scheme = "custom".to_string();
        save_settings(&app, &settings)?;
        (
            settings.appearance_mode.clone(),
            settings.language.clone(),
            settings.custom_theme.clone(),
        )
    };

    let custom = custom_theme_input(&saved_custom_theme);
    let theme = resolve_theme("custom", &appearance_mode, &custom);
    apply_theme_to_workspace(&app, theme, &appearance_mode, &language, &custom)?;

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn set_language(
    app: AppHandle,
    state: State<'_, DesktopState>,
    language: String,
) -> Result<BootstrapPayload, String> {
    let (color_scheme, appearance_mode, custom_theme, language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.language = sanitize_language(&language).to_string();
        save_settings(&app, &settings)?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
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
    persist_service_target_slug(&app, &state, selected_server_slug, false)?;
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };
    maybe_start_service(&app, &state, &service_settings, true)?;

    let (color_scheme, appearance_mode, custom_theme, language, selected_server_slug) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        (
            settings.color_scheme.clone(),
            settings.appearance_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
            settings.service.selected_server_slug.clone(),
        )
    };

    enter_workspace_in_main_window(
        &app,
        &color_scheme,
        &appearance_mode,
        &language,
        &custom_theme_input(&custom_theme),
        &selected_server_slug,
    )?;
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
) -> Result<BootstrapPayload, String> {
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
) -> Result<BootstrapPayload, String> {
    let service_settings = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.service.clone()
    };

    stop_service_process(&state, Some(&service_settings))?;
    build_bootstrap(&app, &state, true)
}

#[tauri::command]
fn refresh_service_servers(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    build_bootstrap(&app, &state, true)
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

    stop_service_process(&state, Some(&service_settings))?;
    force_start_service(&app, &state, &service_settings)?;
    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn save_update_settings(
    app: AppHandle,
    state: State<'_, DesktopState>,
    updates: UpdateSettings,
) -> Result<BootstrapPayload, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?;
    settings.updates = sanitize_update_settings(updates);
    save_settings(&app, &settings)?;
    drop(settings);

    build_bootstrap(&app, &state, false)
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open_url_in_browser(&url)
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
        repository_slug: settings.updates.repository_slug.clone(),
        releases_url: settings.updates.releases_url.clone(),
        latest_release_api_url: format!(
            "https://api.github.com/repos/{}/releases/latest",
            settings.updates.repository_slug
        ),
    };

    Ok(BootstrapPayload {
        app_name: "Slock Desktop".to_string(),
        workspace_url: workspace_url_for_slug(&settings.service.selected_server_slug),
        color_scheme: settings.color_scheme.clone(),
        appearance_mode: appearance_mode.clone(),
        custom_theme: settings.custom_theme.clone(),
        language: sanitize_language(&settings.language).to_string(),
        resolved_language: resolve_desktop_language(&settings.language).to_string(),
        workspace_open: main_window_is_workspace(app),
        themes: meta_catalog(
            &appearance_mode,
            &custom_theme_input(&settings.custom_theme),
        ),
        service,
        updates,
    })
}

fn enter_workspace_in_main_window(
    app: &AppHandle,
    theme_id: &str,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeInput,
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
        let _ = window.set_focus();
        apply_window_theme(&window, theme_mode);
        apply_window_language(app, &window, language, true);
        if window.url().ok().as_ref() != Some(&target_url) {
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
    apply_workspace_window_size(&window);
    let _ = window.set_focus();
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

fn apply_theme_to_workspace(
    app: &AppHandle,
    theme: theme::ThemeDefinition,
    theme_mode: &str,
    language: &str,
    custom_theme: &CustomThemeInput,
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
        ("zh-CN", false) => "Slock 桌面端",
        (_, true) => "Slock Workspace",
        (_, false) => "Slock Desktop",
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
    let copy = native_menu_copy(language);
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
    custom_theme: &CustomThemeInput,
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
    #[cfg(debug_assertions)]
    if let Err(err) = window.eval(workspace::agentation_script()) {
        log::warn!("failed to inject workspace Agentation: {err}");
    }
    Ok(())
}

fn apply_workspace_scripts_to_webview(
    webview: &tauri::Webview,
    theme: theme::ThemeDefinition,
    active_theme_id: &str,
    active_theme_mode: &str,
    active_language: &str,
    resolved_language: &str,
    custom_theme: &CustomThemeInput,
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
    #[cfg(debug_assertions)]
    if let Err(err) = webview.eval(workspace::agentation_script()) {
        log::warn!("failed to inject workspace Agentation: {err}");
    }
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
    let active_server_slug = runtime.active_server_slug.clone().unwrap_or_default();
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
            authenticated,
            configured: false,
            running,
            pid,
            last_error,
            sync_error: None,
            servers: Vec::new(),
        });
    }

    let refresh_needed = refresh_service || cached_servers.is_empty();
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
        authenticated,
        configured,
        running,
        pid,
        last_error,
        sync_error,
        servers,
    })
}

fn maybe_start_service(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    force_for_workspace: bool,
) -> Result<(), String> {
    let should_start = (!settings.selected_server_slug.trim().is_empty() && force_for_workspace)
        || (settings.auto_start_with_workspace && !settings.selected_server_slug.trim().is_empty());

    if should_start {
        if selected_server_is_started(app, state, settings)? {
            return Ok(());
        }
        force_start_service(app, state, settings)?;
    }

    Ok(())
}

fn force_start_service(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<(), String> {
    let selected_server = resolve_selected_server(app, state, settings)?;
    let binding = ensure_machine_binding(app, state, settings, &selected_server)?;

    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    let same_target = runtime.active_server_slug.as_deref() == Some(selected_server.slug.as_str())
        && runtime.active_machine_id.as_deref() == Some(binding.machine_id.as_str());

    if let Some(child) = runtime.child.as_mut() {
        let still_running = child
            .try_wait()
            .map_err(|err| format!("Unable to inspect service state: {err}"))?
            .is_none();
        if still_running && same_target {
            return Ok(());
        }

        if still_running {
            child
                .kill()
                .map_err(|err| format!("Failed to stop existing service: {err}"))?;
            let _ = child.wait();
        }
        runtime.child = None;
    }

    let mut command = Command::new("npx");
    command
        .args([
            "--yes",
            DAEMON_PACKAGE,
            "--server-url",
            settings.server_url.as_str(),
            "--api-key",
            binding.api_key.as_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .map_err(|err| format!("Failed to start service: {err}"))?;
    runtime.last_error = None;
    runtime.active_server_slug = Some(selected_server.slug);
    runtime.active_machine_id = Some(binding.machine_id);
    runtime.child = Some(child);
    Ok(())
}

fn stop_service_process(
    state: &DesktopState,
    service_settings: Option<&ServiceSettings>,
) -> Result<(), String> {
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;
    let active_server_slug = runtime.active_server_slug.clone();

    if let Some(child) = runtime.child.as_mut() {
        let still_running = child
            .try_wait()
            .map_err(|err| format!("Unable to inspect service state: {err}"))?
            .is_none();
        if still_running {
            child
                .kill()
                .map_err(|err| format!("Failed to stop service: {err}"))?;
        }
        let _ = child.wait();
    }

    let target_api_key = service_settings
        .and_then(|settings| {
            let target_slug = active_server_slug
                .as_deref()
                .filter(|slug| !slug.trim().is_empty())
                .unwrap_or_else(|| settings.selected_server_slug.as_str());
            find_service_binding(settings, "", target_slug)
        })
        .map(|binding| binding.api_key)
        .filter(|api_key| !api_key.trim().is_empty());
    let target_server_url = service_settings
        .map(|settings| settings.server_url.as_str())
        .unwrap_or(DEFAULT_SERVER_URL);

    let daemon_pids = find_daemon_process_ids(target_api_key.as_deref(), target_server_url)?;
    for pid in daemon_pids {
        terminate_daemon_process(pid)?;
    }

    runtime.child = None;
    runtime.last_error = None;
    runtime.active_server_slug = None;
    runtime.active_machine_id = None;
    Ok(())
}

fn find_daemon_process_ids(
    target_api_key: Option<&str>,
    target_server_url: &str,
) -> Result<Vec<u32>, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let output = Command::new("ps")
            .args(["-axo", "pid=,command="])
            .output()
            .map_err(|err| format!("Failed to inspect daemon processes: {err}"))?;
        if !output.status.success() {
            return Err("Failed to inspect daemon processes".to_string());
        }

        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(daemon_pids_from_ps_output(
            &listing,
            target_api_key,
            target_server_url,
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = target_api_key;
        let _ = target_server_url;
        Ok(Vec::new())
    }
}

fn daemon_pids_from_ps_output(
    output: &str,
    target_api_key: Option<&str>,
    target_server_url: &str,
) -> Vec<u32> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let split_index = trimmed.find(char::is_whitespace)?;
            let (pid_text, command_text) = trimmed.split_at(split_index);
            let pid = pid_text.parse::<u32>().ok()?;
            let command = command_text.trim();
            if daemon_command_matches(command, target_api_key, target_server_url) {
                Some(pid)
            } else {
                None
            }
        })
        .collect()
}

fn daemon_command_matches(
    command: &str,
    target_api_key: Option<&str>,
    target_server_url: &str,
) -> bool {
    let daemon_marker = command.contains("@slock-ai/daemon") || command.contains("slock-ai/daemon");
    if !daemon_marker || !command.contains("--server-url") || !command.contains(target_server_url) {
        return false;
    }

    if let Some(api_key) = target_api_key.filter(|api_key| !api_key.trim().is_empty()) {
        return command.contains("--api-key") && command.contains(api_key);
    }

    true
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
            return Err(format!("Failed to stop daemon process {pid}"));
        }

        sleep(Duration::from_millis(250));
        if process_is_alive(pid)? {
            let kill_status = Command::new("kill")
                .args(["-KILL", pid_text.as_str()])
                .status()
                .map_err(|err| format!("Failed to force-stop daemon process {pid}: {err}"))?;
            if !kill_status.success() {
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
            api_key: binding.api_key.trim().to_string(),
        };
        if normalized.server_id.is_empty()
            || normalized.server_slug.is_empty()
            || normalized.machine_id.is_empty()
            || normalized.api_key.is_empty()
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

fn api_base_url(server_url: &str) -> String {
    format!("{}/api", sanitize_service_server_url(server_url))
}

fn api_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Slock Desktop")
        .build()
        .map_err(|err| format!("Unable to create desktop API client: {err}"))
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

    let mut snapshots = Vec::with_capacity(servers.len());
    for server in servers {
        let binding = find_service_binding(settings, &server.id, &server.slug);
        let machines = fetch_server_machines(app, state, &server_url, &server.id)?;
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
            api_key_ready: binding
                .as_ref()
                .map(|item| !item.api_key.trim().is_empty())
                .unwrap_or(false),
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
        .find(|server| server.slug == settings.selected_server_slug)
        .cloned()
    {
        return Ok(server);
    }

    if servers.len() == 1 {
        let server = servers[0].clone();
        persist_selected_server_slug(app, state, &server.slug)?;
        return Ok(server);
    }

    Err("Pick a server in the launcher before starting Slock.".to_string())
}

fn ensure_machine_binding(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
    server: &ApiServer,
) -> Result<ServiceMachineBinding, String> {
    let existing_binding = find_service_binding(settings, &server.id, &server.slug);
    if let Some(binding) = existing_binding.as_ref() {
        if !binding.api_key.trim().is_empty() {
            return Ok(binding.clone());
        }
    }

    let server_url = settings.server_url.clone();
    let machines = fetch_server_machines(app, state, &server_url, &server.id)?;
    if let Some(machine) = select_existing_machine(existing_binding.as_ref(), &machines) {
        let api_key = rotate_machine_api_key(app, state, &server_url, &server.id, &machine.id)?;
        let binding = ServiceMachineBinding {
            server_id: server.id.clone(),
            server_slug: server.slug.clone(),
            machine_id: machine.id,
            machine_name: machine.name,
            api_key,
        };
        return upsert_service_binding(app, state, binding);
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
        api_key: payload.api_key,
    };
    upsert_service_binding(app, state, binding)
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

fn selected_server_is_started(
    app: &AppHandle,
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<bool, String> {
    let selected_server = resolve_selected_server(app, state, settings)?;
    let machines = fetch_server_machines(app, state, &settings.server_url, &selected_server.id)?;
    Ok(machines
        .iter()
        .any(|machine| machine_counts_as_started(&machine.status)))
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

fn sanitize_update_settings(updates: UpdateSettings) -> UpdateSettings {
    let repository_slug = updates.repository_slug.trim().to_string();
    let releases_url = if updates.releases_url.trim().is_empty() && !repository_slug.is_empty() {
        format!("https://github.com/{repository_slug}/releases")
    } else {
        updates.releases_url.trim().to_string()
    };

    UpdateSettings {
        repository_slug,
        releases_url,
    }
}

fn sanitize_custom_theme(custom_theme: CustomThemeSettings) -> CustomThemeSettings {
    let name = custom_theme.name.trim();
    CustomThemeSettings {
        name: if name.is_empty() {
            "Custom".to_string()
        } else {
            name.to_string()
        },
        accent: theme::sanitize_hex(&custom_theme.accent).unwrap_or_else(|| "#10a37f".to_string()),
    }
}

fn normalize_app_settings(settings: AppSettings) -> AppSettings {
    let appearance_mode = theme::normalize_mode(&settings.appearance_mode).to_string();
    let custom_theme = sanitize_custom_theme(settings.custom_theme);
    let color_scheme = resolve_theme(
        &settings.color_scheme,
        &appearance_mode,
        &custom_theme_input(&custom_theme),
    )
    .id;

    AppSettings {
        color_scheme,
        appearance_mode,
        custom_theme,
        language: sanitize_language(&settings.language).to_string(),
        session: config::SessionSettings {
            access_token: settings.session.access_token.trim().to_string(),
            refresh_token: settings.session.refresh_token.trim().to_string(),
        },
        service: sanitize_service_settings(settings.service),
        updates: sanitize_update_settings(settings.updates),
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
    let locale = env::var("LC_ALL")
        .or_else(|_| env::var("LC_MESSAGES"))
        .or_else(|_| env::var("LANG"))
        .unwrap_or_default()
        .to_ascii_lowercase();

    if locale.starts_with("zh") {
        "zh-CN"
    } else {
        "en-US"
    }
}

fn custom_theme_input(custom_theme: &CustomThemeSettings) -> CustomThemeInput {
    CustomThemeInput {
        name: custom_theme.name.clone(),
        accent: custom_theme.accent.clone(),
    }
}

fn open_url_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("Failed to open URL: {err}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
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
        })
        .on_page_load(|webview, payload| {
            if webview.label() != MAIN_LABEL
                || !matches!(payload.event(), PageLoadEvent::Finished)
                || !is_workspace_url(payload.url())
            {
                return;
            }

            let (color_scheme, appearance_mode, custom_theme, language) = webview
                .state::<DesktopState>()
                .settings
                .lock()
                .map(|settings| {
                    (
                        settings.color_scheme.clone(),
                        settings.appearance_mode.clone(),
                        settings.custom_theme.clone(),
                        settings.language.clone(),
                    )
                })
                .unwrap_or_else(|_| {
                    (
                        "default".to_string(),
                        "system".to_string(),
                        CustomThemeSettings::default(),
                        "system".to_string(),
                    )
                });
            let custom = custom_theme_input(&custom_theme);
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
                let settings = normalize_app_settings(load_settings(app.handle()));
                save_settings(app.handle(), &settings).map_err(std::io::Error::other)?;
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
                apply_launcher_window_size(&window);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_theme,
            set_theme_mode,
            save_custom_theme,
            set_language,
            save_session_tokens,
            open_workspace,
            save_service_settings,
            refresh_service_servers,
            select_service_server,
            start_service,
            stop_service,
            update_service,
            save_update_settings,
            open_external_url
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app: &AppHandle, event: RunEvent| {
        if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
            let state = app.state::<DesktopState>();
            let _ = stop_service_process(&state, None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        daemon_command_matches, daemon_pids_from_ps_output, select_existing_machine, ApiMachine,
    };
    use crate::config::ServiceMachineBinding;

    #[test]
    fn daemon_command_matching_respects_target_api_key() {
        let command = "node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_current";

        assert!(daemon_command_matches(
            command,
            Some("sk_machine_current"),
            "https://api.slock.ai"
        ));
        assert!(!daemon_command_matches(
            command,
            Some("sk_machine_other"),
            "https://api.slock.ai"
        ));
        assert!(!daemon_command_matches(
            command,
            Some("sk_machine_current"),
            "https://other.slock.ai"
        ));
    }

    #[test]
    fn daemon_pid_parser_keeps_only_matching_target_processes() {
        let output = r#"
  101 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_current
  102 node /tmp/npx/@slock-ai/daemon --server-url https://api.slock.ai --api-key sk_machine_other
  103 node /tmp/another-command --server-url https://api.slock.ai --api-key sk_machine_current
"#;

        let pids =
            daemon_pids_from_ps_output(output, Some("sk_machine_current"), "https://api.slock.ai");

        assert_eq!(pids, vec![101]);
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
            },
            ApiMachine {
                id: "bound".to_string(),
                name: "Bound machine".to_string(),
                status: "offline".to_string(),
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
        }];

        let selected = select_existing_machine(None, &machines);

        assert_eq!(
            selected.map(|machine| machine.id),
            Some("existing".to_string())
        );
    }
}
