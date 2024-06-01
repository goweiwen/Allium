use std::collections::VecDeque;

use anyhow::Result;
use async_trait::async_trait;

use common::command::Command;
use common::constants::ALLIUM_APPS_DIR;
use common::database::Database;
use common::geom::{Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::{Entry, Sort};
use crate::view::entry_list::{EntryList, EntryListState};

pub type AppsState = EntryListState<AppsSort>;

#[derive(Debug)]
pub struct Apps {
    rect: Rect,
    list: EntryList<AppsSort>,
}

impl Apps {
    pub fn new(rect: Rect, _res: Resources, list: EntryList<AppsSort>) -> Result<Self> {
        Ok(Self { rect, list })
    }

    pub fn load_or_new(rect: Rect, res: Resources, state: Option<AppsState>) -> Result<Self> {
        let list = if let Some(state) = state {
            let selected = state.selected;
            let mut list = EntryList::load(rect, res.clone(), state)?;
            list.select(selected);
            list
        } else {
            EntryList::new(
                rect,
                res.clone(),
                AppsSort::Alphabetical(Directory::new(ALLIUM_APPS_DIR.clone())),
            )?
        };

        Self::new(rect, res, list)
    }

    pub fn save(&self) -> AppsState {
        self.list.save()
    }
}

#[async_trait(?Send)]
impl View for Apps {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        self.list.handle_key_event(event, commands, bubble).await
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppsSort {
    Alphabetical(Directory),
}

impl AppsSort {
    pub fn directory(&self) -> &Directory {
        match self {
            AppsSort::Alphabetical(d) => d,
        }
    }
}

impl Sort for AppsSort {
    const HAS_BUTTON_HINTS: bool = false;

    fn button_hint(&self, _locale: &Locale) -> String {
        match self {
            AppsSort::Alphabetical(_) => String::new(),
        }
    }

    fn next(&self) -> Self {
        match self {
            AppsSort::Alphabetical(d) => AppsSort::Alphabetical(d.clone()),
        }
    }

    fn with_directory(&self, directory: Directory) -> Self {
        match self {
            AppsSort::Alphabetical(_) => AppsSort::Alphabetical(directory),
        }
    }

    fn entries(
        &self,
        database: &Database,
        console_mapper: &ConsoleMapper,
        locale: &Locale,
    ) -> Result<Vec<Entry>> {
        let mut entries = self.directory().entries(database, console_mapper, locale)?;
        entries.sort_unstable();
        Ok(entries)
    }

    fn preserve_selection(&self) -> bool {
        false
    }
}
