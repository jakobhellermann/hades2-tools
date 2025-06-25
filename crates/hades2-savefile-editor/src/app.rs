use std::collections::hash_map::Entry;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::time::Instant;

use anyhow::{Context, Result};
use egui::ahash::HashMap;
use egui::{Align, Grid, Layout, ScrollArea, TextEdit, UiBuilder};
use hades2::saves::{LuaValue, Savefile};
use hades2::{Hades2Installation, SaveHandle};

use self::luavalue::Pos;

mod luavalue;

const AUTOLOAD: bool = cfg!(debug_assertions) && true;

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

    saves: Vec<(SaveHandle, Result<Savefile>, bool)>,
}

struct FilterState {
    filter: String,
    filter_changed: bool,
    search_values: bool,

    cached_visible: HashMap<Pos, bool>,
}

struct CurrentSavefile {
    handle: SaveHandle,

    filter: FilterState,

    save: Savefile,
    lua_state: LuaValue<'static>,
    dirty: bool,
}

struct SaveDialog {
    backups: Option<Vec<(SaveHandle, Result<u64>)>>,
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

        let active_profile = hades
            .active_profile_path()
            .context("failed to read active profile")?;

        let saves = hades.saves()?;
        self.state.saves = saves
            .into_iter()
            .map(|handle| {
                let save = handle.read_header_only();
                let is_active = handle
                    .path()
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map_or(false, |stem| stem == active_profile);
                (handle, save, is_active)
            })
            .collect::<Vec<_>>();

        if AUTOLOAD {
            if let [.., (handle, _, _)] = self.state.saves.as_slice() {
                let (save, lua_state) = handle.read().unwrap();
                self.current_savefile = Some(Box::new(CurrentSavefile {
                    filter: FilterState {
                        filter_changed: true,
                        filter: String::new(),
                        search_values: false,
                        cached_visible: HashMap::default(),
                    },
                    handle: handle.clone(),
                    save,
                    lua_state,
                    dirty: false,
                }));
            }
        }

