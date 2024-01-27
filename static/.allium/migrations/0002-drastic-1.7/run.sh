#!/bin/sh
set -eux

dir="$(dirname "$0")"
rm -rf "$ROOT"/.allium/cores/drastic
miniunz -x -o "$dir"/drastic.zip -d "$ROOT"/.allium/cores/
rm "$dir"/drastic.zip

mv "$ROOT"/.allium/cores/drastic/launch.sh "$ROOT"/.allium/cores/drastic/launch_drastic.sh

mv "$dir"/launch.sh "$ROOT"/.allium/cores/drastic/launch.sh
