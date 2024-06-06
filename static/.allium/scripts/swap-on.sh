#!/bin/sh

swapfile="/mnt/SDCARD/cachefile"
if [ ! -e "$swapfile" ]; then
	dd if=/dev/zero of="$swapfile" bs=1M count=128
	mkswap "$swapfile"
fi
swapon "$swapfile"
