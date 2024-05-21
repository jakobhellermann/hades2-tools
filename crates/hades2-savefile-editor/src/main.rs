use hades2::saves::{LuaValue, Savefile};
use hades2::{Hades2Installation, SaveHandle};
use iced::alignment::Vertical;
use iced::border::Radius;
use iced::widget::{column, container, row, scrollable, text, Button, Column, Theme, Toggler};
use iced::{theme, Alignment, Application, Border, Color, Padding};
use iced::{Element, Length, Settings};

pub fn main() -> iced::Result {
    SavefileEditor::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    ToggleMode(bool),

    PickFile,
    FilePicked(SaveHandle, Savefile, LuaValue<'static>),

    OpenSlot(usize),

    Error(String),
    Noop,
}
type Command = iced::Command<Message>;

struct SavefileEditor {
    hades: Option<Hades2Installation>,
    saves: Vec<(SaveHandle, Savefile)>,
    savefile: Option<(SaveHandle, Savefile, LuaValue<'static>)>,
    advanced_mode: bool,
    error: Option<String>,
}

impl SavefileEditor {
    fn hades(&self) -> Option<&Hades2Installation> {
        self.hades.as_ref()
    }

    fn unwrap<T, E: std::fmt::Display>(&mut self, result: Result<T, E>) -> Option<T> {
        match result {
            Ok(val) => Some(val),
            Err(e) => {
                self.error = Some(e.to_string());
                None
            }
        }
    }

    fn saves_header_only(&self) -> Result<Vec<(SaveHandle, Savefile)>, hades2::Error> {
        let Some(hades) = self.hades() else {
            return Ok(Vec::new());
        };

        let saves = hades.saves()?;
        saves
            .into_iter()
            .map(|handle| {
                let savefile = handle.read_header_only()?;
                Ok((handle, savefile))
            })
            .collect::<Result<Vec<_>, hades2::Error>>()
    }
}

impl Application for SavefileEditor {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_: ()) -> (Self, Command) {
        let (hades, error) = match Hades2Installation::detect() {
            Ok(hades) => (Some(hades), None),
            Err(error) => (None, Some(error)),
        };

        let mut app = Self {
            hades,
            saves: Vec::new(),
            savefile: None,
            advanced_mode: true,
            error: error.map(|e| e.to_string()),
        };

        app.saves = app.unwrap(app.saves_header_only()).unwrap_or_default();

        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("Hades II Savefile Editor")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command {
        match message {
            Message::ToggleMode(mode) => self.advanced_mode = mode,
            Message::PickFile => {
                let save_dir = self.hades().map(|hades| hades.save_dir().to_owned());

                return Command::perform(
                    async {
                        let mut picker = rfd::AsyncFileDialog::new().add_filter(".sav", &["sav"]);
                        if let Some(save_dir) = save_dir {
                            picker = picker.set_directory(save_dir);
                        }

                        let file = picker.pick_file().await?;
                        let data = file.read().await;

                        let savefile = hades2::saves::Savefile::parse(&data);
                        Some((file.path().to_owned(), savefile))
                    },
                    |result| {
                        let Some((path, save)) = result else {
                            return Message::Noop;
                        };

                        match save {
                            Ok((save, state)) => Message::FilePicked(
                                SaveHandle::from_path(path).unwrap(),
                                save,
                                state,
                            ),
                            Err(error) => Message::Error(format!(
                                "failed to open {}: {}",
                                path.display(),
                                error
                            )),
                        }
                    },
                );
            }
            Message::FilePicked(handle, save, state) => {
                self.savefile = Some((handle, save, state));
            }
            Message::OpenSlot(i) => {
                let (handle, save) = &self.saves[i];
                let lua_state = handle.read().map(|(_, lua_state)| lua_state);
                match lua_state {
                    Ok(state) => self.savefile = Some((handle.clone(), save.clone(), state)),
                    Err(e) => {
                        self.error = Some(e.to_string());
                    }
                };
            }
            Message::Noop => {}
            Message::Error(error) => self.error = Some(error),
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let title = text("Savefile Editor").size(40);

        let advanced_toggle = Toggler::new(
            "Advanced Mode".to_string(),
            self.advanced_mode,
            Message::ToggleMode,
        )
        .width(Length::Shrink);

        let heading = container(
            row![row![title]
                .align_items(Alignment::Center)
                .spacing(16)
                .width(Length::Fill),]
            .push_maybe(self.savefile.is_some().then(|| advanced_toggle))
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x();

        let content = match &self.savefile {
            Some((_, save, state)) => {
                let mut page = column![text(format!(
                    "todo ({} runs, {} grasp)",
                    save.runs + 1,
                    save.grasp
                ))];

                if self.advanced_mode {
                    page = page.push(text(&format!("{save:#?}")));
                    page = page.push(text(&format!("{state:#?}")[..5000]));
                } else {
                    page = page.push(text("easy"));
                }

                Element::new(page)
            }
            None => {
                Column::with_children(self.saves.iter().enumerate().map(|(i, (handle, save))| {
                    container(
                        row![
                            text(handle.slot()),
                            Button::new("Open").on_press(Message::OpenSlot(i)),
                            text(format!("Runs: {:<2}", save.runs + 1)),
                            text(format!("Grasp: {:<2}", save.grasp))
                        ]
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .spacing(16),
                    )
                    .style(theme::Container::Custom(Box::new(ColoredBg)))
                    .padding(8)
                    .into()
                }))
                .spacing(16)
                .push(container(Button::new("Open File").on_press(Message::PickFile)).padding(4))
                .into()
            }
        };

        let content = column![heading]
            .push(scrollable(content).width(Length::Fill))
            .push_maybe(self.error.as_ref().map(|error| {
                text(error)
                    .height(Length::Fill)
                    .vertical_alignment(Vertical::Bottom)
                    .style(iced::Color::from_rgb8(255, 51, 51))
            }))
            .spacing(16)
            .padding(Padding::new(8.));

        let element = Element::from(content);
        match EXPLAIN {
            true => element.explain(Color::BLACK),
            false => element,
        }
    }
}
const EXPLAIN: bool = false;

struct ColoredBg;
impl container::StyleSheet for ColoredBg {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.3))),
            border: Border {
                radius: Radius::from(8.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
