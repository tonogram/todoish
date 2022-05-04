#![windows_subsystem = "windows"]

use dirs::home_dir;
use eframe::{egui, epaint};
use serde::{Deserialize, Serialize};
use std::{fs, thread, time};

#[derive(Serialize, Deserialize, Clone)]
/// An indivudual item on the todo list.
struct Item {
    /// The name of this item.
    name: String,
    /// Whether or not this item is complete.
    is_done: bool,
    /// Whether or not this item is important. (Drawn with a brighter color.)
    is_important: bool,
    #[serde(skip)]
    /// Whether or not we should begin editing this item on this frame.
    begin_editing: bool,
    #[serde(skip)]
    /// Whether or not the name of this item is currently being edited.
    editing: bool,
}

impl Item {
    /// Create a new item from a given name.
    fn new(name: String) -> Self {
        Self {
            name,
            is_done: false,
            is_important: false,
            begin_editing: false,
            editing: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
/// A named list of todo items.
struct List {
    /// The name of the list.
    name: String,
    /// The items within this list.
    items: Vec<Item>,
    #[serde(skip)]
    /// The contents of the text box used to create a new item. This is not serialized.
    new_item_name: String,
    #[serde(skip)]
    /// Whether or not we should begin editing this item on this frame.
    begin_editing: bool,
    #[serde(skip)]
    /// Whether or not the name of this item is currently being edited.
    editing: bool,
    #[serde(skip)]
    /// Whether or not this list should be toggled open/close on this frame.
    should_toggle: bool,
}

impl List {
    /// Create a new todo list from a given name.
    fn new(name: String) -> Self {
        Self {
            name,
            items: Vec::new(),
            new_item_name: String::new(),
            begin_editing: false,
            editing: false,
            should_toggle: false,
        }
    }
}

// The state of the app.
struct Todoish {
    /// The contents of the text box used to create a new list.
    new_list_name: String,
    /// All of the todo lists.
    lists: Vec<List>,
    /// Whether or not any lists or items have been changed.
    changed: bool,
    /// The last time the todo list was saved.
    last_save: time::Instant,
}

impl Todoish {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Use the system setting to determine the theme. Default to dark when
        // the theme can't be detected.
        match cc.integration_info.prefer_dark_mode {
            Some(true) | None => cc.egui_ctx.set_visuals(egui::Visuals::dark()),
            Some(false) => cc.egui_ctx.set_visuals(egui::Visuals::light()),
        }

        // Attempt to open ~/.todoish and deserialize.
        Self {
            new_list_name: String::new(),
            lists: {
                let mut path = home_dir().expect("Failed to find home directory");
                path.push(".todoish");
                // Default to an empty Vec if the file doesn't exist.
                fs::read(path).map_or(Vec::new(), |bytes| {
                    // Panic if deserialization fails.
                    serde_json::from_slice(&bytes).expect("JSON was incorrectly formatted")
                })
            },
            changed: false,
            last_save: time::Instant::now(),
        }
    }
}

impl eframe::App for Todoish {
    /// Make the clear color transparent.
    fn clear_color(&self, _: &egui::Visuals) -> egui::Rgba {
        egui::Rgba::TRANSPARENT
    }

    /// Paint the frame!
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Round the corners of the window.
        let panel_frame = egui::containers::Frame::window(&ctx.style())
            .rounding(10.0)
            // Disable the shadow effect.
            .shadow(epaint::Shadow {
                extrusion: 0.0,
                color: egui::Color32::TRANSPARENT,
            });

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                {
                    // A fake window title to prevent the app from being closed accidentally.
                    let (rect, resp) = ui.allocate_at_least(
                        egui::vec2(ui.available_width(), 15.0),
                        egui::Sense::drag(),
                    );

                    // We still want to be able to drag the window around!
                    if resp.dragged() {
                        frame.drag_window();
                    }

                    let mut title_bar = ui.child_ui(rect, egui::Layout::left_to_right());
                    // Show "todoish" on the left of the header.
                    title_bar.label("todoish");
                    // Show whether or not the changes have been saved on the right of the header.
                    title_bar.with_layout(egui::Layout::right_to_left(), |ui| {
                        let text =
                            egui::RichText::new(if self.changed { "unsaved" } else { "saved" })
                                .weak();
                        ui.label(text);
                    });
                }

                ui.separator();

                {
                    // The text box for creating a new todo list.
                    let resp = egui::TextEdit::singleline(&mut self.new_list_name)
                        .hint_text("new list")
                        .desired_width(ui.available_width())
                        .show(ui)
                        .response;

                    if resp.lost_focus() {
                        self.new_list_name = self.new_list_name.trim().into();

                        if ui.input().key_pressed(egui::Key::Enter) {
                            self.lists.push(List::new(self.new_list_name.clone()));
                            self.new_list_name = String::new();
                            self.changed = true;
                        }
                    }
                }

                // Put some space between the text box and the lists. The effect
                // is most easily noticable when scrolled down.
                ui.allocate_space(egui::vec2(0.0, 3.0));

                let mut delete = None;
                egui::ScrollArea::vertical()
                    .stick_to_bottom()
                    .show(ui, |ui| {
                        // Loop over every list.
                        let len = self.lists.len();
                        for (idx, list) in self.lists.iter_mut().enumerate() {
                            // Use the current index as the header's ID.
                            // CAVEAT: This may cause weird behavior when deleting
                            // lists, but I feel like it's probably negligible.
                            let id = ui.make_persistent_id(idx);
                            // Create the header and default it to open.
                            let mut header =
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    id,
                                    true,
                                );
                            // Toggle the open state of the header if it was clicked
                            // outside of the arrow on the last frame.
                            if list.should_toggle {
                                header.set_open(!header.is_open());
                                list.should_toggle = false;
                            }
                            let (resp, inner, _) = header
                                // Draw the contents of this header.
                                .show_header(ui, |ui| {
                                    if list.editing {
                                        // If the user wants to edit the name
                                        // of this list, draw a text box instead
                                        // of a label.
                                        let resp = ui.text_edit_singleline(&mut list.name);
                                        // Steal focus immediately after the
                                        // double-click event.
                                        if list.begin_editing {
                                            resp.request_focus();
                                            list.begin_editing = false;
                                        }
                                        // Return to a label when we're
                                        // done editing the name.
                                        if resp.lost_focus() {
                                            self.changed = true;
                                            list.editing = false;
                                        }
                                    } else {
                                        // If we're not editing the name, just
                                        // draw a clickable label instead.
                                        let resp = ui.add(
                                            egui::Label::new(&list.name)
                                                .sense(egui::Sense::click()),
                                        );
                                        // Replace the label with a text box
                                        // when it's double clicked.
                                        if resp.double_clicked() {
                                            list.editing = true;
                                            list.begin_editing = true;
                                        }
                                        // Toggle the open state of the header
                                        // after the widget is clicked.
                                        if resp.clicked() {
                                            list.should_toggle = true;
                                        }
                                    }
                                })
                                .body(|ui| {
                                    let mut delete = None;
                                    // Loop over every item in this list.
                                    for (idx, item) in list.items.iter_mut().enumerate() {
                                        let resp = if item.editing {
                                            // If the user wants to edit the name
                                            // of this item, draw a text box instead
                                            // of a checkbox.
                                            let resp = ui.text_edit_singleline(&mut item.name);
                                            // Steal focus immediately after the
                                            // double-click event.
                                            if item.begin_editing {
                                                resp.request_focus();
                                                item.begin_editing = false;
                                            }
                                            // Return to a checkbox when we're
                                            // done editing the name.
                                            if resp.lost_focus() {
                                                self.changed = true;
                                                item.editing = false;
                                            }
                                            resp
                                        } else {
                                            // If we're not editing the name, just
                                            // draw a normal checkbox instead.
                                            let mut text = egui::RichText::new(&item.name);
                                            // Draw the text distinctly if this item is marked as important.
                                            if item.is_important {
                                                text = text.underline();
                                            }
                                            // Draw the checkbox for this item.
                                            let resp = ui.checkbox(&mut item.is_done, text);
                                            if resp.changed() {
                                                self.changed = true;
                                            }
                                            // Replace the checkbox with a text box
                                            // when it's double clicked.
                                            if resp.double_clicked() {
                                                item.editing = true;
                                                item.begin_editing = true;
                                            }
                                            resp
                                        };
                                        // Draw a context menu if this item is right-clicked.
                                        resp.context_menu(|ui| {
                                            // A check box for marking the item as important.
                                            if ui
                                                .checkbox(
                                                    &mut item.is_important,
                                                    "Mark as important",
                                                )
                                                .changed()
                                            {
                                                self.changed = true;
                                                ui.close_menu();
                                            }
                                            // A buttom for deleting the item.
                                            if ui.button("Delete item").clicked() {
                                                delete = Some(idx);
                                                self.changed = true;
                                                ui.close_menu();
                                            };
                                        });
                                    }
                                    // If an item was marked for deletion, remove it.
                                    // We don't use swap_remove() here since the order
                                    // of items might matter to the user.
                                    if let Some(idx) = delete {
                                        list.items.remove(idx);
                                    }
                                    {
                                        // A text box for adding a new item to this list.
                                        let resp =
                                            egui::TextEdit::singleline(&mut list.new_item_name)
                                                .hint_text("new item")
                                                .desired_width(ui.available_width())
                                                .show(ui)
                                                .response;

                                        if resp.lost_focus() {
                                            list.new_item_name = list.new_item_name.trim().into();

                                            if ui.input().key_pressed(egui::Key::Enter) {
                                                list.items
                                                    .push(Item::new(list.new_item_name.clone()));
                                                list.new_item_name = String::new();
                                                self.changed = true;
                                            }
                                        }
                                    }
                                });
                            // Draw a context menu if this list header is right-clicked.
                            // FIXME The context menu only responds to right-clicks
                            // on the arrow, not the widget.
                            resp.union(inner.response).context_menu(|ui| {
                                // A button for deleting this list.
                                if ui.button("Delete list").clicked() {
                                    delete = Some(idx);
                                    self.changed = true;
                                    ui.close_menu();
                                };
                            });
                            // Place some space between each list for readability.
                            if idx < len - 1 {
                                ui.allocate_space(egui::vec2(0.0, 5.0));
                            }
                        }
                    });
                // If a list was marked for deletion, remove it.
                // We can use swap_remove() here to save a couple CPU cycles,
                // as the order of entire lists doesn't really matter(?)
                if let Some(k) = delete {
                    self.lists.swap_remove(k);
                }
            });
        if self.changed {
            // Draw new frames as long as there are unsaved changes so that there's
            // no risk of leaving them unsaved.
            ctx.request_repaint();
            let elapsed = self.last_save.elapsed().as_secs();
            // Only save if at least 3 seconds have passed since the last save.
            if elapsed >= 3 {
                let lists_copy = self.lists.clone();
                // Save in another thread to keep the UI going.
                thread::spawn(move || {
                    let json = serde_json::to_string(&lists_copy).expect("Failed to serialize");
                    let mut path = home_dir().expect("Failed to find home directory");
                    path.push(".todoish");
                    fs::write(path, json).expect("Failed to write to disk");
                });
                self.last_save = time::Instant::now();
                self.changed = false;
            }
        }
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        // Hide the window header. We don't want to allow the user to accidentally
        // close the window so that their todo lists can always be visible. (a la Tape)
        // The window can still be closed, typically through the system taskbar,
        // so generally it must always be done with explicit intent.
        decorated: false,
        // And of course, since the window isn't decorated, make it transparent
        // So that we're not just stuck with the sharp corners.
        transparent: true,
        initial_window_size: Some(egui::vec2(600.0, 600.0)),
        min_window_size: Some(egui::vec2(500.0, 500.0)),
        ..Default::default()
    };
    eframe::run_native(
        "todoish",
        native_options,
        Box::new(|cc| Box::new(Todoish::new(cc))),
    );
}
