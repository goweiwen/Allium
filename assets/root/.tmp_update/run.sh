#!/bin/sh

DIR="$(dirname "$0")"
cd "$DIR"

# run allium
RUST_BACKTRACE=1 RUST_LOG=debug /mnt/SDCARD/.allium/allium >> /mnt/SDCARD/.allium/allium.log 2>&1

# power off
while true; do
	sync && poweroff && sleep 10
done
