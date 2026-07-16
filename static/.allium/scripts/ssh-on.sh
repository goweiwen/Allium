#!/bin/sh

dir=$(dirname "$0")
if "$dir"/wait-for-wifi.sh; then
    mkdir -p "$ROOT/.allium/state/dropbear"
    # Bind-mount a /etc/passwd with a blank-password root.
    # That lets stock `dropbear -B` allow passwordless login.
    if ! mount | grep -q " /etc/passwd "; then
        mount -o bind "$ROOT/.allium/etc/passwd" /etc/passwd
    fi
    "$ROOT/.allium/bin/dropbear" -R -B -p 22 >/dev/null 2>&1
    exit 0
fi

exit 1
