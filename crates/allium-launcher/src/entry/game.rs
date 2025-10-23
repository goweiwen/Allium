use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs, mem,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::NaiveDate;
use common::constants::ALLIUM_GAMES_DIR;
use common::database::{Game as DbGame, NewGame};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    consoles::CoreName,
    entry::{lazy_image::LazyImage, short_name},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    /// Short name of the game, used to display.
    pub name: String,
    /// Full name of the game, used to sort.
    pub full_name: String,
    /// Path to the game file.
    pub path: PathBuf,
    /// Box art
    pub image: LazyImage,
    /// Extension of the game file.
    pub extension: String,
    /// The core to use for this game. If None, the default core will be used.
    pub core: Option<CoreName>,
    /// Rating from 0 to 10, used to sort.
    pub rating: Option<u8>,
    /// Release date of the game, used to sort.
    pub release_date: Option<NaiveDate>,
    /// Developer of the game.
    pub developer: Option<String>,
    /// Publisher of the game.
    pub publisher: Option<String>,
    /// List of genres of the game.
    pub genres: Vec<String>,
    /// Whether the game is marked as a favorite.
    pub favorite: bool,
    /// Path to the save state screenshot.
    pub screenshot_path: Option<PathBuf>,
}

impl Game {
    pub fn new(path: PathBuf) -> Game {
        let full_name = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let name = short_name(&full_name);
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let image = LazyImage::Unknown(path.clone());
        Game {
            name,
            full_name,
            path,
            image,
            extension,
            core: None,
            rating: None,
            release_date: None,
            developer: None,
            publisher: None,
            genres: Vec::new(),
            favorite: false,
            screenshot_path: None,
        }
    }

    pub fn from_db(game: DbGame) -> Game {
        let full_name = game
            .path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let image = match game.image {
            Some(image) => LazyImage::Found(image),
            None => LazyImage::Unknown(game.path.clone()),
        };
        let extension = game
            .path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();

        Game {
            name: game.name,
            full_name,
            path: game.path,
            image,
            extension,
            core: game.core,
            rating: game.rating,
            release_date: game.release_date,
            developer: game.developer,
            publisher: game.publisher,
            genres: game.genres,
            favorite: game.favorite,
            screenshot_path: game.screenshot_path,
        }
    }

    pub fn image(&mut self) -> Option<&Path> {
        self.image.image()
    }

    /// Attempts to resync the game path with the games directory. Returns the old path if it changed.
    pub fn resync(path: &mut PathBuf) -> Result<Option<PathBuf>> {
        Ok(if path.exists() {
            None
        } else if let Some(name) = path.file_name() {
            if let Some(game) = find(&ALLIUM_GAMES_DIR, name)? {
                info!("Resynced game path: {:?}", game);
                Some(mem::replace(path, game))
            } else {
                None
            }
        } else {
            None
        })
    }
}

impl Ord for Game {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cmp = self.name.cmp(&other.name);

        if cmp == Ordering::Equal {
            match (self.extension.as_str(), other.extension.as_str()) {
                ("cue", _) => Ordering::Less,
                (_, "cue") => Ordering::Greater,
                (a, b) => a.cmp(b),
            }
        } else {
            cmp
        }
    }
}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Game> for NewGame {
    fn from(mut game: Game) -> NewGame {
        let image = game.image().map(Path::to_path_buf);
        NewGame {
            name: game.name,
            path: game.path,
            image,
            core: game.core,
            rating: game.rating,
            release_date: game.release_date,
            developer: game.developer,
            publisher: game.publisher,
            genres: game.genres,
            favorite: game.favorite,
        }
    }
}

fn find(path: &Path, name: &OsStr) -> Result<Option<PathBuf>> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let game = find(&path, name)?;
                if game.is_some() {
                    return Ok(game);
                }
            } else if path.file_name() == Some(name) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}
