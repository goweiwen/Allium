use std::marker::PhantomData;

use iced::widget::{component, Component};
use iced_core::widget::operation::Focusable;
use log::debug;

use crate::app::{Element, Renderer, Theme};

#[allow(missing_debug_implementations)]
pub struct Button<'a, Message, Content>
where
    Content: Into<Element<'a, Event>>,
    Message: 'a,
{
    content: Content,
    on_press: Option<Message>,
    on_focus: Option<Message>,
    on_blur: Option<Message>,
    is_focused: bool,
    phantom: PhantomData<&'a ()>,
}

pub fn button<'a, Message, Content>(content: Content) -> Button<'a, Message, Content>
where
    Content: Into<Element<'a, Event>>,
{
    Button::new(content)
}

impl<'a, Message, Content> Button<'a, Message, Content>
where
    Content: Into<Element<'a, Event>>,
{
    fn new(content: Content) -> Self {
        Self {
            content,
            on_press: None,
            on_focus: None,
            on_blur: None,
            is_focused: false,
            phantom: PhantomData,
        }
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self
    where
        Message: std::fmt::Debug,
    {
        self.on_press = Some(on_press);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed,
    /// if `Some`.
    ///
    /// If `None`, the [`Button`] will be disabled.
    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press;
        self
    }

    /// Sets the message that will be produced when the [`Button`] is focused.
    pub fn on_focus(mut self, on_focus: Message) -> Self {
        self.on_focus = Some(on_focus);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is focused,
    /// if `Some`.
    pub fn on_focus_maybe(mut self, on_focus: Option<Message>) -> Self {
        self.on_focus = on_focus;
        self
    }

    /// Sets the message that will be produced when the [`Button`] is blurred.
    pub fn on_blur(mut self, on_blur: Message) -> Self {
        self.on_blur = Some(on_blur);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is blurred,
    /// if `Some`.
    pub fn on_blur_maybe(mut self, on_blur: Option<Message>) -> Self {
        self.on_blur = on_blur;
        self
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Pressed,
    Focused,
    Unfocused,
}

impl<'a, Message, Content> Component<Message, Theme, Renderer> for Button<'a, Message, Content>
where
    Content: Into<Element<'a, Event>> + Clone,
    Message: Clone,
{
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<Message> {
        debug!("[button] update: {:?}", event);
        match event {
            Event::Pressed => self.on_press.clone(),
            Event::Focused => {
                self.is_focused = true;
                self.on_focus.clone()
            }
            Event::Unfocused => {
                self.is_focused = false;
                self.on_blur.clone()
            }
        }
    }

    fn view(&self, _state: &Self::State) -> crate::app::Element<'_, Self::Event> {
        iced::widget::button(self.content.clone())
            .on_press(Event::Pressed)
            .into()
    }
}

impl<'a, Message, Content> From<Button<'a, Message, Content>> for Element<'a, Message>
where
    Content: 'a + Into<Element<'a, Event>> + Clone,
    Message: 'a + Clone,
{
    fn from(value: Button<'a, Message, Content>) -> Self {
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
