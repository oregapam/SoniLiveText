use crate::types::audio::AudioSubtitle;
use eframe::egui::{Ui, pos2, vec2};
use eframe::epaint::{Color32, FontId};

pub(crate) fn draw_text_with_shadow<'a>(
    ui: &mut Ui,
    lines: impl Iterator<Item = &'a AudioSubtitle>,
    font_size: f32,
    text_color: Color32,
    _interim_visual_height: f32,
) -> f32 {
    let font = FontId::proportional(font_size);
    let painter = ui.painter();
    let rect = ui.ctx().content_rect();
    let outline_color = Color32::BLACK;
    let thickness = 2.0;
    
    // Start from the bottom with some padding
    // let mut current_y = rect.bottom() - 10.0; // This line is removed
    let available_width = rect.width() * 0.8; // Use 80% of width
    let start_x = rect.left() + 10.0;

    // let mut first_item_height = 0.0; // This line is removed

    // Chronological order provided by iterator: [oldest, ..., newest, interim]
    let render_blocks: Vec<&AudioSubtitle> = lines
        .filter(|b| !b.displayed_text.is_empty())
        .collect();

    if render_blocks.is_empty() {
        return 0.0;
    }

    // First pass: Layout blocks and calculate total height
    let mut total_height = 0.0;
    let mut layouts = Vec::with_capacity(render_blocks.len());

    for (index, line) in render_blocks.iter().enumerate() {
        let mut text = String::new();
        if let Some(speaker) = &line.speaker {
            text.push_str(&format!("{} >> ", speaker));
        }
        text.push_str(&line.displayed_text);

        let galley = painter.layout(
            text.clone(),
            font.clone(),
            text_color,
            available_width,
        );
        
        let shadow_galley = painter.layout(
            text,
            font.clone(),
            outline_color,
            available_width,
        );
        
        // Double line break after sentences
        let ends_sentence = line.text.trim_end().ends_with(|c| c == '.' || c == '?' || c == '!');
        let height = galley.size().y;
        let mut block_spacing = 0.0;
        
        // Add spacing if it ends a sentence AND it's not the very last block (interim usually doesn't end with punctuation anyway)
        if ends_sentence && index < render_blocks.len() - 1 {
            block_spacing = font_size * 0.8;
        }

        total_height += height + block_spacing;
        layouts.push((galley, shadow_galley, height, block_spacing));
    }

    // Second pass: Render anchored at the bottom
    let mut current_y = rect.bottom() - 10.0 - total_height;
    
    let mut last_block_height = 0.0;

    for (galley, shadow_galley, height, spacing) in layouts {
        last_block_height = height;
        let pos = pos2(start_x, current_y);

        // Draw shadow
        let offsets = [
            vec2(-thickness, 0.0), vec2(thickness, 0.0),
            vec2(0.0, -thickness), vec2(0.0, thickness),
            vec2(-thickness, -thickness), vec2(-thickness, thickness),
            vec2(thickness, -thickness), vec2(thickness, thickness),
        ];

        for offset in offsets {
            painter.galley(pos + offset, shadow_galley.clone(), outline_color);
        }

        // Draw main text
        painter.galley(pos, galley, text_color);

        current_y += height + spacing;
    }
    
    last_block_height
}
