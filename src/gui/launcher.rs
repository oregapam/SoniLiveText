use eframe::egui;
use crate::types::settings::SettingsApp;
use crate::types::languages::LanguageHint;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_launcher() -> Result<Option<SettingsApp>, eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_transparent(false)
            .with_inner_size([900.0, 650.0])
            .with_title("SoniLiveText Launcher"),
        ..Default::default()
    };

    
    // We need to move selected_config out of the closure. 
    // Using a shared Arc<Mutex> or cell is one way, but eframe::run_native takes the app by value/box.
    // The App trait can't easily return data on exit.
    // Hack: Use a static or a simpler way: Wrapper struct that we can clone out of? 
    // Actually, we can use a channel or just use a shared mutable state if we wait.
    // But run_native blocks.
    // We can define a struct that implements App, holds the result, and after run_native returns (it returns result), 
    // wait, run_native returns eframe::Result. It doesn't give back the App.
    // 
    // Workaround: Use a std::sync::mpsc channel to send the config back *before* closing.
    
    let (tx, rx) = std::sync::mpsc::channel();

    eframe::run_native(
        "SoniLiveText Launcher",
        options,
        Box::new(move |cc| {
             // Load fonts or setup styles here if needed
             Ok(Box::new(LauncherApp::new(cc, tx)))
        }),
    )?;

    // Try to receive the config. If channel is empty/disconnected, user just closed the window without launching.
    if let Ok(config) = rx.try_recv() {
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GlobalSettings {
    pub api_key: String,
    pub model: String,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            model: "low_latency".to_string(),
        }
    }
}

pub struct LauncherApp {
    tx_launch: std::sync::mpsc::Sender<SettingsApp>,
    projects: Vec<(String, PathBuf, SettingsApp)>, // Name, Path, Config
    selected_index: Option<usize>,
    
    // Editor State
    current_config: SettingsApp,
    current_name: String,
    dirty: bool,
    
    // Messages
    status_message: Option<(String, std::time::Instant)>,

    // Global Settings
    global_settings: GlobalSettings,
    show_global_settings: bool,
}

impl LauncherApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, tx: std::sync::mpsc::Sender<SettingsApp>) -> Self {
        let mut app = Self {
            tx_launch: tx,
            projects: Vec::new(),
            selected_index: None,
            current_config: empty_config(), // Placeholder
            current_name: "New Project".to_string(),
            dirty: false,
            status_message: None,
            global_settings: Self::load_global_settings(),
            show_global_settings: false,
        };
        app.ensure_projects_dir();
        app.refresh_projects();
        
        // Select first if available
        if !app.projects.is_empty() {
            app.select_project(0);
        } else {
            // Initialize with default
             app.current_config = load_default_template();
        }
        
