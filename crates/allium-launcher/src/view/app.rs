use std::collections::VecDeque;
use std::fs::{self, File};
use std::marker::PhantomData;

use anyhow::Result;
use async_trait::async_trait;
use common::battery::Battery;
use common::command::Command;
use common::constants::ALLIUM_LAUNCHER_STATE;
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::{Stylesheet, StylesheetColor};
use common::view::{BatteryIndicator, Clock, Label, Row, SearchView, View};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::view::Recents;
use crate::view::apps::AppsState;
use crate::view::games::GamesState;
use crate::view::recents::RecentsState;
use crate::view::search_results::SearchResultsView;
use crate::view::settings::SettingsState;
use crate::view::{Apps, Games, Settings};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    selected: usize,
    recents: RecentsState,
    games: GamesState,
    apps: AppsState,
    settings: SettingsState,
}

#[derive(Debug)]
pub struct App<B>
where
    B: Battery + 'static,
{
    rect: Rect,
    res: Resources,
    status_bar: Row<Box<dyn View>>,
    views: (Recents, Games, Apps, Settings),
    selected: usize,
    tabs: Row<Label<String>>,
    search_results: Option<SearchResultsView>,
    search_view: SearchView,
    dirty: bool,
    _phantom_battery: PhantomData<B>,
}

