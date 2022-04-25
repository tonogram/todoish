#![windows_subsystem = "windows"]

use ahash::AHashMap;
use dirs::home_dir;
use eframe::{egui, epaint, epi};
use serde::{Deserialize, Serialize};
use std::{fs, thread, time};

fn no_shadow() -> epaint::Shadow {
    epaint::Shadow {
        extrusion: 0.0,
        color: egui::Color32::TRANSPARENT,
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Item {
    name: String,
    is_done: bool,
    is_important: bool,
}

struct Todoish {
    new_list_name: String,
    new_item_name: String,
    lists: AHashMap<String, Vec<Item>>,
    changed: bool,
    last_save: time::Instant,
}

impl Default for Todoish {
    fn default() -> Self {
        Self {
            lists: {
                let mut path = home_dir().expect("Failed to find home directory");
                path.push(".todoish");
                fs::read(path).map_or(AHashMap::new(), |bytes| {
                    serde_json::from_slice(&bytes).expect("JSON was incorrectly formatted")
                })
            },
            new_list_name: String::new(),
            new_item_name: String::new(),
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
        let panel_frame = egui::containers::Frame::window(&ctx.style())
            .rounding(10.0)
            .shadow(no_shadow());
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                {
                    let (rect, resp) = ui.allocate_at_least(
                        egui::vec2(ui.available_width(), 15.0),
                        egui::Sense::drag(),
                    );

                    if resp.dragged() {
                        frame.drag_window();
                    }

                    let mut title_bar = ui.child_ui(rect, egui::Layout::left_to_right());
                    title_bar.label("todoish");
                }

                ui.separator();

                {
                    let resp = egui::TextEdit::singleline(&mut self.new_list_name)
                        .hint_text("new list")
                        .desired_width(ui.available_width())
                        .show(ui)
                        .response;

                    if resp.lost_focus() {
                        self.new_list_name = self.new_list_name.trim().into();

                        if ui.input().key_pressed(egui::Key::Enter) {
                            self.lists.insert(self.new_list_name.clone(), vec![]);
                            self.new_list_name = String::new();
                            self.changed = true;
                        }
                    }
                }

                let mut delete = None;
                egui::ScrollArea::vertical()
                    .stick_to_bottom()
                    .show(ui, |ui| {
                        for (k, v) in self.lists.iter_mut() {
                            let resp = egui::CollapsingHeader::new(k)
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut delete = None;
                                    for (idx, i) in v.iter_mut().enumerate() {
                                        let mut text = egui::RichText::new(&i.name);
                                        if i.is_important {
                                            text = text.underline();
                                        }
                                        let resp = ui.checkbox(&mut i.is_done, text);
                                        if resp.changed() {
                                            self.changed = true;
                                        }
                                        resp.context_menu(|ui| {
                                            if ui
                                                .checkbox(&mut i.is_important, "Mark as important")
                                                .changed()
                                            {
                                                self.changed = true;
                                                ui.close_menu();
                                            }
                                            if ui
                                                .menu_button("Edit name", |ui| {
                                                    ui.text_edit_singleline(&mut i.name)
                                                })
                                                .inner
                                                .map_or(false, |x| x.lost_focus())
                                            {
                                                self.changed = true;
                                                ui.close_menu();
                                            }
                                            if ui.button("Delete item").clicked() {
                                                delete = Some(idx);
                                                self.changed = true;
                                                ui.close_menu();
                                            };
                                        });
                                    }
                                    if let Some(idx) = delete {
                                        v.remove(idx);
                                    }
                                    {
                                        let resp =
                                            egui::TextEdit::singleline(&mut self.new_item_name)
                                                .hint_text("new item")
                                                .desired_width(ui.available_width())
                                                .show(ui)
                                                .response;

                                        if resp.lost_focus() {
                                            self.new_item_name = self.new_item_name.trim().into();

                                            if ui.input().key_pressed(egui::Key::Enter) {
                                                v.push(Item {
                                                    name: self.new_item_name.clone(),
                                                    is_done: false,
                                                    is_important: false,
                                                });
                                                self.new_item_name = String::new();
                                                self.changed = true;
                                            }
                                        }
                                    }
                                })
                                .header_response;
                            resp.context_menu(|ui| {
                                if ui.button("Delete list").clicked() {
                                    delete = Some(k.clone());
                                    self.changed = true;
                                    ui.close_menu();
                                };
                            });
                        }
                    });
                if let Some(k) = delete {
                    self.lists.remove(&k);
                }
            });
        if self.changed {
            ctx.request_repaint();
            let elapsed = self.last_save.elapsed().as_secs();
            if elapsed >= 3 {
                let lists_copy = self.lists.clone();
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
        decorated: false,
        transparent: true,
        ..Default::default()
    };
    eframe::run_native(Box::new(app), native_options);
}
