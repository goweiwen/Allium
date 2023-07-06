use std::{
    ffi::OsStr,
    fs, mem,
    path::{Path, PathBuf},
};

use anyhow::Result;
use common::constants::ALLIUM_GAMES_DIR;
use log::info;
use serde::{Deserialize, Serialize};

use crate::entry::short_name;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    /// image is loaded lazily.
    /// None means image hasn't been looked for, Some(None) means no image was found, Some(Some(path)) means an image was found.
    pub image: Option<Option<PathBuf>>,
    pub extension: String,
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
        Game {
            name,
            full_name,
            path,
            image: None,
            extension,
        }
    }

    /// Returns the image path if already cached
    pub fn image_ref(&self) -> Option<&Path> {
        if let Some(Some(ref image)) = self.image {
            Some(image)
        } else {
            None
        }
    }

    /// Searches for the image path, caches it, and returns it
    pub fn image(&mut self) -> Option<&Path> {
        // Search for Imgs folder upwards, recursively
        let mut parent = self.path.clone();
        let mut image = None;
        'image: while parent.pop() {
            let mut image_path = parent.join("Imgs");
            if image_path.is_dir() {
                image_path.extend(self.path.strip_prefix(&parent).unwrap());
                const IMAGE_EXTENSIONS: [&str; 7] =
                    ["png", "jpg", "jpeg", "webp", "gif", "tga", "bmp"];
                for ext in &IMAGE_EXTENSIONS {
                    image_path.set_extension(ext);
                    if image_path.is_file() {
                        image = Some(image_path);
                        break 'image;
                    }
                }
            }
            if parent.to_str() == ALLIUM_GAMES_DIR.to_str() {
                break;
            }
        }
        self.image = Some(image);
        self.image_ref()
    }

    /// Attempts to resync the game path with the games directory. Returns the old path if it changed.
    pub fn resync(&mut self) -> Result<Option<PathBuf>> {
        Ok(if self.path.exists() {
            None
        } else if let Some(name) = self.path.file_name() {
            if let Some(game) = find(&ALLIUM_GAMES_DIR, name)? {
                info!("Resynced game path: {:?}", game);
                Some(mem::replace(&mut self.path, game))
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
        self.full_name.cmp(&other.full_name)
    }
}

impl PartialOrd for Game {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
