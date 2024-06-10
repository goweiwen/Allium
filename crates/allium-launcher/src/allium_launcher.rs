use common::app::widgets::button;
use common::app::{Element, Gamepad, Renderer, Theme};
use common::input::Key;
use iced::program::{Title, Update, View};
use iced::widget::{column, text};
use iced::{Alignment, Command};
use log::debug;

#[derive(Debug, Default)]
pub struct State {
    value: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    KeyPressed(Key),
    KeyReleased(Key),
    Increment,
    Decrement,
}

impl Gamepad for Message {
    fn key_press(key: common::input::Key) -> Self {
        Self::KeyPressed(key)
    }

    fn key_release(key: common::input::Key) -> Self {
        Self::KeyReleased(key)
    }
}

pub struct AlliumLauncher;

impl Title<State> for AlliumLauncher {
    fn title(&self, _state: &State) -> String {
        "Allium".to_string()
    }
}

impl Update<State, Message> for AlliumLauncher {
    fn update(&self, state: &mut State, message: Message) -> impl Into<Command<Message>> {
        debug!("message: {:?}", message);
        match message {
            Message::Increment => {
                state.value += 1;
            }
            Message::Decrement => {
                state.value -= 1;
            }
            Message::KeyPressed(_) => {
                state.value += 1;
            }
            Message::KeyReleased(_) => (),
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
