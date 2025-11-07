#!/bin/sh
set -eu

old_file="$ROOT"/.allium/state/stylesheet.json
new_dir="$ROOT"/Themes/Allium
new_file="$new_dir"/stylesheet.override.json

if [ -f "$old_file" ]; then
    mkdir -p "$new_dir"
    mv "$old_file" "$new_file"
fi
