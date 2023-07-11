use std::path::{Path, PathBuf};

use common::constants::ALLIUM_GAMES_DIR;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LazyImage {
    /// Path to the file
    Unknown(PathBuf),
    /// Path to the found image
    Found(PathBuf),
    NotFound,
}

impl LazyImage {
    pub fn from_path(path: &Path, image: Option<PathBuf>) -> Self {
        match image {
            Some(image) => Self::Found(image),
            _ => Self::Unknown(path.to_path_buf()),
        }
    }

    /// Searches for the image path, caches it, and returns it
    pub fn image(&mut self) -> Option<&Path> {
        let path = match self {
            Self::Unknown(path) => path,
            Self::Found(path) => return Some(path.as_path()),
            Self::NotFound => return None,
        };

        // Search for Imgs folder upwards, recursively
        let mut parent = path.clone();
        let mut image = None;
        let file_name = path.file_name().unwrap();
        'image: while parent.pop() {
            let mut image_path = parent.join("Imgs");
            if image_path.is_dir() {
                const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "gif"];
                image_path.push(file_name);
                for ext in &IMAGE_EXTENSIONS {
                    image_path.set_extension(ext);
                    println!("{:?}", image_path);
                    if image_path.is_file() {
                        image = Some(image_path);
                        break 'image;
                    }
                }
                image_path.pop();
                image_path.extend(path.strip_prefix(&parent).unwrap());
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
        *self = match image {
            Some(image) => Self::Found(image),
            None => Self::NotFound,
        };

        match self {
            Self::Found(path) => Some(path.as_path()),
            _ => None,
        }
    }

    pub fn try_image(&self) -> Option<&Path> {
        match self {
            Self::Found(path) => Some(path.as_path()),
            _ => None,
        }
    }
}

impl From<PathBuf> for LazyImage {
    fn from(path: PathBuf) -> Self {
        Self::Found(path)
    }
}
