use std::{
    collections::{HashSet, VecDeque},
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use common::{constants::ALLIUM_GAMES_DIR, database::Database};
use log::error;
use serde::{Deserialize, Serialize};

use crate::{
    consoles::ConsoleMapper,
    entry::{game::Game, gamelist::GameList, lazy_image::LazyImage, short_name, Entry},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Directory {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    /// image is loaded lazily.
    /// None means image hasn't been looked for, Some(None) means no image was found, Some(Some(path)) means an image was found.
    pub image: LazyImage,
}

impl Ord for Directory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.full_name.cmp(&other.full_name)
    }
}

impl PartialOrd for Directory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for Directory {
    fn default() -> Self {
        Directory {
            name: "Games".to_string(),
            full_name: "Games".to_string(),
            path: ALLIUM_GAMES_DIR.to_owned(),
            image: LazyImage::Unknown(ALLIUM_GAMES_DIR.to_owned()),
        }
    }
}

impl Directory {
    pub fn new(path: PathBuf) -> Directory {
        let full_name = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let name = short_name(&full_name);
        let image = LazyImage::Unknown(path.clone());
        Directory {
            name,
            full_name,
            path,
            image,
        }
    }

    pub fn with_name(path: PathBuf, name: String) -> Directory {
        let full_name = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
            .to_string();
        let image = LazyImage::Unknown(path.clone());
        Directory {
            name,
            full_name,
            path,
            image,
        }
    }

    pub fn image(&mut self) -> Option<&Path> {
        self.image.image()
    }

    fn parse_game_list(&self, game_list: &Path) -> Result<Vec<Entry>> {
        let file = File::open(game_list)?;
        let gamelist: GameList = serde_xml_rs::from_reader(file)?;

        let games = gamelist.games.into_iter().filter_map(|game| {
            let path = self.path.join(&game.path);
            if !path.exists() {
                return None;
            }

            let extension = game
                .path
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_owned();

            let full_name = game.name.clone();

            let image = game.image.or(game.thumbnail);
            let image = match image {
                Some(image) => {
                    let path = self.path.join(image);
                    if path.exists() {
                        LazyImage::Found(path)
                    } else {
                        LazyImage::Unknown(path)
                    }
                }
                None => LazyImage::Unknown(path.clone()),
            };

            Some(Entry::Game(Game {
                path,
                name: game.name,
                full_name,
                image,
                extension,
                core: None,
            }))
        });

        let folders = gamelist.folders.into_iter().filter_map(|folder| {
            let path = self.path.join(&folder.path);
            if !path.exists() {
                return None;
            }

            let name = folder.name;

            Some(Entry::Directory(Directory::with_name(path, name)))
        });

        Ok(folders.chain(games).collect())
    }

    pub fn entries(
        &self,
        database: &Database,
        console_mapper: &ConsoleMapper,
    ) -> Result<Vec<Entry>> {
        let mut entries = vec![];

        let gamelist = self.path.join("gamelist.xml");
        if gamelist.exists() {
            match self.parse_game_list(&gamelist) {
                Ok(res) => entries.extend(res),
                Err(e) => error!("Failed to parse gamelist.xml: {}", e),
            }
        }

        let gamelist = self.path.join("miyoogamelist.xml");
        if gamelist.exists() {
            match self.parse_game_list(&gamelist) {
                Ok(res) => entries.extend(res),
                Err(e) => error!("Failed to parse gamelist.xml: {}", e),
            }
        }

        entries.extend(
            std::fs::read_dir(&self.path)
                .map_err(|e| anyhow!("Failed to open directory: {:?}, {}", &self.path, e))?
                .filter_map(std::result::Result::ok)
                .filter_map(|entry| match Entry::new(entry.path(), console_mapper) {
                    Ok(Some(entry)) => Some(entry),
                    _ => None,
                }),
        );

        let mut uniques = HashSet::new();
        entries.retain(|e| uniques.insert(e.path().to_path_buf()));

        for entry in entries.iter_mut() {
            if let Entry::Game(game) = entry {
                if let Some(core) = database.get_core(&game.path)? {
                    game.core = Some(core);
                }
            }
        }

        Ok(entries)
    }

    /// Populate the database with the games in this directory, pushing any subdirectories onto the
    /// queue.
    pub fn populate_db(
        &self,
        queue: &mut VecDeque<Directory>,
        database: &Database,
        console_mapper: &ConsoleMapper,
    ) -> Result<()> {
        let entries = self.entries(database, console_mapper)?;

        for entry in &entries {
            match entry {
                Entry::Directory(dir) => queue.push_back(dir.clone()),
                Entry::Game(_) | Entry::App(_) => {}
            }
        }

        let games: Vec<_> = entries
            .into_iter()
            .filter_map(|entry| match entry {
                Entry::Game(game) => Some(common::database::NewGame {
                    name: game.name,
                    path: game.path,
                    image: game.image.try_image().map(Path::to_path_buf),
                    core: game.core,
                }),
                _ => None,
            })
            .collect();
        database.update_games(&games)?;

        Ok(())
    }
}

impl From<&Path> for Directory {
    fn from(path: &Path) -> Self {
        Directory::new(path.into())
    }
}
