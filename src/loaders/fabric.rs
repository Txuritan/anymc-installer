use std::borrow::Cow;

use iced::{pick_list, Alignment, Column, Command, Element, Length, PickList, Row, Text};
use iced_native::command::Action;

use crate::loaders::{minecraft, Install};

pub static GAME: &str = "https://meta.fabricmc.net/v2/versions/game";
pub static MAVEN: &str = "https://maven.fabricmc.net/";
pub static META: &str = "https://meta.fabricmc.net/v2/versions/loader";

pub struct Commands;

impl Commands {
    #[tracing::instrument(skip_all, err)]
    pub async fn fetch_minecraft() -> anyhow::Result<Vec<minecraft::Version>> {
        Ok(reqwest::get(GAME).await?.json().await?)
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn fetch_versions() -> anyhow::Result<Vec<Version>> {
        Ok(reqwest::get(META).await?.json().await?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Deserialize)]
pub struct Version {
    pub separator: String,
    pub build: i64,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.version.fmt(f)
    }
}

pub async fn install(_install: Install<Version>) -> anyhow::Result<()> {
    Ok(())
}

#[derive(Debug)]
pub enum Message {
    Error(anyhow::Error),

    Minecraft(minecraft::Message),

    SetMinecraft(anyhow::Result<Vec<minecraft::Version>>),
    SetVersions(anyhow::Result<Vec<Version>>),
}

#[allow(clippy::from_over_into)]
impl Into<Command<Message>> for Message {
    fn into(self) -> Command<Message> {
        Command::single(Action::Future(Box::pin(async { self })))
    }
}

#[derive(Debug, Clone)]
pub enum Interaction {
    Minecraft(minecraft::Interaction),

    SelectVersion(Version),
}

#[derive(Debug, Default)]
pub struct State {
    pub minecraft: minecraft::State,

    pub pick_list: pick_list::State<Version>,
    pub versions: Vec<Version>,
    pub selected_version: Option<Version>,
}

impl State {
    pub fn selected_version(&self) -> Option<Version> {
        self.selected_version.as_ref().cloned()
    }

    pub fn selected_minecraft(&self) -> Option<minecraft::Version> {
        self.minecraft.selected_version.as_ref().cloned()
    }

    pub fn update_interaction(&mut self, interaction: Interaction) -> Command<Message> {
        match interaction {
            Interaction::Minecraft(interaction) => {
                return self
                    .minecraft
                    .update_interaction(interaction)
                    .map(Message::Minecraft)
            }

            Interaction::SelectVersion(version) => self.selected_version = Some(version),
        }

        Command::none()
    }

    pub fn update_message(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Error(err) => eprintln!("{:#?}", err),

            Message::Minecraft(message) => {
                return self
                    .minecraft
                    .update_message(message)
                    .map(Message::Minecraft)
            }

            Message::SetMinecraft(result) => {
                match result {
                    Ok(versions) => self.minecraft.versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }

                if self.minecraft.selected_version.is_none() {
                    self.minecraft.selected_version =
                        self.minecraft.versions.iter().find(|v| v.stable).cloned();
                }
            }
            Message::SetVersions(result) => {
                match result {
                    Ok(versions) => self.versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }

                if self.selected_version.is_none() {
                    self.selected_version = self.versions.first().cloned();
                }
            }
        }

        Command::none()
    }

    pub fn view(&mut self) -> Element<'_, Interaction> {
        Column::new()
            .push(self.minecraft.view().map(Interaction::Minecraft))
            .push(
                Row::new()
                    .push(Text::new("Loader version:").width(Length::Units(140)))
                    .push(
                        PickList::new(
                            &mut self.pick_list,
                            Cow::from(self.versions.clone()),
                            self.selected_version.clone(),
                            Interaction::SelectVersion,
                        )
                        .width(Length::Fill),
                    )
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .spacing(5)
                    .padding(5),
            )
            .into()
    }
}
