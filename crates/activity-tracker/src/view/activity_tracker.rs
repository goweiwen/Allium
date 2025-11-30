use std::collections::{HashMap, VecDeque};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local, TimeZone, Utc};
use common::command::Command;
use common::constants::RECENT_GAMES_LIMIT;
use common::database::{Database, Game, GameSession};
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
    sessions: Vec<GameSession>,
    view_mode: ViewMode,
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
                        ViewMode::MostPlayed.button_hint(&locale),
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
            sessions: Vec::new(),
            view_mode: ViewMode::MostPlayed,
            list,
            button_hints,
        };

        this.load_entries()?;

        Ok(this)
    }

    fn load_entries(&mut self) -> Result<()> {
        let styles = self.res.get::<Stylesheet>();

        match self.view_mode {
            ViewMode::LastPlayed => {
                self.entries = self
                    .res
                    .get::<Database>()
                    .select_last_played(RECENT_GAMES_LIMIT)?;

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
            }
            ViewMode::MostPlayed => {
                self.entries = self
                    .res
                    .get::<Database>()
                    .select_most_played(RECENT_GAMES_LIMIT)?;

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
            }
            ViewMode::SessionHistory => {
                self.sessions = self
                    .res
                    .get::<Database>()
                    .select_sessions_by_day(RECENT_GAMES_LIMIT, 0)?;

                let (names, durations) = self.format_sessions_with_dates();
                self.list.set_items(names, durations);
            }
        }

        Ok(())
    }

    fn format_sessions_with_dates(&self) -> (Vec<String>, Vec<Box<dyn View>>) {
        let styles = self.res.get::<Stylesheet>();
        let mut names = Vec::new();
        let mut durations = Vec::new();
        let mut last_date: Option<String> = None;

        for session in &self.sessions {
            // Convert Unix timestamp to DateTime
            let datetime = Utc.timestamp_opt(session.start_time, 0).unwrap();
            let local_time: DateTime<Local> = datetime.into();
            let date = local_time.format("%Y-%m-%d").to_string();

            // Add date header if it's a new day
            if last_date.as_ref() != Some(&date) {
                let date_label = self.format_date_header(&local_time);
                names.push(format!("─── {} ───", date_label));
                durations.push(Box::new(Label::new(
                    Point::zero(),
                    String::new(),
                    Alignment::Right,
                    Some(self.rect.w / 2 - styles.ui.margin_y as u32),
                )) as Box<dyn View>);
                last_date = Some(date);
            }

            // Add session entry
            names.push(format!("  {}", session.game_name));

            // Format duration
            let hours = session.duration / 3600;
            let minutes = (session.duration % 3600) / 60;
            let duration_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            durations.push(Box::new(Label::new(
                Point::zero(),
                duration_str,
                Alignment::Right,
                Some(self.rect.w / 2 - styles.ui.margin_y as u32),
            )) as Box<dyn View>);
        }

        (names, durations)
    }

    fn format_date_header(&self, date: &DateTime<Local>) -> String {
        let today = Local::now().date_naive();
        let session_date = date.date_naive();

        if session_date == today {
            "Today".to_string()
        } else if session_date == today - chrono::Days::new(1) {
            "Yesterday".to_string()
        } else {
            date.format("%b %d, %Y").to_string()
        }
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
                self.view_mode = self.view_mode.next();
                self.button_hints
                    .right_mut()
                    .get_mut(1)
                    .unwrap()
                    .set_text(self.view_mode.button_hint(&self.res.get::<Locale>()));
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
enum ViewMode {
    LastPlayed,
    MostPlayed,
    SessionHistory,
}

impl ViewMode {
    fn button_hint(&self, locale: &Locale) -> String {
        match self {
            ViewMode::LastPlayed => locale.t("sort-last-played"),
            ViewMode::MostPlayed => locale.t("sort-most-played"),
            ViewMode::SessionHistory => "Session History".to_string(), // TODO: localize
        }
    }

    fn next(self) -> Self {
        match self {
            ViewMode::LastPlayed => ViewMode::MostPlayed,
            ViewMode::MostPlayed => ViewMode::SessionHistory,
            ViewMode::SessionHistory => ViewMode::LastPlayed,
        }
    }
}
