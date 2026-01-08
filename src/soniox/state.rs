use crate::types::audio::AudioSubtitle;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::collections::VecDeque;

pub struct TranscriptionState {
    finishes_lines: VecDeque<AudioSubtitle>,
    interim_line: AudioSubtitle,
    max_lines: usize,
    max_chars_in_block: usize,
    frozen_interim_history: String,
    pub debug_log: VecDeque<String>,
}

impl TranscriptionState {
    pub fn new(max_lines: usize, max_chars_in_block: usize) -> Self {
        assert!(max_lines > 0);

        Self {
            finishes_lines: VecDeque::with_capacity(max_lines - 1),
            interim_line: AudioSubtitle::default(),
            max_lines,
            max_chars_in_block,
            frozen_interim_history: String::new(),
            debug_log: VecDeque::with_capacity(20),
        }
    }

    fn log_debug(&mut self, msg: String) {
        if self.debug_log.len() >= 20 {
            self.debug_log.pop_front();
        }
        self.debug_log.push_back(msg);
    }
    
    pub fn get_debug_log(&self) -> Vec<String> {
        self.debug_log.iter().cloned().collect()
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

    pub fn get_active_char_count(&self) -> usize {
        // Active line is at the front of finishes_lines usually (the one being appended to)
        // OR if interim is separate?
        // Logic in push_final appends to finishes_lines.front().
        // So the "active growing block" is finishes_lines.front().
        self.finishes_lines.front().map(|l| l.text.len()).unwrap_or(0)
    }

    pub fn get_frozen_block_count(&self) -> usize {
        self.finishes_lines.len()
    }

    pub fn get_max_chars(&self) -> usize {
        self.max_chars_in_block
    }

    pub fn handle_transcription(&mut self, response: SonioxTranscriptionResponse) {
        let mut full_interim_text = String::new();
        let mut interim_speaker = Option::<String>::None;
        
        let mut final_text_segment = String::new();
        let mut final_speaker = Option::<String>::None;
        let mut has_final = false;

        for token in response.tokens {
            if token.translation_status.as_deref() == Some("original") {
                continue;
            } else if token.is_final {
                // Final token logic
                if final_speaker != token.speaker {
                     // Flush previous final if exists? 
                     // Typically Soniox sends one final block or sequence.
                     // Simplification: handle immediately
                }
                final_speaker = token.speaker.clone();
                final_text_segment.push_str(&token.text);
                has_final = true;
            } else {
                // Interim logic
                if interim_speaker != token.speaker {
                    interim_speaker = token.speaker.clone();
                    // Reset if speaker changes mid-stream? 
                    // Usually implies new sentence.
                }
                full_interim_text.push_str(&token.text);
            }
        }

        if has_final {
            // Deduplicate against frozen history
            if final_text_segment.starts_with(&self.frozen_interim_history) {
                 // CASE 1: Final is longer or equal to history. 
                 // We kept the prefix safe, now just push the new suffix.
                 let text_to_push = final_text_segment[self.frozen_interim_history.len()..].to_string();
                 self.log_debug(format!("FINAL extends history. Pushing suffix: '{}'", text_to_push));
                 self.push_final(final_speaker.clone(), text_to_push, true);
                 self.frozen_interim_history.clear();
                 
            } else if self.frozen_interim_history.starts_with(&final_text_segment) {
                 // CASE 2: History is LONGER than Final (Aggressive freeze).
                 // We already displayed this part. Do NOT push it again.
                 // Just remove it from history so we expect the *rest* later.
                 self.log_debug(format!("FINAL covered by history. Consuming prefix: '{}' (Remaining history: {})", 
                    final_text_segment, 
                    self.frozen_interim_history.len() - final_text_segment.len()
                 ));
                 // Drain the prefix from history
                 self.frozen_interim_history.drain(..final_text_segment.len());
                 // Do not clear history!
                 // Do not push final!
                 
            } else {
                // CASE 3: Mismatch.
                self.log_debug(format!("FINAL mismatch. History: '{}', Final: '{}'. Pushing Final.", 
                    self.frozen_interim_history, final_text_segment));
                self.push_final(final_speaker.clone(), final_text_segment, true);
                self.frozen_interim_history.clear();
            }
            
            // Also clear interim line because we have a final (or consumed it)
            self.update_interim(interim_speaker.clone(), String::new());
        }

        if !full_interim_text.is_empty() {
             let effective_interim = if full_interim_text.starts_with(&self.frozen_interim_history) {
                full_interim_text[self.frozen_interim_history.len()..].to_string()
            } else {
                full_interim_text.clone()
            };

            let limit = self.max_chars_in_block;
            let safety_buffer = 15; 
            
            if effective_interim.len() > limit + safety_buffer {
                let split_idx = effective_interim.char_indices()
                    .filter(|(i, c)| *i >= limit && c.is_whitespace())
                    .map(|(i, _)| i)
                    .next();

                if let Some(idx) = split_idx {
                    let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                    let frozen_chunk_str = frozen_chunk.to_string();
                    
                    self.log_debug(format!("FREEZE INTERIM. Chunk: '{}' (len: {})", frozen_chunk_str, frozen_chunk_str.len()));

                    self.frozen_interim_history.push_str(&frozen_chunk_str);
                    
                    // Freeze chunk: Instant=true! It was already visible.
                    self.push_final(interim_speaker.clone(), frozen_chunk_str, true);
                    
                    self.update_interim(interim_speaker, remainder.to_string());
                } else {
                     self.update_interim(interim_speaker, effective_interim);
                }
            } else {
                self.update_interim(interim_speaker, effective_interim);
            }
        } else if has_final {
        } else {
            self.update_interim(interim_speaker, String::new());
        }
    }

    fn push_final(&mut self, speaker: Option<String>, text: String, instant: bool) {
        if text.is_empty() {
            return;
        }

        let should_start_new = match self.finishes_lines.front() {
            Some(last) => {
                let current_len = last.text.len();
                let new_len = current_len + text.len();
                let limit = self.max_chars_in_block;
                
                if last.speaker != speaker {
                    self.log_debug("New Block: Speaker changed".to_string());
                    true
                } else if new_len > limit {
                    let last_word = last.text.split_whitespace().last().unwrap_or("<empty>");
                    self.log_debug(format!("New Block: Overflow. {} + {} > {}. Last word: '{}'", current_len, text.len(), limit, last_word));
                    true
                } else {
                    false
                }
            }
            None => true,
        };

        if !should_start_new {
            let last = self.finishes_lines.front_mut().unwrap();
            last.text.push_str(&text);
            if instant {
                last.displayed_text = last.text.clone();
            }
        } else {
             let mut sub = AudioSubtitle::new(speaker, text.clone());
             if instant {
                 sub.displayed_text = text;
             }
             self.finishes_lines.push_front(sub);
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
