pub type AudioSample = Vec<f32>;

use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AudioSubtitle {
    pub(crate) speaker: Option<String>,
    pub(crate) text: String, // Keep for backward compatibility or as "target"
    pub(crate) displayed_text: String,
    pub(crate) last_update: Instant,
}

#[derive(Debug)]
pub enum AudioMessage {
    Audio(AudioSample),
    Stop,
}

impl AudioSubtitle {
    pub fn new(speaker: Option<String>, text: String) -> Self {
        Self {
            speaker,
            text: text.clone(),
            displayed_text: String::new(),
            last_update: Instant::now(),
        }
    }

    pub fn new_complete(speaker: Option<String>, text: String) -> Self {
        Self {
            speaker,
            text: text.clone(),
            displayed_text: text,
            last_update: Instant::now(),
        }
    }

    pub fn update_animation(&mut self, ignore_timer: bool) -> bool {
        if self.displayed_text.len() >= self.text.len() {
            // handle deletion/correction
             if self.displayed_text.len() > self.text.len() {
                 self.displayed_text = self.text.clone();
                 return true;
             }
            return false;
        }

        // Speed: 20ms per char
        if ignore_timer || self.last_update.elapsed() > Duration::from_millis(20) {
            let next_char_index = self.displayed_text.chars().count();
            if let Some(c) = self.text.chars().nth(next_char_index) {
                self.displayed_text.push(c);
                self.last_update = Instant::now();
                return true;
            }
        }
        false
    }
}

impl Default for AudioSubtitle {
    fn default() -> Self {
        let text = "... waiting for the sound ...".to_string();
        Self {
            speaker: None,
            text: text.clone(),
            displayed_text: text,
            last_update: Instant::now(),
        }
    }
}
