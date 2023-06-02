use std::{env, ffi::OsStr, path::PathBuf};

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

use crate::constants::ALLIUM_ROMS_DIR;

pub struct Launcher {
    roms_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub enum Entry {
    Game(Game),
    Directory(Directory),
}

impl Entry {
    pub fn name(&self) -> &str {
        match self {
            Entry::Game(game) => &game.name,
            Entry::Directory(directory) => &directory.name,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Entry::Game(game) => &game.path,
            Entry::Directory(directory) => &directory.path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub image: Option<PathBuf>,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct Directory {
    pub name: String,
    pub path: PathBuf,
}

const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "bmp"];

impl Entry {
    fn new(path: PathBuf) -> Result<Option<Entry>> {
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

        // Don't add images
        if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            return Ok(None);
        }

        let mut name = match path.file_stem().and_then(OsStr::to_str) {
            Some(name) => name.to_owned(),
            None => return Ok(None),
        };

        // Remove numbers
        lazy_static! {
            static ref NUMBERS_RE: Regex = Regex::new(r"^\d+").unwrap();
        }
        name = NUMBERS_RE.replace(&name, "").to_string();

        // Remove trailing parenthesis
        lazy_static! {
            static ref PARENTHESIS_RE: Regex = Regex::new(r"[\(\[].+[\)\]]$").unwrap();
        }
        name = PARENTHESIS_RE.replace(&name, "").to_string();

        // Trim whitespaces
        name = name.trim().to_owned();

        // Directories without extensions can be navigated into
        if extension.is_empty() && path.is_dir() {
            return Ok(Some(Entry::Directory(Directory { name, path })));
        }

        let image = {
            let path = path.with_extension("jpg");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        };

        Ok(Some(Entry::Game(Game {
            name,
            path,
            image,
            extension,
        })))
    }
}

impl Launcher {
    pub fn new() -> Launcher {
        let roms_dir: PathBuf = env::var("ALLIUM_ROMS_DIR")
            .unwrap_or(ALLIUM_ROMS_DIR.to_owned())
            .into();
        Launcher { roms_dir }
    }

    pub fn entries(&self, directory: Option<&Directory>) -> Result<impl Iterator<Item = Entry>> {
        let dir = directory
            .as_ref()
            .map(|directory| &directory.path)
            .unwrap_or(&self.roms_dir);
        Ok(std::fs::read_dir(dir)?
            .flat_map(|entry| entry.ok())
            .flat_map(|entry| match Entry::new(entry.path()) {
                Ok(Some(game)) => Some(game),
                _ => None,
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_games() -> Result<()> {
        env::set_var("ALLIUM_ROMS_DIR", "./assets/Roms");
        let launcher = Launcher::new();
        for entry in launcher.entries(None)? {
            println!("{:?}", entry);
        }
        Ok(())
    }
}
