use crate::types::offset::{OFFSET_WIDTH, WINDOW_HEIGHT};

pub fn get_inner_size(
    screen_width: f32,
    width_override: Option<f32>,
    height_override: Option<f32>,
) -> (f32, f32) {
    // If no width overridden, user full width minus margins (OFFSET_WIDTH * 2)
    // We assume default centering or similar margin logic.
    let width = width_override.unwrap_or(screen_width - OFFSET_WIDTH * 2.);
    let height = height_override.unwrap_or(WINDOW_HEIGHT);
    (width, height)
}
