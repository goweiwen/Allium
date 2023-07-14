#!/bin/sh
set -eu
dist="${PWD}"/dist/.allium/migrations/0001-retroarch-core-overrides/retroarch-core-overrides.zip
dir="$(dirname "$0")"
cd "$dir"
zip -r -9 -q "$dist" ./retroarch-core-overrides/
