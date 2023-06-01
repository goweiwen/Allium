#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ ./retroarch -v -L .retroarch/cores/mednafen_supafaust_libretro.so "$1"