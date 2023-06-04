#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ exec ./retroarch -v -L .retroarch/cores/fake08_libretro.so "$1"