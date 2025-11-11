use std::collections::VecDeque;
use std::marker::PhantomData;

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
use common::view::{BatteryIndicator, Clock, Label, Row, View};
use tokio::sync::mpsc::Sender;

use crate::view::ActivityTracker;

#[derive(Debug)]
pub struct App<B>
where
    B: Battery + 'static,
{
    rect: Rect,
    label: Label<String>,
    row: Row<Box<dyn View>>,
    view: ActivityTracker,
    dirty: bool,
    _phantom_battery: PhantomData<B>,
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
            Point::new(0, 0),
            battery,
            styles.show_battery_level,
        );

        let mut children: Vec<Box<dyn View>> = vec![Box::new(battery_indicator)];

        if styles.show_clock {
            let clock = Clock::new(res.clone(), Point::new(0, 0), Alignment::Right);
            children.push(Box::new(clock));
        }

        let row: Row<Box<dyn View>> = Row::new(
            Point::new(w as i32 - styles.margin_y, y + styles.margin_y),
            children,
            Alignment::Right,
            8,
        );

        let label = Label::new(
            Point::new(x + styles.margin_x, y + styles.margin_y),
            locale.t("activity-tracker-title"),
            Alignment::Left,
            None,
        );

        let rect = Rect::new(
            x,
            y + styles.margin_y * 2 + styles.ui_font.size as i32,
            w,
            h - styles.margin_y as u32 * 2 - styles.ui_font.size,
        );

        drop(styles);
        drop(locale);

        let view = ActivityTracker::new(rect, res)?;

        Ok(Self {
            rect,
            label,
            row,
            view,
            dirty: true,
            _phantom_battery: PhantomData,
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
        drawn |= self.row.should_draw() && self.row.draw(display, styles)?;
        drawn |= self.view.should_draw() && self.view.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.label.should_draw() || self.row.should_draw() || self.view.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.label.set_should_draw();
        self.row.set_should_draw();
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
        vec![&self.row, &self.view]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.row, &mut self.view]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
