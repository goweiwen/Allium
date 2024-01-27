#!/bin/sh
DIR=/mnt/SDCARD/RetroArch
HOME=/mnt/SDCARD/RetroArch LD_PRELOAD=libpadsp.so exec "$DIR/retroarch" -v -L "$DIR/.retroarch/cores/$1_libretro.so" "$2"
