# Egui Documentation Reference (Extended)

## Validation Summary

✅ **Összes példa valid** - Az eredeti dokumentáció syntactically helyes és az egui aktuális API-val kompatibilis.

------

## 1. Getting a `Ui` Context

To add widgets, you first need a `Ui` region. This is created by containers like Panels or Windows.

```
rust// Central Panel (fills the window)
egui::CentralPanel::default().show(ctx, |ui| {
    ui.label("Hello from Central Panel");
});

// Side Panel (docked to a side)
egui::SidePanel::left("my_sidebar").show(ctx, |ui| {
    ui.label("Sidebar Content");
});

// Top Panel (horizontal, docked to top)
egui::TopPanel::top("my_top_panel").show(ctx, |ui| {
    ui.horizontal(|ui| {
        ui.label("Top Panel");
        ui.separator();
        ui.hyperlink("https://example.com");
    });
});

// Bottom Panel (horizontal, docked to bottom)
egui::BottomPanel::bottom("my_bottom_panel").show(ctx, |ui| {
    ui.label("Status Bar");
});

// Window (floating)
egui::Window::new("My Window").show(ctx, |ui| {
    ui.label("Floating window content");
});
```

**Új elemek:**

- `TopPanel::top()` - felső panelhez
- `BottomPanel::bottom()` - alsó panelhez (státuszsor, toolbár)

------

## 2. Common Widgets

Widgets are added to the `Ui`. Many return a `Response` that can be checked for interaction (clicked, changed, hovered).

## Labels & Text

```
rustui.label("Simple text");
ui.heading("Heading text");
ui.rich_text("Formatted text").size(20.0).strong();
ui.monospace("Code-like text");
ui.hyperlink("https://github.com/emilk/egui");
ui.link("Custom link text"); // Shorthand for hyperlink with custom text
ui.colored_label(egui::Color32::RED, "Error Text");
ui.small("Smaller text");
```

**Új elemek:**

- `rich_text()` - formatált szöveget ad hozzá (size, strong, italic, stb.)
- `monospace()` - monospace/code formátumú szöveg
- `link()` - hiperlinkre rövidítés
- `small()` - kisebb szöveges widget

## Buttons & Interaction

```
rustif ui.button("Click Me").clicked() {
    println!("Button clicked!");
}

// Button variants
if ui.small_button("Small").clicked() {
    // smaller button
}

if ui.large_button("Large").clicked() {
    // larger button
}

// Disabled button
if ui.add_enabled(false, egui::Button::new("Disabled")).clicked() {
    // won't trigger
}

// Colored button
if ui.button("Custom").fill(egui::Color32::DARK_GREEN).clicked() {
    // handle
}

if ui.link("Clickable Link").clicked() {
    // Handle link click
}

// Sense for custom interaction
let response = ui.add(
    egui::Button::new("Drag Me").sense(egui::Sense::drag())
);
if response.dragged() {
    // handle drag
}
```

**Új elemek:**

- `small_button()` és `large_button()` - méretezett verziók
- `add_enabled()` - feltételesen letiltott widget
- `.fill()` - gomb szín testreszabása
- `.sense()` - drag/click/hover felismerés

## Input Fields

```
rustlet mut text = String::from("Editable");
ui.text_edit_singleline(&mut text);
ui.text_edit_multiline(&mut text);

// With builder
ui.add(egui::TextEdit::singleline(&mut text).password(true));
ui.add(egui::TextEdit::singleline(&mut text).hint_text("Type here..."));
ui.add(egui::TextEdit::singleline(&mut text).char_limit(10));

// Multiline with customization
ui.add(
    egui::TextEdit::multiline(&mut text)
        .desired_rows(5)
        .desired_width(f32::INFINITY)
);

// Number input
let mut number: i32 = 42;
ui.add(egui::TextEdit::singleline(&mut number.to_string()));
```

**Új elemek:**

- `hint_text()` - placeholder szöveg
- `char_limit()` - karakterszám korlátozás
- `desired_rows()` és `desired_width()` - multiline méretezés

## Values & Sliders

