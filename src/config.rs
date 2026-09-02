use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub primary_mode: String, // "image" or "video"
    pub language: String,
    pub hw_encoder_override: String,
    pub save_directory: String,
    pub default_fps: u32,
    pub crf_quality: u8,
    pub watermark_text: String,
    pub watermark_logo_path: Option<String>,
    pub watermark_position: String, // "BottomRight", "BottomLeft", "TopRight", "TopLeft"
    pub auto_watermark: bool,
    pub cinematic_zoom: bool,
    pub paddle_checkout_url: String,

    // Customizable Global Shortcuts
    pub hotkey_toggle_record: String,
    pub hotkey_snapshot: String,
    pub hotkey_region_select: String,
    pub hotkey_cancel: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_dir = dirs::video_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("WolfRecordings")
            .to_string_lossy()
            .to_string();

        Self {
            primary_mode: "image".to_string(),
            language: "en".to_string(),
            hw_encoder_override: "Auto".to_string(),
            save_directory: default_dir,
            default_fps: 30,
            crf_quality: 23,
            watermark_text: "WOLFITWAY".to_string(),
            watermark_logo_path: None,
            watermark_position: "BottomRight".to_string(),
            auto_watermark: true,
            cinematic_zoom: true,
            paddle_checkout_url: "https://buy.paddle.com/placeholder-wolfitway".to_string(),

            // Default Global Shortcuts
            hotkey_toggle_record: "Super+Shift+R".to_string(),
            hotkey_snapshot: "Super+Shift+S".to_string(),
            hotkey_region_select: "Super+Shift+X".to_string(),
            hotkey_cancel: "Escape".to_string(),
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("wolf-screen-recorder");
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    println!("[Config] Loaded configuration from {:?}", path);
                    return config;
                }
            }
        }
        let default_config = Self::default();
        default_config.save();
        default_config
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Err(e) = fs::write(&path, json) {
                eprintln!("[Config] Failed to save config to {:?}: {}", path, e);
            } else {
                println!("[Config] Saved configuration to {:?}", path);
            }
        }
    }
}
