use crate::core::comic_engine::ReadingDirection;
use crate::ui::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub library_path: PathBuf,
    pub font_size: f32,
    pub line_spacing: f32,
    pub margin_x: i32,
    pub margin_y: i32,
    pub theme: ThemeMode,
    pub default_reading_direction: ReadingDirection,
    pub rgb_led_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            library_path: PathBuf::from("books"),
            font_size: 28.0,
            line_spacing: 1.5,
            margin_x: 40,
            margin_y: 50,
            theme: ThemeMode::PaperSepia,
            default_reading_direction: ReadingDirection::RightToLeft,
            rgb_led_enabled: true,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let candidates = [
            "data/settings.json",
            "settings.json",
            "/mnt/SDCARD/Apps/BrickReader/settings.json",
        ];

        for path in candidates {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::create_dir_all("data");
            let _ = fs::write("data/settings.json", json);
        }
    }

    pub fn set_library_path<P: AsRef<Path>>(&mut self, path: P) {
        self.library_path = path.as_ref().to_path_buf();
        self.save();
    }
}