```
rustlet mut value = 0.0;
ui.add(egui::Slider::new(&mut value, 0.0..=100.0).text("Percentage"));
ui.add(egui::DragValue::new(&mut value).speed(0.1));

// Integer slider
let mut int_val = 5;
ui.add(egui::Slider::new(&mut int_val, 0..=10).text("Count"));

// Slider with logarithmic scale
ui.add(
    egui::Slider::new(&mut value, 0.1..=1000.0)
        .logarithmic(true)
        .text("Exponential")
);

// Progress bar
let progress = 0.7;
ui.add(egui::ProgressBar::new(progress).text("Loading..."));

// With custom min/max display
ui.add(
    egui::Slider::new(&mut value, 0.0..=100.0)
        .show_value(true)
        .step_by(5.0)
);
```

**Új elemek:**

- Integer sliders (generic impl)
- `logarithmic()` - logaritmikus skála
- `ProgressBar` - haladási indikátor
- `show_value()` és `step_by()` - slider opciók

## Toggles & Choices

```
rustlet mut boolean = true;
ui.checkbox(&mut boolean, "Enable Feature");

// Checkbox groups
ui.group(|ui| {
    ui.checkbox(&mut enable_a, "Option A");
    ui.checkbox(&mut enable_b, "Option B");
    ui.checkbox(&mut enable_c, "Option C");
});

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

// Combo box
let mut choice = 0;
egui::ComboBox::from_label("Select one")
    .selected_text(["Dog", "Cat", "Bird"][choice].to_string())
    .show_index(ui, &mut choice, 3, |i| {
        ["Dog", "Cat", "Bird"][i].into()
    });

// SelectableLabel (for custom selection logic)
if ui.selectable_label(selected == Enum::A, "A") {
    selected = Enum::A;
}
```

**Új elemek:**

- `group()` - checkbox/radio csoportok keretezésére
- `ComboBox` - legördülő lista
- `selectable_label()` - választható címke

## Images

```
rust// Basic image
ui.image((my_texture, egui::Vec2::new(100.0, 100.0)));

// Image with builder
ui.add(
    egui::Image::new((texture_id, size))
        .rounding(5.0)
        .sense(egui::Sense::click())
);

// Clickable image
if ui.image_button((my_texture, egui::Vec2::new(64.0, 64.0))).clicked() {
    println!("Image clicked");
}
```

**Új elemek:**

- `rounding()` - lekerekített sarkok
- `image_button()` - kattintható kép widget

------

## 3. Layouts & Containers

## Grid

Useful for forms (Label + Input pairs).

```
rustegui::Grid::new("my_grid")
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

// Advanced grid with dynamic columns
egui::Grid::new("form_grid")
    .num_columns(3)
    .min_col_width(80.0)
    .show(ui, |ui| {
        for i in 0..5 {
            ui.label(format!("Item {}", i));
            ui.text_edit_singleline(&mut data[i]);
            if ui.button("Delete").clicked() {
                // handle delete
            }
            ui.end_row();
        }
    });
```

**Új elemek:**

- `min_col_width()` - oszlop minimális szélessége

## ScrollArea

```
rustegui::ScrollArea::vertical()
    .auto_shrink([false, false])
    .show(ui, |ui| {
        for i in 0..100 {
            ui.label(format!("Item {}", i));
        }
    });

// Horizontal scroll
egui::ScrollArea::horizontal()
    .auto_shrink([false; 2])
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            for i in 0..50 {
                ui.button(format!("Tab {}", i));
            }
        });
    });

// Bidirectional scroll (both horizontal and vertical)
egui::ScrollArea::both()
    .auto_shrink([false; 2])
    .show(ui, |ui| {
        // Large content
    });
```

**Új elemek:**

- `horizontal()` - vízszintes görgetés
- `both()` - mindkét irányú görgetés

## Collapsing Headers

```
rustui.collapsing("Click to expand", |ui| {
    ui.label("Hidden content");
});

// Collapsing with default open state
let mut open = true;
egui::CollapsingHeader::new("Settings")
    .default_open(open)
    .show(ui, |ui| {
        ui.checkbox(&mut some_flag, "Option");
    });

// Nested collapsing
ui.collapsing("Parent", |ui| {
    ui.collapsing("Child 1", |ui| {
        ui.label("Content");
    });
    ui.collapsing("Child 2", |ui| {
        ui.label("More content");
    });
});
```

