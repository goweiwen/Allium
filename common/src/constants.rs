#![allow(unused)]
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use embedded_graphics::prelude::Size;
use lazy_static::lazy_static;

pub const ALLIUM_VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref ALLIUM_SD_ROOT: PathBuf = PathBuf::from(
        &env::var("ALLIUM_SD_ROOT").unwrap_or_else(|_| "/mnt/SDCARD/".to_string())
    );
    pub static ref ALLIUM_CONFIG_DIR: PathBuf = PathBuf::from(
        &env::var("ALLIUM_CONFIG_DIR").unwrap_or_else(|_| "/mnt/SDCARD/.allium".to_string())
    );
    pub static ref ALLIUM_GAMES_DIR: PathBuf = PathBuf::from(
        &env::var("ALLIUM_ROMS_DIR").unwrap_or_else(|_| "/mnt/SDCARD/Roms".to_string())
    );
    pub static ref ALLIUM_SCRIPTS_DIR: PathBuf = ALLIUM_CONFIG_DIR.join("scripts");
    pub static ref ALLIUM_TOOLS_DIR: PathBuf = ALLIUM_CONFIG_DIR.join("tools");

    // Config
    pub static ref ALLIUM_CONFIG_CONSOLES: PathBuf = ALLIUM_CONFIG_DIR.join("config/consoles.toml");

    // State
    pub static ref ALLIUMD_STATE: PathBuf = ALLIUM_CONFIG_DIR.join("state/alliumd.json");
    pub static ref ALLIUM_DATABASE: PathBuf = ALLIUM_CONFIG_DIR.join("state/allium.db");
    pub static ref ALLIUM_LAUNCHER_STATE: PathBuf =
        ALLIUM_CONFIG_DIR.join("state/allium-launcher.json");
    pub static ref ALLIUM_MENU_STATE: PathBuf =
        ALLIUM_CONFIG_DIR.join("state/allium-menu.json");
    pub static ref ALLIUM_GAME_INFO: PathBuf = ALLIUM_CONFIG_DIR.join("state/current_game");
    pub static ref ALLIUM_STYLESHEET: PathBuf = ALLIUM_CONFIG_DIR.join("state/stylesheet.json");
    pub static ref ALLIUM_DISPLAY_SETTINGS: PathBuf = ALLIUM_CONFIG_DIR.join("state/display.json");
    pub static ref ALLIUM_WIFI_SETTINGS: PathBuf = ALLIUM_CONFIG_DIR.join("state/wifi.json");

    // Binaries & Scripts
    pub static ref ALLIUM_LAUNCHER: PathBuf = env::var("ALLIUM_LAUNCHER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ALLIUM_CONFIG_DIR.join("allium-launcher"));
    pub static ref ALLIUM_MENU: PathBuf = env::var("ALLIUM_MENU")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ALLIUM_CONFIG_DIR.join("allium-menu"));
    pub static ref ALLIUM_RETROARCH: PathBuf = env::var("ALLIUM_RETROARCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ALLIUM_CONFIG_DIR.join("cores/retroarch/launch.sh"));
}
pub const RETROARCH_UDP_SOCKET: &str = "127.0.0.1:55355";

pub const MAXIMUM_FRAME_TIME: Duration = Duration::from_millis(100);
pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(5);
pub const CLOCK_UPDATE_INTERVAL: Duration = Duration::from_secs(1);
pub const BUTTON_DIAMETER: u32 = 31;
pub const IMAGE_SIZE: Size = Size::new(250, 376);
pub const LISTING_JUMP_SIZE: i32 = 5;
pub const LISTING_SIZE: i32 = 10;
pub const RECENT_GAMES_LIMIT: i64 = 100;
pub const SELECTION_HEIGHT: u32 = 42;
