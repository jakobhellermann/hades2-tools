use std::collections::hash_map::Entry;
use std::fs::OpenOptions;
use std::io::BufWriter;

use anyhow::Result;
use egui::ahash::HashMap;
use egui::{Grid, ScrollArea};
use hades2::saves::{LuaValue, Savefile};
use hades2::{Hades2Installation, SaveHandle};

mod luavalue;

const AUTOLOAD: bool = true;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    advanced_mode: bool,

    #[serde(skip)]
    save_dialog: Option<SaveDialog>,

    #[serde(skip)]
    state: State,
    #[serde(skip)]
    current_savefile: Option<Box<CurrentSavefile>>,
}

struct State {
    hades: Option<Hades2Installation>,
    error: Option<String>,

    saves: Vec<(SaveHandle, Savefile)>,
}

struct CurrentSavefile {
    handle: SaveHandle,

    filter: String,

    save: Savefile,
    lua_state: LuaValue<'static>,
    dirty: bool,
}

struct SaveDialog {
    backups: Option<Vec<(SaveHandle, u64)>>,
}

impl App {
    fn hades(&self) -> Option<&Hades2Installation> {
        self.state.hades.as_ref()
    }

    fn handle_error<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(val) => Some(val),
            Err(e) => {
                self.state.error = Some(e.to_string());
                None
            }
        }
    }

    fn reset_error(&mut self) {
        self.state.error = None;
    }

    fn load_savefiles(&mut self) -> Result<()> {
        let Some(hades) = self.hades() else {
            return Ok(());
        };

        let saves = hades.saves()?;
        self.state.saves = saves
            .into_iter()
            .map(|handle| {
                let save = handle.read_header_only()?;
                Ok((handle, save))
            })
            .collect::<Result<Vec<_>>>()?;

        if AUTOLOAD {
            if let [.., (handle, _)] = self.state.saves.as_slice() {
                let (save, lua_state) = handle.read().unwrap();
                self.current_savefile = Some(Box::new(CurrentSavefile {
                    filter: String::new(),
                    handle: handle.clone(),
                    save,
                    lua_state,
                    dirty: false,
                }));
            }
        }

        Ok(())
    }
}
impl CurrentSavefile {
    fn save(&self) -> Result<()> {
        let out = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create(false)
                .open(self.handle.path())?,
        );
        self.save.serialize(out, &self.lua_state)?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        let (hades, error) = match Hades2Installation::detect() {
            Ok(val) => (Some(val), None),
            Err(val) => (None, Some(format!("{val}"))),
        };
        let mut app = Self {
            advanced_mode: false,
            save_dialog: None,
            state: State {
                hades,
                error,
                saves: Vec::new(),
            },
            current_savefile: None,
        };

        let res = app.load_savefiles();
        app.handle_error(res);

