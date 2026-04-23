mod config;
mod theme;
mod workspace;

use config::{
    load_settings, save_settings, AppSettings, CustomThemeSettings, ServiceSettings, UpdateSettings,
};
use serde::Serialize;
use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
};
use tauri::{webview::PageLoadEvent, AppHandle, Manager, RunEvent, State, Theme, Url};
use theme::{meta_catalog, resolve_theme, CustomThemeInput};

const MAIN_LABEL: &str = "main";
const WORKSPACE_URL: &str = "https://app.slock.ai";

pub struct DesktopState {
    settings: Mutex<AppSettings>,
    service: Mutex<ServiceRuntime>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    app_name: &'static str,
    workspace_url: &'static str,
    active_theme_id: String,
    active_theme_mode: String,
    custom_theme: CustomThemeSettings,
    active_language: String,
    workspace_open: bool,
    themes: Vec<theme::ThemeMeta>,
    service: ServiceSnapshot,
    updates: UpdateSnapshot,
}

struct ServiceRuntime {
    child: Option<Child>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceSnapshot {
    command_path: String,
    working_directory: String,
    args: Vec<String>,
    auto_start_with_workspace: bool,
    configured: bool,
    running: bool,
    pid: Option<u32>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSnapshot {
    current_version: String,
    repository_slug: String,
    releases_url: String,
    latest_release_api_url: String,
}

#[tauri::command]
fn bootstrap(app: AppHandle, state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    build_bootstrap(&app, &state)
}

#[tauri::command]
fn set_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_id: String,
) -> Result<BootstrapPayload, String> {
    let (active_theme_id, active_theme_mode, custom_theme, active_language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        let theme = resolve_theme(
            &theme_id,
            &settings.theme_mode,
            &custom_theme_input(&settings.custom_theme),
        );
        settings.active_theme = theme.id.clone();
        save_settings(&app, &settings)?;
        (
            settings.active_theme.clone(),
            settings.theme_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
    let theme = resolve_theme(&active_theme_id, &active_theme_mode, &custom);
    apply_theme_to_workspace(&app, theme, &active_theme_mode, &active_language, &custom)?;

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn set_theme_mode(
    app: AppHandle,
    state: State<'_, DesktopState>,
    theme_mode: String,
) -> Result<BootstrapPayload, String> {
    let (active_theme_id, active_theme_mode, custom_theme, active_language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.theme_mode = theme::normalize_mode(&theme_mode).to_string();
        save_settings(&app, &settings)?;
        (
            settings.active_theme.clone(),
            settings.theme_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
    let theme = resolve_theme(&active_theme_id, &active_theme_mode, &custom);
    apply_theme_to_workspace(&app, theme, &active_theme_mode, &active_language, &custom)?;

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn save_custom_theme(
    app: AppHandle,
    state: State<'_, DesktopState>,
    custom_theme: CustomThemeSettings,
) -> Result<BootstrapPayload, String> {
    let (active_theme_mode, active_language, saved_custom_theme) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.custom_theme = sanitize_custom_theme(custom_theme);
        settings.active_theme = "custom".to_string();
        save_settings(&app, &settings)?;
        (
            settings.theme_mode.clone(),
            settings.language.clone(),
            settings.custom_theme.clone(),
        )
    };

    let custom = custom_theme_input(&saved_custom_theme);
    let theme = resolve_theme("custom", &active_theme_mode, &custom);
    apply_theme_to_workspace(&app, theme, &active_theme_mode, &active_language, &custom)?;

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn set_language(
    app: AppHandle,
    state: State<'_, DesktopState>,
    language: String,
) -> Result<BootstrapPayload, String> {
    let (active_theme_id, active_theme_mode, custom_theme, active_language) = {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.language = sanitize_language(&language).to_string();
        save_settings(&app, &settings)?;
        (
            settings.active_theme.clone(),
            settings.theme_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    let custom = custom_theme_input(&custom_theme);
    let theme = resolve_theme(&active_theme_id, &active_theme_mode, &custom);
    apply_theme_to_workspace(&app, theme, &active_theme_mode, &active_language, &custom)?;

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn open_workspace(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let (theme_id, theme_mode, custom_theme, language) = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        maybe_start_service(&state, &settings.service)?;
        (
            settings.active_theme.clone(),
            settings.theme_mode.clone(),
            settings.custom_theme.clone(),
            settings.language.clone(),
        )
    };

    enter_workspace_in_main_window(
        &app,
        &theme_id,
        &theme_mode,
        &language,
        &custom_theme_input(&custom_theme),
    )?;
    build_bootstrap(&app, &state)
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

    build_bootstrap(&app, &state)
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

    force_start_service(&state, &service_settings)?;
    build_bootstrap(&app, &state)
}

#[tauri::command]
fn stop_service(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    stop_service_process(&state)?;
    build_bootstrap(&app, &state)
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

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    open_url_in_browser(&url)
}

fn build_bootstrap(
    app: &AppHandle,
    state: &State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .clone();

    let service = collect_service_snapshot(state, &settings.service)?;
    let active_theme_mode = theme::normalize_mode(&settings.theme_mode).to_string();
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
        app_name: "Slock Desktop",
        workspace_url: WORKSPACE_URL,
        active_theme_id: settings.active_theme.clone(),
        active_theme_mode: active_theme_mode.clone(),
        custom_theme: settings.custom_theme.clone(),
        active_language: sanitize_language(&settings.language).to_string(),
        workspace_open: main_window_is_workspace(app),
        themes: meta_catalog(
            &active_theme_mode,
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
) -> Result<(), String> {
    let theme = resolve_theme(theme_id, theme_mode, custom_theme);
    let window = app
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "Main window is unavailable".to_string())?;

    if window_is_workspace(&window) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        apply_window_theme(&window, theme_mode);
        apply_window_language(&window, language, true);
        return apply_workspace_scripts_to_window(
            &window,
            theme,
            theme_id,
            theme_mode,
            language,
            custom_theme,
        );
    }

    let target_url = WORKSPACE_URL
        .parse::<Url>()
        .map_err(|err| err.to_string())?;

    apply_window_language(&window, language, true);
    apply_window_theme(&window, theme_mode);
    let _ = window.set_focus();
    window.navigate(target_url).map_err(|err| err.to_string())
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
        apply_window_language(&window, language, window_is_workspace(&window));
        if window_is_workspace(&window) {
            let active_theme_id = theme.id.clone();
            apply_workspace_scripts_to_window(
                &window,
                theme,
                &active_theme_id,
                theme_mode,
                language,
                custom_theme,
            )?;
        }
    }

    Ok(())
}

fn apply_window_theme(window: &tauri::WebviewWindow, theme_mode: &str) {
    let native_theme = match theme::normalize_mode(theme_mode) {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None,
    };
    let _ = window.set_theme(native_theme);
}

fn apply_window_language(window: &tauri::WebviewWindow, language: &str, workspace: bool) {
    let title = match (sanitize_language(language), workspace) {
        ("zh-CN", true) => "Slock 工作区",
        ("zh-CN", false) => "Slock 桌面端",
        (_, true) => "Slock Workspace",
        (_, false) => "Slock Desktop",
    };
    let _ = window.set_title(title);
}

fn apply_workspace_scripts_to_window(
    window: &tauri::WebviewWindow,
    theme: theme::ThemeDefinition,
    active_theme_id: &str,
    active_theme_mode: &str,
    active_language: &str,
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
            &meta_catalog(active_theme_mode, custom_theme),
        ))
        .map_err(|err| err.to_string())
}

fn apply_workspace_scripts_to_webview(
    webview: &tauri::Webview,
    theme: theme::ThemeDefinition,
    active_theme_id: &str,
    active_theme_mode: &str,
    active_language: &str,
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
            &meta_catalog(active_theme_mode, custom_theme),
        ))
        .map_err(|err| err.to_string())
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

fn collect_service_snapshot(
    state: &DesktopState,
    settings: &ServiceSettings,
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

    Ok(ServiceSnapshot {
        command_path: settings.command_path.clone(),
        working_directory: settings.working_directory.clone(),
        args: settings.args.clone(),
        auto_start_with_workspace: settings.auto_start_with_workspace,
        configured: !settings.command_path.trim().is_empty(),
        running,
        pid,
        last_error: runtime.last_error.clone(),
    })
}

fn maybe_start_service(state: &DesktopState, settings: &ServiceSettings) -> Result<(), String> {
    if settings.auto_start_with_workspace && !settings.command_path.trim().is_empty() {
        force_start_service(state, settings)?;
    }

    Ok(())
}

fn force_start_service(state: &DesktopState, settings: &ServiceSettings) -> Result<(), String> {
    if settings.command_path.trim().is_empty() {
        return Err("Service command path is empty".to_string());
    }

    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;

    if let Some(child) = runtime.child.as_mut() {
        if child
            .try_wait()
            .map_err(|err| format!("Unable to inspect service state: {err}"))?
            .is_none()
        {
            return Ok(());
        }
        runtime.child = None;
    }

    let mut command = Command::new(&settings.command_path);
    command
        .args(&settings.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if !settings.working_directory.trim().is_empty() {
        command.current_dir(&settings.working_directory);
    }

    let child = command
        .spawn()
        .map_err(|err| format!("Failed to start service: {err}"))?;
    runtime.last_error = None;
    runtime.child = Some(child);
    Ok(())
}

fn stop_service_process(state: &DesktopState) -> Result<(), String> {
    let mut runtime = state
        .service
        .lock()
        .map_err(|_| "Unable to lock service runtime".to_string())?;

    if let Some(child) = runtime.child.as_mut() {
        child
            .kill()
            .map_err(|err| format!("Failed to stop service: {err}"))?;
        let _ = child.wait();
    }

    runtime.child = None;
    runtime.last_error = None;
    Ok(())
}

fn sanitize_service_settings(service: ServiceSettings) -> ServiceSettings {
    ServiceSettings {
        command_path: service.command_path.trim().to_string(),
        working_directory: service.working_directory.trim().to_string(),
        args: service
            .args
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .filter(|arg| !arg.is_empty())
            .collect(),
        auto_start_with_workspace: service.auto_start_with_workspace,
    }
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

fn sanitize_language(language: &str) -> &'static str {
    match language {
        "zh-CN" => "zh-CN",
        "en-US" => "en-US",
        "system" => "system",
        _ => "system",
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
            }),
        })
        .on_page_load(|webview, payload| {
            if webview.label() != MAIN_LABEL
                || !matches!(payload.event(), PageLoadEvent::Finished)
                || !is_workspace_url(payload.url())
            {
                return;
            }

            let (active_theme_id, active_theme_mode, custom_theme, active_language) = webview
                .state::<DesktopState>()
                .settings
                .lock()
                .map(|settings| {
                    (
                        settings.active_theme.clone(),
                        settings.theme_mode.clone(),
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
            let theme = resolve_theme(&active_theme_id, &active_theme_mode, &custom);

            if let Err(err) = apply_workspace_scripts_to_webview(
                webview,
                theme,
                &active_theme_id,
                &active_theme_mode,
                &active_language,
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
                let settings = load_settings(app.handle());
                let state = app.state::<DesktopState>();
                let mut current = state
                    .settings
                    .lock()
                    .map_err(|_| std::io::Error::other("settings-lock"))?;
                *current = settings;
            }

            if let Some(window) = app.get_webview_window(MAIN_LABEL) {
                let (theme_mode, language) = app
                    .state::<DesktopState>()
                    .settings
                    .lock()
                    .map(|settings| (settings.theme_mode.clone(), settings.language.clone()))
                    .unwrap_or_else(|_| ("system".to_string(), "system".to_string()));
                apply_window_language(&window, &language, false);
                apply_window_theme(&window, &theme_mode);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_theme,
            set_theme_mode,
            save_custom_theme,
            set_language,
            open_workspace,
            save_service_settings,
            start_service,
            stop_service,
            save_update_settings,
            open_external_url
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app: &AppHandle, event: RunEvent| {
        if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
            let state = app.state::<DesktopState>();
            let _ = stop_service_process(&state);
        }
    });
}
