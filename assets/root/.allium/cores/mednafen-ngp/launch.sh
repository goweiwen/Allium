#!/bin/sh
cd /mnt/SDCARD/RetroArch/
HOME=/mnt/SDCARD/RetroArch/ exec ./retroarch.sh -v -L .retroarch/cores/mednafen_ngp_rearmed_libretro.so "$1"
