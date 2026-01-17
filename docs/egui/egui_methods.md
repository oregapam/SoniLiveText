# Egui Documentation Reference

## 1. Getting a `Ui` Context
To add widgets, you first need a `Ui` region. This is created by containers like Panels or Windows.

```rust
// Central Panel (fills the window)
egui::CentralPanel::default().show(ctx, |ui| {
    ui.label("Hello from Central Panel");
});

// Side Panel (docked to a side)
egui::SidePanel::left("my_sidebar").show(ctx, |ui| {
    ui.label("Sidebar Content");
});

// Window (floating)
egui::Window::new("My Window").show(ctx, |ui| {
    ui.label("Floating window content");
});
```

## 2. Common Widgets
Widgets are added to the `Ui`. Many return a `Response` that can be checked for interaction (clicked, changed, hovered).

### Labels & Text
```rust
ui.label("Simple text");
ui.heading("Heading text");
ui.hyperlink("https://github.com/emilk/egui");
ui.colored_label(egui::Color32::RED, "Error Text");
```

### Buttons & Interaction
```rust
if ui.button("Click Me").clicked() {
    println!("Button clicked!");
}

if ui.link("Clickable Link").clicked() {
    // Handle link click
}
```

### Input Fields
```rust
let mut text = String::from("Editable");
ui.text_edit_singleline(&mut text);
ui.text_edit_multiline(&mut text);
ui.add(egui::TextEdit::singleline(&mut text).password(true)); // usage with builder
```

### Values & Sliders
```rust
let mut value = 0.0;
ui.add(egui::Slider::new(&mut value, 0.0..=100.0).text("Percentage"));
ui.add(egui::DragValue::new(&mut value).speed(0.1));
```

### Toggles & Choices
```rust
let mut boolean = true;
ui.checkbox(&mut boolean, "Enable Feature");

#[derive(PartialEq)]
enum Enum { A, B, C }
let mut selected = Enum::A;

ui.radio_value(&mut selected, Enum::A, "Option A");
ui.radio_value(&mut selected, Enum::B, "Option B");

// Vertical layout for radios
ui.vertical(|ui| {
    ui.radio_value(&mut selected, Enum::A, "A");
    ui.radio_value(&mut selected, Enum::B, "B");
});
```

### Images
```rust
// ui.image((texture_id, size));
ui.image((my_texture, egui::Vec2::new(100.0, 100.0)));
```

## 3. Layouts & Containers

### Grid
Useful for forms (Label + Input pairs).
```rust
egui::Grid::new("my_grid")
    .num_columns(2)
    .spacing([40.0, 4.0])
    .striped(true)
    .show(ui, |ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut name);
        ui.end_row();

        ui.label("Age:");
        ui.add(egui::DragValue::new(&mut age));
        ui.end_row();
    });
```

### ScrollArea
```rust
egui::ScrollArea::vertical()
    .auto_shrink([false, false]) // Don't shrink to fit content
    .show(ui, |ui| {
        for i in 0..100 {
            ui.label(format!("Item {}", i));
        }
    });
```

### Collapsing Headers
```rust
ui.collapsing("Click to expand", |ui| {
    ui.label("Hidden content");
});
```

### Manual Layout Direction
```rust
ui.horizontal(|ui| {
    ui.label("Same");
    ui.button("Row");
});

ui.vertical(|ui| {
    ui.label("Stacked");
    ui.label("Vertically");
});
```

## 4. Helper Methods
```rust
ui.separator(); // Horizontal line
ui.add_space(20.0); // Detailed spacing
```

## 5. Viewports (Multi-window)
```rust
ctx.show_viewport_immediate(
    egui::ViewportId::from_hash_of("id"),
    egui::ViewportBuilder::default().with_title("New Window"),
    |ctx, class| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("New Window Content");
        });
    }
);
```

## 6. Advanced Topics

### Custom Painting
You can draw custom shapes using the `Painter`.
```rust
let painter = ui.painter();
painter.rect_filled(
    ui.max_rect(),
    5.0, // rounding
    egui::Color32::from_rgb(200, 100, 100),
);
painter.line_segment(
    [egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0)],
    egui::Stroke::new(2.0, egui::Color32::YELLOW),
);
```

### Input Handling
Check keys and mouse state.
```rust
// Check keys
if ui.input(|i| i.key_pressed(egui::Key::A)) {
    println!("A pressed");
}

// Check modifiers
if ui.input(|i| i.modifiers.ctrl) {
    // Ctrl is held
}

// Mouse position
if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
    println!("Mouse at {:?}", pos);
}
```

### Styling
Customize the look of widgets.
```rust
let mut visuals = egui::Visuals::dark();
visuals.widgets.noninteractive.bg_fill = egui::Color32::from_black_alpha(200);
ctx.set_visuals(visuals);

// Override per-widget style
ui.style_mut().spacing.item_spacing = egui::vec2(10.0, 5.0);
```

### Animations
```rust
let how_on = ui.ctx().animate_bool("my_id", true);
// use how_on (0.0 to 1.0) to lerp colors or sizes
```

### Drag and Drop
```rust
// Drag source
ui.horizontal(|ui| {
    ui.label("Drag me:");
    let item_id = egui::Id::new("my_drag_item");
    if ui.add(egui::Button::new("📦").sense(egui::Sense::drag())).dragged() {
        ui.ctx().translate_cursor(egui::vec2(0.0, -10.0)); // Visual effect
        // Set payload
        // (Use dnd_drag_payload in newer versions or manage state manually)
    }
});

// Drop target
let response = ui.dnd_drop_zone::<String>(egui::Frame::default(), |ui| {
    ui.label("Drop here");
});
if let Some(payload) = response.payload {
    println!("Dropped: {:?}", payload);
}
```

## 7. Memory & ID Management

### ID Clashes
If you create widgets in a loop with the same labels, you might get ID clashes. Use `push_id` to create a unique scope.
```rust
for i in 0..10 {
    ui.push_id(i, |ui| {
        ui.collapsing("Details", |ui| {
            ui.label("Content");
        });
    });
}
```

### Persistence
You can store state in `ui.data()` or `ui.memory()`.
```rust
// Generate a separate ID for storage
let id = ui.make_persistent_id("my_state");
let val: Option<bool> = ui.memory(|mem| mem.data.get_temp(id));
```

## 8. Optimization Tips
*   **Request Repaint:** Only repaint when needed. `ctx.request_repaint()` works, but `request_repaint_after(Duration)` is better for animations.
*   **Texture Memory:** In `eframe::Options`, you can set `reduce_texture_memory = true` to free RAM after uploading to GPU.
*   **Debug Clashes:** Run in debug mode to see visual warnings when ID clashes occur (enabled by default via `warn_on_id_clash`).

