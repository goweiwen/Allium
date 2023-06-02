#!/bin/sh

# configure wifi network
# WIFI_SSID=myssid
# WIFI_PASS=
# echo "ctrl_interface=/var/run/wpa_supplicant" > /appconfigs/wpa_supplicant.conf
# echo "update_config=1" >> /appconfigs/wpa_supplicant.conf
# echo $'\n'"network={" >> /appconfigs/wpa_supplicant.conf
# echo $'\tssid="'"$WIFI_SSID"$'"' >> /appconfigs/wpa_supplicant.conf
# if [ ! -z "$WIFI_PASS" ]; then
# 	echo $'\tpsk="'"$WIFI_PASS"$'"' >> /appconfigs/wpa_supplicant.conf
# else
# 	echo $'\t'"key_mgmt=NONE" >> /appconfigs/wpa_supplicant.conf
# fi
# echo "}" >> /appconfigs/wpa_supplicant.conf

# stop telnet
# killall telnetd > /dev/null 2>&1 &

if [ -f /mnt/SDCARD/.tmp_update/8188fu.ko ] && [ -f /appconfigs/wpa_supplicant.conf ]; then
	# install wifi mod
	if ! cat /proc/modules | grep -c 8188fu; then
		insmod /mnt/SDCARD/.tmp_update/8188fu.ko
	fi

	# wifi up
	ifconfig lo up
	/customer/app/axp_test wifion
	sleep 2
	ifconfig wlan0 up
	/customer/app/wpa_supplicant -B -D nl80211 -iwlan0 -c /appconfigs/wpa_supplicant.conf
	udhcpc -i wlan0 -s /etc/init.d/udhcpc.script > /dev/null 2>&1 &

	# start FTP
	tcpsvd -E 0.0.0.0 21 ftpd -w /mnt/SDCARD > /dev/null 2>&1 &

	# start NTP
	# ntpd -p 216.239.35.12 -S "/sbin/hwclock -w -u" > /dev/null 2>&1 &
fi
