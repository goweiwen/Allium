# Allium

Allium is a custom launcher for the Miyoo Mini and Miyoo Mini Plus handheld devices, similar to [OnionOS](https://github.com/OnionUI/Onion) and [MiniUI](https://github.com/shauninman/MiniUI).

![Allium's main menu](assets/screenshots/main-menu.png)
![Allium's ingame menu](assets/screenshots/ingame-menu.png)

## Project Goals

The goal of Allium is to replace MainUI (stock UI) with a faster and more user-friendly UI.

### Goals
- It just works
- Fast
- Clean UI
- RetroArch (with Netplay)
- Box art
- Support running on both Miyoo Mini and Miyoo Mini Plus without changes

## Installation

Download the latest release and extract into your SD card. e.g. `E:/`.

The SD card layout should look like this:
- .allium
- .tmp_update
- BIOS
- RetroArch
- Roms
- Saves

## Features

### Done
- Supports stock SD card layout without configuration
- Box art (PNG, JPG, GIF, TGA, BMP)
- Launch RetroArch for all supported cores
- Battery indicator
- Volume control
- In-game menu (view game name, battery %, save, load, reset, access RetroArch menu, quit)
- Automatic game save/resume when powering off/on

### Todo
(roughly in order of priority)
- Settings page:
    - Button colors
    - Theme color
    - Toggle box art
    - WiFi
    - Clock
- Recents list (sort by frecency)
- Brightness control
- File-system database to cache folder structure
- UI improvements:
    - Folder icon
    - Battery icon
    - Volume indicator
    - Brightness indicator
    - Error toast (e.g. no core found for game)
    - Anti-aliased circles
- WiFi stuff:
    - NTP
    - OTA update
    - Metadata/box art scraper
    - Cloud save sync
    - Seamless netplay from ingame menu

### Known bugs
- Volume resets when RetroArch launches
- Battery indicator draws over previous value

## Development

Allium comes with a simulator that can be used for development. The simulator requires SDL2 to be installed.

### Requirements
1. `make`, `cargo`
2. [SDL2](https://github.com/Rust-SDL2/rust-sdl2#sdl20-development-libraries) (optional, if simulator is not used)
3. [cross](https://github.com/cross-rs/cross): `cargo install cross --git https://github.com/cross-rs/cross` (optional, for cross-compilation)

### Simulator
```
# Run main menu (allium-launcher)
make simulator-launcher

# Run ingame menu (allium-menu)
make simulator-menu
```

### Building

Running `make` will build Allium and RetroArch, then copy the built and static files into `dist/`.
```
make all
cp -r dist/. <sdcard>
```
