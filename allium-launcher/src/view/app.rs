use std::collections::VecDeque;
use std::fs::{self, File};

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::{ALLIUM_LAUNCHER_STATE, ALLIUM_SD_ROOT};
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{BatteryIndicator, Label, Row, View};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::view::browser::BrowserState;
use crate::view::settings::SettingsState;
use crate::view::Recents;
use crate::view::{Browser, Settings};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    selected: usize,
    games: BrowserState,
    apps: BrowserState,
    settings: SettingsState,
}

#[derive(Debug)]
pub struct App<B>
where
    B: Battery + 'static,
{
    rect: Rect,
    battery_indicator: BatteryIndicator<B>,
    views: (Browser, Recents, Browser, Settings),
    selected: usize,
    tabs: Row<Label<String>>,
    dirty: bool,
}

impl<B> App<B>
where
    B: Battery + 'static,
{
    pub fn new(
        rect: Rect,
        res: Resources,
        views: (Browser, Recents, Browser, Settings),
        selected: usize,
        battery: B,
    ) -> Result<Self> {
        let Rect { x, y, w, h: _h } = rect;

        let battery_indicator = BatteryIndicator::new(Point::new(w as i32 - 12, y + 4), battery);

        let mut tabs = Row::new(
            Point::new(x + 12, y + 8),
            {
                let locale = res.get::<Locale>();
                vec![
                    Label::new(Point::zero(), locale.t("tab-games"), Alignment::Left, None),
                    Label::new(
                        Point::zero(),
                        locale.t("tab-recents"),
                        Alignment::Left,
                        None,
                    ),
                    Label::new(Point::zero(), locale.t("tab-apps"), Alignment::Left, None),
                    Label::new(
                        Point::zero(),
                        locale.t("tab-settings"),
                        Alignment::Left,
                        None,
                    ),
                ]
            },
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

    pub fn load_or_new(rect: Rect, res: Resources, battery: B) -> Result<Self> {
        let tab_rect = {
            let styles = res.get::<Stylesheet>();
            Rect::new(
                rect.x,
                rect.y + styles.ui_font.size as i32 + 8,
                rect.w,
                rect.h - styles.ui_font.size - 8,
            )
        };

        if ALLIUM_LAUNCHER_STATE.exists() {
            let file = File::open(ALLIUM_LAUNCHER_STATE.as_path())?;
            if let Ok(state) = serde_json::from_reader::<_, AppState>(file) {
                let views = (
                    Browser::load(tab_rect, res.clone(), state.games)?,
                    Recents::new(tab_rect, res.clone())?,
                    Browser::load(tab_rect, res.clone(), state.apps)?,
                    Settings::new(tab_rect, res.clone(), state.settings)?,
                );
                return Self::new(rect, res, views, state.selected, battery);
            }
            warn!("failed to deserialize state file, deleting");
            fs::remove_file(ALLIUM_LAUNCHER_STATE.as_path())?;
        }

        let views = (
            Browser::new(
                tab_rect,
                res.clone(),
                ALLIUM_SD_ROOT.join("Roms").as_path().into(),
                0,
            )?,
            Recents::new(tab_rect, res.clone())?,
            Browser::new(
                tab_rect,
                res.clone(),
                ALLIUM_SD_ROOT.join("Apps").as_path().into(),
                0,
            )?,
            Settings::new(tab_rect, res.clone(), Default::default())?,
        );
        let selected = 0;
        Self::new(rect, res, views, selected, battery)
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_LAUNCHER_STATE.as_path())?;
        let state = AppState {
            selected: self.selected,
            games: self.views.0.save(),
            apps: self.views.2.save(),
            settings: self.views.3.save(),
        };
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    fn view(&self) -> &dyn View {
        match self.selected {
            0 => &self.views.0,
            1 => &self.views.1,
            2 => &self.views.2,
            3 => &self.views.3,
            _ => unreachable!(),
        }
    }

    fn view_mut(&mut self) -> &mut dyn View {
        match self.selected {
            0 => &mut self.views.0,
            1 => &mut self.views.1,
            2 => &mut self.views.2,
            3 => &mut self.views.3,
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
        self.set_should_draw();
        self.tabs
            .get_mut(self.selected)
            .unwrap()
            .color(StylesheetColor::Highlight);
    }

    fn next(&mut self) {
        let selected = (self.selected + 1).rem_euclid(4);
        self.tab_change(selected)
    }

    fn prev(&mut self) {
        let selected = (self.selected as isize - 1).rem_euclid(4);
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
        if self.dirty {
            display.load(self.bounding_box(styles))?;
            self.dirty = false;
        }

        let mut drawn = false;
        if self.battery_indicator.should_draw() && self.battery_indicator.draw(display, styles)? {
            drawn = true;
        }

        if self.tabs.should_draw() && self.tabs.draw(display, styles)? {
            drawn = true;
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
        self.dirty = true;
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
            3 => &mut self.views.3,
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
