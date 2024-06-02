use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{BatteryIndicator, Label, View};
use tokio::sync::mpsc::Sender;

use crate::view::ActivityTracker;

#[derive(Debug)]
pub struct App<B>
where
    B: Battery + 'static,
{
    rect: Rect,
    label: Label<String>,
    battery_indicator: BatteryIndicator<B>,
    view: ActivityTracker,
    dirty: bool,
}

impl<B> App<B>
where
    B: Battery + 'static,
{
    pub fn new(rect: Rect, res: Resources, battery: B) -> Result<Self> {
        let Rect { x, y, w, h } = rect;
        let styles = res.get::<Stylesheet>();
        let locale = res.get::<Locale>();

        let battery_indicator = BatteryIndicator::new(
            res.clone(),
            Point::new(w as i32 - 12, y + 8),
            battery,
            styles.show_battery_level,
        );

        let label = Label::new(
            Point::new(x + 12, y + 8),
            locale.t("activity-tracker-title"),
            Alignment::Left,
            None,
        );

        let rect = Rect::new(
            x,
            y + 8 + styles.ui_font.size as i32 + 8,
            w,
            h - 8 - styles.ui_font.size - 8,
        );

        drop(styles);
        drop(locale);

        let view = ActivityTracker::new(rect, res)?;

        Ok(Self {
            rect,
            label,
            battery_indicator,
            view,
            dirty: true,
        })
    }
}

#[async_trait(?Send)]
impl<B> View for App<B>
where
    B: Battery,
{
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        if self.dirty {
            display.load(self.bounding_box(styles))?;
            self.dirty = false;
        }

        let mut drawn = false;

        drawn |= self.label.should_draw() && self.label.draw(display, styles)?;
        drawn |=
            self.battery_indicator.should_draw() && self.battery_indicator.draw(display, styles)?;
        drawn |= self.view.should_draw() && self.view.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.label.should_draw() || self.battery_indicator.should_draw() || self.view.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.label.set_should_draw();
        self.battery_indicator.set_should_draw();
        self.view.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        self.view.handle_key_event(event, commands, bubble).await
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.battery_indicator, &self.view]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.battery_indicator, &mut self.view]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
