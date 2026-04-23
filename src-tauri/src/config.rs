use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSettings {
    pub command_path: String,
    pub working_directory: String,
    pub args: Vec<String>,
    pub auto_start_with_workspace: bool,
}

impl Default for ServiceSettings {
    fn default() -> Self {
        Self {
            command_path: String::new(),
            working_directory: String::new(),
            args: Vec::new(),
            auto_start_with_workspace: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    pub repository_slug: String,
    pub releases_url: String,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            repository_slug: "codephage2020/slock-tauri".to_string(),
            releases_url: "https://github.com/codephage2020/slock-tauri/releases".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_color_scheme", alias = "activeTheme")]
    pub color_scheme: String,
    #[serde(default = "default_appearance_mode", alias = "themeMode")]
    pub appearance_mode: String,
    #[serde(default)]
    pub custom_theme: CustomThemeSettings,
    #[serde(default = "default_language")]
    pub language: String,
    pub service: ServiceSettings,
    pub updates: UpdateSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_scheme: default_color_scheme(),
            appearance_mode: default_appearance_mode(),
            custom_theme: CustomThemeSettings::default(),
            language: default_language(),
            service: ServiceSettings::default(),
            updates: UpdateSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomThemeSettings {
    pub name: String,
    pub accent: String,
}

impl Default for CustomThemeSettings {
    fn default() -> Self {
        Self {
            name: "Custom".to_string(),
            accent: "#10a37f".to_string(),
        }
    }
}

fn default_color_scheme() -> String {
    "default".to_string()
}

fn default_appearance_mode() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "system".to_string()
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

pub fn save_settings<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let payload = serde_json::to_vec_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir.join(SETTINGS_FILE))
}