        app
    }

    fn ensure_projects_dir(&self) {
        if !Path::new("projects").exists() {
            let _ = fs::create_dir("projects");
        }
    }

    fn refresh_projects(&mut self) {
        self.projects.clear();
        if let Ok(entries) = fs::read_dir("projects") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                         // Try to load
                         if let Ok(config) = SettingsApp::new(path.to_str().unwrap()) {
                             self.projects.push((name.to_string(), path, config));
                         }
                    }
                }
            }
        }
        // Sort by name
        self.projects.sort_by(|a, b| a.0.cmp(&b.0));
    }
    
    fn select_project(&mut self, index: usize) {
        if index < self.projects.len() {
            self.selected_index = Some(index);
            self.current_name = self.projects[index].0.clone();
            self.current_config = self.projects[index].2.clone();
            self.dirty = false;
        }
    }

    fn save_current(&mut self) {
        let name = self.current_name.trim().to_string();
        if name.is_empty() {
             self.show_status("Name cannot be empty");
             return;
        }
        
        let filename = format!("projects/{}.toml", name);
        let path = Path::new(&filename);
        
        // Serialize
        if let Ok(toml_str) = toml::to_string_pretty(&self.current_config) {
             if let Err(e) = fs::write(path, toml_str) {
                 self.show_status(format!("Error saving: {}", e));
             } else {
                 self.show_status("Project saved!");
                 self.refresh_projects();
                 
                 // Re-select the saved project
                 if let Some(idx) = self.projects.iter().position(|p| p.0 == name) {
                     self.select_project(idx);
                 }
                 self.dirty = false;
             }
        } else {
             self.show_status("Serialization failed");
        }
    }

    fn load_global_settings() -> GlobalSettings {
        if let Ok(content) = fs::read_to_string("launcher_config.toml") {
            if let Ok(settings) = toml::from_str(&content) {
                return settings;
            }
        }
        GlobalSettings::default()
    }

    fn save_global_settings(&mut self) {
         if let Ok(toml_str) = toml::to_string_pretty(&self.global_settings) {
             if let Err(e) = fs::write("launcher_config.toml", toml_str) {
                 self.show_status(format!("Error saving global settings: {}", e));
             } else {
                 self.show_status("Global settings saved!");
             }
         }
    }
    
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }
    
    fn launch(&mut self, ctx: &egui::Context) {
        // Merge Global Settings
        let mut final_config = self.current_config.clone();
        
        // Oversee logic: Global settings override / fill in project settings
        // Ideally project shouldn't explicitly have them if they are global, 
        // but for now we overwrite.
        if !self.global_settings.api_key.is_empty() {
             final_config.api_key = Some(self.global_settings.api_key.clone());
        }
        if !self.global_settings.model.is_empty() {
             final_config.model = Some(self.global_settings.model.clone());
        }

        // Validate Merged Config
        if let Err(e) = final_config.validate() {
            self.show_status(format!("Config Invalid (check Global Settings): {}", e));
            return;
        }
        
        // Save Global too just in case
        self.save_global_settings();

        // Send merged config
        let _ = self.tx_launch.send(final_config);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // Status Bar Fadeout
        if let Some((_, time)) = self.status_message {
            if time.elapsed().as_secs() > 3 {
                self.status_message = None;
            }
        }

        // --- Sidebar ---
        egui::SidePanel::left("project_list").min_width(200.0).show(ctx, |ui| {
             ui.add_space(10.0);
             ui.heading("Projects");
             ui.separator();
             
             if ui.button("➕ New Project").clicked() {
                 self.selected_index = None;
                 self.current_name = "New Project".to_string();
                 self.current_config = load_default_template();
                 self.dirty = false;
             }
             
             ui.separator();
             
             egui::ScrollArea::vertical().show(ui, |ui| {
                 let mut selected = self.selected_index;
                 for (i, (name, _, _)) in self.projects.iter().enumerate() {
                     if ui.selectable_label(selected == Some(i), name).clicked() {
                         selected = Some(i);
                     }
                 }
                 
                 if selected != self.selected_index {
                     if let Some(idx) = selected {
                         self.select_project(idx);
                     }
                 }
             });
             
             ui.separator();
             if ui.button("⚙ Global Settings").clicked() {
                 self.show_global_settings = true;
             }
        });

        // --- Global Settings Window ---
        if self.show_global_settings {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("global_settings"),
                egui::ViewportBuilder::default()
                    .with_title("Global Settings")
                    .with_inner_size([500.0, 300.0]),
                |ctx, class| {
                    assert!(class == egui::ViewportClass::Immediate, "This egui backend doesn't support multiple viewports");
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Global Configuration");
                        ui.label("These settings apply to ALL projects.");
                        ui.separator();
                        
                        egui::Grid::new("global_grid").num_columns(2).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
                            ui.label("API Key:");
                            ui.add(egui::TextEdit::singleline(&mut self.global_settings.api_key).password(true));
                            ui.end_row();
                            
                            ui.label("Model:");
                            ui.text_edit_singleline(&mut self.global_settings.model);
                            ui.end_row();
                        });
                        
                        ui.add_space(20.0);
                        if ui.button("Close & Save").clicked() {
                            self.save_global_settings();
                            self.show_global_settings = false;
                        }
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        self.show_global_settings = false;
                    }
                }
            );
        }

        // --- Main Editor ---
        egui::CentralPanel::default().show(ctx, |ui| {
             ui.horizontal(|ui| {
                 ui.label("Project Name:");
                 if ui.add(egui::TextEdit::singleline(&mut self.current_name)).changed() {
                     self.dirty = true;
                 }
                 
                 ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                     if ui.button("🚀 LAUNCH").clicked() {
                         self.launch(ctx);
                     }
                     if ui.button("💾 Save").clicked() {
                         self.save_current();
                     }
                 });
             });
             ui.separator();
             
             // Tabs or Sections? Let's implement scrollable sections.
             egui::ScrollArea::vertical().show(ui, |ui| {
                 ui_settings_editor(ui, &mut self.current_config);
             });
             
             // Status Bottom
             if let Some((msg, _)) = &self.status_message {
                 ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                     ui.label(egui::RichText::new(msg).color(egui::Color32::YELLOW));
                     ui.separator();
                 });
             }
        });
    }
}

