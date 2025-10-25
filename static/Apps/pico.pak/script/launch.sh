#!/bin/sh
export picodir=/mnt/SDCARD/Apps/pico.pak
export picoconfig="$picodir"/.lexaloffle/
export sysdir=/mnt/SDCARD/.tmp_update
export miyoodir=/mnt/SDCARD/miyoo
cd $picodir
export PATH="$PWD/bin:$PATH"
export HOME=$picodir

get_curvol() {
    awk '/LineOut/ {if (!printed) {gsub(",", "", $8); print $8; printed=1}}' /proc/mi_modules/mi_ao/mi_ao0
}

get_curmute() {
    awk '/LineOut/ {if (!printed) {gsub(",", "", $8); print $6; printed=1}}' /proc/mi_modules/mi_ao/mi_ao0
}

is_process_running() {
  process_name="$1"
  if [ -z "$(pgrep -f "$process_name")" ]; then
    return 1
  else
    return 0
  fi
}

kill_audio_servers() {
    is_process_running "audioserver" && pkill -9 -f "audioserver"
    is_process_running "audioserver.mod" && killall -q "audioserver.mod"
}

runifnecessary() {
    cnt=0
    a=`ps | grep $1 | grep -v grep`
    a=$(pgrep $1)
    while [ "$a" == "" ] && [ $cnt -lt 8 ]; do
        $2 $3 &
        sleep 0.5
        cnt=$(expr $cnt + 1)
        a=$(pgrep $1)
    done
}

set_snd_level() {
    local target_vol="$1"
    local target_mute="$2"
    local current_vol
    local current_mute
    local start_time
    local elapsed_time

    start_time=$(date +%s)
    while [ ! -e /proc/mi_modules/mi_ao/mi_ao0 ]; do
        sleep 0.2
        elapsed_time=$(( $(date +%s) - start_time ))
        if [ "$elapsed_time" -ge 30 ]; then
            echo "Timed out waiting for /proc/mi_modules/mi_ao/mi_ao0"
            return 1
        fi
    done

    start_time=$(date +%s)
    while true; do
        echo "set_ao_volume 0 ${target_vol}dB" > /proc/mi_modules/mi_ao/mi_ao0
        echo "set_ao_volume 1 ${target_vol}dB" > /proc/mi_modules/mi_ao/mi_ao0
        echo "set_ao_mute ${target_mute}" > /proc/mi_modules/mi_ao/mi_ao0

        current_vol=$(get_curvol)
        current_mute=$(get_curmute)

        if [ "$current_vol" = "$target_vol" ] && [ "$current_mute" = "$target_mute" ]; then
            echo "Volume set to ${current_vol}dB, Mute status: ${current_mute}"
            return 0
        fi

        elapsed_time=$(( $(date +%s) - start_time ))
        if [ "$elapsed_time" -ge 360 ]; then
            echo "Timed out trying to set volume and mute status"
            return 1
        fi

        sleep 0.2
    done
}

# some users have reported black screens at boot. we'll check if the file exists, then check the keys to see if they match the known good config
fixconfig() {
    config_file="${picodir}/.lexaloffle/pico-8/config.txt"

    if [ ! -f "$config_file" ]; then
        echo "Config file not found, creating with default values."
        return
    fi

    echo "Config checker: Validating display settings in config.txt"

    set_window_size="window_size 640 480"
    set_screen_size="screen_size 640 480"
    set_windowed="windowed 0"
    set_window_position="window_position -1 -1"
    set_frameless="frameless 1"
    set_fullscreen_method="fullscreen_method 0"
    set_blit_method="blit_method 2"
    set_transform_screen="transform_screen 134"

    for setting in window_size screen_size windowed window_position frameless fullscreen_method blit_method transform_screen; do
        case $setting in
            window_size) new_value="$set_window_size" ;;
            screen_size) new_value="$set_screen_size" ;;
            windowed) new_value="$set_windowed" ;;
            window_position) new_value="$set_window_position" ;;
            frameless) new_value="$set_frameless" ;;
            fullscreen_method) new_value="$set_fullscreen_method" ;;
            blit_method) new_value="$set_blit_method" ;;
            transform_screen) new_value="$set_transform_screen" ;;
        esac

        if grep -q "^$setting" "$config_file"; then
            sed -i "s/^$setting.*/$new_value/" "$config_file"
            echo "Updated setting: $setting"
        else
            echo "$new_value" >> "$config_file"
            echo "Added missing setting: $setting"
        fi
    done

    echo "Updated settings:"
    grep -E "window_size|screen_size|windowed|window_position|frameless|fullscreen_method|blit_method|transform_screen" "$config_file"
}

# when wifi is restarted, udhcpc and wpa_supplicant may be started with libpadsp.so preloaded, this is bad as they can hold mi_ao open even after audioserver has been killed.
libpadspblocker() { 
    wpa_pid=$(ps -e | grep "[w]pa_supplicant" | awk 'NR==1{print $1}')
    udhcpc_pid=$(ps -e | grep "[u]dhcpc" | awk 'NR==1{print $1}')
    if [ -n "$wpa_pid" ] && [ -n "$udhcpc_pid" ]; then
        if grep -q "libpadsp.so" /proc/$wpa_pid/maps || grep -q "libpadsp.so" /proc/$udhcpc_pid/maps; then
            echo "Network Checker: $wpa_pid(WPA) and $udhcpc_pid(UDHCPC) found preloaded with libpadsp.so"
            unset LD_PRELOAD
            killall -9 wpa_supplicant
            killall -9 udhcpc 
            $miyoodir/app/wpa_supplicant -B -D nl80211 -iwlan0 -c /appconfigs/wpa_supplicant.conf & 
            udhcpc -i wlan0 -s /etc/init.d/udhcpc.script &
            echo "Network Checker: Removing libpadsp.so preload on wpa_supp/udhcpc"
        fi
    fi
}

start_pico() {
    export LD_LIBRARY_PATH="$picodir/lib:/lib:/config/lib:/mnt/SDCARD/miyoo/lib:/mnt/SDCARD/.tmp_update/lib:/mnt/SDCARD/.tmp_update/lib/parasyte:/sbin:/usr/sbin:/bin:/usr/bin:$LD_LIBRARY_PATH"
    export SDL_VIDEODRIVER=mmiyoo
    export SDL_AUDIODRIVER=mmiyoo
    export EGL_VIDEODRIVER=mmiyoo
    
    fixconfig
    kill_audio_servers
    libpadspblocker
    set_snd_level "${curvol}" "${curmute}" &
    pico8_dyn -splore -preblit_scale 3 -pixel_perfect 0
}

main() {
    echo performance > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
    curvol=$(get_curvol) 
    curmute=$(get_curmute)
    mount --bind /mnt/SDCARD/Roms/PICO /mnt/SDCARD/App/pico/.lexaloffle/pico-8/carts
    start_pico
    umount /mnt/SDCARD/App/pico/.lexaloffle/pico-8/carts
    echo ondemand > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
}

main

