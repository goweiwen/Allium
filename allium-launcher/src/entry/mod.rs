pub mod app;
pub mod directory;
pub mod game;
mod gamelist;
pub mod lazy_image;

use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use anyhow::Result;
use common::database::Database;
use common::locale::Locale;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::consoles::ConsoleMapper;
use crate::entry::app::App;
use crate::entry::directory::Directory;
use crate::entry::game::Game;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Entry {
    Directory(Directory),
    App(App),
    Game(Game),
}

impl Entry {
    pub fn new(path: PathBuf, console_mapper: &ConsoleMapper) -> Result<Option<Entry>> {
        // Don't add hidden files starting with .
        let file_name = match path.file_name().and_then(OsStr::to_str) {
            Some(file_name) => file_name,
            None => return Ok(None),
        };
        if file_name.starts_with('.') {
            return Ok(None);
        }

        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_owned();

        // Exclude Imgs and Guide directories
        if file_name == "Imgs"
            || file_name == "Guides"
            || file_name == "gamelist.xml"
            || file_name == "miyoogamelist.xml"
        {
            return Ok(None);
        }

        // Exclude DB
        const EXCLUDE_EXTENSIONS: [&str; 1] = ["db"];
        if EXCLUDE_EXTENSIONS.contains(&extension.as_str()) {
            return Ok(None);
        }

        if path.is_dir() {
            // Directories without extensions can be navigated into
            if extension.is_empty() {
                return Ok(Some(Entry::Directory(
                    if let Some(console) = console_mapper.get_console_by_dir(&path) {
                        Directory::with_name(path, console.name.clone())
                    } else {
                        Directory::new(path)
                    },
                )));
            }

            // Apps are directories with .pak extension and have a config.json file inside
            if extension == "pak" && path.join("config.json").exists() {
                return Ok(Some(Entry::App(App::new(path)?)));
            }
        }

        Ok(Some(Entry::Game(Game::new(path))))
    }

    pub fn path<'a>(&'a self) -> &'a Path {
        match self {
            Entry::Game(game) => &game.path,
            Entry::Directory(directory) => &directory.path,
            Entry::App(app) => &app.directory,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Entry::Game(game) => &game.name,
            Entry::Directory(directory) => &directory.name,
            Entry::App(app) => &app.name,
        }
    }

    pub fn image(&mut self) -> Option<&Path> {
        match self {
            Entry::Game(game) => game.image(),
            Entry::Directory(dir) => dir.image(),
            Entry::App(app) => app.image.as_deref(),
        }
    }
}

fn short_name(name: &str) -> String {
    // Remove numbers
    lazy_static! {
        static ref NUMBERS_RE: Regex = Regex::new(r"^\d+[.\)]").unwrap();
    }
    let name = NUMBERS_RE.replace(name, "").to_string();

    // Remove trailing parenthesis
    lazy_static! {
        static ref PARENTHESIS_RE: Regex = Regex::new(r"[\(\[].+[\)\]]$").unwrap();
    }
    let name = PARENTHESIS_RE.replace(&name, "").to_string();

    // Trim whitespaces
    let name = name.trim().to_owned();

    name
}

pub trait Sort: Debug + Clone {
    const HAS_BUTTON_HINTS: bool = true;
    fn button_hint(&self, locale: &Locale) -> String;
    fn next(&self) -> Self;
    fn with_directory(&self, directory: Directory) -> Self;
    fn entries(&self, database: &Database, console_mapper: &ConsoleMapper) -> Result<Vec<Entry>>;
}
