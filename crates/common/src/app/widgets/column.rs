use std::marker::PhantomData;

use iced::widget::{component, Component};
use iced_core::widget::operation::Focusable;
use log::debug;

use crate::app::{Element, Renderer, Theme};

/// Creates a [`Column`] with the given children.
///
/// [`Column`]: crate::Column
#[macro_export]
macro_rules! column {
    () => (
        $crate::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::Column::with_children([$($crate::core::Element::from($x)),+])
    );
}

#[allow(missing_debug_implementations)]
pub struct Column<'a, Message>
where
    Message: 'a,
{
    content: Vec<Element<'a, Event>>,
    is_focused: bool,
}

pub fn column<'a, Message>(content: Vec<Element<'a, Event>>) -> Column<'a, Message> {
    Column::new(content)
}

impl<'a, Message> Column<'a, Message> {
    fn new(content: Vec<Element<'a, Event>>) -> Self {
        Self {
            content,
            is_focused: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Focused,
    Unfocused,
}

impl<'a, Message> Component<Message, Theme, Renderer> for Column<'a, Message>
where
    Message: Clone,
{
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<Message> {
        match event {
            Event::Focused => {
                self.is_focused = true;
            }
            Event::Unfocused => {
                self.is_focused = false;
            }
        }
        // TODO: children
        None
    }

    fn view(&self, _state: &Self::State) -> crate::app::Element<'_, Self::Event> {
        iced::widget::column(self.content.clone()).into()
    }
}

impl<'a, Message> From<Column<'a, Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(value: Column<'a, Message>) -> Self {
        component(value)
    }
}

// impl<'a, Message, Content> Focusable for Button<'a, Message, Content>
// where
//     Content: Into<Element<'a, Event>>,
// {
//     fn is_focused(&self) -> bool {
//         self.is_focused
//     }

//     fn focus(&mut self) {
//         self.is_focused = true;
//     }

//     fn unfocus(&mut self) {
//         self.is_focused = false;
//     }
// }
