#!/bin/sh

/mnt/SDCARD/.tmp_update/script/set_sound_level.sh &
"$ROOT"/.allium/cores/drastic/drastic/launch.sh "$@"

if [ -f /mnt/SDCARD/.tmp_update/script/start_audioserver.sh ]; then
    /mnt/SDCARD/.tmp_update/script/start_audioserver.sh
fi
/mnt/SDCARD/.tmp_update/script/set_sound_level.sh
