#!/bin/sh

GITHUB_REPOSITORY="https://github.com/goweiwen/Allium"
RELEASE_FILE="allium-arm-unknown-linux-gnueabihf.zip"

CURRENT=$(cat /mnt/SDCARD/.allium/version.txt)
echo "Current version: $CURRENT"

LATEST=$(curl --silent --location -o /dev/null -w %\{url_effective\} $GITHUB_REPOSITORY/releases/latest | cut -d "/" -f 8)
echo "Latest version: $LATEST"

if [ "$CURRENT" = "$LATEST" ]; then
	echo "You are on the latest version." >&2
	exit 0
fi

available_space=$(df -m /mnt/SDCARD | tail -1 | awk '{print $4}')
if [ "$available_space" -lt 300 ]; then
	echo "You need 300MB of free space to update Allium." >&2
	exit 0
fi

cd /mnt/SDCARD/.allium || exit

if ! curl --silent --location -o allium-ota.zip "$GITHUB_REPOSITORY/releases/download/$LATEST/$RELEASE_FILE"; then
	echo "Update download failed." >&2
	exit 0
fi

if [ ! -f allium-ota.zip ]; then
	echo "Update download failed." >&2
	exit 0
fi

if ! miniunz -x -o "allium-ota.zip" -d "/mnt/SDCARD/"; then
	say "Update extraction failed." >&2
	exit 0
fi

rm -f "allium-ota.zip"

sync
echo "Rebooting..."
sleep 2

shutdown -r

while true; do
	sync && reboot && sleep 10
done
