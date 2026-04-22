#!/bin/sh
cd /mnt/SDCARD/Apps/BetterWifi.pak/
export sysdir=/mnt/SDCARD/.tmp_update
export LD_LIBRARY_PATH="/mnt/SDCARD/Apps/BetterWifi.pak/lib:/lib:/config/lib:$sysdir/lib:$sysdir/lib/parasyte"
export PATH="$sysdir/bin:$PATH"
export ZDOTDIR=share/zsh
export TERM=vt102
export TERMINFO=share/terminfo/
bin/zsh -x /mnt/SDCARD/Apps/BetterWifi.pak/script/wifitools.sh
