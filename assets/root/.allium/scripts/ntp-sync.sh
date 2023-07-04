#!/bin/sh

if /mnt/SDCARD/.allium/scripts/wait-for-wifi.sh; then
    if ntpd -nq -S "/sbin/hwclock -w -u" -p 0.pool.ntp.org -p 1.pool.ntp.org -p 2.pool.ntp.org; then
        exit 0
    fi
fi

exit 1