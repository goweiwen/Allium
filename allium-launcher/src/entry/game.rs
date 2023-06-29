use std::path::{Path, PathBuf};

use common::constants::ALLIUM_GAMES_DIR;
use serde::{Deserialize, Serialize};

use crate::entry::short_name;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Game {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    /// image is loaded lazily.
    /// None means image hasn't been looked for, Some(None) means no image was found, Some(Some(path)) means an image was found.
    image: Option<Option<PathBuf>>,
    pub extension: String,
}

impl Game {
    pub fn new(path: PathBuf) -> Game {
        let full_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string();
        let name = short_name(&full_name);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
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
