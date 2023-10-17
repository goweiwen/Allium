#!/bin/sh
set -eux

dir="$(dirname "$0")"
rm -rf "$ROOT"/.allium/cores/drastic
miniunz -x -o "$dir"/drastic.zip -d "$ROOT"/.allium/cores/
rm -rf "$dir"/drastic.zip

cat >> "$ROOT"/.allium/cores/drastic/launch.sh <<EOF

if [ -f /mnt/SDCARD/.tmp_update/script/start_audioserver.sh ]; then
    /mnt/SDCARD/.tmp_update/script/start_audioserver.sh
fi
EOF