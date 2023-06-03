#!/bin/sh

DIR="$(dirname "$0")"
cd "$DIR"

RUST_BACKTRACE=1 RUST_LOG=trace /mnt/SDCARD/.allium/alliumd >> /mnt/SDCARD/.allium/alliumd.log 2>&1 &

# run Allium
while true; do
	RUST_BACKTRACE=1 RUST_LOG=trace /mnt/SDCARD/.allium/allium >> /mnt/SDCARD/.allium/allium.log 2>&1
done