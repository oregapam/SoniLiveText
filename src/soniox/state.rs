use crate::types::audio::AudioSubtitle;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct TranscriptionState {
    pub finishes_lines: VecDeque<AudioSubtitle>,
    pub interim_line: AudioSubtitle,
    pub(crate) max_lines: usize,
    pub(crate) max_chars_in_block: usize,
    pub(crate) frozen_interim_history: String,
    pub(crate) frozen_blocks_count: usize,
    pub debug_log: VecDeque<String>,
    pub(crate) event_queue: VecDeque<(Instant, SonioxTranscriptionResponse)>,

    pub(crate) last_final_ms: f64,
    pub(crate) show_interim: bool,
    pub(crate) stability_timeout: Duration,
    pub(crate) last_interim_update: Instant,
}

impl TranscriptionState {
    pub fn new(max_lines: usize, max_chars_in_block: usize) -> Self {
        assert!(max_lines > 0);

        Self {
            finishes_lines: VecDeque::with_capacity(max_lines),
            interim_line: AudioSubtitle::default(),
            max_lines,
            max_chars_in_block,
            frozen_interim_history: String::new(),
            frozen_blocks_count: 0,
            debug_log: VecDeque::with_capacity(20),
            event_queue: VecDeque::new(),

            last_final_ms: 0.0,
            show_interim: true,
            stability_timeout: Duration::from_millis(1000),
            last_interim_update: Instant::now(),
        }
    }

    pub fn log_debug(&mut self, msg: String) {
        if self.debug_log.len() >= 20 {
            self.debug_log.pop_front();
        }
        self.debug_log.push_back(msg);
    }
    
