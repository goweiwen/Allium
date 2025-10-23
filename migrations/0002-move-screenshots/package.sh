#!/bin/sh
set -eu
dist="${PWD}"/dist/.allium/migrations/0002-move-screenshots/
dir="$(dirname "$0")"
cd "$dir"
mkdir -p "$dist"
cp ../../static/.allium/migrations/0002-move-screenshots/name.txt "$dist"
cp ../../static/.allium/migrations/0002-move-screenshots/run.sh "$dist"
chmod +x "$dist"/run.sh
