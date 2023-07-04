#!/bin/sh

dir=$(dirname "$0")
if "$dir"/wait-for-wifi.sh; then
    cd /mnt/SDCARD/ || exit
    telnetd -l sh
    exit 0
fi

exit 1