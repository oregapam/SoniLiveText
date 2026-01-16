use eframe::egui;
use crate::types::settings::SettingsApp;
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
    
    fn show_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }
    
    fn launch(&mut self, ctx: &egui::Context) {
        // Validate
        if let Err(e) = self.current_config.validate() {
            self.show_status(format!("Config Invalid: {}", e));
            return;
        }
        
        // Save before launch if dirty (optional, but good UX)
        // For now, let's just create a temporary config or just require save?
        // Let's autosave if it's an existing project, or just use current state.
        // We just send the current state.
        
        let _ = self.tx_launch.send(self.current_config.clone());
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
        });

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
        ui.label("API Key:");
        let mut api_key = cfg.api_key.clone().unwrap_or_default();
        if ui.add(egui::TextEdit::singleline(&mut api_key).password(true)).changed() {
            cfg.api_key = Some(api_key);
        }
        ui.end_row();

        ui.label("Context:");
        let mut context = cfg.context.clone().unwrap_or_default();
        if ui.add(egui::TextEdit::multiline(&mut context).desired_rows(2)).changed() {
             cfg.context = Some(context);
        }
        ui.end_row();
        
        ui.label("Model:");
        let mut model = cfg.model.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut model).changed() {
            cfg.model = Some(model);
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
    // Target Language (Generic placeholder for now as enum matching is tedious in UI without macro, 
    // but we can assume default 'en')
    
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
    });
    
    ui.add_space(20.0);
    ui.heading("Logging");
    ui.horizontal(|ui| {
        let mut save = cfg.save_transcription.unwrap_or(false);
        if ui.checkbox(&mut save, "Save Transcription to file").changed() {
            cfg.save_transcription = Some(save);
        }
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
