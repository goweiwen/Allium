use std::collections::{HashMap, VecDeque};

use anyhow::Result;
use async_trait::async_trait;
use common::command::Command;
use common::constants::RECENT_GAMES_LIMIT;
use common::database::{Database, Game};
use common::display::Display;
use common::geom::{Alignment, Point, Rect};
use common::locale::Locale;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::{ButtonHint, ButtonHints, Label, SettingsList, View};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct ActivityTracker {
    rect: Rect,
    res: Resources,
    entries: Vec<Game>,
    sort: Sort,
    list: SettingsList,
    button_hints: ButtonHints<String>,
}

impl ActivityTracker {
    pub fn new(rect: Rect, res: Resources) -> Result<Self> {
        let Rect { x, y, w, .. } = rect;

        let styles = res.get::<Stylesheet>();

        let mut button_hints = {
            let locale = res.get::<Locale>();
            ButtonHints::new(
                res.clone(),
                vec![],
                vec![
                    ButtonHint::new(
                        res.clone(),
                        Point::zero(),
                        Key::B,
                        locale.t("button-back"),
                        Alignment::Right,
                    ),
                    ButtonHint::new(
                        res.clone(),
                        Point::zero(),
                        Key::Y,
                        Sort::MostPlayed.button_hint(&locale),
                        Alignment::Right,
                    ),
                ],
            )
        };

        let button_hints_rect = button_hints.bounding_box(&styles);
        let list_height = (button_hints_rect.y - y) as u32;

        let list = SettingsList::new(
            res.clone(),
            Rect::new(
                x + styles.ui.margin_x,
                y,
                w - styles.ui.margin_x as u32 * 2,
                list_height,
            ),
            Vec::new(),
            Vec::new(),
            res.get::<Stylesheet>().ui.ui_font.size + styles.ui.padding_y as u32,
        );

        drop(styles);

        let mut this = Self {
            rect,
            res,
            entries: Vec::new(),
            sort: Sort::MostPlayed,
            list,
            button_hints,
        };

        this.load_entries()?;

        Ok(this)
    }

    fn load_entries(&mut self) -> Result<()> {
        let styles = self.res.get::<Stylesheet>();
        self.entries = match self.sort {
            Sort::LastPlayed => self
                .res
                .get::<Database>()
                .select_last_played(RECENT_GAMES_LIMIT)?,
            Sort::MostPlayed => self
                .res
                .get::<Database>()
                .select_most_played(RECENT_GAMES_LIMIT)?,
        };

        let locale = self.res.get::<Locale>();
        self.list.set_items(
            self.entries.iter().map(|e| e.name.to_string()).collect(),
            self.entries
                .iter()
                .map(|e| {
                    let mut map = HashMap::new();
                    map.insert(
                        "hours_decimal".into(),
                        format!("{:.1}", (e.play_time.num_minutes() as f32 / 60.0)).into(),
                    );
                    map.insert("hours".into(), e.play_time.num_hours().into());
                    map.insert("minutes".into(), (e.play_time.num_minutes() % 60).into());
                    locale.ta("activity-tracker-play-time", &map)
                })
                .map(|s| {
                    Box::new(Label::new(
                        Point::zero(),
                        s,
                        Alignment::Right,
                        Some(self.rect.w / 2 - styles.ui.margin_y as u32),
                    )) as Box<dyn View>
                })
                .collect(),
        );

        Ok(())
    }
}

#[async_trait(?Send)]
impl View for ActivityTracker {
    fn draw(
        &mut self,
        display: &mut <DefaultPlatform as Platform>::Display,
        styles: &Stylesheet,
    ) -> Result<bool> {
        let mut drawn = false;

        drawn |= self.list.should_draw() && self.list.draw(display, styles)?;

        if self.button_hints.should_draw() {
            display.load(Rect::new(
                0,
                display.size().h as i32 - 48,
                display.size().w,
                48,
            ))?;
            self.button_hints.set_should_draw();
            if self.button_hints.draw(display, styles)? {
                drawn = true;
            }
        }

        Ok(drawn)
    }

    fn should_draw(&self) -> bool {
        self.list.should_draw() || self.button_hints.should_draw()
    }

    fn set_should_draw(&mut self) {
        self.list.set_should_draw();
        self.button_hints.set_should_draw();
    }

    async fn handle_key_event(
        &mut self,
        event: KeyEvent,
        commands: Sender<Command>,
        bubble: &mut VecDeque<Command>,
    ) -> Result<bool> {
        match event {
            KeyEvent::Pressed(Key::Y) => {
                self.sort = self.sort.next();
                self.button_hints
                    .right_mut()
                    .get_mut(1)
                    .unwrap()
                    .set_text(self.sort.button_hint(&self.res.get::<Locale>()));
                self.load_entries()?;
                Ok(true)
            }
            KeyEvent::Pressed(Key::B) => {
                commands.send(Command::Exit).await?;
                Ok(true)
            }
            _ => self.list.handle_key_event(event, commands, bubble).await,
        }
    }

    fn children(&self) -> Vec<&dyn View> {
        vec![&self.list, &self.button_hints]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn View> {
        vec![&mut self.list, &mut self.button_hints]
    }

    fn bounding_box(&mut self, _styles: &Stylesheet) -> Rect {
        self.rect
    }

    fn set_position(&mut self, _point: Point) {
        unimplemented!()
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Sort {
    LastPlayed,
    MostPlayed,
}

impl Sort {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            Sort::LastPlayed => locale.t("sort-last-played"),
            Sort::MostPlayed => locale.t("sort-most-played"),
        }
    }

    fn next(self) -> Self {
        match self {
            Sort::LastPlayed => Sort::MostPlayed,
            Sort::MostPlayed => Sort::LastPlayed,
        }
    }
}
