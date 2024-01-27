#!/bin/sh
set -eux

dir="$(dirname "$0")"
miniunz -x -o "$dir"/retroarch-core-overrides.zip -d "$dir"
find "$dir"/retroarch-core-overrides -type f | while read -r file; do
    path="$ROOT"/Saves/CurrentProfile/config/${file#"$dir"/retroarch-core-overrides/}
    if [ ! -f "$path" ]; then
        echo "Moving $file to $path"
        mkdir -p "${path%/*}"
        mv "$file" "$path"
    fi
done
rm -rf "$dir"/retroarch-core-overrides