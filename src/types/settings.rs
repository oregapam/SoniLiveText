use crate::errors::SonioxWindowsErrors;
use crate::types::languages::LanguageHint;
use config::{Config, ConfigError, File};
use log::LevelFilter;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct SettingsApp {
    pub(crate) language_hints: Option<Vec<LanguageHint>>,
    pub(crate) context: Option<String>,
    pub(crate) api_key: Option<String>,
    pub(crate) target_language: Option<LanguageHint>,
    pub(crate) enable_translate: Option<bool>,
    enable_high_priority: Option<bool>,
    enable_speakers: Option<bool>,
    level: Option<String>,
    pub(crate) font_size: Option<f32>,
    pub(crate) text_color: Option<(u8, u8, u8)>,
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

    pub fn validate(&self) -> Result<(), String> {
        let mut missing_fields = Vec::new();
        if self.language_hints.is_none() { missing_fields.push("language_hints"); }
        if self.context.is_none() { missing_fields.push("context"); }
        if self.api_key.is_none() { missing_fields.push("api_key"); }
        // target_language is optional if enable_translate is false, but let's stick to the list for now or keep it rigid?
        // The previous code had it mandatory. Let's keep it mandatory as per previous struct.
        if self.target_language.is_none() { missing_fields.push("target_language"); }
        if self.enable_translate.is_none() { missing_fields.push("enable_translate"); }
        if self.enable_high_priority.is_none() { missing_fields.push("enable_high_priority"); }
        if self.enable_speakers.is_none() { missing_fields.push("enable_speakers"); }
        if self.level.is_none() { missing_fields.push("level"); }
        if self.font_size.is_none() { missing_fields.push("font_size"); }
        if self.text_color.is_none() { missing_fields.push("text_color"); }
        if self.window_width.is_none() { missing_fields.push("window_width"); }

        if !missing_fields.is_empty() {
             return Err(format!("Missing mandatory fields in config.toml: {}", missing_fields.join(", ")));
        }
        Ok(())
    }

    pub fn language_hints(&self) -> &[LanguageHint] {
        self.language_hints.as_ref().expect("Validated")
    }

    pub fn context(&self) -> &str {
        self.context.as_ref().expect("Validated")
    }

    pub fn api_key(&self) -> &str {
        self.api_key.as_ref().expect("Validated")
    }

    pub fn target_language(&self) -> LanguageHint {
         self.target_language.clone().expect("Validated")
    }

    pub fn enable_speakers(&self) -> bool {
        self.enable_speakers.expect("Validated")
    }

    pub fn enable_translate(&self) -> bool {
        self.enable_translate.expect("Validated")
    }

    pub fn enable_high_priority(&self) -> bool {
        self.enable_high_priority.expect("Validated")
    }

    pub fn font_size(&self) -> f32 {
        self.font_size.expect("Validated")
    }

    pub fn level(&self) -> Result<LevelFilter, SonioxWindowsErrors> {
        LevelFilter::from_str(self.level.as_ref().expect("Validated")).map_err(|_| {
            SonioxWindowsErrors::Internal(
                "field `level` isn't valid. did u mean `info`, `debug` and `warn`?",
            )
        })
    }

    pub fn text_color(&self) -> eframe::egui::Color32 {
        let (r, g, b) = self.text_color.expect("Validated");
        eframe::egui::Color32::from_rgb(r, g, b)
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
