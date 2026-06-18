#!/bin/sh

killall dropbear
# Restore the original read-only /etc/passwd.
umount /etc/passwd 2>/dev/null
