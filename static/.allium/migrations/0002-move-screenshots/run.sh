#!/bin/sh
set -eu

old_dir="$ROOT"/.allium/screenshots
new_dir="$ROOT"/Saves/CurrentProfile/screenshots

if [ -d "$old_dir" ]; then
    mkdir -p "$new_dir"
    if [ -n "$(ls -A "$old_dir" 2>/dev/null)" ]; then
        mv "$old_dir"/* "$new_dir"/
    fi
    rmdir "$old_dir"
fi