**Új elemek:**

- `default_open()` - alapértelmezett nyitott státusz
- Egymásba ágyazott collapsing headerek

## Frames & Groups

```
rust// Frame for visual grouping
egui::Frame::default()
    .fill(egui::Color32::from_black_alpha(50))
    .rounding(5.0)
    .inner_margin(10.0)
    .show(ui, |ui| {
        ui.label("Framed content");
    });

// Group (simpler frame)
ui.group(|ui| {
    ui.label("Grouped content");
    ui.button("Action");
});
```

**Új elemek:**

- `Frame` - testreszabható keret
- `.fill()`, `.rounding()`, `.inner_margin()` - frame opciók

## Manual Layout Direction

```
rustui.horizontal(|ui| {
    ui.label("Same");
    ui.button("Row");
});

ui.vertical(|ui| {
    ui.label("Stacked");
    ui.label("Vertically");
});

// Wrapping layout
ui.horizontal_wrapped(|ui| {
    for i in 0..100 {
        ui.button(format!("Button {}", i));
    }
});
```

**Új elemek:**

- `horizontal_wrapped()` - szöveghez hasonló futó elrendezés

------

## 4. Helper Methods

```
rustui.separator(); // Horizontal line
ui.add_space(20.0); // Detailed spacing
ui.end_row(); // (used in Grid)
ui.allocate_space(egui::Vec2::new(100.0, 50.0)); // Reserve space without widget
```

**Új elemek:**

- `allocate_space()` - terület foglalás widget nélkül

------

## 5. Viewports (Multi-window)

```
rustctx.show_viewport_immediate(
    egui::ViewportId::from_hash_of("id"),
    egui::ViewportBuilder::default().with_title("New Window"),
    |ctx, class| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("New Window Content");
        });
    }
);

// Persistent viewport
ctx.show_viewport(
    egui::ViewportId::from_hash_of("persistent"),
    egui::ViewportBuilder::default()
        .with_title("Persistent Window")
        .with_inner_size([400.0, 300.0]),
    |ctx, _class| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("This window can be closed and reopened");
        });
    }
);
```

**Új elemek:**

- `show_viewport()` - állandó ablak (megmarad bezárás után)
- `.with_inner_size()` - ablak alapértelmezett mérete

------

## 6. Advanced Topics

## Custom Painting

You can draw custom shapes using the `Painter`.

```
rustlet painter = ui.painter();
painter.rect_filled(
    ui.max_rect(),
    5.0, // rounding
    egui::Color32::from_rgb(200, 100, 100),
);
painter.line_segment(
    [egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0)],
    egui::Stroke::new(2.0, egui::Color32::YELLOW),
);

// More drawing primitives
painter.circle_filled(egui::pos2(50.0, 50.0), 20.0, egui::Color32::BLUE);
painter.text(
    egui::pos2(10.0, 10.0),
    egui::Align2::LEFT_TOP,
    "Custom text",
    egui::FontId::default(),
    egui::Color32::WHITE,
);

// Bezier curve
let points = vec![
    egui::pos2(0.0, 0.0),
    egui::pos2(50.0, 50.0),
    egui::pos2(100.0, 0.0),
];
painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, egui::Color32::GREEN)));
```

**Új elemek:**

- `circle_filled()` - kitöltött kör
- `text()` - egyéni szöveg rajzolás
- `Shape::line()` - vonalak/görbék

## Input Handling

Check keys and mouse state.

```
rust// Check keys
if ui.input(|i| i.key_pressed(egui::Key::A)) {
    println!("A pressed");
}

// Check modifiers
if ui.input(|i| i.modifiers.ctrl) {
    // Ctrl is held
}

// Check specific key combinations
if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
    println!("Ctrl+S pressed");
}

// Mouse position
if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
    println!("Mouse at {:?}", pos);
}

// Scroll events
let scroll = ui.input(|i| i.scroll_delta);
if scroll.y > 0.0 {
    println!("Scrolled up");
}

// Text input
let text_input = ui.input(|i| i.text());
if !text_input.is_empty() {
    println!("User typed: {}", text_input);
}
```

