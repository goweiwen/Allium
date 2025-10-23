#!/bin/sh
set -eux

old_dir="$ROOT"/.allium/screenshots
new_dir="$ROOT"/Saves/CurrentProfile/screenshots

if [ -d "$old_dir" ]; then
    mkdir -p "$new_dir"
    
    if [ "$(ls -A "$old_dir" 2>/dev/null)" ]; then
        find "$old_dir" -type f -name "*.png" | while read -r file; do
            filename=$(basename "$file")
            new_path="$new_dir"/"$filename"
            if [ ! -f "$new_path" ]; then
                mv "$file" "$new_path"
            fi
        done
        
        if [ ! "$(ls -A "$old_dir" 2>/dev/null)" ]; then
            rmdir "$old_dir"
        fi
    fi
fi
