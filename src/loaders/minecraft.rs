use std::borrow::Cow;

use iced::{pick_list, Alignment, Checkbox, Command, Element, Length, PickList, Row, Text};
use iced_native::command::Action;

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Deserialize)]
pub struct Version {
    pub version: String,
    pub stable: bool,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

#[derive(Debug)]
pub enum Message {
    Error(anyhow::Error),
}

#[allow(clippy::from_over_into)]
impl Into<Command<Message>> for Message {
    fn into(self) -> Command<Message> {
        Command::single(Action::Future(Box::pin(async { self })))
    }
}

#[derive(Debug, Clone)]
pub enum Interaction {
    SelectVersion(Version),
    ShowSnapshots(bool),
}

#[derive(Debug, Default)]
pub struct State {
    pub pick_list: pick_list::State<Version>,
    pub versions: Vec<Version>,
    pub selected_version: Option<Version>,
    pub show_snapshots: bool,
}

impl State {
    pub fn update_interaction(&mut self, interaction: Interaction) -> Command<Message> {
        match interaction {
            Interaction::SelectVersion(version) => self.selected_version = Some(version),
            Interaction::ShowSnapshots(show) => self.show_snapshots = show,
        }

        Command::none()
    }

    pub fn update_message(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Error(err) => eprintln!("{:#?}", err),
        }

        Command::none()
    }

    pub fn view(&mut self) -> Element<'_, Interaction> {
        Row::new()
            .push(Text::new("Minecraft version:").width(Length::Units(140)))
            .push(
                PickList::new(
                    &mut self.pick_list,
                    Cow::from_iter(
                        self.versions
                            .iter()
                            .filter(|v| self.show_snapshots || v.stable)
                            .cloned(),
                    ),
                    self.selected_version.clone(),
                    Interaction::SelectVersion,
                )
                .width(Length::Fill),
            )
            .push(Checkbox::new(
                self.show_snapshots,
                "Show snapshots",
                Interaction::ShowSnapshots,
            ))
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5)
            .into()
    }
}
