#!/bin/sh

model="$1"
root=/mnt/SDCARD
log_file=/tmp/allium-clean-shutdown.log

# Release inherited logging pipes before stopping processes such as tee.
exec </dev/null >"$log_file" 2>&1
cd /
unset LD_PRELOAD
export PATH=/sbin:/usr/sbin:/bin:/usr/bin

echo "=== clean shutdown started ==="
date

# Swap must be disabled before its backing file can be unmounted.
swapoff "$root/cachefile" 2>/dev/null || true

# Stop services using executables, working directories, or data on the card.
for service in collie dufs ftp ssh syncthing telnet; do
    script="$root/.allium/scripts/$service-off.sh"
    [ -f "$script" ] && /bin/sh "$script" 2>/dev/null || true
done

"$root/.tmp_update/script/stop_audioserver.sh" 2>/dev/null || true

sync

# Stop any remaining process that still references the SD filesystem.
for process_id in $(fuser -m "$root" 2>/dev/null); do
    [ "$process_id" = "$$" ] || kill "$process_id" 2>/dev/null || true
done

sleep 1

for process_id in $(fuser -m "$root" 2>/dev/null); do
    [ "$process_id" = "$$" ] || kill -9 "$process_id" 2>/dev/null || true
done

# SSH bind-mounts this file from the SD card.
umount /etc/passwd 2>/dev/null || true

sync

if ! umount "$root"; then
    echo "failed to unmount $root; refusing to power off"
    exit 1
fi

if grep -q '/dev/mmcblk0p1' /proc/mounts; then
    echo "sd card remains mounted; refusing to power off"
    grep '/dev/mmcblk0p1' /proc/mounts
    exit 1
fi

echo "sd card cleanly unmounted"
sync

case "$model" in
    283)
        exec /sbin/reboot
        ;;
    285|354)
        exec /sbin/poweroff
        ;;
    *)
        echo "unknown miyoo model: $model"
        exit 1
        ;;
esac
