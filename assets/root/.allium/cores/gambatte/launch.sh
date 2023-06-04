#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ exec ./retroarch -v -L .retroarch/cores/gambatte_libretro.so "$1"