        Ok(())
    }

    fn save_dialog(&mut self, ctx: &egui::Context) {
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

                    match save_dialog.backups.as_deref() {
                        None | Some([]) => {
                            ui.label("No backups found!");
                        }
                        Some(backups) => {
                            ui.label("Backups");
                            ui.indent("backups", |ui| {
                                for (backup, timestamp) in backups {
                                    let bak = backup.backup_index().unwrap();
                                    match timestamp {
                                        Ok(timestamp) => {
                                            let time = format_time(*timestamp);
                                            let diff =
                                                current.save.timestamp as i64 - *timestamp as i64;

                                            ui.label(format!(
                                                "{} - {time} - {}",
                                                bak,
                                                format_ago(diff)
                                            ));
                                        }
                                        Err(e) => {
                                            ui.horizontal(|ui| {
                                                ui.label(bak.to_string());
                                                show_error(ui, e.to_string())
                                            });
                                        }
                                    }
                                }
                            });
                        }
                    };
                    ui.add_enabled_ui(false, |ui| {
                        let _ = ui.button("Make backup").on_disabled_hover_text(
                            "Not implemented yet. You have to make manual backups.",
                        );
                    });

                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
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
    }
}
impl CurrentSavefile {
    fn save(&self) -> Result<()> {
        let mut out = Vec::new();
        self.save.serialize(&mut out, &self.lua_state)?;
        Savefile::parse(&out).unwrap();
        dbg!();

        let mut file = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create(false)
                .open(self.handle.path())?,
        );
        file.write_all(&out)?;
        // self.save.serialize(out, &self.lua_state)?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        let (hades, error) = match Hades2Installation::detect() {
            Ok(val) => (Some(val), None),
            Err(val) => (
                None,
                Some(format!(
                    "Could not detect Hades II installation: {val}\nPlease open an issue on github."
                )),
            ),
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
                Some(ref current) => {
                    format!("Savefile Editor - Slot {}", current.handle.slot())
                }
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
                show_error(ui, error);
            }

            let mut current_savefile = self.current_savefile.take();
            match current_savefile {
                None => self.ui_save_select(ui, ctx),
                Some(ref mut current) => {
                    self.ui_current(ui, ctx, current);
                    self.current_savefile = current_savefile;
                }
            }

            self.save_dialog(ctx);
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
            ui.label("Active");
            ui.label("Version");
            ui.label("Last Modified");
            ui.end_row();

            for (handle, save, is_active) in &self.state.saves {
                match save {
                    Ok(save) => {
                        ui.label(handle.slot().to_string());
                        ui.label((save.runs + 1).to_string());
                        ui.label(save.grasp.to_string());
                        if *is_active {
                            ui.label("✔");
                        } else {
                            ui.vertical(|_| ());
                        }

                        ui.label(save.version.to_string());
                        ui.label(format_time(save.timestamp));

                        if ui.button("Open").clicked() {
                            load_slot = Some(handle.clone());
                        }
                    }
                    Err(err) => {
                        ui.label(handle.slot().to_string());
                        show_error(ui, err.to_string());
                    }
                };

                ui.end_row();
            }
        });

        if let Some(handle) = load_slot.take() {
            let result = handle.read();
            if let Some((save, lua_state)) = self.handle_error(result) {
                self.current_savefile = Some(Box::new(CurrentSavefile {
                    filter: FilterState {
                        filter_changed: true,
                        filter: String::new(),
                        search_values: false,
                        cached_visible: HashMap::default(),
                    },
                    handle,
                    save,
                    lua_state,
                    dirty: false,
                }));
            }
        }

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
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

        let rect_full = ui.available_rect_before_wrap();

        let mut changed = false;
        if self.advanced_mode {
            /*egui::CollapsingHeader::new("Header").show(ui, |ui| {
                let mut runs_human = save.runs + 1;
                changed |= numeric(ui, "Runs", &mut runs_human);
                save.runs = runs_human.saturating_sub(1);
                changed |= numeric(
                    ui,
                    "Accumulated Meta Points",
                    &mut save.accumulated_meta_points,
                );
                changed |= numeric(ui, "Active Shrine Points", &mut save.active_shrine_points);
                changed |= numeric(ui, "Grasp", &mut save.grasp);
                changed |= checkbox(ui, "Easy Mode", &mut save.easy_mode);
                changed |= checkbox(ui, "Hard Mode", &mut save.hard_mode);
            });
            ui.separator();*/

            if filter.filter_changed {
                filter.cached_visible = time("filter", || {
                    record_filter(
                        lua_state,
                        &filter.filter.to_lowercase(),
                        filter.search_values,
                    )
                });
                filter.filter_changed = false;
            }

            ScrollArea::vertical().show(ui, |ui| {
                *dirty |= luavalue::show_value(
                    ui,
                    lua_state,
                    Pos::default(),
                    Some(&filter.cached_visible),
                );
                ui.allocate_space(ui.available_size());
            });
        } else {
            Grid::new("easy mode").show(ui, |ui| {
                let mut runs_human = save.runs + 1;
                changed |= numeric(ui, "Runs", &mut runs_human);
                save.runs = runs_human.saturating_sub(1);
                changed |= numeric(ui, "Grasp", &mut save.grasp);

                changed |= valpath::<u32>(
                    ui,
                    "Ash",
                    "GameState.Resources.CardUpgradePoints",
                    lua_state,
                );
                changed |= valpath::<u32>(
                    ui,
                    "Charon Cards",
                    "GameState.Resources.CharonPoints",
                    lua_state,
                );

                /*changed |= numeric(ui, "Meta Points", &mut save.accumulated_meta_points);
                changed |= numeric(ui, "Active Shrine Points", &mut save.active_shrine_points);
                changed |= checkbox(ui, "Easy Mode", &mut save.easy_mode);
                changed |= checkbox(ui, "Hard Mode", &mut save.hard_mode);*/

                *dirty |= changed;
            });

            ui.add_space(8.0);
            ui.add_enabled_ui(false, |ui| {
                let _ = ui.button("Unlock all cards");
                let _ = ui.button("Max out relationships");
            });
        }

        ui.allocate_new_ui(
            UiBuilder::new().max_rect(rect_full.shrink2([8., 0.].into())),
            |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    ui.checkbox(&mut self.advanced_mode, "Advanced Mode");

                    if self.advanced_mode {
                        ui.horizontal(|ui| {
                            let res = TextEdit::singleline(&mut filter.filter)
                                .hint_text("^resources")
                                .desired_width(80.)
                                .show(ui)
                                .response;
                            filter.filter_changed |= res.changed();
                            if ui.input_mut(|input| {
                                input.consume_key(egui::Modifiers::CTRL, egui::Key::F)
                            }) {
                                res.request_focus();
                            }
                            ui.label("Filter");
                        });

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut filter.search_values, "");
                            ui.label("Search in values");
                        });
                    }
                });
            },
        );

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(*dirty || true, |ui| {
                    if ui.button("Save").clicked() {
                        self.reset_error();
                        let res = self.hades().map(|hades| {
                            hades.backups(current.handle.slot()).map(|backups| {
                                backups
                                    .into_iter()
                                    .map(|backup| {
                                        let timestamp =
                                            backup.read_header_only().map(|s| s.timestamp);
                                        (backup, timestamp)
                                    })
                                    .collect::<Vec<_>>()
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
    format!("{} {:02}:{:02}", time.date(), time.hour(), time.minute())
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

fn matches_filter(
    key: &LuaValue,
    val: &LuaValue,
    filter_lowercase: &str,
    search_values: bool,
) -> bool {
    let key = key.primitive_to_str().unwrap_or_default().to_lowercase();

    let (starts_with, filter_lowercase) = match filter_lowercase.strip_prefix('^') {
        Some(rest) => (true, rest),
        None => (false, filter_lowercase),
    };
    let (ends_with, filter_lowercase) = match filter_lowercase.strip_suffix('$') {
        Some(rest) => (true, rest),
        None => (false, filter_lowercase),
    };

    let matches = |search: &str| {
        search
            .find(filter_lowercase)
            .filter(|&s| {
                (!starts_with || s == 0) && (!ends_with || s + filter_lowercase.len() == key.len())
            })
            .is_some()
    };

    matches(&key)
        || (search_values && matches(&val.primitive_to_str().unwrap_or_default().to_lowercase()))
}

fn record_filter<'l>(
    root: &LuaValue,
    filter_lowercase: &str,
    search_values: bool,
) -> HashMap<Pos, bool> {
    // INVARIANT: if X in nodes_visible then ancestors(X) in nodes_visible
    let mut nodes_visible = HashMap::default();

    let mut ancestor_scratch = Vec::new();
    visit_with_ancestors(
        root,
        &mut ancestor_scratch,
        &mut |key, val, ancestors, pos| {
            if matches_filter(key, val, filter_lowercase, search_values) {
                nodes_visible.insert(pos, true);

                for ancestor in ancestors.iter().rev() {
                    let was_occupied = match nodes_visible.entry(ancestor.clone()) {
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
        Pos::default(),
    );

    nodes_visible
}

pub fn visit_with_ancestors<'l>(
    val: &'l LuaValue<'l>,
    ancestors: &mut Vec<Pos>,
    f_key: &mut impl FnMut(&'l LuaValue<'l>, &'l LuaValue<'l>, &[Pos], Pos),
    pos: Pos,
) {
    match val {
        LuaValue::Nil => {}
        LuaValue::Bool(_) => {}
        LuaValue::Number(_) => {}
        LuaValue::String(_) => {}
        LuaValue::Table(table) => {
            ancestors.push(pos.clone());
            for (i, (key, val)) in table.iter().enumerate() {
                let new_pos = pos.push(i.try_into().unwrap());
                f_key(key, val, ancestors.as_slice(), new_pos.clone());
                visit_with_ancestors(val, ancestors, f_key, new_pos);
            }
            ancestors.pop();
        }
    }
}

fn show_error(ui: &mut egui::Ui, error: impl Into<String>) {
    ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(255, 51, 51)));
}

fn valpath<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    path: &str,
    lua_state: &mut LuaValue<'_>,
) -> bool {
    ui.label(label);

    let (path, key) = path.rsplit_once('.').unwrap();

    let parent = path.split('.').fold(lua_state, |acc, segment| {
        let table = acc.as_table_mut().expect("invalid path");
        table.get_or_insert(segment, LuaValue::EMPTY_TABLE)
    });
    let number = parent
        .as_table_mut()
        .unwrap()
        .get_or_insert(key, LuaValue::Number(0.0))
        .as_number_mut()
        .unwrap();

    let mut edit = T::from_f64(*number);
    let changed = ui.add(egui::DragValue::new(&mut edit)).changed();
    if changed {
        *number = T::to_f64(edit);
    }
    ui.end_row();

    changed
}
fn numeric<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) -> bool {
    ui.label(label);
    let changed = ui.add(egui::DragValue::new(val)).changed();
    ui.end_row();
    changed
}
/*fn checkbox(ui: &mut egui::Ui, label: &str, val: &mut bool) -> bool {
ui.label(label);
let changed = ui.checkbox(val, "").changed();
ui.end_row();
changed
}*/

fn time<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let ret = f();
    println!("{name}: {}ms", start.elapsed().as_millis());
    ret
}
