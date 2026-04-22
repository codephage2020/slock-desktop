mod config;
mod theme;

use config::{load_settings, save_settings, AppSettings, ServiceSettings, UpdateSettings};
use serde::Serialize;
use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
};
use tauri::{
    webview::{PageLoadEvent, WebviewWindowBuilder},
    AppHandle, Manager, RunEvent, State, Url, WebviewUrl,
};
use theme::{meta_catalog, resolve_theme};

const WORKSPACE_LABEL: &str = "workspace";
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
    let theme = resolve_theme(&theme_id);

    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        settings.active_theme = theme.id.to_string();
        save_settings(&app, &settings)?;
    }

    if let Some(window) = app.get_webview_window(WORKSPACE_LABEL) {
        apply_theme_to_workspace(&window, theme)?;
    }

    build_bootstrap(&app, &state)
}

#[tauri::command]
fn open_workspace(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let theme_id = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Unable to lock desktop settings".to_string())?;
        maybe_start_service(&state, &settings.service)?;
        settings.active_theme.clone()
    };

    ensure_workspace_window(&app, &theme_id)?;
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
        workspace_open: app.get_webview_window(WORKSPACE_LABEL).is_some(),
        themes: meta_catalog(),
        service,
        updates,
    })
}

fn ensure_workspace_window(app: &AppHandle, theme_id: &str) -> Result<(), String> {
    let theme = resolve_theme(theme_id);

    if let Some(window) = app.get_webview_window(WORKSPACE_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return apply_theme_to_workspace(&window, theme);
    }

    let target_url = WORKSPACE_URL
        .parse::<Url>()
        .map_err(|err| err.to_string())?;
    let app_handle = app.clone();

    WebviewWindowBuilder::new(app, WORKSPACE_LABEL, WebviewUrl::External(target_url))
        .title("Slock Workspace")
        .inner_size(1480.0, 980.0)
        .min_inner_size(960.0, 720.0)
        .initialization_script(theme::injected_script(theme))
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let next_theme_id = {
                    let state = app_handle.state::<DesktopState>();
                    state
                        .settings
                        .lock()
                        .map(|settings| settings.active_theme.clone())
                        .unwrap_or_else(|_| "default".to_string())
                };

                if let Err(err) =
                    apply_theme_to_workspace(&window, resolve_theme(&next_theme_id))
                {
                    log::error!("failed to apply workspace theme: {err}");
                }
            }
        })
        .build()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn apply_theme_to_workspace(
    window: &tauri::WebviewWindow,
    theme: theme::ThemeDefinition,
) -> Result<(), String> {
    window
        .eval(&theme::injected_script(theme))
        .map_err(|err| err.to_string())
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

fn maybe_start_service(
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<(), String> {
    if settings.auto_start_with_workspace && !settings.command_path.trim().is_empty() {
        force_start_service(state, settings)?;
    }

    Ok(())
}

fn force_start_service(
    state: &DesktopState,
    settings: &ServiceSettings,
) -> Result<(), String> {
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

            if let Some(window) = app.get_webview_window("main") {
                window.set_title("Slock Desktop / Theme Studio")?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_theme,
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
