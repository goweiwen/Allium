#!/bin/sh

# Check if update file exists
if [ ! -f /mnt/SDCARD/allium-ota.zip ]; then
	echo "Update file not found at /mnt/SDCARD/allium-ota.zip" >&2
	exit 1
fi

# Check available space
available_space=$(df -m /mnt/SDCARD | tail -1 | awk '{print $4}')
if [ "$available_space" -lt 300 ]; then
	echo "You need 300MB of free space to update Allium." >&2
	exit 1
fi

say "Updating Allium.\
Please wait..."

# Extract update
if ! miniunz -x -o "/mnt/SDCARD/allium-ota.zip" -d "/mnt/SDCARD/"; then
	echo "Update extraction failed." >&2
	exit 1
fi

# Remove update file
rm -f "/mnt/SDCARD/allium-ota.zip"

sync
echo "Rebooting..."
sleep 2

shutdown -r

while true; do
	sync && reboot && sleep 10
done
