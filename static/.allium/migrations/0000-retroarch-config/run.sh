#!/bin/sh
set -eux

dir="$(dirname "$0")"
miniunz -x -o "$dir"/retroarch-config.zip -d "$dir"
find "$dir"/retroarch-config -type f | while read -r file; do
    path="$ROOT"/RetroArch/${file#"$dir"/retroarch-config/}
    if [ ! -f "$path" ]; then
        echo "Moving $file to $path"
        mkdir -p "${path%/*}"
        mv "$file" "$path"
    fi
done
rm -rf "$dir"/retroarch-config