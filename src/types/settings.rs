use crate::errors::SonioxWindowsErrors;
use crate::types::languages::LanguageHint;
use config::{Config, ConfigError, File};
use log::LevelFilter;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct SettingsApp {
    pub(crate) language_hints: Vec<LanguageHint>,
    pub(crate) context: String,
    pub(crate) api_key: String,
    pub(crate) target_language: LanguageHint,
    pub(crate) enable_translate: bool,
    enable_high_priority: bool,
    enable_speakers: bool,
    level: String,
    pub(crate) font_size: f32,
    pub(crate) text_color: (u8, u8, u8),
    pub(crate) window_width: Option<f32>,
    pub(crate) window_height: Option<f32>,
    pub(crate) window_anchor: Option<String>,
    pub(crate) window_offset: Option<(f32, f32)>,
    #[serde(default)]
    pub(crate) audio_input: String,
}

impl SettingsApp {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(path))
            .build()?;
        s.try_deserialize()
    }

    pub fn language_hints(&self) -> &[LanguageHint] {
        &self.language_hints
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn target_language(&self) -> LanguageHint {
        self.target_language
    }

    pub fn enable_speakers(&self) -> bool {
        self.enable_speakers
    }

    pub fn enable_translate(&self) -> bool {
        self.enable_translate
    }

    pub fn enable_high_priority(&self) -> bool {
        self.enable_high_priority
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn level(&self) -> Result<LevelFilter, SonioxWindowsErrors> {
        LevelFilter::from_str(&self.level).map_err(|_| {
            SonioxWindowsErrors::Internal(
                "field `level` isn't valid. did u mean `info`, `debug` and `warn`?",
            )
        })
    }

    pub fn text_color(&self) -> eframe::egui::Color32 {
        eframe::egui::Color32::from_rgb(self.text_color.0, self.text_color.1, self.text_color.2)
    }

    pub fn get_position(&self, screen_width: f32, screen_height: f32, window_width: f32, window_height: f32) -> (f32, f32) {
        let anchor = self.window_anchor.as_deref().unwrap_or("bottom_center");
        let offset = self.window_offset.unwrap_or((0.0, -100.0));
        let (offset_x, offset_y) = offset;

        // Refined Logic (Anchor Matching):
        // X calculation
        let x = if anchor.ends_with("_left") || anchor == "left" {
            0.0
        } else if anchor.ends_with("_right") || anchor == "right" {
             screen_width - window_width
        } else {
            // center / top / bottom -> horizontal center
            (screen_width - window_width) / 2.0
        };
        
        // Y calculation
        let y = if anchor.starts_with("top_") || anchor == "top" {
            0.0
        } else if anchor.starts_with("center_") || anchor == "center" {
             (screen_height - window_height) / 2.0
        } else {
             // bottom is default
             screen_height - window_height
        };

        (x + offset_x, y + offset_y)
    }

    pub fn window_width(&self) -> Option<f32> {
        self.window_width
    }

    pub fn window_height(&self) -> Option<f32> {
        self.window_height
    }
}
