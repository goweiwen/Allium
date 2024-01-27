#!/bin/sh

dir=$(dirname $0)
if "$dir"/wait-for-wifi.sh; then
    cd /mnt/SDCARD/ || exit
    tcpsvd -E 0.0.0.0 21 ftpd -w /mnt/SDCARD > /dev/null &
    exit 0
fi

exit 1