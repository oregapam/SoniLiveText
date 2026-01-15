use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use crate::types::soniox::{SonioxTranscriptionRequest, SonioxTranslationObject};
use crate::soniox::modes::SonioxMode;
// use wasapi::{DeviceEnumerator, Direction, initialize_mta};

pub struct TranslateMode;

use crate::soniox::state::TranscriptionState;
use crate::types::soniox::SonioxTranscriptionResponse;
use std::time::Instant;

impl SonioxMode for TranslateMode {
    fn create_request<'a>(&self, settings: &'a SettingsApp, audio_format: (u32, u16)) -> Result<SonioxTranscriptionRequest<'a>, SonioxWindowsErrors> {
        let (sample_rate, channels) = audio_format;
        
        let translation_obj = SonioxTranslationObject {
            r#type: "one_way",
            target_language: Some(settings.target_language()),
            ..Default::default()
        };

        let request = SonioxTranscriptionRequest {
            api_key: settings.api_key(),
            model: settings.model(),
            audio_format: "pcm_s16le",
            sample_rate: Some(sample_rate),
            num_channels: Some(channels as u32),
            context: Some(settings.context()),
            language_hints: settings.language_hints(),
            enable_speaker_diarization: Some(settings.enable_speakers()),
            enable_non_final_tokens: Some(true),
            enable_endpoint_detection: Some(true),
            translation: Some(translation_obj),
            ..Default::default()
        };

        Ok(request)
    }

    fn handle_incoming(&self, state: &mut TranscriptionState, response: SonioxTranscriptionResponse) {
        let is_purely_interim = !response.tokens.iter().any(|t| t.is_final);
        
        if is_purely_interim {
            if let Some((_, last_response)) = state.event_queue.back_mut() {
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
        state.event_queue.push_back((Instant::now(), response));
    }

    fn process_event(&self, state: &mut TranscriptionState, response: SonioxTranscriptionResponse) {
        let mut full_interim_text = String::new();
        let mut interim_speaker = Option::<String>::None;
        let mut final_text_segment = String::new();
        let mut final_speaker = Option::<String>::None;
        let mut has_final = false;

        let mut max_ms = state.last_final_ms;

        for token in response.tokens {
            // Sanitizer: Filter out <end> tags or empty text
            if token.text.contains("<end>") {
                continue;
            }

            // Strict Mode: In TranslateMode, we ONLY want tokens explicitly marked as "translation".
            // "original" tokens (source language) must be filtered out to avoid mixed output.
            let is_translation = token.translation_status.as_deref() == Some("translation");
            
            if !is_translation {
                continue;
            }

            // Timing update: track the furthest point finalized by the AI
            // Note: Translation tokens usually follow the timing of original tokens, 
            // but might not have their own timestamps. verification needed if this logic is relevant for translation.
            // For now, if we have a timestamp, use it.
            if token.is_final {
                if let Some(end_ms) = token.end_ms {
                    if end_ms > max_ms {
                        max_ms = end_ms;
                    }
                }
            }

            if token.is_final {
                // Deduplicate based on end_ms if available.
                if let Some(end_ms) = token.end_ms {
                    if end_ms <= state.last_final_ms {
                        continue;
                    }
                }

                final_speaker = token.speaker.clone();
                final_text_segment.push_str(&token.text);
                has_final = true;
            } else {
                // INTERIM processing.
                // Since we filter strictly for "translation", this will now accumulate 
                // only the translated interim text, preventing the "original text flash".
                if interim_speaker != token.speaker {
                    interim_speaker = token.speaker.clone();
                }
                full_interim_text.push_str(&token.text);
            }
        }

        state.last_final_ms = max_ms;

        if has_final {
            // Log the authoritative final text (decoupled from screen state/freezing)
            state.log_final_text(&final_text_segment);

            if final_text_segment.starts_with(&state.frozen_interim_history) {
                 let text_to_push = final_text_segment[state.frozen_interim_history.len()..].to_string();
                 state.log_debug(format!("FINAL: Pushing suffix '{}'", text_to_push.trim()));
                 state.push_final(final_speaker.clone(), text_to_push, false);
                 state.frozen_blocks_count = 0;
                 state.frozen_interim_history.clear();
            } else if state.frozen_interim_history.starts_with(&final_text_segment) {
                 state.log_debug(format!("FINAL: Already covered by history '{}'", final_text_segment.trim()));
                 state.frozen_interim_history.drain(..final_text_segment.len());
            } else {
                state.log_debug(format!("BACKTRACK: {} ghosts because of '{}'", state.frozen_blocks_count, final_text_segment.trim()));
                for _ in 0..state.frozen_blocks_count {
                    state.finishes_lines.pop_front();
                }
                state.push_final(final_speaker.clone(), final_text_segment, false);
                state.frozen_blocks_count = 0;
                state.frozen_interim_history.clear();
            }
            // CRITICAL: Don't call update_interim("") here if we are about to call it with text below.
            // That's what causes the "spin". We'll update it at the very end of this function.
        }

        let mut next_interim_text = String::new();

        if !full_interim_text.is_empty() {
             if !full_interim_text.starts_with(&state.frozen_interim_history) {
                 state.log_debug("Interim drift! Resetting ghosts.".to_string());
                 for _ in 0..state.frozen_blocks_count {
                     state.finishes_lines.pop_front();
                 }
                 state.frozen_blocks_count = 0;
                 state.frozen_interim_history.clear();
             }

             let effective_interim = full_interim_text[state.frozen_interim_history.len()..].to_string();
             // Dynamic limit for splitting is higher than the wrapping limit to allow natural flow.
             let split_limit = state.max_chars_in_block.max(100); 

             if let Some(idx) = crate::soniox::state::find_sentence_split(&effective_interim, split_limit) {
                let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                let frozen_chunk_str = frozen_chunk.to_string();
                state.log_debug(format!("FREEZE (Sentence): '{}'", frozen_chunk_str.trim()));
                state.frozen_interim_history.push_str(&frozen_chunk_str);
                let added = state.push_final(interim_speaker.clone(), frozen_chunk_str, false);
                state.frozen_blocks_count += added;
                next_interim_text = remainder.to_string();
             } else if effective_interim.len() > split_limit + 50 { // Even more slack
                let split_idx = effective_interim.char_indices()
                    .filter(|(i, c)| *i >= split_limit && c.is_whitespace())
                    .map(|(i, _)| i)
                    .next();

                if let Some(idx) = split_idx {
                    let (frozen_chunk, remainder) = effective_interim.split_at(idx);
                    let frozen_chunk_str = frozen_chunk.to_string();
                    state.log_debug(format!("FREEZE (Size): '{}'", frozen_chunk_str.trim()));
                    state.frozen_interim_history.push_str(&frozen_chunk_str);
                    let added = state.push_final(interim_speaker.clone(), frozen_chunk_str, false);
                    state.frozen_blocks_count += added;
                    next_interim_text = remainder.to_string();
                } else {
                     next_interim_text = effective_interim;
                }
             } else {
                next_interim_text = effective_interim;
             }
        }
        
        // Final update to interim line
        if state.interim_line.text != next_interim_text {
            state.last_interim_update = Instant::now();
        }
        state.update_interim(interim_speaker, next_interim_text);
    }
}
