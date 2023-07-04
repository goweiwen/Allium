#!/bin/sh
# Adapted from DotUI-X: https://github.com/anzz1/DotUI-X/blob/master/extras/Tools/WiFi.pak/wifion.sh

dir=$(dirname "$0")
"$dir/telnet-off.sh"
"$dir/ftp-off.sh"

killall wpa_supplicant > /dev/null 2>&1 &
killall udhcpc > /dev/null 2>&1 &
ifconfig wlan0 down
/customer/app/axp_test wifioff
