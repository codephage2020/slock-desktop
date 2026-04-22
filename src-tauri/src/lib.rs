mod config;
mod theme;

use config::{load_settings, save_settings, AppSettings};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    webview::{PageLoadEvent, WebviewWindowBuilder},
    AppHandle, Manager, State, Url, WebviewUrl,
};
use theme::{meta_catalog, resolve_theme};

const WORKSPACE_LABEL: &str = "workspace";
const WORKSPACE_URL: &str = "https://app.slock.ai";

pub struct DesktopState {
    settings: Mutex<AppSettings>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    app_name: &'static str,
    workspace_url: &'static str,
    active_theme_id: String,
    workspace_open: bool,
    themes: Vec<theme::ThemeMeta>,
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
        settings.active_theme.clone()
    };

    ensure_workspace_window(&app, &theme_id)?;
    build_bootstrap(&app, &state)
}

fn build_bootstrap(
    app: &AppHandle,
    state: &State<'_, DesktopState>,
) -> Result<BootstrapPayload, String> {
    let active_theme_id = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock desktop settings".to_string())?
        .active_theme
        .clone();

    Ok(BootstrapPayload {
        app_name: "Slock Desktop",
        workspace_url: WORKSPACE_URL,
        active_theme_id,
        workspace_open: app.get_webview_window(WORKSPACE_LABEL).is_some(),
        themes: meta_catalog(),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DesktopState {
            settings: Mutex::new(AppSettings::default()),
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
        .invoke_handler(tauri::generate_handler![bootstrap, set_theme, open_workspace])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
