#!/bin/sh
# Adapted from DotUI-X: https://github.com/anzz1/DotUI-X/blob/master/extras/Tools/WiFi.pak/wifion.sh



if ! cat /proc/modules | grep -c 8188fu; then
	insmod /mnt/SDCARD/.tmp_update/8188fu.ko
fi
ifconfig lo up
/customer/app/axp_test wifion
sleep 2
ifconfig wlan0 up
/customer/app/wpa_supplicant -B -D nl80211 -iwlan0 -c /appconfigs/wpa_supplicant.conf
ln -sf /dev/null /tmp/udhcpc.log
udhcpc -i wlan0 -s /etc/init.d/udhcpc.script > /dev/null 2>&1 &