        app
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::MOCHA);
        cc.egui_ctx.set_zoom_factor(1.3);

        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Self::default()
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.current_savefile = None;
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let heading = match self.current_savefile {
                Some(ref current) => format!("Savefile Editor - Slot {}", current.handle.slot()),
                None => "Savefile Editor".to_string(),
            };
            ui.horizontal(|ui| {
                if self.current_savefile.is_some() {
                    if ui.button("⏴").clicked() {
                        self.current_savefile = None;
                        self.reset_error();
                    }
                }
                ui.heading(heading);
            });
            ui.separator();

            if let Some(error) = &self.state.error {
                ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(255, 51, 51)));
            }

            let mut current_savefile = self.current_savefile.take();
            match current_savefile {
                None => self.ui_save_select(ui, ctx),
                Some(ref mut current) => {
                    self.ui_current(ui, ctx, current);
                    self.current_savefile = current_savefile;
                }
            }

            let mut cancel = false;
            let mut save_result = None;
            if let Some(save_dialog) = &self.save_dialog {
                let current = self.current_savefile.as_deref().unwrap();
                current.save.timestamp;

                egui::Window::new("Save")
                    .title_bar(false)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .fixed_size([300.0, 200.0])
                    .show(ctx, |ui| {
                        ui.heading("Save?");

                        match save_dialog.backups {
                            Some(ref backups) => {
                                ui.label("Backups");
                                ui.indent("backups", |ui| {
                                    for &(ref backup, timestamp) in backups {
                                        let time = format_time(timestamp);
                                        let diff = current.save.timestamp as i64 - timestamp as i64;

                                        ui.label(format!(
                                            "{} - {time} - {}",
                                            backup.backup_index().unwrap(),
                                            format_ago(diff)
                                        ));
                                    }
                                });

                                ui.add_enabled_ui(false, |ui| {
                                    let _ = ui.button("Make backup");
                                });
                            }
                            None => {
                                ui.label("No backups found!");
                            }
                        };

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                                if ui.button("Save (Overwrite current!)").clicked() {
                                    save_result = Some(current.save());
                                }
                                cancel = ui.button("Cancel").clicked();
                            });
                        });
                    });

                if cancel {
                    self.save_dialog = None;
                }
                if let Some(save_result) = save_result {
                    if save_result.is_ok() {
                        self.save_dialog = None;
                    }
                    self.handle_error(save_result);
                }
            }
        });
    }
}

impl App {
    fn ui_save_select(&mut self, ui: &mut egui::Ui, _: &egui::Context) {
        let mut load_slot = None;

        egui::Grid::new("saves").show(ui, |ui| {
            ui.label("Slot");
            ui.label("Runs");
            ui.label("Grasp");
            ui.label("Last Modified");
            ui.end_row();

            for (handle, save) in &self.state.saves {
                ui.label(handle.slot().to_string());
                ui.label((save.runs + 1).to_string());
                ui.label(save.grasp.to_string());

                ui.label(format_time(save.timestamp));

                if ui.button("Open").clicked() {
                    load_slot = Some(handle.clone());
                }

                ui.end_row();
            }
        });

        if let Some(handle) = load_slot.take() {
            let result = handle.read();
            if let Some((save, lua_state)) = self.handle_error(result) {
                self.current_savefile = Some(Box::new(CurrentSavefile {
                    filter: String::new(),
                    handle,
                    save,
                    lua_state,
                    dirty: false,
                }));
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.add(egui::Hyperlink::from_label_and_url(
                "Source code on GitHub",
                "https://github.com/jakobhellermann/hades2-tools/",
            ));
        });
    }

    fn ui_current(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        current: &mut CurrentSavefile,
    ) {
        let CurrentSavefile {
            handle: _,
            save,
            lua_state,
            dirty,
            filter,
        } = current;
        let was_dirty = *dirty;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.checkbox(&mut self.advanced_mode, "Advanced Mode");
        });

        fn numeric<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) -> bool {
            ui.label(label);
            let changed = ui.add(egui::DragValue::new(val)).changed();
            ui.end_row();
            changed
        }
        fn checkbox(ui: &mut egui::Ui, label: &str, val: &mut bool) -> bool {
            ui.label(label);
            let changed = ui.checkbox(val, "").changed();
            ui.end_row();
            changed
        }

        if self.advanced_mode {
            ui.text_edit_singleline(filter);

            let nodes_visible = record_filter(lua_state, &filter.to_lowercase());
            dbg!(&nodes_visible);

            ScrollArea::vertical().show(ui, |ui| {
                *dirty |= luavalue::show_value(ui, lua_state, (0, 0), Some(&nodes_visible));
                ui.allocate_space(ui.available_size());
            });
        } else {
            Grid::new("easy mode").show(ui, |ui| {
                let mut changed = false;

                let mut runs_human = save.runs + 1;
                changed |= numeric(ui, "Runs", &mut runs_human);
                save.runs = runs_human.saturating_sub(1);
                changed |= numeric(ui, "Grasp", &mut save.grasp);
                changed |= numeric(ui, "Meta Points", &mut save.accumulated_meta_points);
                changed |= numeric(ui, "Active Shrine Points", &mut save.active_shrine_points);
                changed |= checkbox(ui, "Easy Mode", &mut save.easy_mode);
                changed |= checkbox(ui, "Hard Mode", &mut save.hard_mode);

                *dirty |= changed;
            });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(*dirty || true, |ui| {
                    if ui.button("Save").clicked() {
                        self.reset_error();
                        let res = self.hades().map(|hades| {
                            hades.backups(current.handle.slot()).and_then(|backups| {
                                backups
                                    .into_iter()
                                    .map(|backup| {
                                        let timestamp = backup.read_header_only()?.timestamp;
                                        Ok((backup, timestamp))
                                    })
                                    .collect::<Result<Vec<_>>>()
                            })
                        });
                        if let Some(backups) = self.handle_error(res.transpose()) {
                            self.save_dialog = Some(SaveDialog { backups });
                        }
                    }
                });
            });
        });

        if was_dirty != *dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "{}{}",
                crate::TITLE,
                if *dirty { "*" } else { "" }
            )));
        }
    }
}

