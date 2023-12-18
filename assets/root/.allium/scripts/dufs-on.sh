#!/bin/sh

dir=$(dirname "$0")
if "$dir"/wait-for-wifi.sh; then
    cd /mnt/SDCARD/ || exit
    "$ROOT/.allium/bin/dufs" --allow-all --port 80
    exit 0
fi

exit 1
