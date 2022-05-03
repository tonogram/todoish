#![windows_subsystem = "windows"]

use dirs::home_dir;
use eframe::{egui, epaint, epi};
use serde::{Deserialize, Serialize};
use std::{fs, thread, time};

/// Disable the shadow effect that is drawn by default.
fn no_shadow() -> epaint::Shadow {
    epaint::Shadow {
        extrusion: 0.0,
        color: egui::Color32::TRANSPARENT,
    }
}

#[derive(Serialize, Deserialize, Clone)]
/// An indivudual item on the todo list.
struct Item {
    /// The name of this item.
    name: String,
    /// Whether or not this item is complete.
    is_done: bool,
    /// Whether or not this item is important. (Drawn with a brighter color.)
    is_important: bool,
}

impl Item {
    /// Create a new item from a given name.
    fn new(name: String) -> Self {
        Self {
            name,
            is_done: false,
            is_important: false,
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
}

impl List {
    /// Create a new todo list from a given name.
    fn new(name: String) -> Self {
        Self {
            name,
            items: Vec::new(),
            new_item_name: String::new(),
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

impl Default for Todoish {
    fn default() -> Self {
        Self {
            new_list_name: String::new(),
            lists: {
                let mut path = home_dir().expect("Failed to find home directory");
                path.push(".todoish");
                fs::read(path).map_or(Vec::new(), |bytes| {
                    serde_json::from_slice(&bytes).expect("JSON was incorrectly formatted")
                })
            },
            changed: false,
            last_save: time::Instant::now(),
        }
    }
}

impl epi::App for Todoish {
    fn name(&self) -> &str {
        "todoish"
    }

    fn clear_color(&self) -> egui::Rgba {
        egui::Rgba::TRANSPARENT
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        // Round the corners of the window.
        let panel_frame = egui::containers::Frame::window(&ctx.style())
            .rounding(10.0)
            .shadow(no_shadow());

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
                    title_bar.label("todoish");
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
                            // Draw the header of this list.
                            let resp = egui::CollapsingHeader::new(&list.name)
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut delete = None;
                                    // Loop over every item in this list.
                                    for (idx, i) in list.items.iter_mut().enumerate() {
                                        let mut text = egui::RichText::new(&i.name);
                                        // Draw the text distinctly if this item is marked as important.
                                        if i.is_important {
                                            text = text.underline();
                                        }
                                        // Draw the checkbox for this item.
                                        let resp = ui.checkbox(&mut i.is_done, text);
                                        if resp.changed() {
                                            self.changed = true;
                                        }
                                        // Draw a context menu if this item is right-clicked.
                                        resp.context_menu(|ui| {
                                            // A text box for retroactively editing the item name.
                                            if ui.text_edit_singleline(&mut i.name).lost_focus() {
                                                self.changed = true;
                                                ui.close_menu();
                                            }
                                            // A check box for marking the item as important.
                                            if ui
                                                .checkbox(&mut i.is_important, "Mark as important")
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
                                })
                                .header_response;
                            // Draw a context menu if this list header is right-clicked.
                            resp.context_menu(|ui| {
                                // A text box for retroactively editing the list name.
                                // TODO Context menus containing text boxes have
                                // broken behavior when sourced from a
                                // CollapsingHeader. Why? I have no idea.
                                // if ui.text_edit_singleline(&mut list.name).lost_focus() {
                                //     self.changed = true;
                                //     ui.close_menu();
                                // }
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
    let app = Todoish::default();
    let native_options = eframe::NativeOptions {
        // Hide the window header. We don't want to allow the user to accidentally
        // close the window so that their todo lists can always be visible. (a la Tape)
        // The window can still be closed, typically through the system taskbar,
        // so generally it must always be done with explicit intent.
        decorated: false,
        // And of course, since the window isn't decorated, make it transparent
        // So that we're not just stuck with the sharp corners.
        transparent: true,
        min_window_size: Some(egui::vec2(500.0, 500.0)),
        ..Default::default()
    };
    eframe::run_native(Box::new(app), native_options);
}
