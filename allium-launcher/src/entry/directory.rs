use std::path::{Path, PathBuf};

use common::constants::ALLIUM_GAMES_DIR;
use serde::{Deserialize, Serialize};

use crate::entry::short_name;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Directory {
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
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
        }
    }
}

impl Directory {
    pub fn new(path: PathBuf) -> Directory {
        let full_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string();
        let name = short_name(&full_name);
        Directory {
            name,
            full_name,
            path,
        }
    }
}

impl From<&Path> for Directory {
    fn from(path: &Path) -> Self {
        Directory::new(path.into())
    }
}
