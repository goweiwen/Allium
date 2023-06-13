#!/bin/sh
# Adapted from DotUI-X: https://github.com/anzz1/DotUI-X/blob/master/extras/Tools/WiFi.pak/wifion.sh

killall ntpd > /dev/null 2>&1 &
killall telnetd > /dev/null 2>&1 &
killall ftpd > /dev/null 2>&1 &
killall tcpsvd > /dev/null 2>&1 &
killall dropbear > /dev/null 2>&1 &
killall wpa_supplicant > /dev/null 2>&1 &
killall udhcpc > /dev/null 2>&1 &
ifconfig wlan0 down
/customer/app/axp_test wifioff
