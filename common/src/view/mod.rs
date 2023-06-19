mod battery_indicator;
mod button_hint;
mod button_icon;
mod image;
mod input;
mod label;
mod list;
mod null;
mod row;
mod scroll_list;
mod settings_list;

use std::collections::VecDeque;
use std::fmt;

pub use self::battery_indicator::BatteryIndicator;
pub use self::button_hint::ButtonHint;
pub use self::button_icon::ButtonIcon;
pub use self::image::{Image, ImageMode};
pub use self::input::button::Button;
pub use self::input::color_picker::ColorPicker;
pub use self::input::keyboard;
pub use self::input::percentage::Percentage;
pub use self::input::text_box::TextBox;
pub use self::input::toggle::Toggle;
pub use self::label::Label;
pub use self::list::List;
pub use self::null::NullView;
pub use self::row::Row;
pub use self::scroll_list::ScrollList;
pub use self::settings_list::SettingsList;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::command::Command;
use crate::geom::{Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::stylesheet::{Stylesheet, StylesheetColor};

#[async_trait(?Send)]
pub trait View {
    /// Update the view.
    fn update(&mut self) -> Result<()> {
        Ok(())
    }

    /// Draw the view. Returns true if the view was drawn.
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool>;

    /// Returns true if the view should be drawn.
    fn should_draw(&self) -> bool;

    /// Sets whether the view should be drawn.
    fn set_should_draw(&mut self);

    /// Handle a key event. Returns true if the event was consumed.
    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        // Sends to the root.
        commands: Sender<Command>,
        // Bubbles the signal upwards, starting from the parent view to the top.
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool>;

    /// Returns a list of references to the children of the view.
    fn children(&self) -> Vec<&dyn View>;

    /// Returns a list of mutable references to the children of the view.
    fn children_mut(&mut self) -> Vec<&mut dyn View>;

    /// Get the bounding box of the view.
    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.children_mut()
            .iter_mut()
            .map(|c| c.bounding_box(styles))
            .fold(Rect::zero(), |acc, r| acc.union(&r))
    }

    /// Sets the position of the view.
    fn set_position(&mut self, point: Point);

    /// Sets the background color of the view.
    fn set_background_color(&mut self, _color: StylesheetColor) {
        self.children_mut()
            .iter_mut()
            .for_each(|c| c.set_background_color(_color));
    }
}

impl fmt::Debug for dyn View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "View")
    }
}

#[async_trait(?Send)]
impl View for Box<dyn View> {
    fn update(&mut self) -> Result<()> {
        (**self).update()
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        (**self).draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        (**self).should_draw()
    }

    fn set_should_draw(&mut self) {
        (**self).set_should_draw()
    }

    /// Handle a key event. Returns true if the event was consumed.
    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        // Sends to the root.
        commands: Sender<Command>,
        // Bubbles the signal upwards, starting from the parent view to the top.
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        (**self).handle_key_event(event, commands, bubble).await
    }

    /// Returns a list of references to the children of the view.
    fn children(&self) -> Vec<&dyn View> {
        (**self).children()
    }

    /// Returns a list of mutable references to the children of the view.
    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        (**self).children_mut()
    }

    /// Get the bounding box of the view.
    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        (**self).bounding_box(styles)
    }

    /// Sets the position of the view.
    fn set_position(&mut self, point: Point) {
        (**self).set_position(point)
    }

    /// Sets the background color of the view.
    fn set_background_color(&mut self, _color: StylesheetColor) {
        (**self).set_background_color(_color)
    }
}
