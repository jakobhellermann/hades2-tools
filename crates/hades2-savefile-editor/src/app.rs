use anyhow::Result;
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
            if let [(handle, _), ..] = self.state.saves.as_slice() {
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

impl Default for App {
    fn default() -> Self {
        let (hades, error) = match Hades2Installation::detect() {
            Ok(val) => (Some(val), None),
            Err(val) => (None, Some(format!("{val}"))),
        };
        let mut app = Self {
            advanced_mode: false,
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
                    }
                }
                ui.heading(heading);
            });
            ui.separator();

            let mut current_savefile = self.current_savefile.take();
            match current_savefile {
                None => self.ui_save_select(ui, ctx),
                Some(ref mut current) => {
                    self.ui_current(ui, ctx, current);
                    self.current_savefile = current_savefile;
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

                let time =
                    time::OffsetDateTime::from_unix_timestamp(save.timestamp as i64).unwrap();
                ui.label(format!("{} {}", time.date(), time.time()));

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

        fn numeric<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, val: &mut T) {
            ui.label(label);
            ui.add(egui::DragValue::new(val));
            ui.end_row();
        }
        fn checkbox(ui: &mut egui::Ui, label: &str, val: &mut bool) {
            ui.label(label);
            ui.checkbox(val, "");
            ui.end_row();
        }

        if self.advanced_mode {
            ui.text_edit_singleline(filter);

            ScrollArea::vertical().show(ui, |ui| {
                *dirty |= luavalue::show_value(ui, lua_state, (0, 0));
                ui.allocate_space(ui.available_size());
            });
        } else {
            Grid::new("easy mode").show(ui, |ui| {
                numeric(ui, "Runs", &mut save.runs);
                numeric(ui, "Grasp", &mut save.grasp);
                numeric(ui, "Meta Points", &mut save.accumulated_meta_points);
                numeric(ui, "Active Shrine Points", &mut save.active_shrine_points);
                checkbox(ui, "Easy Mode", &mut save.easy_mode);
                checkbox(ui, "Hard Mode", &mut save.hard_mode);
            });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(*dirty, |ui| {
                    if ui.button("Save").clicked() {
                        dbg!();
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
