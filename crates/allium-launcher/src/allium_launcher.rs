use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Instant, SystemTime};

use anyhow::Result;
use common::command::Command;
use common::constants::{
    ALLIUM_GAMES_DIR, ALLIUM_SD_ROOT, ALLIUM_THEMES_DIR, ALLIUM_WALLPAPER_CACHE,
    ALLIUM_WALLPAPER_CACHE_KEY,
};
use common::display::color::Color;
use common::geom;
use common::locale::{Locale, LocaleSettings};
use common::resources::Resources;
use common::view::View;
use enum_map::EnumMap;
use image::RgbaImage;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

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
        // run_event_loop paints over the whole frame before drawing
        let display = platform.display_partial()?;
        let battery = platform.battery()?;

        let mut console_mapper = ConsoleMapper::new();
        console_mapper.load_config()?;

        let mut res = TypeMap::new();
        res.insert(Database::new()?);
        res.insert(console_mapper);
        res.insert(Stylesheet::load()?);
        res.insert(Locale::new(&LocaleSettings::load()?.lang));
        res.insert(Into::<geom::Size>::into(display.size()));
        let res = Resources::new(res);

        let view = App::load_or_new(display.bounding_box(), res.clone(), battery)?;

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
                let path = resolve_wallpaper_path(wallpaper);
                if let Err(e) = set_wallpaper(&mut self.display, &path) {
                    error!("Failed to set wallpaper: {}", e);
                }
            }

            self.display.clear(styles.ui.background_color)?;
        }

        self.display.save()?;

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut keys: EnumMap<Key, bool> = EnumMap::default();

        #[cfg(not(feature = "debug-ui"))]
        let mut frame_interval = tokio::time::interval(common::constants::UI_FRAME_INTERVAL);
        #[cfg(feature = "debug-ui")]
        let mut frame_interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));

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
                    self.handle_command(Command::Redraw).await?;
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
                    let _ = cmd.exec();
                }
                #[cfg(not(feature = "miyoo"))]
                {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        let _ = process::Command::new("/bin/sh")
                            .arg("-c")
                            .arg("make simulator bin=allium-menu")
                            .exec();
                    }

                    #[cfg(not(unix))]
                    process::exit(0);
                }
            }
            Command::ReloadStylesheet(styles, needs_relayout) => {
                debug!("reloading stylesheet");

                if let Some(wallpaper) = styles.wallpaper.as_deref() {
                    let path = resolve_wallpaper_path(wallpaper);
                    if let Err(e) = set_wallpaper(&mut self.display, &path) {
                        error!("Failed to set wallpaper: {}", e);
                    }
                }
                self.display.clear(styles.ui.background_color)?;
                self.display.save()?;

                self.res.insert(*styles);
                self.res.insert(Locale::new(&LocaleSettings::load()?.lang));

                if needs_relayout {
                    self.view.save()?;
                    self.view = App::load_or_new(
                        self.display.bounding_box(),
                        self.res.clone(),
                        self.platform.battery()?,
                    )?;
                } else {
                    self.view.set_should_draw();
                }
            }
            Command::SaveDisplaySettings(mut settings) => {
                debug!("saving display settings");
                self.platform.set_display_settings(&mut settings)?;
                settings.save()?;
            }
            Command::SaveLocaleSettings(settings) => {
                debug!("saving locale settings");
                settings.save()?;
                self.res.insert(Locale::new(&settings.lang));
                self.view.save()?;
                self.view = App::load_or_new(
                    self.display.bounding_box(),
                    self.res.clone(),
                    self.platform.battery()?,
                )?;
            }
            Command::Redraw => {
                trace!("redrawing");
                self.display.load(self.display.bounding_box())?;
                self.view.set_should_draw();
            }
            Command::StartSearch => {
                debug!("starting search");
                self.view.start_search();
            }
            Command::Search(query) => {
                debug!("searching");
                self.view.search(query)?;
            }
            Command::Toast(text, duration) => {
                debug!("showing toast: {:?}", text);
                self.toast = Some(Toast::new(text, duration));
            }
            Command::ImageToast(image, text, duration) => {
                debug!("showing image toast: {:?}", text);
                self.toast = Some(Toast::with_image(image, text, duration));
            }
            Command::DismissToast => {
                debug!("dismissing toast");
                self.toast = None;
                self.display.load(self.display.bounding_box())?;
                self.view.set_should_draw();
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
                            "directory".into(),
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
                    self.display.bounding_box(),
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

fn resolve_wallpaper_path(wallpaper: &Path) -> PathBuf {
    // If wallpaper path is absolute, use it as-is
    if wallpaper.is_absolute() {
        return wallpaper.to_path_buf();
    }

    // Load the current theme and check if wallpaper exists in theme directory
    let theme = common::stylesheet::Theme::load();
    let theme_wallpaper = ALLIUM_THEMES_DIR.join(&theme.0).join(wallpaper);
    if theme_wallpaper.exists() {
        return theme_wallpaper;
    }

    // Fall back to SD root
    ALLIUM_SD_ROOT.join(wallpaper)
}

/// The source file and panel size a cached wallpaper was resized for
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct WallpaperCacheKey {
    source: PathBuf,
    len: u64,
    modified: SystemTime,
    width: u32,
    height: u32,
}

impl WallpaperCacheKey {
    fn new(source: &Path, size: geom::Size) -> Result<Self> {
        let meta = fs::metadata(source)?;
        Ok(Self {
            source: source.to_path_buf(),
            len: meta.len(),
            modified: meta.modified()?,
            width: size.w,
            height: size.h,
        })
    }
}

fn load_cached_wallpaper(key: &WallpaperCacheKey) -> Option<RgbaImage> {
    let json = fs::read_to_string(ALLIUM_WALLPAPER_CACHE_KEY.as_path()).ok()?;
    if serde_json::from_str::<WallpaperCacheKey>(&json).ok()? != *key {
        return None;
    }
    let image = ::image::open(ALLIUM_WALLPAPER_CACHE.as_path())
        .ok()?
        .into_rgba8();
    (image.dimensions() == (key.width, key.height)).then_some(image)
}

fn store_cached_wallpaper(key: &WallpaperCacheKey, image: &RgbaImage) -> Result<()> {
    // The key goes first and comes back last, so it only ever matches a frame written in full
    if let Err(e) = fs::remove_file(ALLIUM_WALLPAPER_CACHE_KEY.as_path())
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err(e.into());
    }
    image.save(ALLIUM_WALLPAPER_CACHE.as_path())?;
    fs::write(
        ALLIUM_WALLPAPER_CACHE_KEY.as_path(),
        serde_json::to_string(key)?,
    )?;
    Ok(())
}

fn set_wallpaper(display: &mut impl Display, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let size = display.size();
    let key = WallpaperCacheKey::new(path, size)?;

    let image = match load_cached_wallpaper(&key) {
        Some(image) => image,
        None => {
            let image = ::image::open(path)?
                .resize_to_fill(size.w, size.h, image::imageops::FilterType::Lanczos3)
                .into_rgba8();
            if let Err(e) = store_cached_wallpaper(&key, &image) {
                warn!("Failed to cache wallpaper: {}", e);
            }
            image
        }
    };

    common::display::image::draw_image(
        &mut display.pixmap_mut(),
        &image,
        common::geom::Point::new(0, 0),
    );

    Ok(())
}