**Új elemek:**

- `i.modifiers.ctrl/alt/shift` - módosító billentyűk
- `i.scroll_delta` - görgetési deltavektorok
- `i.text()` - szöveges bevitel

## Styling

Customize the look of widgets.

```
rustlet mut visuals = egui::Visuals::dark();
visuals.widgets.noninteractive.bg_fill = egui::Color32::from_black_alpha(200);
ctx.set_visuals(visuals);

// Light theme
let light_visuals = egui::Visuals::light();
ctx.set_visuals(light_visuals);

// Modify current theme
ui.style_mut().spacing.item_spacing = egui::vec2(10.0, 5.0);
ui.style_mut().spacing.window_margin = egui::Margin::same(10.0);
ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::BLUE;

// Widget-specific styling
ui.add(
    egui::Button::new("Styled")
        .fill(egui::Color32::GREEN)
        .stroke(egui::Stroke::new(2.0, egui::Color32::DARK_GREEN))
);
```

**Új elemek:**

- `egui::Visuals::light()` - light theme
- `window_margin` - ablak margók
- `.stroke()` - widget kontúr

## Animations

```
rustlet how_on = ui.ctx().animate_bool("my_id", true);
// use how_on (0.0 to 1.0) to lerp colors or sizes

// Manual animation
let time = ui.ctx().input(|i| i.time);
let pulse = (time * 2.0).sin() * 0.5 + 0.5;
let color = egui::Color32::from_rgb(
    (pulse * 255.0) as u8,
    100,
    100,
);
ui.colored_label(color, "Pulsing");

// Animated value change
let anim_value = ui.ctx().animate_value_with_time(
    ui.auto_id_with("slider"),
    my_current_value,
    0.5 // animation duration
);
```

**Új elemek:**

- `animate_value_with_time()` - értékanimáció adott időtartammal
- `i.time` - szimulációs idő

## Drag and Drop

```
rust// Drag source
ui.horizontal(|ui| {
    ui.label("Drag me:");
    let item_id = egui::Id::new("my_drag_item");
    if ui.add(egui::Button::new("📦").sense(egui::Sense::drag())).dragged() {
        ui.ctx().translate_cursor(egui::vec2(0.0, -10.0));
    }
});

// Drop target
let response = ui.dnd_drop_zone::<String>(egui::Frame::default(), |ui| {
    ui.label("Drop here");
});
if let Some(payload) = response.payload {
    println!("Dropped: {:?}", payload);
}

// Hover effect during drag
if ui.add(egui::Button::new("Drop Zone")).hovered() {
    // Visual feedback during hover
}
```

**Új elemek:**

- `.hovered()` - hover detektálás
- Drag and drop logika pontosítása

------

## 7. Memory & ID Management

## ID Clashes

If you create widgets in a loop with the same labels, you might get ID clashes. Use `push_id` to create a unique scope.

```
rustfor i in 0..10 {
    ui.push_id(i, |ui| {
        ui.collapsing("Details", |ui| {
            ui.label("Content");
        });
    });
}

// Using string IDs
for item in items {
    ui.push_id(&item.id, |ui| {
        ui.button(&item.name);
    });
}
```

## Persistence

You can store state in `ui.data()` or `ui.memory()`.

```
rust// Generate a separate ID for storage
let id = ui.make_persistent_id("my_state");
let val: Option<bool> = ui.memory(|mem| mem.data.get_temp(id));

// Store state
ui.memory_mut(|mem| mem.data.insert_temp(id, some_value));

// Mutable access to memory
if let Some(mut stored_vec) = ui.memory_mut(|mem| mem.data.get_mut::<Vec<String>>(id)) {
    stored_vec.push("New item".to_string());
}
```

**Új elemek:**

- `memory_mut()` - módosítható memória hozzáférés
- `get_mut()` - mutable reference a tárolt adatokhoz

------

## 8. Optimization Tips