fn format_time(timestamp: u64) -> String {
    let time = time::OffsetDateTime::from_unix_timestamp(timestamp as i64).unwrap();
    format!("{} {}", time.date(), time.time())
}

fn format_ago(seconds: i64) -> String {
    if seconds < 60 {
        return format!("{} seconds ago", seconds);
    }

    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{} minutes ago", minutes);
    }

    let hours = minutes / 60;
    if hours < 60 {
        return format!("{} hours ago", hours);
    }

    let days = hours / 24;
    return format!("{} days ago", days);
}

fn matches_filter(key: &LuaValue, val: &LuaValue, filter_lowercase: &str) -> bool {
    key.primitive_to_str()
        .map_or(false, |s| s.to_lowercase().contains(filter_lowercase))
        || val
            .primitive_to_str()
            .map_or(false, |s| s.to_lowercase().contains(filter_lowercase))
}

fn record_filter<'l>(root: &LuaValue, filter_lowercase: &str) -> HashMap<(usize, usize), bool> {
    // INVARIANT: if X in nodes_visible then ancestors(X) in nodes_visible
    let mut nodes_visible = HashMap::default();

    let mut ancestor_scratch = Vec::new();
    visit_with_ancestors(
        root,
        &mut ancestor_scratch,
        &mut |key, val, ancestors, pos| {
            if matches_filter(key, val, filter_lowercase) {
                nodes_visible.insert(pos, false);

                for &ancestor in ancestors.iter().rev() {
                    let was_occupied = match nodes_visible.entry(ancestor) {
                        Entry::Occupied(_) => true,
                        Entry::Vacant(vacant) => {
                            vacant.insert(false);
                            false
                        }
                    };
                    if was_occupied {
                        // break;
                    }
                }
            }
        },
        (0, 0),
    );

    nodes_visible
}

pub fn visit_with_ancestors<'l>(
    val: &'l LuaValue<'l>,
    ancestors: &mut Vec<(usize, usize)>,
    f_key: &mut impl FnMut(&'l LuaValue<'l>, &'l LuaValue<'l>, &[(usize, usize)], (usize, usize)),
    pos: (usize, usize),
) {
    match val {
        LuaValue::Nil => {}
        LuaValue::Bool(_) => {}
        LuaValue::Number(_) => {}
        LuaValue::String(_) => {}
        LuaValue::Table(table) => {
            ancestors.push(pos);
            for (i, (key, val)) in table.iter().enumerate() {
                let new_pos = (pos.0 + 1, i);
                f_key(key, val, ancestors.as_slice(), new_pos);
                visit_with_ancestors(val, ancestors, f_key, new_pos);
            }
            ancestors.pop();
        }
    }
}
