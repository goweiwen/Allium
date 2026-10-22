#!/bin/sh
export picodir=/mnt/SDCARD/Apps/pico.pak
export picoconfig="$picodir"/.lexaloffle/
export sysdir=/mnt/SDCARD/.tmp_update
export miyoodir=/mnt/SDCARD/miyoo
cd $picodir
export PATH="$PWD/bin:$PATH"
export HOME=$picodir

# The panel's own timing, which a rewritten framebuffer geometry cannot mislead; fbset and
# then the Mini's panel are the fallbacks.
panel=$(sed -n 's/.*Current TimingWidth=\([0-9]\{3,4\}\)[^0-9]\{1,\}\([0-9]\{3,4\}\).*/\1 \2/p' \
    /proc/mi_modules/fb/mi_fb0 2>/dev/null | head -1)
[ -n "$panel" ] || panel=$(fbset 2>/dev/null | sed -n 's/^mode "\([0-9]*\)x\([0-9]*\).*/\1 \2/p' | head -1)
[ -n "$panel" ] || panel="640 480"
panel_w=${panel% *}
panel_h=${panel#* }

# The cart is square: the panel's shorter side, centred.
side=$panel_h
[ "$panel_w" -lt "$side" ] && side=$panel_w
rect="$(((panel_w - side) / 2)),$(((panel_h - side) / 2)),$side,$side"

# some users have reported black screens at boot. we'll check if the file exists, then check the keys to see if they match the known good config
fixconfig() {
    config_file="${picodir}/.lexaloffle/pico-8/config.txt"

    if [ ! -f "$config_file" ]; then
        echo "Config file not found, creating with default values."
        return
    fi

    echo "Config checker: Validating display settings in config.txt"

    # The SDL2 resamples the cart nearest to the panel, so MI_GFX never resizes, and so never filters.
    set_window_size="window_size $panel_w $panel_h"
    set_screen_size="screen_size $side $side"
    set_pixel_perfect="pixel_perfect 1"
    set_windowed="windowed 0"
    set_window_position="window_position -1 -1"
    set_frameless="frameless 1"
    set_fullscreen_method="fullscreen_method 2"
    set_blit_method="blit_method 2"
    set_transform_screen="transform_screen 0"
    set_host_framerate_control="host_framerate_control 0"
    # The SDL2 passes A and B through as SPACE and LCTRL; these are their scancodes.
    set_button_keys="button_keys 0 0 0 0 44 224 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"

    for setting in window_size screen_size pixel_perfect windowed window_position frameless fullscreen_method blit_method transform_screen host_framerate_control button_keys; do
        case $setting in
            window_size) new_value="$set_window_size" ;;
            screen_size) new_value="$set_screen_size" ;;
            pixel_perfect) new_value="$set_pixel_perfect" ;;
            windowed) new_value="$set_windowed" ;;
            window_position) new_value="$set_window_position" ;;
            frameless) new_value="$set_frameless" ;;
            fullscreen_method) new_value="$set_fullscreen_method" ;;
            blit_method) new_value="$set_blit_method" ;;
            transform_screen) new_value="$set_transform_screen" ;;
            host_framerate_control) new_value="$set_host_framerate_control" ;;
            button_keys) new_value="$set_button_keys" ;;
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
    grep -E "window_size|screen_size|pixel_perfect|windowed|window_position|frameless|fullscreen_method|blit_method|transform_screen|host_framerate_control|button_keys" "$config_file"
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
    export SDL_VIDEODRIVER=Mini
    export EGL_VIDEODRIVER=Mini
    # SDL lists its own software renderer first, and its frames never reach the panel.
    export SDL_RENDER_DRIVER="Miyoo Mini"
    export SDL_MINI_VOLUME_FILE=/tmp/volume
    # X and Y report B's and A's keys: with button_keys set, PICO-8 ignores its defaults.
    export SDL_MINI_KEYMAP="Left Shift=Left Ctrl,Left Alt=Space"
    # L2 toggles a pointer on the d-pad, kept on the cart where PICO-8 can place it.
    export SDL_MINI_MOUSE=toggle
    export SDL_MINI_MOUSE_RECT="$rect"
    # Bezels are drawn unscaled, so each panel size has its own set; SELECT+LEFT/RIGHT walks it.
    bezels="$picodir/res/bezel/${panel_w}x${panel_h}"
    [ -d "$bezels" ] && export SDL_MINI_BEZEL="$bezels"
    export SDL_MINI_BEZEL_SAVE="$bezels/.selected"
    # The scaling mode and screen effect persist here; setting these enables both and their keys.
    export SDL_MINI_SCALE_SAVE="$picodir/.scale"
    export SDL_MINI_EFFECT_SAVE="$picodir/.effect"
    # The cart is 128x128 whatever size PICO-8 hands over, so the effects line up with it.
    export SDL_MINI_EFFECT_CELLS=128

    fixconfig
    /mnt/SDCARD/.tmp_update/script/stop_audioserver.sh
    libpadspblocker
    /mnt/SDCARD/.tmp_update/script/set_sound_level.sh &
    # The cart leaves PICO-8 at 128x128; the SDL2 resamples it to the panel, so a pre-scale would only cost.
    pico8_dyn -splore -width "$panel_w" -height "$panel_h" -draw_rect "$rect" -preblit_scale 1
}

main() {
    echo performance > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
    /mnt/SDCARD/.allium/scripts/swap-on.sh
    mount --bind /mnt/SDCARD/Roms/PICO /mnt/SDCARD/Apps/pico.pak/.lexaloffle/pico-8/carts
    # Two panels tall: the SDL2 double-buffers in the framebuffer.
    fbset -g "$panel_w" "$panel_h" "$panel_w" "$((panel_h * 2))" 32
    start_pico
    cat /dev/zero > /dev/fb0 2>/dev/null
    /mnt/SDCARD/.tmp_update/script/start_audioserver.sh
    /mnt/SDCARD/.tmp_update/script/set_sound_level.sh
    umount /mnt/SDCARD/Apps/pico.pak/.lexaloffle/pico-8/carts
    /mnt/SDCARD/.allium/scripts/swap-off.sh
    echo ondemand > /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
}

main
