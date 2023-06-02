#!/bin/sh

DIR="$(dirname "$0")"
cd "$DIR"

# run Allium
while true; do
	RUST_BACKTRACE=1 RUST_LOG=trace /mnt/SDCARD/.allium/allium >> /mnt/SDCARD/.allium/allium.log 2>&1
done