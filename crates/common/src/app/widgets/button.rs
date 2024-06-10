use iced::widget::Component;
use iced_core::widget::operation::Focusable;

use crate::app::{Element, Focus, Renderer, Theme};

#[allow(missing_debug_implementations)]
pub struct Button<'a, Message, Element>
where
    Element: Into<iced::Element<'a, Event, Theme, Renderer>>,
{
    content: Element,
    on_change: Box<dyn Fn(Option<u32>) -> Message>,
    is_focused: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Focus,
    Unfocus,
}

impl<'a, Message> Component<Message, Theme, Renderer> for Button<'a, Message>
where
    Message: 'a + Clone + Focus,
{
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Focus => {
                self.is_focused = true;
                Some(Message::focus())
            }
            Event::Unfocus => {
                self.is_focused = false;
                Some(Message::unfocus())
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event> {
        iced::widget::button(self.content.clone()).into()
    }
}

impl<'a, Message> Focusable for Button<'a, Message> {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}
