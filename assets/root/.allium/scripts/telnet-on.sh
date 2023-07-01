#!/bin/sh

dir=$(dirname "$0")
if "$dir"/wait-for-wifi.sh; then
    cd /mnt/SDCARD/
    telnetd -l sh > /dev/null &
    exit 0
fi

exit 1