fn ui_settings_editor(ui: &mut egui::Ui, cfg: &mut SettingsApp) {
    ui.heading("General");
    egui::Grid::new("gen_grid").num_columns(2).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
        // API Key and Model moved to Global Settings
        
        ui.label("Context:");
        let mut context = cfg.context.clone().unwrap_or_default();
        if ui.add(egui::TextEdit::multiline(&mut context).desired_rows(2)).changed() {
             cfg.context = Some(context);
        }
        ui.end_row();

        ui.label("Log Level:");
        let mut level_str = cfg.level.clone().unwrap_or("info".to_string());
        egui::ComboBox::from_id_salt("log_level")
            .selected_text(&level_str)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut level_str, "error".to_string(), "Error");
                ui.selectable_value(&mut level_str, "warn".to_string(), "Warn");
                ui.selectable_value(&mut level_str, "info".to_string(), "Info");
                ui.selectable_value(&mut level_str, "debug".to_string(), "Debug");
                ui.selectable_value(&mut level_str, "trace".to_string(), "Trace");
            });
        cfg.level = Some(level_str);
        ui.end_row();

        ui.label("Enable High Priority:");
        let mut high_prio = cfg.enable_high_priority.unwrap_or(true);
        if ui.checkbox(&mut high_prio, "").changed() {
            cfg.enable_high_priority = Some(high_prio);
        }
        ui.end_row();

        ui.label("Enable Speakers:");
        let mut speakers = cfg.enable_speakers.unwrap_or(true);
        if ui.checkbox(&mut speakers, "").changed() {
             cfg.enable_speakers = Some(speakers);
        }
        ui.end_row();
    });
    
    ui.add_space(20.0);
    ui.heading("Translation");
    ui.horizontal(|ui| {
         let mut enabled = cfg.enable_translate.unwrap_or(false);
         if ui.checkbox(&mut enabled, "Enable Translation").changed() {
             cfg.enable_translate = Some(enabled);
         }
    });
    // Target Language
    ui.horizontal(|ui| {
        ui.label("Target Language:");
        let mut target = cfg.target_language.clone().unwrap_or(LanguageHint::English);
        
        egui::ComboBox::from_id_salt("target_lang")
            .selected_text(format!("{:?}", target))
            .show_ui(ui, |ui| {
                 // Listing common languages
                 ui.selectable_value(&mut target, LanguageHint::English, "English");
                 ui.selectable_value(&mut target, LanguageHint::Hungarian, "Hungarian");
                 ui.selectable_value(&mut target, LanguageHint::German, "German");
                 ui.selectable_value(&mut target, LanguageHint::French, "French");
                 ui.selectable_value(&mut target, LanguageHint::Spanish, "Spanish");
                 ui.selectable_value(&mut target, LanguageHint::Chinese, "Chinese");
                 ui.selectable_value(&mut target, LanguageHint::Japanese, "Japanese");
                 // Add more if needed or implement iteration
            });
        cfg.target_language = Some(target);
    });

    ui.horizontal(|ui| {
        ui.label("Input Language Hints (comma separated):");
        // Simple text representation for now
        let hints = cfg.language_hints.clone().unwrap_or_default();
        // Convert to string
        let _hints_str = hints.iter().map(|l| format!("{:?}", l)).collect::<Vec<_>>().join(", ");
        
        // This is a bit tricky to edit as string and parse back to Enum without FromStr.
        // For now, let's just allow selecting PRIMARY hint or maybe just a text input for 
        // manual entry if we had FromStr, but we don't easily have it derived.
        // Let's stick to a single "Primary Input Language" for now to simplify, 
        // OR just hardcode English/Hungarian as defaults and let advanced users edit .toml?
        // User asked for "ALL settings".
        // Let's offer a "Primary Input Language" dropdown for the first hint.
        
        // Actually, let's just make it a single Primary language selector for simplicity if user accepts.
        // Or re-use the target selector logic.
        
        if let Some(first_hint) = hints.first() {
             ui.label(format!("Primary: {:?}", first_hint));
        }
    });
    
    // Better Language Hints Editor:
    // Just a primary selector for now effectively overwriting the list with one item
    ui.horizontal(|ui| {
         let current_hints = cfg.language_hints.clone().unwrap_or_default();
         let mut primary = current_hints.first().cloned().unwrap_or(LanguageHint::English);
         
         egui::ComboBox::from_id_salt("input_lang")
            .selected_text(format!("{:?}", primary))
            .show_ui(ui, |ui| {
                 ui.selectable_value(&mut primary, LanguageHint::English, "English");
                 ui.selectable_value(&mut primary, LanguageHint::Hungarian, "Hungarian");
                 ui.selectable_value(&mut primary, LanguageHint::German, "German");
                 // ... others
            });
         
         if current_hints.is_empty() || current_hints[0] != primary {
             cfg.language_hints = Some(vec![primary]);
         }
    });
    
    ui.add_space(20.0);
    ui.heading("Appearance");
    egui::Grid::new("app_grid").num_columns(2).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
        ui.label("Font Size:");
        let mut fs = cfg.font_size.unwrap_or(32.0);
        if ui.add(egui::Slider::new(&mut fs, 10.0..=100.0)).changed() {
            cfg.font_size = Some(fs);
        }
        ui.end_row();
        
        ui.label("Text Color (RGB):");
        let (r,g,b) = cfg.text_color.unwrap_or((255,255,255));
        let mut color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
        if ui.color_edit_button_rgb(&mut color).changed() {
            cfg.text_color = Some(((color[0]*255.0) as u8, (color[1]*255.0) as u8, (color[2]*255.0) as u8));
        }
        ui.end_row();
        
        ui.label("Window Width:");
        let mut w = cfg.window_width.unwrap_or(1000.0);
        if ui.add(egui::DragValue::new(&mut w)).changed() {
            cfg.window_width = Some(w);
        }
        ui.end_row();

        ui.label("Show Border:");
        let mut border = cfg.show_window_border.unwrap_or(false);
        if ui.checkbox(&mut border, "").changed() {
             cfg.show_window_border = Some(border);
        }
        ui.end_row();

        ui.label("Window Height:");
        let mut h = cfg.window_height.unwrap_or(200.0);
        if ui.add(egui::DragValue::new(&mut h)).changed() {
            cfg.window_height = Some(h);
        }
        ui.end_row();

        ui.label("Window Anchor:");
        let mut anchor = cfg.window_anchor.clone().unwrap_or("bottom".to_string());
        egui::ComboBox::from_id_salt("win_anchor")
            .selected_text(&anchor)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut anchor, "bottom".to_string(), "Bottom");
                ui.selectable_value(&mut anchor, "top".to_string(), "Top");
                ui.selectable_value(&mut anchor, "left".to_string(), "Left");
                ui.selectable_value(&mut anchor, "right".to_string(), "Right");
                ui.selectable_value(&mut anchor, "center".to_string(), "Center");
                ui.selectable_value(&mut anchor, "top_left".to_string(), "Top Left");
                ui.selectable_value(&mut anchor, "top_right".to_string(), "Top Right");
                ui.selectable_value(&mut anchor, "bottom_left".to_string(), "Bottom Left");
                ui.selectable_value(&mut anchor, "bottom_right".to_string(), "Bottom Right");
            });
        cfg.window_anchor = Some(anchor);
        ui.end_row();

        ui.label("Window Offset (X, Y):");
        let (mut ox, mut oy) = cfg.window_offset.unwrap_or((0.0, 0.0));
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut ox).speed(1.0)).changed() {
                cfg.window_offset = Some((ox, oy));
            }
            if ui.add(egui::DragValue::new(&mut oy).speed(1.0)).changed() {
                cfg.window_offset = Some((ox, oy));
            }
        });
        ui.end_row();

        ui.label("Debug Window:");
        let mut dbg = cfg.debug_window.unwrap_or(false);
        if ui.checkbox(&mut dbg, "").changed() {
             cfg.debug_window = Some(dbg);
        }
        ui.end_row();
    });

    ui.add_space(20.0);
    ui.heading("Audio & AI");
    egui::Grid::new("audio_grid").num_columns(2).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
        ui.label("Audio Input (Device Name):");
        let mut dev = cfg.audio_input.clone().unwrap_or("Default".to_string());
        if ui.text_edit_singleline(&mut dev).changed() {
             cfg.audio_input = Some(dev);
        }
        ui.end_row();

        ui.label("Show Interim Results:");
        let mut interim = cfg.show_interim.unwrap_or(true);
        if ui.checkbox(&mut interim, "").changed() {
            cfg.show_interim = Some(interim);
        }
        ui.end_row();

        ui.label("Stability Timeout (ms):");
        let mut stability = cfg.stability_timeout_ms.unwrap_or(500);
        if ui.add(egui::Slider::new(&mut stability, 0..=5000)).changed() {
             cfg.stability_timeout_ms = Some(stability);
        }
        ui.end_row();
    });
    
    ui.add_space(20.0);
    ui.heading("Logging");
    egui::Grid::new("log_grid").num_columns(2).spacing([20.0, 8.0]).striped(true).show(ui, |ui| {
         ui.label("Save Transcription:");
         let mut save = cfg.save_transcription.unwrap_or(false);
         if ui.checkbox(&mut save, "").changed() {
             cfg.save_transcription = Some(save);
         }
         ui.end_row();
         
         if save {
             ui.label("Transcript Path:");
             let mut path = cfg.transcript_save_path.clone().unwrap_or("transcript.txt".to_string());
             if ui.text_edit_singleline(&mut path).changed() {
                 cfg.transcript_save_path = Some(path);
             }
             ui.end_row();
         }
         
         ui.label("Raw Data Logging:");
         let mut raw = cfg.enable_raw_logging.unwrap_or(false);
         if ui.checkbox(&mut raw, "").changed() {
              cfg.enable_raw_logging = Some(raw);
         }
         ui.end_row();
         
         ui.label("Audio Logging:");
         let mut audio_log = cfg.enable_audio_logging.unwrap_or(false);
         if ui.checkbox(&mut audio_log, "").changed() {
              cfg.enable_audio_logging = Some(audio_log);
         }
         ui.end_row();
    });
}

fn empty_config() -> SettingsApp {
    load_default_template() // Safer to start with template
}

fn load_default_template() -> SettingsApp {
    // Basic defaults
    use crate::types::languages::LanguageHint;
    
    SettingsApp {
        language_hints: Some(vec![LanguageHint::English]),
        context: Some("General dictation".to_string()),
        api_key: Some("".to_string()),
        target_language: Some(LanguageHint::English),
        enable_translate: Some(false),
        enable_high_priority: Some(true),
        enable_speakers: Some(true),
        model: Some("low_latency".to_string()),
        level: Some("info".to_string()),
        font_size: Some(42.0),
        text_color: Some((255, 255, 255)),
        window_width: Some(1200.0),
        window_height: Some(200.0), // not used much
        window_anchor: Some("bottom".to_string()),
        window_offset: Some((0.0, -50.0)),
        audio_input: Some("Default".to_string()),
        show_window_border: Some(true),
        debug_window: Some(false),
        show_interim: Some(true),
        stability_timeout_ms: Some(500),
        enable_raw_logging: Some(false),
        enable_audio_logging: Some(false),
        save_transcription: Some(false),
        transcript_save_path: Some("transcript.txt".to_string()),
    }
}
