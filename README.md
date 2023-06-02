# Allium

Allium is a custom launcher for the Miyoo Mini and Miyoo Mini Plus handheld devices, similar to [OnionOS](https://github.com/OnionUI/Onion) and [MiniUI](https://github.com/shauninman/MiniUI).

![Allium's main menu](assets/screenshots/main-menu.png)

## Project Goals

The goal of Allium is to replace MainUI (stock UI) with a faster and more user-friendly UI.

Goals:
- Fast
- Clean UI
- RetroArch (with Netplay)
- Box art
- Support running on both Miyoo Mini and Miyoo Mini Plus without changes

## Features
- Games list (from `/mnt/SDCARD/Roms`)
- Launch RetroArch for supported cores
- Battery indicator

## Todo
- File-system database to cache folder structure
- Save launcher state to file-system (for suspend, resuming after RetroArch quits)
- In-game menu
- Add config for all supported RetroArch cores
- Recents list (sort by frecency)
- Settings page:
    - Button colors
    - Theme color
    - Toggle box art
    - WiFi

## Building

### Miyoo Mini (Plus)

[cross](https://github.com/cross-rs/cross) is used for cross-compilation.

```
cross build --release --target=arm-unknown-linux-gnueabihf
cp ./target/arm-unknown-linux-gnueabihf/release/allium <sdcard>
```

## Development

Allium comes with a simulator that can be used for development. The simulator requires SDL2 to be installed.

```
cargo run --target=x86_64-pc-windows-msvc
cargo run --target=x86_64-unknown-linux-gnu
```