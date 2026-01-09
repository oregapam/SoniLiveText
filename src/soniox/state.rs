use crate::types::audio::AudioSubtitle;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct TranscriptionState {
    finishes_lines: VecDeque<AudioSubtitle>,
    interim_line: AudioSubtitle,
    max_lines: usize,
    max_chars_in_block: usize,
    frozen_interim_history: String,
    frozen_blocks_count: usize,
    pub debug_log: VecDeque<String>,
    event_queue: VecDeque<(Instant, SonioxTranscriptionResponse)>,
    smart_delay_ms: u64,
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
            frozen_blocks_count: 0,
            debug_log: VecDeque::with_capacity(20),
            event_queue: VecDeque::new(),
            smart_delay_ms: 0,
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

    pub fn update_animation(&mut self) -> bool {
        // Process buffered events first
        self.process_pending_events();

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

    pub fn set_max_chars(&mut self, max_chars: usize) {
        self.max_chars_in_block = max_chars;
    }

    pub fn set_smart_delay(&mut self, delay_ms: u64) {
        self.smart_delay_ms = delay_ms;
    }

    pub fn handle_transcription(&mut self, response: SonioxTranscriptionResponse) {
        // Smart Buffering & Collapsing Logic
        // If the NEW response is purely Interim (no final parts), check if the last queued item is also purely Interim.
        // If so, and speaker matches, we can REPLACE the old one with the new one.
        // This effectively "collapses" the jittery intermediate updates.
        
        let is_purely_interim = !response.tokens.iter().any(|t| t.is_final);
        
        if is_purely_interim {
            if let Some((_, last_response)) = self.event_queue.back_mut() {
                let last_is_purely_interim = !last_response.tokens.iter().any(|t| t.is_final);
                if last_is_purely_interim {
                    // Check speaker match (heuristic: check first token speaker)
                    let new_speaker = response.tokens.first().map(|t| &t.speaker);
                    let last_speaker = last_response.tokens.first().map(|t| &t.speaker);
                    
                    if new_speaker == last_speaker {
                        // COLLAPSE: Update the text content, keep the timestamp? 
                        // If we keep timestamp, we process it sooner (good for latency).
                        // If we update timestamp, we delay it more (good for stability).
                        // Decision: Update timestamp to ensure the *new* text gets its full delay time to settle.
                        *last_response = response;
                        // Actually, we should probably update the timestamp to `now` if we want "stability delay".
                        // If we keep old timestamp, it might process immediately if old one was about to expire.
                        // Let's UPDATE timestamp to `Instant::now()` so the new text has to prove its stability.
                        // Wait, if we keep resetting timestamp, a constantly changing interim will NEVER appear?
                        // That's bad. The user wants to see it eventually.
                        // Better: Keep the ORIGINAL timestamp. The "slot" is due to be displayed. We just show the latest info in that slot.
                        // This minimizes latency.
                        // NO OP on timestamp.
                        return;
                    }
                }
            }
        }

        self.event_queue.push_back((Instant::now(), response));
    }

    fn process_transcription_event(&mut self, response: SonioxTranscriptionResponse) {
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
                 // We committed to history. Reset count.
                 self.frozen_blocks_count = 0;
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
                 // Do not reset count here! We are still "floating" on the remaining history.
                 
            } else {
                // CASE 3: Mismatch.
                // BACKTRACK!
                self.log_debug(format!("FINAL mismatch. Backtracking {} blocks. History: '{}' -> Final: '{}'", 
                    self.frozen_blocks_count, self.frozen_interim_history, final_text_segment));
                
                // Pop the unreliable ghost blocks
                for _ in 0..self.frozen_blocks_count {
                    self.finishes_lines.pop_front();
                }
                
                // Push correct text
                self.push_final(final_speaker.clone(), final_text_segment, true);
                
                // Reset
                self.frozen_blocks_count = 0;
                self.frozen_interim_history.clear();
            }
            
            // Also clear interim line because we have a final (or consumed it)
            self.update_interim(interim_speaker.clone(), String::new());
        }

        if !full_interim_text.is_empty() {
             // Check if interim matches our frozen history
             if !full_interim_text.starts_with(&self.frozen_interim_history) {
                 self.log_debug(format!("Interim mismatch! Resetting {} ghosts. H: '{}' N: '{}'", 
                    self.frozen_blocks_count, self.frozen_interim_history, full_interim_text));
                 
                 // Retroactively fix the drift
                 for _ in 0..self.frozen_blocks_count {
                     self.finishes_lines.pop_front();
                 }
                 self.frozen_blocks_count = 0;
                 self.frozen_interim_history.clear();
             }

             // Now we are synced (history is empty or a valid prefix)
             let effective_interim = full_interim_text[self.frozen_interim_history.len()..].to_string();

             let limit = self.max_chars_in_block;
            // Increased safety buffer to prevent premature freezing of sentences.
            // If it fits within limit + 25 chars, we let it flow to push_final 
            // where we have "orphan guard" logic.
            let safety_buffer = 25; 
            
            // PRIORITY 1: Freeze at Sentence End (if available and fits)
            // Look for [.?!] followed by whitespace (or end? No, need stability)
            let sentence_split_idx = effective_interim.char_indices()
                .zip(effective_interim.chars().skip(1)) // ( (i, c), next_c )
                .filter(|((i, c), next_c)| {
                     *i < limit && 
                     (*c == '.' || *c == '?' || *c == '!') && 
                     next_c.is_whitespace()
                })
                .map(|((i, _), _)| i + 1) // Include the punctuation
                .next(); // Take the FIRST one to prioritize "One sentence per line"

            if let Some(idx) = sentence_split_idx {
                let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                let frozen_chunk_str = frozen_chunk.to_string();
                
                self.log_debug(format!("FREEZE (Sentence): '{}'", frozen_chunk_str));

                self.frozen_interim_history.push_str(&frozen_chunk_str);
                
                let added = self.push_final(interim_speaker.clone(), frozen_chunk_str, true);
                self.frozen_blocks_count += added;
                
                // UN-HIDE: Show interim tail for real-time feedback
                self.update_interim(interim_speaker, remainder.to_string());
                // self.update_interim(interim_speaker, String::new());
                
            } else if effective_interim.len() > limit + safety_buffer {
                // PRIORITY 2: Freeze at Limit (Overflow preventer)
                let split_idx = effective_interim.char_indices()
                    .filter(|(i, c)| *i >= limit && c.is_whitespace())
                    .map(|(i, _)| i)
                    .next();

                if let Some(idx) = split_idx {
                    let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                    let frozen_chunk_str = frozen_chunk.to_string();
                    
                    self.log_debug(format!("FREEZE (Overflow): '{}' (len: {})", frozen_chunk_str, frozen_chunk_str.len()));

                    self.frozen_interim_history.push_str(&frozen_chunk_str);
                    
                    let added = self.push_final(interim_speaker.clone(), frozen_chunk_str, true);
                    self.frozen_blocks_count += added;
                    
                    // UN-HIDE: Show interim tail for real-time feedback
                    self.update_interim(interim_speaker, remainder.to_string());
                    // self.update_interim(interim_speaker, String::new());
                } else {
                     // UN-HIDE: Show interim
                     self.update_interim(interim_speaker, effective_interim);
                     // self.update_interim(interim_speaker, String::new());
                }
            } else {
                // UN-HIDE: Show interim
                self.update_interim(interim_speaker, effective_interim);
                // self.update_interim(interim_speaker, String::new());
            }
        } else if has_final {
        } else {
            self.update_interim(interim_speaker, String::new());
        }
    }

    // Returns number of NEW blocks created
    fn push_final(&mut self, speaker: Option<String>, mut text: String, instant: bool) -> usize {
        if text.is_empty() {
            return 0;
        }

        let mut blocks_added = 0;

        // Recursive / Iterative splitting for massive text
        loop {
             if text.is_empty() { break; }

             let (chunk, remainder) = if text.len() > self.max_chars_in_block {
                 // ORPHAN GUARD:
                 // If the text is only slightly longer than the limit (e.g. +15 chars),
                 // and it's a single sentence/phrase, forcing a split creates a small "orphan" line on the next block.
                 // We prefer to keep it as ONE block and let the UI wrapping handle it effectively.
                 // This reduces the "stairs" effect.
                 if text.len() <= self.max_chars_in_block + 15 {
                     (text, None)
                 } else {
                     // Too long, must split
                     let limit = self.max_chars_in_block;
                     let split_idx = text.char_indices()
                        .filter(|(i, c)| *i <= limit && c.is_whitespace())
                        .map(|(i, _)| i)
                        .last()
                        .or_else(|| {
                            text.char_indices()
                                .filter(|(i, c)| *i > limit && *i < limit + 10 && c.is_whitespace())
                                .map(|(i, _)| i)
                                .next()
                        })
                        .unwrap_or(limit.min(text.len()));
                     
                     let (c, r) = text.split_at(split_idx);
                     (c.to_string(), Some(r.to_string()))
                 }
             } else {
                 (text, None)
             };
             
             // Check if we should start a new block
             let should_start_new = match self.finishes_lines.front() {
                Some(last) => {
                    // Start new if:
                    // 1. Speaker changed
                    // 2. Length would exceed limit (UNLESS mid-word)
                    // 3. Last block ended with sentence punctuation (.?!) -> FORCE NEW LINE
                    let ends_sentence = last.text.trim_end().ends_with(|c| c == '.' || c == '?' || c == '!');
                    let is_continuation = !last.text.is_empty() 
                        && !last.text.ends_with(char::is_whitespace) 
                        && !chunk.starts_with(char::is_whitespace);
                    
                    // Loose speaker matching logic removed as we now ignore speaker changes entirely.

                    // USER REQUEST: Ignore speaker changes.
                    // Only start new block if:
                    // 1. Line overflow (checked below)
                    // 2. Previous block ended with sentence punctuation (.?!)
                    
                    // Note: We intentionally IGNORE speaker differences here to keep the flow.
                    
                    if (last.text.len() + chunk.len()) > self.max_chars_in_block + 15 {
                        if is_continuation {
                            // Exceptional case: We are in the middle of a word (e.g. "vis" + "ion").
                            // Do NOT split. Append even if it overflows.
                            let last_word = last.text.split_whitespace().last().unwrap_or("<empty>");
                            self.log_debug(format!("Overflow ignored (Mid-word): '{}' + '{}'", last_word, chunk));
                            false
                        } else {
                             let last_word = last.text.split_whitespace().last().unwrap_or("<empty>");
                             self.log_debug(format!("New Block: Overflow. {} + {} > {}. Last: '{}'", 
                                last.text.len(), chunk.len(), self.max_chars_in_block, last_word));
                            true
                        }
                    } else if ends_sentence {
                        self.log_debug("New Block: Sentence ends previous line.".to_string());
                        true
                    } else {
                        // Merge!
                        false
                    }
                }
                None => true,
            };

            if !should_start_new {
                let last = self.finishes_lines.front_mut().unwrap();
                // Smart merge: ensure space separator if needed
                if !last.text.ends_with(char::is_whitespace) && !chunk.starts_with(char::is_whitespace) {
                    last.text.push(' ');
                }
                last.text.push_str(&chunk);
                if instant {
                    last.displayed_text = last.text.clone();
                }
            } else {
                 let mut sub = AudioSubtitle::new(speaker.clone(), chunk);
                 if instant {
                     sub.displayed_text = sub.text.clone();
                 }
                 self.finishes_lines.push_front(sub);
                 blocks_added += 1;
            }

            if self.finishes_lines.len() > self.max_lines - 1 {
                self.finishes_lines.pop_back();
            }
            
            if let Some(r) = remainder {
                text = r;
            } else {
                break;
            }
        }
        
        blocks_added
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



