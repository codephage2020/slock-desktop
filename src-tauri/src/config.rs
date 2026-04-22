use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub active_theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            active_theme: "default".to_string(),
        }
    }
}

pub fn load_settings<R: Runtime>(app: &AppHandle<R>) -> AppSettings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return AppSettings::default(),
    };

    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
) -> Result<(), String> {
    let path = settings_path(app)?;
    let payload = serde_json::to_vec_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir.join(SETTINGS_FILE))
}
