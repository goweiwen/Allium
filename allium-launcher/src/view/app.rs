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
use crate::view::browser::BrowserState;
use crate::view::Recents;
use crate::view::{Browser, Settings};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    selected: usize,
    browser: BrowserState,
}

#[derive(Debug)]
pub struct App<B>
where
    B: Battery,
{
    rect: Rect,
    battery_indicator: BatteryIndicator<B>,
    views: (Browser, Recents, Settings),
    selected: usize,
    tabs: Row<Label<&'static str>>,
    dirty: bool,
}

impl<B> App<B>
where
    B: Battery,
{
    pub fn new(
        rect: Rect,
        mut views: (Browser, Recents, Settings),
        selected: usize,
        database: Database,
        device_mapper: Rc<DeviceMapper>,
        battery: B,
    ) -> Result<Self> {
        let Rect { x, y, w, h: _h } = rect;

        views.0.init(database.clone(), device_mapper.clone());
        views.1.init(database, device_mapper)?;

        let mut battery_indicator =
            BatteryIndicator::new(Point::new(w as i32 - 12, y + 8), Alignment::Right);
        battery_indicator.init(battery);

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
        tabs.get_mut(selected)
            .unwrap()
            .color(StylesheetColor::Highlight);

        Ok(Self {
            rect,
            views,
            selected,
            battery_indicator,
            tabs,
            dirty: true,
        })
    }

    pub fn load_or_new(
        rect: Rect,
        database: Database,
        device_mapper: Rc<DeviceMapper>,
        battery: B,
    ) -> Result<Self> {
        let tab_rect = Rect::new(rect.x, rect.y + 46, rect.w, rect.h - 46);

        if ALLIUM_LAUNCHER_STATE.exists() {
            let file = File::open(ALLIUM_LAUNCHER_STATE.as_path())?;
            if let Ok(state) = serde_json::from_reader::<_, AppState>(file) {
                let views = (
                    Browser::load(tab_rect, state.browser)?,
                    Recents::new(tab_rect)?,
                    Settings::new(tab_rect)?,
                );
                return Self::new(
                    rect,
                    views,
                    state.selected,
                    database,
                    device_mapper,
                    battery,
                );
            }
            warn!("failed to deserialize state file, deleting");
            fs::remove_file(ALLIUM_LAUNCHER_STATE.as_path())?;
        }

        let views = (
            Browser::new(tab_rect, Default::default(), 0)?,
            Recents::new(tab_rect)?,
            Settings::new(rect)?,
        );
        let selected = 0;
        Self::new(rect, views, selected, database, device_mapper, battery)
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_LAUNCHER_STATE.as_path())?;
        let state = AppState {
            selected: self.selected,
            browser: self.views.0.save(),
        };
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    fn view(&self) -> &dyn View {
        match self.selected {
            0 => &self.views.0,
            1 => &self.views.1,
            2 => &self.views.2,
            _ => unreachable!(),
        }
    }

    fn view_mut(&mut self) -> &mut dyn View {
        match self.selected {
            0 => &mut self.views.0,
            1 => &mut self.views.1,
            2 => &mut self.views.2,
            _ => unreachable!(),
        }
    }

    fn tab_change(&mut self, selected: usize) {
        self.tabs
            .get_mut(self.selected)
            .unwrap()
            .color(StylesheetColor::Foreground);
        self.selected = selected;
        self.view_mut().set_should_draw();
        self.dirty = true;
        self.tabs
            .get_mut(self.selected)
            .unwrap()
            .color(StylesheetColor::Highlight);
    }

    fn next(&mut self) {
        let selected = (self.selected + 1).rem_euclid(3);
        self.tab_change(selected)
    }

    fn prev(&mut self) {
        let selected = (self.selected as isize - 1).rem_euclid(3);
        self.tab_change(selected as usize)
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

        if self.dirty {
            display.load(self.view_mut().bounding_box(styles))?;
            self.dirty = false;
        }

        if self.view().should_draw() && self.view_mut().draw(display, styles)? {
            drawn = true;
        }
        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.battery_indicator.should_draw() || self.view().should_draw() || self.tabs.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.battery_indicator.set_should_draw();
        self.view_mut().set_should_draw();
        self.tabs.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if self
            .view_mut()
            .handle_key_event(event, commands, bubble)
            .await?
        {
            return Ok(true);
        }

        match event {
            KeyEvent::Pressed(Key::L) => {
                trace!("switch state prev");
                self.prev();
                Ok(true)
            }
            KeyEvent::Pressed(Key::R) => {
                trace!("switch state next");
                self.next();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.battery_indicator, self.view(), &self.tabs]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        let view: &mut dyn View = match self.selected {
            0 => &mut self.views.0,
            1 => &mut self.views.1,
            2 => &mut self.views.2,
            _ => unreachable!(),
        };
        vec![&mut self.battery_indicator, view, &mut self.tabs]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
