#![allow(unused)]
use std::time::Duration;

use embedded_graphics::prelude::Size;

pub const ALLIUM_CONFIG_DIR: &str = "/mnt/SDCARD/.allium";
pub const ALLIUM_ROMS_DIR: &str = "/mnt/SDCARD/Roms";
pub const RETROARCH_UDP_SOCKET: &str = "127.0.0.1:55355";

pub const ALLIUMD_STATE: &str = "/mnt/SDCARD/.allium/alliumd.json";
pub const ALLIUM_LAUNCHER: &str = "/mnt/SDCARD/.allium/allium-launcher";
pub const ALLIUM_MENU: &str = "/mnt/SDCARD/.allium/allium-menu";
pub const ALLIUM_CORE_ID: &str = "/tmp/allium_core.pid";

pub const BUTTON_DIAMETER: u32 = 34;
pub const SELECTION_HEIGHT: u32 = 34;
pub const SELECTION_MARGIN: u32 = 8;
pub const IMAGE_SIZE: Size = Size::new(250, 376);
pub const LISTING_SIZE: i32 = 10;
pub const LISTING_JUMP_SIZE: i32 = 5;
pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(5);
