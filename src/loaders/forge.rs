use iced::{Alignment, Command, Element, Length, Row};
use iced_native::command::Action;

use crate::loaders::{minecraft, Install};

pub async fn install(_install: Install<bool>) -> anyhow::Result<()> {
    Ok(())
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
pub enum Interaction {}

#[derive(Debug, Default)]
pub struct State {}

impl State {
    pub fn selected_version(&self) -> Option<bool> {
        todo!()
    }

    pub fn selected_minecraft(&self) -> Option<minecraft::Version> {
        todo!()
    }

    pub fn update_interaction(&mut self, _interaction: Interaction) -> Command<Message> {
        // match interaction {}

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
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(5)
            .padding(5)
            .into()
    }
}
