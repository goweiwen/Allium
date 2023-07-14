#!/bin/sh
set -eu
dist="${PWD}"/dist/.allium/migrations/0000-retroarch-config/retroarch-config.zip
dir="$(dirname "$0")"
cd "$dir"
zip -r -9 -q "$dist" ./retroarch-config/
