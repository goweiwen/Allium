use std::collections::VecDeque;
use std::fs::{self, File};
use std::rc::Rc;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::ALLIUM_LAUNCHER_STATE;
use common::database::Database;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{BatteryIndicator, Label, Row, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::{trace, warn};

use crate::devices::DeviceMapper;
use crate::view::browser::Directory;
use crate::view::Recents;
use crate::view::{Browser, Settings};

#[derive(Debug, Serialize, Deserialize)]
struct State {
    views: (Browser, Recents, Settings),
    selected: usize,
    dirty: bool,
}

impl State {
    fn new(rect: Rect) -> Self {
        let views = (
            Browser::new(rect, Directory::default()).unwrap(),
            Recents::new(rect).unwrap(),
            Settings::new(rect).unwrap(),
        );
        Self {
            views,
            selected: 0,
            dirty: false,
        }
    }

    fn init(&mut self, database: Database, device_mapper: Rc<DeviceMapper>) -> Result<()> {
        self.views
            .0
            .init(database.clone(), Rc::clone(&device_mapper));
        self.views.1.init(database, device_mapper)?;
        Ok(())
    }

    fn load() -> Result<Option<Self>> {
        if ALLIUM_LAUNCHER_STATE.exists() {
            if let Ok(json) = fs::read_to_string(ALLIUM_LAUNCHER_STATE.as_path()) {
                if let Ok(state) = serde_json::from_str::<Self>(&json) {
                    return Ok(Some(state));
                }
                warn!("failed to deserialize state file, deleting");
                fs::remove_file(ALLIUM_LAUNCHER_STATE.as_path())?;
            }
        }
        Ok(None)
    }

    fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_LAUNCHER_STATE.as_path())?;
        serde_json::to_writer(file, &self)?;
        Ok(())
    }

    const VIEW_COUNT: usize = 3;

    fn next(&mut self) {
        self.selected = (self.selected + 1).rem_euclid(Self::VIEW_COUNT);
        self.dirty = true;
    }

    fn prev(&mut self) {
        self.selected = (self.selected as isize - 1).rem_euclid(Self::VIEW_COUNT as isize) as usize;
        self.dirty = true;
    }

    fn view_mut(&mut self) -> &mut dyn View {
        match self.selected {
            0 => &mut self.views.0,
            1 => &mut self.views.1,
            2 => &mut self.views.2,
            _ => unreachable!(),
        }
    }

    fn view(&self) -> &dyn View {
        match self.selected {
            0 => &self.views.0,
            1 => &self.views.1,
            2 => &self.views.2,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct App<B>
where
    B: Battery,
{
    rect: Rect,
    battery_indicator: BatteryIndicator<B>,
    state: State,
    tabs: Row<Label<&'static str>>,
}

impl<B> App<B>
where
    B: Battery,
{
    pub fn new(
        rect: Rect,
        database: Database,
        device_mapper: Rc<DeviceMapper>,
        battery: B,
    ) -> Result<Self> {
        let Rect { x, y, w, h } = rect;

        let mut battery_indicator =
            BatteryIndicator::new(Point::new(w as i32 - 12, y + 8), Alignment::Right);
        battery_indicator.init(battery);

        let mut state = match State::load()? {
            Some(state) => state,
            None => State::new(Rect::new(x, y + 46, w, h - 46)),
        };
        state.init(database, device_mapper)?;

        let mut tabs = Row::new(
            Point::new(x + 12, y + 8),
            vec![
                Label::new(Point::zero(), "Games", Alignment::Left, None),
                Label::new(Point::zero(), "Recents", Alignment::Left, None),
                Label::new(Point::zero(), "Settings", Alignment::Left, None),
            ],
            Alignment::Left,
            12,
        );
        tabs.get_mut(state.selected)
            .unwrap()
            .color(StylesheetColor::Highlight);

        Ok(Self {
            rect,
            battery_indicator,
            state,
            tabs,
        })
    }

    pub fn save(&self) -> Result<()> {
        self.state.save()
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
        let mut drawn = false;
        if self.battery_indicator.should_draw() && self.battery_indicator.draw(display, styles)? {
            drawn = true;
        }

        if self.tabs.should_draw() && self.tabs.draw(display, styles)? {
            drawn = true;
        }

        if self.state.dirty {
            display.load(self.state.view_mut().bounding_box(styles))?;
            self.state.dirty = false;
        }

        if self.state.view().should_draw() && self.state.view_mut().draw(display, styles)? {
            drawn = true;
        }
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.battery_indicator.should_draw()
            || self.state.view().should_draw()
            || self.tabs.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.battery_indicator.set_should_draw();
        self.state.view_mut().set_should_draw();
        self.tabs.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::L) => {
                trace!("switch state prev");
                self.tabs
                    .get_mut(self.state.selected)
                    .unwrap()
                    .color(StylesheetColor::Foreground);
                self.state.prev();
                self.state.view_mut().set_should_draw();
                self.tabs
                    .get_mut(self.state.selected)
                    .unwrap()
                    .color(StylesheetColor::Highlight);
                return Ok(true);
            }
            KeyEvent::Pressed(Key::R) => {
                trace!("switch state next");
                self.tabs
                    .get_mut(self.state.selected)
                    .unwrap()
                    .color(StylesheetColor::Foreground);
                self.state.next();
                self.state.view_mut().set_should_draw();
                self.tabs
                    .get_mut(self.state.selected)
                    .unwrap()
                    .color(StylesheetColor::Highlight);
                return Ok(true);
            }
            _ => {
                self.state
                    .view_mut()
                    .handle_key_event(event, commands, bubble)
                    .await
            }
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.battery_indicator, self.state.view(), &self.tabs]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![
            &mut self.battery_indicator,
            self.state.view_mut(),
            &mut self.tabs,
        ]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
