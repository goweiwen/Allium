use iced::font::{Family, Weight};
use iced::widget::{button, column, text};
use iced::{Alignment, Border, Font};

type Element<'a, Message> = iced_core::Element<'a, Message, iced::Theme, iced_tiny_skia::Renderer>;

#[derive(Default)]
pub struct Counter {
    value: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}

impl Counter {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
        }
    }

    pub fn view(&self) -> impl Into<Element<Message>> {
        column![
            button("Increment")
                .style(|theme, status| {
                    let mut style = button::secondary(theme, status);
                    style.border = Border::rounded(16);
                    style
                })
                .on_press(Message::Increment),
            text(self.value).size(50).font(Font {
                family: Family::SansSerif,
                weight: Weight::Bold,
                ..Default::default()
            }),
            button("Decrement").on_press(Message::Decrement)
        ]
        .padding(20)
        .align_items(Alignment::Center)
    }
}
