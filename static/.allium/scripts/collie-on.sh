#!/bin/sh

dir=$(dirname "$0")
if "$dir"/wait-for-wifi.sh; then
    cd "$ROOT" || exit
    "$ROOT/.allium/bin/collie" --bind 0.0.0.0 >/dev/null 2>&1
    exit 0
fi

exit 1
