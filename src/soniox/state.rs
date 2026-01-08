use crate::types::audio::AudioSubtitle;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::collections::VecDeque;

pub struct TranscriptionState {
    finishes_lines: VecDeque<AudioSubtitle>,
    interim_line: AudioSubtitle,
    max_lines: usize,
}

impl TranscriptionState {
    pub fn new(max_lines: usize) -> Self {
        assert!(max_lines > 0);

        Self {
            finishes_lines: VecDeque::with_capacity(max_lines - 1),
            interim_line: AudioSubtitle::default(),
            max_lines,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &AudioSubtitle> {
        std::iter::once(&self.interim_line).chain(&self.finishes_lines)
    }

    pub fn update_animation(&mut self) -> bool {
        let mut request_repaint = false;
        if self.interim_line.update_animation() {
            request_repaint = true;
        }
        for line in &mut self.finishes_lines {
            // Only animate newest line if needed, or all lines?
            // Usually only the newest inserted line needs animation, but
            // let's just update all to be safe.
            if line.update_animation() {
                request_repaint = true;
            }
        }
        request_repaint
    }

    pub fn handle_transcription(&mut self, response: SonioxTranscriptionResponse) {
        let mut interim_text = String::new();
        let mut interim_speaker = Option::<String>::None;
        let mut final_text = String::new();
        let mut final_speaker = Option::<String>::None;

        for token in response.tokens {
            log::debug!("Token from WS: {:?}", token);

            if token.translation_status.as_deref() == Some("original") {
                continue;
            } else if token.is_final {
                if final_speaker != token.speaker {
                    self.push_final(final_speaker, final_text);
                    final_speaker = token.speaker;
                    final_text = String::new();
                }

                final_text.push_str(&token.text);
            } else {
                if interim_speaker != token.speaker {
                    interim_speaker = token.speaker;
                    interim_text = String::new();
                }

                interim_text.push_str(&token.text);
            }
        }

        self.push_final(final_speaker, final_text);
        self.update_interim(interim_speaker, interim_text);
    }

    fn push_final(&mut self, speaker: Option<String>, text: String) {
        if text.is_empty() {
            return;
        }
        match self.finishes_lines.front_mut() {
            Some(last) if last.speaker == speaker => {
                last.text.push_str(&text);
                // Don't auto-update displayed_text here to allow animation
            },
            _ => self
                .finishes_lines
                .push_front(AudioSubtitle::new(speaker, text)),
        }

        if self.finishes_lines.len() > self.max_lines - 1 {
            self.finishes_lines.pop_back();
        }
    }

    fn update_interim(&mut self, speaker: Option<String>, text: String) {
        // If the new interim text is DIFFERENT from the old one, we should reset animation?
        // Or just update target.
        // For interim, usually it updates rapidly. Animation might just lag behind.
        
        match self.finishes_lines.front_mut() {
            Some(last) if last.speaker == speaker => {
                 self.interim_line = AudioSubtitle::new_complete(None, text);
            }
            _ => {
                // For interim, usually we want to see it immediately if it updates fast.
                // But for translation, it might jump.
                // Let's use animation for interim too.
                
                // If text is completely different, maybe we should reset displayed_text?
                // But typically Soniox appends.
                
                 self.interim_line.speaker = speaker;
                 self.interim_line.text = text;
            },
        }
    }
}
