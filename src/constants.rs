use std::time::Duration;

use embedded_graphics::prelude::Size;

pub const ALLIUM_CONFIG_DIR: &str = "/mnt/SDCARD/.allium";
pub const ALLIUM_ROMS_DIR: &str = "/mnt/SDCARD/Roms";

pub const BUTTON_DIAMETER: u32 = 34;
pub const SELECTION_HEIGHT: u32 = 34;
pub const SELECTION_MARGIN: u32 = 8;
pub const IMAGE_SIZE: Size = Size::new(250, 250);
pub const LISTING_SIZE: i32 = 10;
pub const LISTING_JUMP_SIZE: i32 = 5;
pub const BATTERY_UPDATE_INTERVAL: Duration = Duration::from_secs(5);