    pub fn get_debug_log(&self) -> Vec<String> {
        self.debug_log.iter().cloned().collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &AudioSubtitle> {
        // Return in chronological order: [oldest_final, ..., newest_final, interim]
        let interim_iter = if self.show_interim {
            Some(&self.interim_line).into_iter()
        } else {
            None.into_iter()
        };
        self.finishes_lines.iter().rev()
            .chain(interim_iter)
    }

    pub fn set_max_chars(&mut self, max_chars: usize) {
        self.max_chars_in_block = max_chars;
    }

    pub fn get_max_chars(&self) -> usize {
        self.max_chars_in_block
    }



    pub fn set_stability_params(&mut self, show_interim: bool, timeout_ms: u64) {
        self.show_interim = show_interim;
        self.stability_timeout = Duration::from_millis(timeout_ms);
    }

    pub fn get_active_char_count(&self) -> usize {
        self.finishes_lines.front().map(|l| l.text.len()).unwrap_or(0)
    }

    pub fn get_frozen_block_count(&self) -> usize {
        self.finishes_lines.len()
    }

    pub fn process_pending_events(&mut self, mode: &dyn crate::soniox::modes::SonioxMode) {
        while let Some((_, response)) = self.event_queue.pop_front() {
            mode.process_event(self, response);
        }
    }



    pub fn update_animation(&mut self, mode: &dyn crate::soniox::modes::SonioxMode) -> bool {
        self.process_pending_events(mode);

        // Check for stability timeout
        // Check for stability timeout
        if !self.interim_line.text.is_empty() && self.last_interim_update.elapsed() >= self.stability_timeout {
            let text_clone = self.interim_line.text.clone();
            
            // Smart Freeze: Only freeze up to the last word boundary (whitespace)
            // This prevents "Iamthe" merging by ensuring we only commit complete words.
            if let Some(last_space_idx) = text_clone.rfind(char::is_whitespace) {
                let split_idx = last_space_idx + 1; // Include the space
                let (frozen_part, remainder) = text_clone.split_at(split_idx);
                let frozen_string = frozen_part.to_string();
                let remainder_string = remainder.to_string();

                self.log_debug(format!("STABILITY: Freezing '{}'", frozen_string.trim()));
                
                let speaker = self.interim_line.speaker.clone();
                self.frozen_interim_history.push_str(&frozen_string);
                let added = self.push_final(speaker, frozen_string, false);
                self.frozen_blocks_count += added;
                
                // Keep the remainder as the new interim line
                self.interim_line.text = remainder_string;
                // Reset displayed text to restart typing for the remainder
                self.interim_line.displayed_text.clear();
                // Reset timer so the remainder has a fair chance to complete
                self.last_interim_update = Instant::now();
            }
        }

        let mut request_repaint = false;
        let mut animation_blocked = false;
        
        // How many lines are waiting to be typed?
        let mut waiting_count = 0;
        for line in &self.finishes_lines {
            if line.displayed_text.len() < line.text.len() {
                waiting_count += 1;
            }
        }
        if self.interim_line.displayed_text.len() < self.interim_line.text.len() {
            waiting_count += 1;
        }

        // Animate final blocks in chronological order (oldest first)
        for line in self.finishes_lines.iter_mut().rev() {
            if animation_blocked {
                break;
            }
            
            // If we have a backlog, speed up the typewriter (20ms -> 10ms or less)
            let speed_boost = if waiting_count > 1 { (waiting_count as usize).min(4) } else { 1 };
            for i in 0..speed_boost {
                if line.update_animation(i > 0) {
                    request_repaint = true;
                }
            }

            if line.displayed_text.len() < line.text.len() {
                animation_blocked = true;
            }
        }

        // Only animate interim if all final lines are finished
        if !animation_blocked {
            if self.interim_line.update_animation(false) {
                request_repaint = true;
            }
        }

        request_repaint
    }



    pub(crate) fn push_final(&mut self, speaker: Option<String>, mut text: String, instant: bool) -> usize {
        if text.is_empty() { return 0; }
        let mut added = 0;

        loop {
            if text.is_empty() { break; }
            
            // 1. Determine local chunk (Split ONLY if it contains an internal sentence ending)
            // Character-based splitting is disabled for "Word-like" flow.
            let (chunk, remainder) = if let Some(idx) = find_sentence_split(&text, 9999) {
                let (c, r) = text.split_at(idx);
                (c.to_string(), Some(r.to_string()))
            } else {
                (text.clone(), None)
            };

            // 2. Decide if we start a new block or merge
            let (should_start_new, _reason) = match self.finishes_lines.front() {
                Some(last) => {
                    let last_trimmed = last.text.trim_end();
                    let ends_sentence = last_trimmed.ends_with(|c| c == '.' || c == '?' || c == '!');
                    
                    // Fallback to prevent infinite block growth if there's no punctuation
                    let too_long = last.text.len() > 200; 
                    let is_mid_word = !last.text.ends_with(char::is_whitespace) && !chunk.starts_with(char::is_whitespace);
                    
                    if ends_sentence {
                        (true, "End of sentence")
                    } else if too_long && !is_mid_word {
                        (true, "Safety overflow")
                    } else {
                        (false, "")
                    }
                }
                None => (true, "Initial"),
            };

            if should_start_new {
                // self.log_debug(format!("BLOCK: New ({})", reason));
                let mut sub = AudioSubtitle::new(speaker.clone(), chunk);
                if instant { sub.displayed_text = sub.text.clone(); }
                self.finishes_lines.push_front(sub);
                added += 1;
            } else {
                // Merge logic
                let last = self.finishes_lines.front_mut().unwrap();
                let last_ends_with_space = last.text.ends_with(char::is_whitespace);
                let chunk_starts_with_space = chunk.starts_with(char::is_whitespace);
                
                if !last_ends_with_space && chunk_starts_with_space && chunk.trim_start().len() <= 2 {
                    // Hungarian fragment fix (milli + ó)
                    last.text.push_str(chunk.trim_start());
                } else {
                    last.text.push_str(&chunk);
                }
                if instant { last.displayed_text = last.text.clone(); }
            }

            if self.finishes_lines.len() >= self.max_lines {
                self.finishes_lines.pop_back();
            }

            if let Some(r) = remainder {
                text = r;
            } else {
                break;
            }
        }
        added
    }

    pub(crate) fn update_interim(&mut self, speaker: Option<String>, text: String) {
        // If the text is the same, do nothing.
        if self.interim_line.text == text && self.interim_line.speaker == speaker {
            return;
        }

        self.interim_line.speaker = speaker;
        let old_text = std::mem::replace(&mut self.interim_line.text, text);
        
        // Anti-spin / Typewriter preservation:
        // If the new text is just an expansion of the old text, 
        // DO NOT reset displayed_text.
        if self.interim_line.text.starts_with(&old_text) {
             // Good! Typewriter will just continue.
        } else {
            // It's a revision or a new phrase.
            // If the current displayed text is NOT a prefix of the new text, 
            // then we MUST reset the typewriter.
            if !self.interim_line.text.starts_with(&self.interim_line.displayed_text) {
                // self.log_debug("REVISION: Resetting typewriter".to_string());
                self.interim_line.displayed_text = String::new();
            }
        }
        
        // Immediate completion if it's identical or backwards (safety)
        if self.interim_line.displayed_text.len() > self.interim_line.text.len() {
             self.interim_line.displayed_text = self.interim_line.text.clone();
        }
    }
}

pub(crate) fn find_sentence_split(text: &str, limit: usize) -> Option<usize> {
    text.char_indices()
        .zip(text.chars().skip(1))
        .filter(|((i, c), next_c)| {
            *i < limit && (*c == '.' || *c == '?' || *c == '!') && next_c.is_whitespace()
        })
        .map(|((i, _), _)| i + 1)
        .next()
}
