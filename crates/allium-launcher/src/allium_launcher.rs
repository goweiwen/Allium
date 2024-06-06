use common::app::{Element, Renderer, Theme};
use iced::program::{Title, Update, View};
use iced::widget::{button, column, text, Column};
use iced::Alignment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct State {
    value: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}

pub struct AlliumLauncher;

impl Title<State> for AlliumLauncher {
    fn title(&self, _state: &State) -> String {
        "Allium".to_string()
    }
}

impl Update<State, Message> for AlliumLauncher {
    fn update(&self, state: &mut State, message: Message) {
        match message {
            Message::Increment => {
                state.value += 1;
            }
            Message::Decrement => {
                state.value -= 1;
            }
        }
    }
}

impl<'a> View<'a, State, Message, Theme, Renderer> for AlliumLauncher {
    fn view(&self, state: &'a State) -> impl Into<Element<'a, Message>> {
        column![
            button("Increment").on_press(Message::Increment),
            text(state.value).size(50),
            button("Decrement").on_press(Message::Decrement)
        ]
        .padding(20.0)
        .align_items(Alignment::Center)
    }
}
