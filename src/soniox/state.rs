use crate::types::audio::AudioSubtitle;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct TranscriptionState {
    pub finishes_lines: VecDeque<AudioSubtitle>,
    pub interim_line: AudioSubtitle,
    max_lines: usize,
    max_chars_in_block: usize,
    frozen_interim_history: String,
    frozen_blocks_count: usize,
    pub debug_log: VecDeque<String>,
    event_queue: VecDeque<(Instant, SonioxTranscriptionResponse)>,
    smart_delay_ms: u64,
    last_final_ms: f64,
    show_interim: bool,
    stability_timeout: Duration,
    last_interim_update: Instant,
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
            smart_delay_ms: 0,
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

    pub fn set_smart_delay(&mut self, delay_ms: u64) {
        self.smart_delay_ms = delay_ms;
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

    pub fn process_pending_events(&mut self) {
        let now = Instant::now();
        let delay = Duration::from_millis(self.smart_delay_ms);

        while let Some((timestamp, _)) = self.event_queue.front() {
            if now.duration_since(*timestamp) >= delay {
                let (_, response) = self.event_queue.pop_front().unwrap();
                self.process_transcription_event(response);
            } else {
                break;
            }
        }
    }

    pub fn handle_transcription(&mut self, response: SonioxTranscriptionResponse) {
        let is_purely_interim = !response.tokens.iter().any(|t| t.is_final);
        
        if is_purely_interim {
            if let Some((_, last_response)) = self.event_queue.back_mut() {
                let last_is_purely_interim = !last_response.tokens.iter().any(|t| t.is_final);
                if last_is_purely_interim {
                    let new_speaker = response.tokens.first().map(|t| &t.speaker);
                    let last_speaker = last_response.tokens.first().map(|t| &t.speaker);
                    if new_speaker == last_speaker {
                        *last_response = response;
                        return;
                    }
                }
            }
        }
        self.event_queue.push_back((Instant::now(), response));
    }

    pub fn update_animation(&mut self) -> bool {
        self.process_pending_events();

        // Check for stability timeout
        if !self.interim_line.text.is_empty() && self.last_interim_update.elapsed() >= self.stability_timeout {
            let text = std::mem::take(&mut self.interim_line.text);
            let speaker = self.interim_line.speaker.clone();
            self.log_debug(format!("STABILITY: Freezing after timeout: '{}'", text.trim()));
            self.frozen_interim_history.push_str(&text);
            let added = self.push_final(speaker, text, false);
            self.interim_line.displayed_text.clear();
            self.frozen_blocks_count += added;
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

    fn process_transcription_event(&mut self, response: SonioxTranscriptionResponse) {
        let mut full_interim_text = String::new();
        let mut interim_speaker = Option::<String>::None;
        let mut final_text_segment = String::new();
        let mut final_speaker = Option::<String>::None;
        let mut has_final = false;

        let mut max_ms = self.last_final_ms;

        for token in response.tokens {
            let is_original = token.translation_status.as_deref() == Some("original");
            let is_translation = token.translation_status.as_deref() == Some("translation");
            
            // Timing update: track the furthest point finalized by the AI
            if is_original && token.is_final {
                if let Some(end_ms) = token.end_ms {
                    if end_ms > max_ms {
                        max_ms = end_ms;
                    }
                }
            }

            if token.is_final {
                // Deduplicate based on end_ms if available.
                // Note: Translation tokens often lack end_ms, but they are typically 
                // sent once per finalized segment.
                if let Some(end_ms) = token.end_ms {
                    if end_ms <= self.last_final_ms {
                        continue;
                    }
                }

                // If we are in translation mode, we only want to display "translation" tokens.
                // If translation mode is OFF, we want everything (which will have no status or "original").
                let show_this_token = if is_original {
                    // Only show original final tokens if we AREN'T expecting translations
                    // (Actually, if we see ANY translation token in the stream, we should probably stick to them)
                    false 
                } else {
                    // This is either a translated token or a normal one (no translate mode)
                    true
                };

                if show_this_token {
                    final_speaker = token.speaker.clone();
                    final_text_segment.push_str(&token.text);
                    has_final = true;
                }
            } else {
                // INTERIM processing.
                // We show original interim text as feedback until the translation arrives.
                if interim_speaker != token.speaker {
                    interim_speaker = token.speaker.clone();
                }
                full_interim_text.push_str(&token.text);
            }
        }

        self.last_final_ms = max_ms;

        if has_final {
            if final_text_segment.starts_with(&self.frozen_interim_history) {
                 let text_to_push = final_text_segment[self.frozen_interim_history.len()..].to_string();
                 self.log_debug(format!("FINAL: Pushing suffix '{}'", text_to_push.trim()));
                 self.push_final(final_speaker.clone(), text_to_push, false);
                 self.frozen_blocks_count = 0;
                 self.frozen_interim_history.clear();
            } else if self.frozen_interim_history.starts_with(&final_text_segment) {
                 self.log_debug(format!("FINAL: Already covered by history '{}'", final_text_segment.trim()));
                 self.frozen_interim_history.drain(..final_text_segment.len());
            } else {
                self.log_debug(format!("BACKTRACK: {} ghosts because of '{}'", self.frozen_blocks_count, final_text_segment.trim()));
                for _ in 0..self.frozen_blocks_count {
                    self.finishes_lines.pop_front();
                }
                self.push_final(final_speaker.clone(), final_text_segment, false);
                self.frozen_blocks_count = 0;
                self.frozen_interim_history.clear();
            }
            // CRITICAL: Don't call update_interim("") here if we are about to call it with text below.
            // That's what causes the "spin". We'll update it at the very end of this function.
        }

        let mut next_interim_text = String::new();

        if !full_interim_text.is_empty() {
             if !full_interim_text.starts_with(&self.frozen_interim_history) {
                 self.log_debug("Interim drift! Resetting ghosts.".to_string());
                 for _ in 0..self.frozen_blocks_count {
                     self.finishes_lines.pop_front();
                 }
                 self.frozen_blocks_count = 0;
                 self.frozen_interim_history.clear();
             }

             let effective_interim = full_interim_text[self.frozen_interim_history.len()..].to_string();
             // Dynamic limit for splitting is higher than the wrapping limit to allow natural flow.
             let split_limit = self.max_chars_in_block.max(100); 

             if let Some(idx) = find_sentence_split(&effective_interim, split_limit) {
                let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                let frozen_chunk_str = frozen_chunk.to_string();
                self.log_debug(format!("FREEZE (Sentence): '{}'", frozen_chunk_str.trim()));
                self.frozen_interim_history.push_str(&frozen_chunk_str);
                let added = self.push_final(interim_speaker.clone(), frozen_chunk_str, false);
                self.frozen_blocks_count += added;
                next_interim_text = remainder.to_string();
             } else if effective_interim.len() > split_limit + 50 { // Even more slack
                let split_idx = effective_interim.char_indices()
                    .filter(|(i, c)| *i >= split_limit && c.is_whitespace())
                    .map(|(i, _)| i)
                    .next();

                if let Some(idx) = split_idx {
                    let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                    let frozen_chunk_str = frozen_chunk.to_string();
                    self.log_debug(format!("FREEZE (Size): '{}'", frozen_chunk_str.trim()));
                    self.frozen_interim_history.push_str(&frozen_chunk_str);
                    let added = self.push_final(interim_speaker.clone(), frozen_chunk_str, false);
                    self.frozen_blocks_count += added;
                    next_interim_text = remainder.to_string();
                } else {
                     next_interim_text = effective_interim;
                }
             } else {
                next_interim_text = effective_interim;
             }
        }
        
        // Final update to interim line
        if self.interim_line.text != next_interim_text {
            self.last_interim_update = Instant::now();
        }
        self.update_interim(interim_speaker, next_interim_text);
    }

    fn push_final(&mut self, speaker: Option<String>, mut text: String, instant: bool) -> usize {
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
            let (should_start_new, reason) = match self.finishes_lines.front() {
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

    fn update_interim(&mut self, speaker: Option<String>, text: String) {
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

fn find_sentence_split(text: &str, limit: usize) -> Option<usize> {
    text.char_indices()
        .zip(text.chars().skip(1))
        .filter(|((i, c), next_c)| {
            *i < limit && (*c == '.' || *c == '?' || *c == '!') && next_c.is_whitespace()
        })
        .map(|((i, _), _)| i + 1)
        .next()
}
