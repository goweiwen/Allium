#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ exec ./retroarch.sh -v -L .retroarch/cores/gpsp_libretro.so "$1"