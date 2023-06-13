#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ exec ./retroarch.sh -v -L ".retroarch/cores/$1_libretro.so" "$2"
