use crate::gui::draw::draw_text_with_shadow;
use crate::soniox::state::TranscriptionState;
use crate::types::audio::AudioMessage;
use crate::types::soniox::SonioxTranscriptionResponse;
use crate::windows::utils::{initialize_tool_window, initialize_window, make_window_click_through};
use eframe::egui::{CentralPanel, Context, Visuals};
use eframe::epaint::Color32;
use eframe::{App, Frame};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const POLL_INTERVAL: Duration = Duration::from_millis(20);

use crate::soniox::modes::SonioxMode;

pub struct SubtitlesApp {
    rx_transcription: UnboundedReceiver<SonioxTranscriptionResponse>,
    tx_audio: UnboundedSender<AudioMessage>,
    tx_exit: UnboundedSender<bool>,
    initialized_windows: bool,
    enable_high_priority: bool,
    font_size: f32,
    text_color: Color32,
    subtitles_state: TranscriptionState,
    show_window_border: bool,
    interim_current_height: f32,
    debug_window_enabled: bool,
    mode: Box<dyn SonioxMode + Send + Sync>, 
}

impl SubtitlesApp {
    pub fn new(
        rx_transcription: UnboundedReceiver<SonioxTranscriptionResponse>,
        tx_exit: UnboundedSender<bool>,
        tx_audio: UnboundedSender<AudioMessage>,
        enable_high_priority: bool,
        font_size: f32,
        text_color: Color32,
        show_window_border: bool,
        window_width: f32,
        debug_window_enabled: bool,
        smart_delay_ms: u64,
        show_interim: bool,
        stability_timeout_ms: u64,
        mode: Box<dyn SonioxMode + Send + Sync>,
    ) -> Self {
        // ... (preserving logic)
        let usable_width = window_width * 0.88;
        let avg_char_width = font_size * 0.46;
        let chars_per_line = usable_width / avg_char_width;
        let max_chars = ((chars_per_line * 0.95) as usize).max(50);

        let mut subtitles_state = TranscriptionState::new(50, max_chars);
        subtitles_state.set_smart_delay(smart_delay_ms);
        subtitles_state.set_stability_params(show_interim, stability_timeout_ms);

        Self {
            rx_transcription,
            tx_exit,
            tx_audio,
            enable_high_priority,
            font_size,
            text_color,
            initialized_windows: false,
            subtitles_state,
            show_window_border,
            interim_current_height: 0.0,
            debug_window_enabled,
            mode,
        }
    }
}

impl App for SubtitlesApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let mut app_frame = eframe::egui::Frame::default().fill(Color32::TRANSPARENT);
        if self.show_window_border {
            app_frame = app_frame.stroke(eframe::egui::Stroke::new(2.0, self.text_color));
        }

        // Capture main window rect for debug info
        let main_rect = ctx.input(|i| i.viewport().inner_rect.unwrap_or(eframe::egui::Rect::ZERO));

        // Dynamically update max_chars based on current window width
        // Middle Ground Tuning: 88% width, 0.46 char width factor.
        // This allows more text than the conservative default (0.8/0.5) 
        // Recalculate max chars based on current window width
        let usable_width = main_rect.width() * 0.88;
        let avg_char_width = self.font_size * 0.46;
        let chars_per_line = usable_width / avg_char_width;
        let max_chars = (chars_per_line as usize).max(50);
        self.subtitles_state.set_max_chars(max_chars);

        // Separate Native Debug Window
        if self.debug_window_enabled {
            ctx.show_viewport_immediate(
                eframe::egui::ViewportId::from_hash_of("debug_viewport"),
                eframe::egui::ViewportBuilder::default()
                    .with_title("SoniLiveText Debug")
                    .with_inner_size([300.0, 500.0])
                    .with_always_on_top(),
                |ctx, _class| {
                    eframe::egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Debug Info");
                        ui.separator();
                        ui.label(format!("Max Chars/Block: {}", self.subtitles_state.get_max_chars()));
                        ui.label(format!("Active Char Count: {}", self.subtitles_state.get_active_char_count()));
                        ui.label(format!("Frozen Blocks: {}", self.subtitles_state.get_frozen_block_count()));
                        
                        ui.label(format!("Main Window: {:.0} x {:.0}", main_rect.width(), main_rect.height()));
                        
                        ui.label(format!("Interim Height: {:.2}", self.interim_current_height));
                        ui.label(format!("Font Size: {:.1}", self.font_size));
                        if self.subtitles_state.get_active_char_count() > self.subtitles_state.get_max_chars() {
                            ui.colored_label(Color32::RED, "OVERFLOW / FREEZING");
                        }
                        
                        ui.separator();
                        ui.label("Recent Events:");
                        eframe::egui::ScrollArea::vertical().max_height(ui.available_height() - 20.0).show(ui, |ui| {
                            for msg in self.subtitles_state.get_debug_log().iter().rev() {
                                 ui.label(eframe::egui::RichText::new(msg).size(12.0));
                            }
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // How to handle close? Just ignore or hide?
                        // For now, let it close, but next frame it might reappear if we call this again?
                        // Actually show_viewport_immediate re-creates it if needed.
                        // If user closes it, maybe we should stop calling it?
                        // But for dev, let's keep it persistent.
                    }
                },
            );
        }

        CentralPanel::default()
            .frame(app_frame)
            .show(ctx, |ui| {
                make_window_click_through(frame);
                if !self.initialized_windows {
                    initialize_window(frame);
                    self.initialized_windows = true;
                }
                if self.enable_high_priority {
                    initialize_tool_window(frame);
                }
                if let Ok(transcription) = self.rx_transcription.try_recv() {
                    self.mode.handle_incoming(&mut self.subtitles_state, transcription);
                    // Data changed, need repaint
                    ctx.request_repaint();
                }
                
                if self.subtitles_state.update_animation(self.mode.as_ref()) {
                    ctx.request_repaint();
                }

                ui.vertical(|ui| {
                    let target_height = draw_text_with_shadow(
                        ui,
                        self.subtitles_state.iter(),
                        self.font_size,
                        self.text_color,
                        self.interim_current_height,
                    );
                    
                    // Smoothly animate towards target height
                    let diff = target_height - self.interim_current_height;
                    // If difference is significant, animate
                    if diff.abs() > 0.1 {
                        // Speed factor. 60 FPS. 
                        // Move 10% of the diff per frame -> nice ease out.
                        self.interim_current_height += diff * 0.1;
                        ctx.request_repaint();
                    } else {
                        self.interim_current_height = target_height;
                    }
                });
                
                // Ensure we poll for new data even if no events come in
                ctx.request_repaint_after(POLL_INTERVAL);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.tx_audio.send(AudioMessage::Stop);
        let _ = self.tx_exit.send(true);
        self.rx_transcription.close();
    }

    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
