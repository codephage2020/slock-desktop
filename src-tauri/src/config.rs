use crate::theme::ThemeStyleConfig;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMachineBinding {
    #[serde(default)]
    pub server_id: String,
    #[serde(default)]
    pub server_slug: String,
    #[serde(default)]
    pub machine_id: String,
    #[serde(default)]
    pub machine_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_key: String,
    /// How this binding was established: "desktop_created", "pid_scan", or "user_bound"
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSettings {
    #[serde(default = "default_service_server_url")]
    pub server_url: String,
    #[serde(default)]
    pub selected_server_slug: String,
    pub auto_start_with_workspace: bool,
    #[serde(default = "default_close_app_behavior")]
    pub close_app_behavior: String,
    #[serde(default)]
    pub machines: Vec<ServiceMachineBinding>,
}

impl Default for ServiceSettings {
    fn default() -> Self {
        Self {
            server_url: default_service_server_url(),
            selected_server_slug: String::new(),
            auto_start_with_workspace: false,
            close_app_behavior: default_close_app_behavior(),
            machines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettings {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub accounts: Vec<SavedAccountSettings>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAccountSettings {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub avatar_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_color_scheme", alias = "activeTheme")]
    pub color_scheme: String,
    #[serde(default = "default_style_scheme")]
    pub style_scheme: String,
    #[serde(default = "default_appearance_mode", alias = "themeMode")]
    pub appearance_mode: String,
    #[serde(
        default,
        alias = "customTheme",
        deserialize_with = "deserialize_custom_themes"
    )]
    pub custom_themes: Vec<CustomThemeSettings>,
    #[serde(default)]
    pub custom_styles: Vec<ThemeStyleConfig>,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub session: SessionSettings,
    #[serde(default)]
    pub service: ServiceSettings,
    #[serde(default)]
    pub agent_templates: Vec<AgentTemplate>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_scheme: default_color_scheme(),
            style_scheme: default_style_scheme(),
            appearance_mode: default_appearance_mode(),
            custom_themes: Vec::new(),
            custom_styles: Vec::new(),
            language: default_language(),
            session: SessionSettings::default(),
            service: ServiceSettings::default(),
            agent_templates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_agent_id: Option<String>,
    pub config: AgentTemplateConfig,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTemplateConfig {
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub max_turns: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomThemeSettings {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub accent: String,
}

impl Default for CustomThemeSettings {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Custom".to_string(),
            accent: "#10a37f".to_string(),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CustomThemesField {
    List(Vec<CustomThemeSettings>),
    Single(CustomThemeSettings),
    None,
}

fn deserialize_custom_themes<'de, D>(deserializer: D) -> Result<Vec<CustomThemeSettings>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = CustomThemesField::deserialize(deserializer).unwrap_or(CustomThemesField::None);
    Ok(match value {
        CustomThemesField::List(items) => items,
        CustomThemesField::Single(item) => vec![item],
        CustomThemesField::None => Vec::new(),
    })
}

fn default_color_scheme() -> String {
    crate::theme::default_color_scheme().to_string()
}

fn default_style_scheme() -> String {
    crate::theme::default_style_scheme().to_string()
}

fn default_appearance_mode() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "system".to_string()
}

fn default_service_server_url() -> String {
    "https://api.slock.ai".to_string()
}

fn default_close_app_behavior() -> String {
    "ask".to_string()
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

pub fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir.join(SETTINGS_FILE))
}
