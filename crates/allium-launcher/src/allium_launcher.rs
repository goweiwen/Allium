use std::collections::VecDeque;
use std::path::Path;
use std::process;
use std::time::Instant;

use anyhow::Result;
use common::command::Command;
use common::constants::{ALLIUM_GAMES_DIR, ALLIUM_SD_ROOT};
use common::display::color::Color;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::resources::Resources;
use common::view::View;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::prelude::*;
use enum_map::EnumMap;
use log::{error, info, trace, warn};

use common::database::Database;
use common::display::Display;
use common::platform::{DefaultPlatform, Key, KeyEvent, Platform};
use common::stylesheet::Stylesheet;
use type_map::TypeMap;

use crate::consoles::ConsoleMapper;
use crate::entry::directory::Directory;
use crate::entry::game::Game;
use crate::view::{App, Toast};

#[derive(Debug)]
pub struct AlliumLauncher<P: Platform> {
    platform: P,
    display: P::Display,
    res: Resources,
    view: App<P::Battery>,
    toast: Option<Toast>,
}

impl AlliumLauncher<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let display = platform.display()?;
        let battery = platform.battery()?;

        let mut console_mapper = ConsoleMapper::new();
        console_mapper.load_config()?;

        let mut res = TypeMap::new();
        res.insert(Database::new()?);
        res.insert(console_mapper);
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        let display_size = Into::<geom::Size>::into(display.size());
        res.insert(geom::SupportedResolution::from_size(display_size));
        let res = Resources::new(res);

        let view = App::load_or_new(display.bounding_box().into(), res.clone(), battery)?;

        Ok(AlliumLauncher {
            platform,
            display,
            res,
            view,
            toast: None,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        {
            let styles = self.res.get::<Stylesheet>();

            if let Some(wallpaper) = styles.wallpaper.as_deref() {
                let path = ALLIUM_SD_ROOT.join(wallpaper);
                if let Err(e) = set_wallpaper(&mut self.display, &path) {
                    error!("Failed to set wallpaper: {}", e);
                }
            }

            self.display.clear(styles.background_color)?;
        }

        self.display.save()?;

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut keys: EnumMap<Key, bool> = EnumMap::default();

        let mut frame_interval = tokio::time::interval(tokio::time::Duration::from_micros(166_667));

        let mut last_frame = Instant::now();
        loop {
            let dt = last_frame.elapsed();
            self.view.update(dt);
            last_frame = Instant::now();

            let mut drawn = self.view.should_draw()
                && self
                    .view
                    .draw(&mut self.display, &self.res.get::<Stylesheet>())?;

            if let Some(toast) = self.toast.as_mut() {
                if toast.has_expired() {
                    self.toast = None;
                } else {
                    drawn |= toast.draw(&mut self.display, &self.res.get::<Stylesheet>())?;
                }
            }

            if drawn {
                self.display.flush()?;
            }

            #[cfg(unix)]
            tokio::select! {
                _ = frame_interval.tick() => {}
                _ = sigterm.recv() => {
                    self.handle_command(Command::Exit).await?;
                }
                cmd = rx.recv() => {
                    if let Some(cmd) = cmd {
                        self.handle_command(cmd).await?;
                    }
                }
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    match event {
                        KeyEvent::Pressed(key) => {
                            keys[key] = true;
                        }
                        KeyEvent::Released(key) => {
                            keys[key] = false;
                        }
                        KeyEvent::Autorepeat(_) => {}
                    }

                    // Ignore menu key presses
                    if !keys[Key::Menu] && !matches!(event, KeyEvent::Released(Key::Menu)) {
                        self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                    }
                }
                else => {}
            }

            #[cfg(not(unix))]
            tokio::select! {
                event = self.platform.poll() => {
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
                else => {}
            }
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Exit => {
                info!("goodbye from allium launcher");
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                process::exit(0);
            }
            #[allow(unused_mut)]
            Command::Exec(mut cmd) => {
                info!("executing command: {:?}", cmd);
                self.view.save()?;
                self.display.clear(Color::new(0, 0, 0))?;
                self.display.flush()?;
                #[cfg(feature = "miyoo")]
                {
                    use std::os::unix::process::CommandExt;
                    cmd.exec();
                }
                #[cfg(not(feature = "miyoo"))]
                {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        process::Command::new("/bin/sh")
                            .arg("-c")
                            .arg("make simulator-menu")
                            .exec();
                    }

                    #[cfg(not(unix))]
                    process::exit(0);
                }
            }
            Command::SaveStylesheet(mut styles) => {
                trace!("saving stylesheet");
                styles.load_fonts()?;
                styles.save()?;

                {
                    let old_styles = self.res.get::<Stylesheet>();
                    if old_styles.wallpaper != styles.wallpaper
                        || old_styles.background_color != styles.background_color
                    {
                        if let Some(wallpaper) = styles.wallpaper.as_deref() {
                            let path = ALLIUM_SD_ROOT.join(wallpaper);
                            if let Err(e) = set_wallpaper(&mut self.display, &path) {
                                error!("Failed to set wallpaper: {}", e);
                            }
                        }
                        self.display.clear(styles.background_color)?;
                        self.display.save()?;
                    }
                }

                self.res.insert(*styles);
                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box().into(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
            }
            Command::SaveDisplaySettings(mut settings) => {
                trace!("saving display settings");
                self.platform.set_display_settings(&mut settings)?;
                settings.save()?;
            }
            Command::SaveLocaleSettings(settings) => {
                trace!("saving locale settings");
                settings.save()?;
                self.res.insert(Locale::new(&settings.lang));
                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box().into(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
            }
            Command::Redraw => {
                trace!("redrawing");
                self.display.load(self.display.bounding_box().into())?;
                self.view.set_should_draw();
            }
            Command::StartSearch => {
                trace!("starting search");
                self.view.start_search();
            }
            Command::Search(query) => {
                trace!("searching");
                self.view.search(query)?;
            }
            Command::Toast(text, duration) => {
                trace!("showing toast: {:?}", text);
                self.toast = Some(Toast::new(text, duration));
            }
            Command::PopulateDb => {
                #[cfg(feature = "miyoo")]
                {
                    std::process::Command::new("show")
                        .arg("--clear")
                        .spawn()?
                        .wait()?;
                    std::process::Command::new("say")
                        .arg(self.res.get::<Locale>().t("populating-database"))
                        .spawn()?
                        .wait()?;
                }

                let mut queue = VecDeque::with_capacity(10);
                queue.push_back(Directory::new(ALLIUM_GAMES_DIR.clone()));

                let database = self.res.get::<Database>();
                let console_mapper = self.res.get::<ConsoleMapper>();

                database.delete_all_directories()?;
                database.delete_all_unplayed_games()?;

                let mut games = database.select_all_games()?;
                for game in games.iter_mut() {
                    if let Some(old) = Game::resync(&mut game.path)? {
                        if let Err(e) = database.update_game_path(&old, &game.path) {
                            warn!("failed to update game path: {}", e);
                        }
                    } else if !game.path.exists() {
                        database.delete_game(&game.path)?;
                    }
                }

                while let Some(dir) = queue.pop_front() {
                    #[cfg(feature = "miyoo")]
                    {
                        std::process::Command::new("show")
                            .arg("--clear")
                            .spawn()?
                            .wait()?;
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "directory".to_string(),
                            dir.path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into(),
                        );
                        std::process::Command::new("say")
                            .arg(self.res.get::<Locale>().ta("populating-games", &map))
                            .spawn()?
                            .wait()?;
                    }
                    dir.populate_db(&mut queue, &database, &console_mapper, &self.res.get())?;
                }

                database.set_has_indexed(true)?;

                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box().into(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
            }
            command => {
                warn!("unhandled command: {:?}", command);
            }
        }
        Ok(())
    }
}

fn set_wallpaper(display: &mut impl Display, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let rect = display.bounding_box().size;

    let image = ::image::open(path)?;
    let image = image.resize_to_fill(
        rect.width,
        rect.height,
        image::imageops::FilterType::Lanczos3,
    );
    let image = image.into_rgba8();
    let image: ImageRaw<'_, Color> = ImageRaw::new(&image, rect.width);
    let image = embedded_graphics::image::Image::new(&image, display.bounding_box().top_left);
    image.draw(display)?;
    Ok(())
}
