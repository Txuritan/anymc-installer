use std::path::PathBuf;

use iced::{
    alignment::Horizontal,
    button, executor, text_input,
    window::{self, Icon},
    Alignment, Application, Button, Checkbox, Column, Command, Element, Length, ProgressBar, Row,
    Rule, Settings, Text, TextInput,
};
use iced_aw::{TabLabel, Tabs};
use iced_native::command::Action;
use native_dialog::FileDialog;
use png::Transformations;

use crate::{
    loaders::{self, fabric, forge, minecraft, quilt, Loader, LoaderVersion, Side},
    utils, ICON,
};

pub fn run(args: crate::Args) -> anyhow::Result<()> {
    let settings = Settings {
        flags: args,
        window: window::Settings {
            size: (600, 300),
            resizable: false,
            icon: Some(create_icon()?),
            ..Default::default()
        },
        ..Default::default()
    };

    State::run(settings)?;
    Ok(())
}

fn create_icon() -> anyhow::Result<Icon> {
    let mut decoder = png::Decoder::new(ICON);
    decoder.set_transformations(Transformations::EXPAND);

    let mut reader = decoder.read_info()?;

    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;

    let bytes = &buffer[..info.buffer_size()];

    let icon = Icon::from_rgba(bytes.to_vec(), info.width, info.height)?;

    Ok(icon)
}

#[derive(Debug)]
enum Message {
    Interaction(Interaction),

    Error(anyhow::Error),

    BrowseLocation,

    Fabric(fabric::Message),
    Forge(forge::Message),
    Quilt(quilt::Message),

    Install,
    InstallDone(anyhow::Result<()>),
}

#[derive(Debug, Clone)]
enum Interaction {
    SelectLoader(usize),
    SelectSide(usize),

    BrowseLocation,
    ChangeLocation(PathBuf),

    Fabric(fabric::Interaction),
    Forge(forge::Interaction),
    Quilt(quilt::Interaction),

    ClientGenerateProfile(bool),
    ServerDownloadJar(bool),
    ServerGenerateScript(bool),

    Install,
    InstallPrevent,
}

#[allow(clippy::from_over_into)]
impl Into<Command<Message>> for Message {
    fn into(self) -> Command<Message> {
        Command::single(Action::Future(Box::pin(async { self })))
    }
}

#[derive(Debug, Default)]
struct State {
    selected_loader: Loader,
    selected_side: Side,

    fabric: fabric::State,
    forge: forge::State,
    quilt: quilt::State,

    install_location_input: text_input::State,
    install_location: PathBuf,
    install_location_browse: button::State,

    client_generate_profile: bool,

    server_download_jar: bool,
    server_generate_script: bool,

    install_button: button::State,
    install_running: bool,
    install_progress: f32,
}

impl Application for State {
    type Executor = executor::Default;

    type Message = Message;

    type Flags = crate::Args;