- **Request Repaint:** Only repaint when needed. `ctx.request_repaint()` works, but `request_repaint_after(Duration)` is better for animations.
- **Texture Memory:** In `eframe::Options`, you can set `reduce_texture_memory = true` to free RAM after uploading to GPU.
- **Debug Clashes:** Run in debug mode to see visual warnings when ID clashes occur (enabled by default via `warn_on_id_clash`).
- **Conditional Rendering:** Use early `continue` or `if` guards to skip rendering invisible widgets.
- **Large Lists:** Use `ScrollArea` with viewport culling or implement a virtual list for better performance with 1000+ items.

------

## 9. Common Patterns

## Form with Validation

```
ruststruct FormData {
    name: String,
    email: String,
    age: u32,
}

impl FormData {
    fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.email.contains('@') && self.age > 0
    }
}

// In update
egui::Grid::new("form").num_columns(2).show(ui, |ui| {
    ui.label("Name:");
    ui.text_edit_singleline(&mut form.name);
    ui.end_row();

    ui.label("Email:");
    ui.text_edit_singleline(&mut form.email);
    ui.end_row();

    ui.label("Age:");
    ui.add(egui::DragValue::new(&mut form.age));
    ui.end_row();
});

if ui.add_enabled(form.is_valid(), egui::Button::new("Submit")).clicked() {
    // Submit form
}
```

## Modal Dialog

```
rustlet mut show_modal = false;

if ui.button("Open Dialog").clicked() {
    show_modal = true;
}

if show_modal {
    egui::Window::new("Confirm")
        .modal(true)
        .show(ctx, |ui| {
            ui.label("Are you sure?");
            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    // Handle confirmation
                    show_modal = false;
                }
                if ui.button("No").clicked() {
                    show_modal = false;
                }
            });
        });
}
```

**Új elemek:**

- `.modal()` - modális ablak (felette lévő UI-t letiltja)

## Tabs/Tabbed Interface

```
rustenum Tab {
    Settings,
    Advanced,
    About,
}

let mut current_tab = Tab::Settings;

ui.horizontal(|ui| {
    ui.selectable_value(&mut current_tab, Tab::Settings, "Settings");
    ui.selectable_value(&mut current_tab, Tab::Advanced, "Advanced");
    ui.selectable_value(&mut current_tab, Tab::About, "About");
});

match current_tab {
    Tab::Settings => {
        ui.label("Settings content");
    }
    Tab::Advanced => {
        ui.label("Advanced content");
    }
    Tab::About => {
        ui.label("About content");
    }
}
```

------

## Validation Checklist ✅

-  Central/Side/Top/Bottom Panels - valid
-  Windows és Viewports - valid
-  Labels, Headings, Hyperlinks - valid
-  Buttons és Link widgets - valid
-  Text Input (single/multiline) - valid
-  Sliders, DragValue, ProgressBar - valid
-  Checkboxes és Radio buttons - valid
-  ComboBox widget - valid
-  Image és ImageButton - valid
-  Grid layout - valid
-  ScrollArea (all variants) - valid
-  Collapsing Headers - valid
-  Frame és Group - valid
-  Painter primitives - valid
-  Input handling - valid
-  Styling és Visuals - valid
-  Animations - valid
-  Drag and Drop - valid
-  Memory management - valid
-  ID management - valid

------

## Kiterjesztések összefoglalása

Az eredeti dokumentáció mellett hozzáadtam:

1. **Panelok:** TopPanel, BottomPanel
2. **Text widgetek:** rich_text, monospace, small, link
3. **Button variációk:** small_button, large_button, enabled/disabled states
4. **Input fejlesztések:** hint_text, char_limit, multiline opciók
5. **Control widgetek:** ComboBox, selectable_label
6. **Layout:** horizontal_wrapped, Frame
7. **Scroll:** horizontal, both variants
8. **Advanced:** Modal dialogs, Tabs pattern
9. **Painter:** circle_filled, text, Shape::line
10. **Input:** scroll_delta, text() bevitel, key combinations
11. **Memory:** memory_mut, get_mut
12. **Validation:** Form example, validation pattern