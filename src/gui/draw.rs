use crate::types::audio::AudioSubtitle;
use eframe::egui::{Ui, pos2, vec2};
use eframe::epaint::{Color32, FontId};

pub(crate) fn draw_text_with_shadow<'a>(
    ui: &mut Ui,
    lines: impl Iterator<Item = &'a AudioSubtitle>,
    font_size: f32,
    text_color: Color32,
) {
    let font = FontId::proportional(font_size);
    let painter = ui.painter();
    let rect = ui.ctx().content_rect();
    let outline_color = Color32::BLACK;
    let thickness = 2.0;
    
    // Start from the bottom with some padding
    let mut current_y = rect.bottom() - 10.0;
    let available_width = rect.width() * 0.8; // Use 80% of width
    let start_x = rect.left() + 10.0;

    for line in lines {
        let mut text = String::new();
        if let Some(speaker) = &line.speaker {
            text.push_str(&format!("{} >> ", speaker));
        }
        text.push_str(&line.displayed_text);

        if text.trim().is_empty() {
            continue;
        }

        // Create main text galley with wrapping
        let galley = painter.layout(
            text.clone(),
            font.clone(),
            text_color,
            available_width,
        );

        // Create shadow text galley with wrapping
        let shadow_galley = painter.layout(
            text,
            font.clone(),
            outline_color,
            available_width,
        );

        // Calculate position - convert bottom-up coordinate to top-left for the galley
        // Egali galleys are drawn from top-left.
        // We want the bottom of the galley to be at current_y.
        let galley_height = galley.size().y;
        let pos = pos2(start_x, current_y - galley_height);

        // Draw shadow
        let offsets = [
            vec2(-thickness, 0.0),
            vec2(thickness, 0.0),
            vec2(0.0, -thickness),
            vec2(0.0, thickness),
            vec2(-thickness, -thickness),
            vec2(-thickness, thickness),
            vec2(thickness, -thickness),
            vec2(thickness, thickness),
        ];

        for offset in offsets {
            painter.galley(pos + offset, shadow_galley.clone(), outline_color);
        }

        // Draw main text
        painter.galley(pos, galley, text_color);

        // Move up for the next line, adding some spacing
        current_y -= galley_height + (font_size * 0.2);
        
        // Stop if we've gone above the screen
        if current_y < rect.top() {
            break;
        }
    }
}
