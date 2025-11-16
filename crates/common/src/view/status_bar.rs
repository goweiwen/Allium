use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::battery::Battery;
use crate::geom::{Alignment, Point, Rect};
use crate::platform::{DefaultPlatform, KeyEvent, Platform};
use crate::resources::Resources;
use crate::stylesheet::Stylesheet;
use crate::view::{BatteryIndicator, Clock, Command, Row, View};

#[derive(Debug)]
pub struct StatusBar<B>
where
    B: Battery + 'static,
{
    row: Row<Box<dyn View>>,
    _phantom: std::marker::PhantomData<B>,
}

impl<B> StatusBar<B>
where
    B: Battery + 'static,
{
    pub fn new(res: Resources, point: Point, battery: B) -> Self {
        let styles = res.get::<Stylesheet>();

        let battery_indicator = BatteryIndicator::new(
            res.clone(),
            Point::new(0, 0),
            battery,
            styles.status_bar.show_battery_level,
        );

        let mut children: Vec<Box<dyn View>> = vec![Box::new(battery_indicator)];

        if styles.status_bar.show_clock {
            let clock = Clock::new(res.clone(), Point::new(0, 0), Alignment::Right);
            children.push(Box::new(clock));
        }

        let row = Row::new(point, children, Alignment::Right, 8);

        drop(styles);

        Self {
            row,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn row(&self) -> &Row<Box<dyn View>> {
        &self.row
    }

    pub fn row_mut(&mut self) -> &mut Row<Box<dyn View>> {
        &mut self.row
    }
}

#[async_trait(?Send)]
impl<B> View for StatusBar<B>
where
    B: Battery,
{
    fn update(&mut self, dt: std::time::Duration) {
        self.row.update(dt);
    }

    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        self.row.draw(display, styles)
    }

    fn should_draw(&self) -> bool {
        self.row.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.row.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        self.row.handle_key_event(event, commands, bubble).await
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.row as &dyn View]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.row as &mut dyn View]
    }

    fn bounding_box(&mut self, styles: &Stylesheet) -> Rect {
        self.row.bounding_box(styles)
    }

    fn set_position(&mut self, point: Point) {
        self.row.set_position(point);
    }
}
