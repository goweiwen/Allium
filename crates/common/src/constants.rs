#![allow(unused)]
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lazy_static::lazy_static;

use crate::geom::Size;

pub const ALLIUM_VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref ALLIUM_SD_ROOT: PathBuf = PathBuf::from(
        &env::var("ALLIUM_SD_ROOT").unwrap_or_else(|_| "/mnt/SDCARD/".to_string())
    );
    pub static ref ALLIUM_BASE_DIR: PathBuf = PathBuf::from(
        &env::var("ALLIUM_BASE_DIR").map_or_else(|_| ALLIUM_SD_ROOT.join(".allium"), PathBuf::from)
    );
    pub static ref ALLIUM_GAMES_DIR: PathBuf = PathBuf::from(
        &env::var("ALLIUM_GAMES_DIR").map_or_else(|_| ALLIUM_SD_ROOT.join("Roms"), PathBuf::from)
    );
    pub static ref ALLIUM_APPS_DIR: PathBuf = PathBuf::from(
        &env::var("ALLIUM_APPS_DIR").map_or_else(|_| ALLIUM_SD_ROOT.join("Apps"), PathBuf::from)
    );

    // Folders
    pub static ref ALLIUM_SCRIPTS_DIR: PathBuf = ALLIUM_BASE_DIR.join("scripts");
    pub static ref ALLIUM_TOOLS_DIR: PathBuf = ALLIUM_BASE_DIR.join("tools");
    pub static ref ALLIUM_FONTS_DIR: PathBuf = ALLIUM_BASE_DIR.join("fonts");
    pub static ref ALLIUM_LOCALES_DIR: PathBuf = ALLIUM_BASE_DIR.join("locales");
    pub static ref ALLIUM_IMAGES_DIR: PathBuf = ALLIUM_BASE_DIR.join("images");
    pub static ref ALLIUM_SCREENSHOTS_DIR: PathBuf = ALLIUM_SD_ROOT.join("Saves/CurrentProfile/screenshots");

    // Config
    pub static ref ALLIUM_CONFIG_CONSOLES: PathBuf = ALLIUM_BASE_DIR.join("config/consoles.toml");
    pub static ref ALLIUM_CONFIG_CORES: PathBuf = ALLIUM_BASE_DIR.join("config/cores.toml");

    // State
    pub static ref ALLIUMD_STATE: PathBuf = ALLIUM_BASE_DIR.join("state/alliumd.json");
    pub static ref ALLIUM_LAUNCHER_STATE: PathBuf =
        ALLIUM_BASE_DIR.join("state/allium-launcher.json");
    pub static ref ALLIUM_MENU_STATE: PathBuf =
        ALLIUM_BASE_DIR.join("state/allium-menu.json");
    pub static ref ALLIUM_GAME_INFO: PathBuf = ALLIUM_BASE_DIR.join("state/current_game");
    pub static ref ALLIUM_STYLESHEET: PathBuf = ALLIUM_BASE_DIR.join("state/stylesheet.json");
    pub static ref ALLIUM_DISPLAY_SETTINGS: PathBuf = ALLIUM_BASE_DIR.join("state/display.json");
    pub static ref ALLIUM_LOCALE_SETTINGS: PathBuf = ALLIUM_BASE_DIR.join("state/locale.json");
    pub static ref ALLIUM_POWER_SETTINGS: PathBuf = ALLIUM_BASE_DIR.join("state/power.json");
    pub static ref ALLIUM_WIFI_SETTINGS: PathBuf = ALLIUM_BASE_DIR.join("state/wifi.json");
    pub static ref ALLIUM_TIMEZONE: PathBuf = ALLIUM_BASE_DIR.join("state/timezone");

    // Database
    pub static ref ALLIUM_DATABASE: PathBuf = env::var("ALLIUM_DATABASE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| ALLIUM_SD_ROOT.join("Saves/CurrentProfile/allium.db"));

    // Binaries & Scripts
    pub static ref ALLIUM_LAUNCHER: PathBuf = ALLIUM_BASE_DIR.join("bin/allium-launcher");
    pub static ref ALLIUM_MENU: PathBuf = ALLIUM_BASE_DIR.join("bin/allium-menu");
    pub static ref ALLIUM_RETROARCH: PathBuf = ALLIUM_BASE_DIR.join("cores/retroarch/launch.sh");
}

// Styles
pub const IMAGE_WIDTH: u32 = 250;
pub const SELECTION_MARGIN: u32 = 8;
pub const SAVE_STATE_IMAGE_WIDTH: u32 = 333;

/// After the battery level drops below this threshold, the charging LED will blink at 0.5Hz.
pub const BATTERY_WARNING_THRESHOLD: i32 = 15;
/// After the battery level drops below this threshold, the device will shut down.
pub const BATTERY_SHUTDOWN_THRESHOLD: i32 = 5;

/// The interval at which the battery level is updated.
pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(10);

/// The interval at which the clock is updated.
pub const CLOCK_UPDATE_INTERVAL: Duration = Duration::from_secs(60);

/// How long to wait until the device is considered idle.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// The number of items to jump when pressing left/right in a listing.
pub const LISTING_JUMP_SIZE: i32 = 5;

/// If a key autorepeat is received after this duration, it will be ignored.
pub const MAXIMUM_FRAME_TIME: Duration = Duration::from_millis(100);

/// Maximum number of recent games to retrieve from the database.
pub const RECENT_GAMES_LIMIT: i64 = 100;

/// RetroArch network command interface.
pub const RETROARCH_UDP_SOCKET: &str = "127.0.0.1:55355";

/// Long press duration for the menu button.
pub const LONG_PRESS_DURATION: Duration = Duration::from_millis(1000);
