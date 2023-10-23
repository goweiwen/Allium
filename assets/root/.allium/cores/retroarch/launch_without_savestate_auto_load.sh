#!/bin/sh
DIR=/mnt/SDCARD/RetroArch
if [ -f "$DIR/.retroarch/retroarch.cfg" ]; then
    cp "$DIR/.retroarch/retroarch.cfg" "/tmp/retroarch.cfg"
    sed -i 's/savestate_auto_load = "true"/savestate_auto_load = "false"/g' "/tmp/retroarch.cfg"
fi
HOME=/mnt/SDCARD/RetroArch LD_PRELOAD=libpadsp.so exec "$DIR/retroarch" -v -L "$DIR/.retroarch/cores/$1_libretro.so" "$2" -c /tmp/retroarch.cfg