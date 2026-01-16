#![windows_subsystem = "windows"]

use eframe::egui::ViewportBuilder;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use eframe::icon_data::from_png_bytes;
use sonilivetext::errors::SonioxWindowsErrors;
use sonilivetext::gui::utils::get_inner_size;
use sonilivetext::initialize_app;
use sonilivetext::windows::utils::{get_screen_size, show_error};
use std::sync::Arc;

const FONT_BYTES: &[u8] = include_bytes!("../assets/MPLUSRounded1c-Medium.ttf");
const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.png");

async fn run() -> Result<(), SonioxWindowsErrors> {
    // 1. Run Launcher (Phase 1)
    // We run the launcher in the main thread (blocking).
    // Note: eframe::run_native handles its own event loop.
    let launcher_result = sonilivetext::gui::launcher::run_launcher();
    
    let settings = match launcher_result {
        Ok(Some(s)) => s,
        Ok(None) => return Ok(()), // User closed without launching
        Err(e) => {
            log::error!("Launcher failed: {}", e);
            return Err(SonioxWindowsErrors::Internal(format!("Launcher error: {}", e)));
        }
    };

    // 2. Validate & Run Overlay (Phase 2)
    // The settings are now provided by the launcher, not loaded from config.toml directly.
    let (width, height) = get_screen_size();
    
    // We already have the settings struct, but let's re-validate just to be safe/consistent
    if let Err(msg) = settings.validate() {
        show_error(&msg);
        log::error!("{}", msg);
        std::process::exit(1);
    }

    // Validate model (BLOCKING)
    if let Err(e) = sonilivetext::soniox::validation::validate_model(&settings) {
        log::error!("Model validation failed: {}", e);
        
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR};

        unsafe {
            let msg = format!("Configuration Error:\n{}\n\nPlease check config.toml and try again.", e);
            
            // Convert to UTF-16 for Windows API
            let wide_msg: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
            
            MessageBoxW(
                None,
                windows::core::PCWSTR(wide_msg.as_ptr()),
                w!("SoniLiveText Error"),
                MB_OK | MB_ICONERROR
            );
        }
        std::process::exit(1);
    }

    let window_width = settings.window_width();
    let window_height = settings.window_height();
    
    // With mandatory width, get_inner_size is simpler.
    let (final_w, final_h) = get_inner_size(
        // screen width needed? Actually now we have specific width.
        // But get_inner_size might handle height default.
        width as f32, // potentially unused if we passed width directly to it, but let's check utils modification plan
        Some(window_width),
        Some(window_height),
    );
    
    // However, if window_width is NOT set, get_inner_size relied on position to calculate margin.
    // If we want to use anchor, we probably don't want the old "margin from position" logic for width.
    // Let's assume a default width if not set, or keep it safe.
    // The old logic was: width - pos_x - OFFSET*2. pos_x was OFFSET_WIDTH.
    // So default width was roughly screen_width - OFFSET*4.
    
    // For now, let's call get_position.
    let position = settings.get_position(width as f32, height as f32, final_w, final_h);
    
    // Re-calculate size if needed? No, size is fixed/resolved.
    // But get_inner_size might need the FINAL position if we keep the "dynamic width" logic based on margins.
    // Let's look at get_inner_size again.
    
    let app = initialize_app(settings)?;
    
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id("sublive")
            .with_icon(from_png_bytes(ICON_BYTES).expect("Failed to load icon"))
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_min_inner_size((final_w, final_h))
            .with_inner_size((final_w, final_h))
            .with_max_inner_size((final_w, final_h))
            .with_position(position),
        ..Default::default()
    };

    log::info!("Starting application");
    eframe::run_native(
        "Subtitles Live",
        native_options,
        Box::new(move |cc| {
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert(
                "mplus".to_owned(),
                Arc::new(FontData::from_static(FONT_BYTES)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "mplus".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("mplus".to_owned());
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        show_error(&format!("{}", err));
        log::error!("error in sonilivetext!: {:?}", err);
        std::process::exit(1);
    }
}