impl<B> App<B>
where
    B: Battery + 'static,
{
    pub fn new(
        rect: Rect,
        res: Resources,
        views: (Recents, Games, Apps, Settings),
        selected: usize,
        battery: B,
    ) -> Result<Self> {
        let Rect { x, y, w, h: _h } = rect;
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

        let status_bar: Row<Box<dyn View>> = Row::new(
            Point::new(w as i32 - styles.margin_x, y + styles.margin_y),
            children,
            Alignment::Right,
            styles.margin_y,
        );

        let mut tabs = Row::new(
            Point::new(x + styles.margin_x, y + styles.margin_y),
            {
                let mut tabs = vec![
                    Label::new(
                        Point::zero(),
                        locale.t("tab-recents"),
                        Alignment::Left,
                        None,
                    ),
                    Label::new(Point::zero(), locale.t("tab-games"), Alignment::Left, None),
                    Label::new(Point::zero(), locale.t("tab-apps"), Alignment::Left, None),
                    Label::new(
                        Point::zero(),
                        locale.t("tab-settings"),
                        Alignment::Left,
                        None,
                    ),
                ];
                for tab in tabs.iter_mut() {
                    tab.color(StylesheetColor::Tab);
                    tab.stroke_color(StylesheetColor::TabStroke);
                    tab.font_size(styles.tab_font_size);
                }
                tabs
            },
            Alignment::Left,
            styles.margin_x,
        );
        tabs.get_mut(selected)
            .unwrap()
            .color(StylesheetColor::TabSelected)
            .stroke_color(StylesheetColor::TabSelectedStroke);

        // let mut title = Label::new(
        //     Point::new(x + 24, y + styles.margin_y),
        //     title(&locale, selected),
        //     Alignment::Left,
        //     None,
        // );
        // title.font_size(styles.tab_font_size);

        drop(styles);
        drop(locale);

        Ok(Self {
            rect,
            res: res.clone(),
            views,
            selected,
            status_bar,
            tabs,
            search_results: None,
            search_view: SearchView::new(res),
            // title,
            dirty: true,
            _phantom_battery: PhantomData,
        })
    }

    pub fn load_or_new(rect: Rect, res: Resources, battery: B) -> Result<Self> {
        let tab_rect = {
            let styles = res.get::<Stylesheet>();
            let font_size = (styles.ui_font.size as f32
                * styles.tab_font_size.max(styles.status_bar_font_size))
                as u32;
            Rect::new(
                rect.x,
                rect.y + font_size as i32 + styles.margin_y + styles.margin_y / 2,
                rect.w,
                rect.h - font_size - styles.margin_y as u32 - styles.margin_y as u32 / 2,
            )
        };

        if ALLIUM_LAUNCHER_STATE.exists() {
            let file = File::open(ALLIUM_LAUNCHER_STATE.as_path())?;
            if let Ok(state) = serde_json::from_reader::<_, AppState>(file) {
                let views = (
                    Recents::load_or_new(tab_rect, res.clone(), Some(state.recents))?,
                    Games::load_or_new(tab_rect, res.clone(), Some(state.games)).unwrap_or_else(
                        |_| Games::load_or_new(tab_rect, res.clone(), None).unwrap(),
                    ),
                    Apps::load_or_new(tab_rect, res.clone(), Some(state.apps))?,
                    Settings::new(
                        tab_rect,
                        res.clone(),
                        if state.selected == 3 {
                            // Only load settings if it was the last selected tab
                            state.settings
                        } else {
                            Default::default()
                        },
                    )?,
                );
                return Self::new(rect, res, views, state.selected, battery);
            }
            warn!("failed to deserialize state file, deleting");
            fs::remove_file(ALLIUM_LAUNCHER_STATE.as_path())?;
        }

        let views = (
            Recents::load_or_new(tab_rect, res.clone(), None)?,
            Games::load_or_new(tab_rect, res.clone(), None)?,
            Apps::load_or_new(tab_rect, res.clone(), None)?,
            Settings::new(tab_rect, res.clone(), Default::default())?,
        );
        let selected = 1;
        Self::new(rect, res, views, selected, battery)
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_LAUNCHER_STATE.as_path())?;
        let state = AppState {
            selected: self.selected,
            recents: self.views.0.save(),
            games: self.views.1.save(),
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
            .color(StylesheetColor::Tab)
            .stroke_color(StylesheetColor::TabStroke);
        self.selected = selected;
        self.view_mut().set_should_draw();
        self.set_should_draw();
        self.tabs
            .get_mut(self.selected)
            .unwrap()
            .color(StylesheetColor::TabSelected)
            .stroke_color(StylesheetColor::TabSelectedStroke);
        // self.title.set_text(self.title());
    }

    fn next(&mut self) {
        let selected = (self.selected + 1).rem_euclid(4);
        self.tab_change(selected)
    }

    fn prev(&mut self) {
        let selected = (self.selected as isize - 1).rem_euclid(4);
        self.tab_change(selected as usize)
    }

    pub fn start_search(&mut self) {
        self.search_view.activate();
        self.status_bar.set_should_draw();
    }

    pub fn search(&mut self, query: String) -> Result<()> {
        let search_view = SearchResultsView::new(self.rect, self.res.clone(), query)?;
        self.search_results = Some(search_view);
        Ok(())
    }

    pub fn close_search_results(&mut self) {
        self.search_results = None;
        self.search_view.deactivate();
        self.set_should_draw();
    }

    // fn title(&self) -> String {
    //     title(&self.res.get::<Locale>(), self.selected)
    // }
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

        if self.search_results.is_none() {
            if self.tabs.should_draw() || self.status_bar.should_draw() {
                display.load(
                    self.tabs
                        .bounding_box(styles)
                        .union(&self.status_bar.bounding_box(styles)),
                )?;
                drawn |= self.tabs.should_draw() && self.tabs.draw(display, styles)?;
                drawn |= self.status_bar.should_draw() && self.status_bar.draw(display, styles)?;
            }

            if !self.search_view.is_active() {
                drawn |= self.view().should_draw() && self.view_mut().draw(display, styles)?;
            }
        }

        drawn |= self.search_view.draw(display, styles)?;

        if let Some(search_results) = &mut self.search_results {
            drawn |= search_results.should_draw()
                && search_results.draw(display, styles)?
                && self.status_bar.draw(display, styles)?;
            drawn |= self.status_bar.should_draw() && self.status_bar.draw(display, styles)?;
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.status_bar.should_draw()
            || self.view().should_draw()
            || self.tabs.should_draw()
            || self.search_view.should_draw()
            || self
                .search_results
                .as_ref()
                .is_some_and(|sr| sr.should_draw())
    }

    fn set_should_draw(&mut self) {
        self.dirty = true;
        self.status_bar.set_should_draw();
        self.view_mut().set_should_draw();
        self.tabs.set_should_draw();
        self.search_view.set_should_draw();
        if let Some(search_results) = &mut self.search_results {
            search_results.set_should_draw();
        }
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        if let Some(search_results) = &mut self.search_results
            && search_results
                .handle_key_event(event, commands.clone(), bubble)
                .await?
        {
            let mut close_search = false;
            for cmd in bubble.iter() {
                match cmd {
                    Command::CloseView => {
                        close_search = true;
                    }
                    Command::Search(_) => {}
                    _ => {}
                }
            }
            if close_search {
                self.close_search_results();
            }
            bubble.clear();
            return Ok(true);
        }

        if self.search_view.is_active()
            && self
                .search_view
                .handle_key_event(event, commands.clone(), bubble)
                .await?
        {
            for cmd in bubble.iter() {
                if let Command::Search(query) = cmd {
                    self.search(query.clone())?;
                }
            }
            bubble.clear();
            return Ok(true);
        }

        if self
            .view_mut()
            .handle_key_event(event, commands, bubble)
            .await?
        {
            return Ok(true);
        }
        match event {
            KeyEvent::Pressed(Key::Left) => {
                trace!("switch state prev");
                self.prev();
                Ok(true)
            }
            KeyEvent::Pressed(Key::Right) => {
                trace!("switch state next");
                self.next();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        let mut children: Vec<&dyn View> = vec![&self.status_bar, self.view(), &self.tabs];
        if let Some(search_results) = &self.search_results {
            children.push(search_results);
        }
        children
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        let view: &mut dyn View = match self.selected {
            0 => &mut self.views.0,
            1 => &mut self.views.1,
            2 => &mut self.views.2,
            3 => &mut self.views.3,
            _ => unreachable!(),
        };
        let mut children: Vec<&mut dyn View> = vec![&mut self.status_bar, view, &mut self.tabs];
        if let Some(search_results) = &mut self.search_results {
            children.push(search_results);
        }
        children
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}

// fn title(locale: &Locale, selected: usize) -> String {
//     match selected {
//         0 => locale.t("tab-recents"),
//         1 => locale.t("tab-games"),
//         2 => locale.t("tab-apps"),
//         3 => locale.t("tab-settings"),
//         _ => unreachable!(),
//     }
// }
