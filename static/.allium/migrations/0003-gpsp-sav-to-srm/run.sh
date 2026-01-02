#!/bin/sh
set -eu

gpsp_dir="$ROOT"/Saves/CurrentProfile/gpSP

if [ -d "$gpsp_dir" ]; then
    find "$gpsp_dir" -type f -name "*.sav" | while read -r file; do
        new_file="${file%.sav}.srm"
        if [ -f "$new_file" ]; then
            echo "Skipping $file: $new_file already exists"
            continue
        fi
        echo "Renaming $file to $new_file"
        mv "$file" "$new_file"
    done
fi