    #[rustfmt::skip]
    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                install_location: utils::get_minecraft_directory().unwrap_or_default(),
                ..Default::default()
            },
            Command::batch([
                Command::perform(fabric::Commands::fetch_minecraft(), fabric::Message::SetMinecraft).map(Message::Fabric),
                Command::perform(fabric::Commands::fetch_versions(), fabric::Message::SetVersions).map(Message::Fabric),

                Command::perform(quilt::Commands::fetch_minecraft(), quilt::Message::SetMinecraft).map(Message::Quilt),
                Command::perform(quilt::Commands::fetch_versions(), quilt::Message::SetVersions).map(Message::Quilt),
            ]),
        )
    }

    fn title(&self) -> String {
        "anymc-installer".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Interaction(interaction) => match interaction {
                Interaction::SelectLoader(tab) => self.selected_loader = Loader::from(tab as u8),
                Interaction::SelectSide(tab) => self.selected_side = Side::from(tab as u8),

                Interaction::BrowseLocation => return Message::BrowseLocation.into(),
                Interaction::ChangeLocation(location) => self.install_location = location,

                Interaction::Fabric(message) => {
                    return self.fabric.update_interaction(message).map(Message::Fabric)
                }
                Interaction::Forge(message) => {
                    return self.forge.update_interaction(message).map(Message::Forge)
                }
                Interaction::Quilt(message) => {
                    return self.quilt.update_interaction(message).map(Message::Quilt)
                }

                Interaction::ClientGenerateProfile(enable) => self.client_generate_profile = enable,
                Interaction::ServerDownloadJar(enable) => self.server_download_jar = enable,
                Interaction::ServerGenerateScript(enable) => self.server_generate_script = enable,

                Interaction::Install => return Message::Install.into(),
                Interaction::InstallPrevent => {}
            },
            Message::Error(err) => eprintln!("{:#?}", err),
            Message::Fabric(message) => {
                return self.fabric.update_message(message).map(Message::Fabric)
            }
            Message::Forge(message) => {
                return self.forge.update_message(message).map(Message::Forge)
            }
            Message::Quilt(message) => {
                return self.quilt.update_message(message).map(Message::Quilt)
            }
            Message::BrowseLocation => {
                let mut dialog = FileDialog::new();

                let working_dir = std::env::current_dir();
                if self.install_location.is_dir() {
                    dialog = dialog.set_location(&self.install_location);
                } else if working_dir.is_ok() {
                    dialog = dialog.set_location(working_dir.as_deref().unwrap())
                }

                match dialog.show_open_single_dir() {
                    Ok(Some(path)) => self.install_location = path,
                    Ok(None) => (),
                    Err(error) => return Message::Error(error.into()).into(),
                }
            }
            Message::Install => {
                let minecraft_version = match self.selected_loader {
                    Loader::Fabric => self.fabric.selected_minecraft(),
                    Loader::Forge => self.forge.selected_minecraft(),
                    Loader::Quilt => self.quilt.selected_minecraft(),
                };
                let minecraft_version = if let Some(version) = minecraft_version {
                    version
                } else {
                    return Message::Error(anyhow::anyhow!("No Minecraft version selected!"))
                        .into();
                };

                let loader_version = match self.selected_loader {
                    Loader::Fabric => self.fabric.selected_version().map(LoaderVersion::Fabric),
                    Loader::Forge => self.forge.selected_version().map(LoaderVersion::Forge),
                    Loader::Quilt => self.quilt.selected_version().map(LoaderVersion::Quilt),
                };
                let loader_version = if let Some(version) = loader_version {
                    version
                } else {
                    return Message::Error(anyhow::anyhow!("No Loader version selected!")).into();
                };

                return Command::perform(
                    loaders::install(
                        loader_version,
                        self.selected_side,
                        self.install_location.clone(),
                        minecraft_version,
                        match self.selected_side {
                            Side::Client => self.client_generate_profile,
                            Side::Server => self.server_generate_script,
                        },
                    ),
                    Message::InstallDone,
                );
            }
            Message::InstallDone(_result) => {}
        }

        Command::none()
    }

    #[rustfmt::skip]
    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        let column = Column::new()
            .padding(5)
            .spacing(5)
            .push(Row::new()
                .padding(5)
                .spacing(15)
                .push(Tabs::new(u8::from(self.selected_loader).into(), Interaction::SelectLoader)
                    .push(TabLabel::Text("Fabric".to_string()), Row::new())
                    .push(TabLabel::Text("Forge".to_string()), Row::new())
                    .push(TabLabel::Text("Quilt".to_string()), Row::new())
                    )
                .push(Tabs::new(u8::from(self.selected_side).into(), Interaction::SelectSide)
                    .push(TabLabel::Text("Client".to_string()), Row::new())
                    .push(TabLabel::Text("Server".to_string()), Row::new()))
                )
            .push(match self.selected_loader {
                Loader::Fabric => self.fabric.view().map(Interaction::Fabric),
                Loader::Forge => self.forge.view().map(Interaction::Forge),
                Loader::Quilt => self.quilt.view().map(Interaction::Quilt),
            })
            .push(Rule::horizontal(5))
            .push(Row::new()
                .push(Text::new("Directory:").width(Length::Units(140)))
                .push(TextInput::new(&mut self.install_location_input, "Install Location", self.install_location.to_str().unwrap(), |s| Interaction::ChangeLocation(PathBuf::from(s))).padding(5))
                .push(Button::new(&mut self.install_location_browse, Text::new("Browse...")).on_press(Interaction::BrowseLocation))
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .spacing(5)
                .padding(5))
            .push(match self.selected_side {
                Side::Client => Row::new()
                    .push(Text::new("Options:").width(Length::Units(140)))
                    .push(Checkbox::new(self.client_generate_profile, "Generate profile", Interaction::ClientGenerateProfile))
                    .spacing(5)
                    .padding(5),
                Side::Server => Row::new()
                    .push(Text::new("Options:").width(Length::Units(140)))
                    .push(Checkbox::new(self.server_download_jar, "Download server jar", Interaction::ServerDownloadJar))
                    .push(Checkbox::new(self.server_generate_script, "Generate launch script", Interaction::ServerGenerateScript))
                    .spacing(5)
                    .padding(5),
            })
            .push(Rule::horizontal(5))
            .push(Button::new(
                &mut self.install_button,
                Text::new("Install")
                    .horizontal_alignment(Horizontal::Center)
                    .width(Length::Fill),
                )
                .width(Length::Fill)
                .on_press(if self.install_running { Interaction::InstallPrevent } else { Interaction::Install }))
            .push(ProgressBar::new(0.0..=1.0, self.install_progress));

        let content: Element<Interaction> = column.into();
        content.map(Message::Interaction)
    }
}
