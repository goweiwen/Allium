cd /mnt/SDCARD/Apps/BetterWifi.pak/
typeset -x sysdir=/mnt/SDCARD/.tmp_update
typeset -x LD_LIBRARY_PATH="/mnt/SDCARD/Apps/BetterWifi.pak/lib:/lib:/config/lib:$sysdir/lib:$sysdir/lib/parasyte"
typeset -x PATH="$sysdir/bin:$PATH"

init_static_globals() {
	typeset -gr WPACLI=/customer/app/wpa_cli || { print "ERROR: 'wpa_cli' not found" ; return 1 }
	typeset -gr DIALOG=/mnt/SDCARD/Apps/BetterWifi.pak/bin/dialog || { print "ERROR: 'dialog' not found" ; return 1 }

	typeset -gr MAXHEIGHT=$(( $LINES - 0 ))
	typeset -gr MAXWIDTH=$(( $COLUMNS - 0 ))
	typeset -gr CHOICE_HEIGHT=12

	typeset -gr DIALOG_OK=0
	typeset -gr DIALOG_CANCEL=1
}

# extra utilities
kill_udhcpc() {
if pgrep udhcpc > /dev/null; then
	killall -9 udhcpc
fi
}

start_udhcpc(){
udhcpc -i wlan0 -s /etc/init.d/udhcpc.script > /dev/null 2>&1 &	
}

shortdialoginfo () {
    $DIALOG --no-lines --infobox "$@" 3 30
}

longdialoginfo() {
    $DIALOG --no-lines --infobox "$@" 3 60
}

wpa_config() {
    local network_id=$1

    if [ -z "$network_id" ]; then
        longdialoginfo  "Network ID not provided. Exiting."
		sleep 2
        return 1
    fi

    $WPACLI disable_network all > /dev/null 2>&1 &
    $WPACLI select_network "$network_id" > /dev/null 2>&1 &
    $WPACLI enable_network "$network_id" > /dev/null 2>&1 &
    $WPACLI save_config > /dev/null 2>&1 &
}

net_check() {
    local selected_ssid="$1"

    start_time=$(date +%s)
    timeout=30

    while true; do
        IP=$(ip route get 1 2>/dev/null | awk '{print $NF;exit}')
        if [ -z "$IP" ]; then
            current_time=$(date +%s)
            elapsed_time=$((current_time - start_time))
            if [ "$elapsed_time" -ge "$timeout" ]; then
                longdialoginfo "Failed to connect to $selected_ssid."
                sleep 2
                break
            fi
            continue
        else
            longdialoginfo  "Connected to $selected_ssid"
            sleep 0.5
            longdialoginfo  "Got IP: $IP"
            sleep 0.5
            if ping -q -c 4 -W 1 8.8.8.8 >/dev/null 2>&1; then
                longdialoginfo "Internet access is available."
                sleep 0.5
                break
            else
                longdialoginfo "Internet access is not available yet."
                sleep 1
                break
            fi
        fi
        sleep 1
    done
}

conn_cleanup() {
	kill_udhcpc
	longdialoginfo "Contacting DHCP"
	start_udhcpc
	sleep 6
}

# Lets get co n n e c t e d
main() {

	init_static_globals
	while true; do
		mainmenu 
	done
	
}

mainmenu() {
TITLE="Wifi Control"
MENU="Choose one of the following options:"

OPTIONS=(1 "Add new network and connect"
		 2 "Connect to stored network"
         3 "Remove a network"
         4 "Toggle a network (enable/disable)"
         5 "WPS connection"
         6 "Scan networks" 
		 7 "Store a network manually (no connection)"
         8 "Show stored networks"
         9 "Get current status"
		 10 "Restart WiFi"
         11 "Backup wpa_supplicant.conf"
		 12 "Restore backup wpa_supplicant.conf"
		 13 "Wipe wpa_supplicant.conf"
         14 "Exit")

CHOICE=$($DIALOG --colors --no-lines \
				--clear \
                --backtitle "$BACKTITLE" \
                --title "$TITLE" \
                --menu "$MENU" \
                $MAXHEIGHT $MAXWIDTH $CHOICE_HEIGHT \
                "${OPTIONS[@]}" \
                2>&1 >/dev/tty)

case $CHOICE in
        1)
            add_new
            ;;
        2)
            connect_stored
            ;;
        3)
            remove_stored
            ;;
        4)
            enable_disable_stored
            ;;
        5)
            wps_menu
            ;;
        6)
            scan_ssids
            ;;
        7)
            store_new
            ;;
        8)
            show_networks
            ;;
		9)
            show_info
            ;;	
        10)
            restart_wifi
            ;;
		11)
            backup_wpa_supp
            ;;
		12)
            restore_backup
            ;;
		13)
            reset_wpa_supplicant
            ;;
        14)
			longdialoginfo  "Exiting..."
			sleep 1
            exit 0
            ;;
esac
}

add_new() {
	longdialoginfo  "Scanning....."
    SELECTED_SSID=$(scan_ssids)
    IFS=";" read -r SSID bssid <<< "$SELECTED_SSID"
	echo $SSID
	
    if [ -z "$SSID" ]; then
        longdialoginfo "Exit requested or SSID not set..."
		sleep 1
        return 1
    fi	
	
	local PASSWORD=$($DIALOG --no-lines --inputbox "Enter the WPA Key\n(Press X for keyboard)" 0 0 2>&1 >/dev/tty)
	echo "$PASSWORD"
	longdialoginfo "Connecting to $SSID with pwd: $PASSWORD..."

	kill_udhcpc
	
	new_id=$($WPACLI -i wlan0 add_network | tail -n 1)
	$WPACLI set_network "$new_id" ssid \"$SSID\" > /dev/null 2>&1
    $WPACLI set_network "$new_id" psk \"$PASSWORD\" > /dev/null 2>&1

	wpa_config "$new_id"
	
    sleep 2
	
	conn_cleanup
	net_check "$SSID"
	show_info
}

connect_stored() {
    longdialoginfo "Getting available Wi-Fi networks..."
	sleep 1
    local networks=$($WPACLI -i wlan0 list_networks | tail -n+2 | awk '{$1=$1; $NF=$NF; print $1, substr($0, index($0,$2), length($0) - length($NF) - index($0,$2) + 1)}')

    if [ -z "$networks" ]; then
        longdialoginfo  "No stored networks found."
        sleep 1
        return
    fi

    local options=()
    while read -r id ssid; do
        options+=("$id" "$ssid") 
    done <<< "$networks"

    local cmd=($DIALOG --no-lines --menu "Available networks:" 25 40 10)
    local selected_id=$("${cmd[@]}" "${options[@]}" 2>&1 >/dev/tty)

    if [ -z "$selected_id" ]; then
        longdialoginfo "Exit requested or no network selected."
        sleep 1
        return
    fi

    selected_ssid=$(echo "$networks" | awk -v id="$selected_id" '$1 == id {for(i=2;i<=NF;i++) {if ($i == "any") break; printf("%s", $i); if (i < NF) printf(" ");} printf("\n");}')

    longdialoginfo "Selected network: ID=$selected_id SSID=$selected_ssid"
	sleep 1

    longdialoginfo "Connecting to $selected_ssid..."
    $WPACLI -i wlan0 select_network $selected_id > /dev/null 2>&1
    $WPACLI -i wlan0 enable_network $selected_id > /dev/null 2>&1
    $WPACLI -i wlan0 save_config > /dev/null 2>&1
    # $WPACLI -i wlan0 reassociate > /dev/null 2>&1
	
	conn_cleanup
	net_check "$selected_ssid"
    unset selected_ssid
    unset selected_status
    unset networks
	show_info
}

enable_disable_stored() {
    longdialoginfo "Getting available Wi-Fi networks..."
    sleep 1
    attempt_connect=0
    local networks=$($WPACLI -i wlan0 list_networks | tail -n+2 | awk '{$1=$1; $NF=$NF; print $1, substr($0, index($0,$2), length($0) - length($NF) - index($0,$2) + 1), $NF}')
  
    if [ -z "$networks" ]; then
        longdialoginfo "No stored networks found."
        sleep 1
        return
    fi

    local options=()
    while read -r id ssid network_status; do
        options+=("$id" "$ssid ($network_status)")
    done <<< "$networks"

    local cmd=($DIALOG --colors --no-lines --menu "Available networks:" 25 40 10)
    selected_id=$("${cmd[@]}" "${options[@]}" 2>&1 >/dev/tty)

    if [ -z "$selected_id" ]; then
        longdialoginfo "Exit requested or no network selected."
        sleep 2
        return
    fi
    
    selected_ssid=$(echo "$networks" | awk -v id="$selected_id" '$1 == id {for(i=2;i<=NF;i++) {if ($i == "any") break; printf("%s", $i); if (i < NF) printf(" ");} printf("\n");}')
    selected_status=$(echo "$networks" | awk -v id="$selected_id" '$1 == id {print $NF}')
    selected_status=$(echo "$selected_status" | tr -d '[]')

    if [ "$selected_status" = "CURRENT" ] || [ "$selected_status" = "any" ] || [ -z "$selected_status" ]; then
        $WPACLI -i wlan0 disable_network $selected_id > /dev/null 2>&1
        longdialoginfo "SSID=$selected_ssid, ID=$selected_id -> DISABLED"
        sleep 2
    else
        $WPACLI -i wlan0 enable_network $selected_id > /dev/null 2>&1
        longdialoginfo "SSID=$selected_ssid, ID=$selected_id -> ENABLED"
        attempt_connect=1
        sleep 2
    fi
    
    $WPACLI -i wlan0 save_config > /dev/null 2>&1
    
    if [ "$attempt_connect" -eq 1 ]; then
        $DIALOG --no-lines --yesno "Try and connect to this network now?" 0 0
        if [ $? -eq 0 ]; then
            $WPACLI -i wlan0 reconfigure > /dev/null 2>&1
            conn_cleanup
            net_check "$selected_ssid"
            show_info
        fi
    fi
    
    attempt_connect=0
    unset selected_ssid
    unset selected_status
    unset networks
}


remove_stored() {
    longdialoginfo "Getting available Wi-Fi networks..."
	sleep 1
    networks=$($WPACLI -i wlan0 list_networks | tail -n+2 | awk '{print $1 " " $2}' | sed 's/\[.*\]//g')

    if [ -z "$networks" ]; then
        longdialoginfo  "No stored networks found."
        sleep 1
        return
    fi

    local options=()
    while read -r id ssid; do
        options+=("$id" "$ssid") 
    done <<< "$networks"

    local cmd=($DIALOG --no-lines --menu "Available networks:" 25 40 10)
    selected_id=$("${cmd[@]}" "${options[@]}" 2>&1 >/dev/tty)

    if [ -z "$selected_id" ]; then
        longdialoginfo "Exit requested or no network selected."
        sleep 1
        return
    fi

    selected_ssid=$(echo "$networks" | awk -v id="$selected_id" '$1 == id {print $2}')

    longdialoginfo "Removing network with ID $selected_id..."
    sleep 1
    $WPACLI -i wlan0 remove_network $selected_id > /dev/null 2>&1
	sync  
    longdialoginfo "Network with ID $selected_id removed."
    sleep 1
    longdialoginfo "Changes will show in wpa_supp on next app reload."
    sleep 1
}

wps_menu() {
    local options=(
        1 "Basic WPS"
        2 "Pin WPS"
        3 "Exit"
    )

    local cmd=(
        $DIALOG --no-lines --title "WPS" --menu "Select WPS connection method:" $MAXHEIGHT $MAXWIDTH $CHOICE_HEIGHT
    )
    local choice=$("${cmd[@]}" "${options[@]}" 2>&1 >/dev/tty)

	case $choice in
		1)
			wps_connection "basic"
			;;
		2)
			wps_connection "pin"
			;;
		3)
			return
			;;
	esac
}

wps_connection() {
    local wps_method
    local wps_choice=$1
    local wps_pin

    if [ "$wps_choice" = "basic" ]; then
        wps_method="wps_pbc"
    elif [ "$wps_choice" = "pin" ]; then
        wps_method="wps_pin"
        wps_pin=$($DIALOG --no-lines --inputbox "Enter WPS Pin" 0 0 2>&1 >/dev/tty)
        if [ -z "$wps_pin" ]; then
            longdialoginfo "No WPS PIN entered. Exiting."
            sleep 2
            return
        fi
    else
        longdialoginfo "Invalid choice. Exiting."
		sleep 1
        return
    fi
	
	$DIALOG --no-lines --yesno "Info: Have you pressed your WPS button?" 0 0
	if [ $? -eq 1 ]; then
		longdialoginfo "Press your WPS and return"
		sleep 1
		return
	fi
	
    longdialoginfo "Scanning for networks..."
	sleep 2
    local selected_ssid=$(scan_ssids)
    IFS=";" read -r ssid bssid <<< "$selected_ssid"

    if [ -z "$selected_ssid" ]; then
        longdialoginfo "No SSID selected. Exiting."
		sleep 1
        return
    fi
	
	longdialoginfo "Disconnecting current network..."
	sleep 2
	$WPACLI disable_network all > /dev/null 2>&1
	kill_udhcpc

    longdialoginfo "Initiating WPS connection to $ssid using $wps_method..."
    if [ "$wps_choice" = "pin" ]; then
        $WPACLI $wps_method $bssid $wps_pin > /dev/null 2>&1
    else
        $WPACLI $wps_method $bssid > /dev/null 2>&1
    fi
	sleep 2

    conn_cleanup
	net_check "$selected_ssid"
	show_info
}

scan_ssids() {
    $WPACLI scan > /dev/null 2>&1
    sleep 5
    scan_results=$($WPACLI -i wlan0 scan_results)

	typeset tmpfile=/mnt/SDCARD/Apps/BetterWifi.pak/scan.txt

	IFS=$' \t'

	echo "[ssids]" > "$tmpfile"
	count=1
	while IFS= read -r line; do
    if [[ $line != "bssid / frequency / signal level / flags / ssid" ]]; then
        read -r bssid frequency signal_level flags ssid <<< "$line"

        ssid=${ssid#"SSID: "}

        ssid=$(echo "$ssid" | sed -e 's/\\[[:cntrl:]]//g')

        echo "ssid$count = $ssid;$bssid" >> "$tmpfile"
        ((count++))
    fi
	done <<< "$scan_results"

	grep -v '^[[:space:]]*$' "$tmpfile" > "$tmpfile.tmp"
	mv "$tmpfile.tmp" "$tmpfile"

    local ssids_section="[ssids]"
    local current_section=""
    local ssids=()

    while IFS="=" read -r key value; do
        key=$(echo "$key" | xargs)
        value=$(echo "$value" | xargs)

        [ -z "$key" ] || [ "${key#\#}" != "$key" ] && continue

        if [ "$key" = "$ssids_section" ]; then
            current_section="ssids"
            continue
        fi

        if [ "$current_section" = "ssids" ]; then
            ssids+=("$value")
        fi
    done < "$tmpfile"

    if [ ${#ssids[@]} -eq 0 ]; then
        $DIALOG --no-lines --msgbox "No SSIDs found." 8 40
        return
    fi

    local options=()
    for ssid in "${ssids[@]}"; do
        options+=("$ssid" "SSID")
    done

    local cmd=($DIALOG --no-lines --title "SSID Menu" --menu "Choose SSID:" 25 40 10)
    choice=$("${cmd[@]}" "${options[@]}" 2>&1 >/dev/tty)
	rm $tmpfile
    echo "$choice"
}


show_info() {
    wifi_status=$($WPACLI -i wlan0 status)

    formatted_status=$(echo "$wifi_status" | awk -F= '{printf "%s: %s\n\n", toupper($1), $2}')

    $DIALOG --no-lines \
           --title "Status" \
           --msgbox "$formatted_status" $MAXHEIGHT $MAXWIDTH
}

store_new() {
    local input_file=$(mktemp)
    
    $DIALOG --no-lines --inputbox "Enter the SSID\n(Press X for keyboard)" 0 0 2>"$input_file" >/dev/tty
    local new_ssid=$(cat "$input_file")
    
    if [ -z "$new_ssid" ]; then
        longdialoginfo  "Cancelled"
		sleep 1
        rm "$input_file" 
        return
    fi
    
    $DIALOG --no-lines --inputbox "Enter the WPA Key\n(Press X for keyboard)" 0 0 2>"$input_file" >/dev/tty
    local new_password=$(cat "$input_file")
    
    if [ -z "$new_password" ]; then
        longdialoginfo  "Cancelled"
		sleep 1
        rm "$input_file"
        return
    fi
    
    new_id=$($WPACLI -i wlan0 add_network | tail -n 1)
    $WPACLI -i wlan0 set_network $new_id ssid "\"$new_ssid\"" > /dev/null 2>&1
    $WPACLI -i wlan0 set_network $new_id psk "\"$new_password\"" > /dev/null 2>&1
    $WPACLI -i wlan0 enable_network $new_id > /dev/null 2>&1
    $WPACLI -i wlan0 save_config > /dev/null 2>&1
    longdialoginfo "New network added with ID: $new_id, SSID: $new_ssid"
    sleep 2
    
    rm "$input_file"
}

show_networks() {
	sync
    $WPACLI -i wlan0 reconfigure > /dev/null 2>&1
    local wpa_supplicant_conf=/appconfigs/wpa_supplicant.conf
    local title="Wi-Fi Network List (Scrollable)"

    if [ ! -f "$wpa_supplicant_conf" ]; then
        $DIALOG --no-lines --msgbox "wpa_supplicant.conf file not found." 8 40
        return
    fi

    network_list=$(<$wpa_supplicant_conf)

    $DIALOG --no-lines --title "$title" --msgbox "$network_list" $MAXHEIGHT $MAXWIDTH
}

# change_ap_pass() {
    # local new_pass=""
    # while true; do
        # new_pass=$($DIALOG --no-lines --inputbox "New AP password" 0 0 3>&1 1>&2 2>&3 3>&-)
        
        # if [ $? -eq 1 ]; then
            # longdialoginfo "Password change cancelled."
			# sleep 2
            # return
        # fi
        
        # if [ -z "$new_pass" ]; then
            # longdialoginfo "Password cannot be empty. Please try again."
			# sleep 2
            # continue
        # fi

        # if [ ${#new_pass} -lt 8 ]; then
            # longdialoginfo "Password should be at least 8 characters. Please try again."
			# sleep 2
            # continue
        # fi

        # break
    # done

    # sed -i -r "s/(wpa_passphrase=).*/\1$new_pass/" /mnt/SDCARD/.tmp_update/config/hostapd.conf
    # if [ $? -eq 0 ]; then
        # longdialoginfo "Password updated successfully."
		# sleep 2
    # else
        # longdialoginfo "Failed to update the password."
		# sleep 2
    # fi
# }

restart_wifi() {
    $DIALOG --no-lines --yesno "Warning: Do you want to restart WiFi?" 0 0
    response=$?
    if [ $response -eq 0 ]; then
        longdialoginfo  "Restarting Wi-Fi..."
		
        sleep 1
		kill_udhcpc
		killall -9 wpa_supplicant
		
		ifconfig wlan0 down
		sleep 1
		ifconfig wlan0 up
		
        $WPACLI -i wlan0 disconnect >/dev/null 2>&1

        $WPACLI -i wlan0 terminate >/dev/null 2>&1

        $WPACLI -i wlan0 reconfigure >/dev/null 2>&1
		
		longdialoginfo "Adaptor reset, starting wpa_supplicant"
		sleep 1
		
		/config/wifi/wpa_supplicant -B -D nl80211 -iwlan0 -c /appconfigs/wpa_supplicant.conf >/dev/null 2>&1 &
		sleep 1
		udhcpc -i wlan0 -s /etc/init.d/udhcpc.script > /dev/null 2>&1 &	
		
		$WPACLI -i wlan0 reconnect >/dev/null 2>&1
        longdialoginfo "Wi-Fi has been fully reset."
        sleep 1
    else
        longdialoginfo "Aborted. Wi-Fi reset was not performed."
        sleep 1
		return
    fi
    sleep 1
}

backup_wpa_supp() {
    local wpa_supplicant_conf="/appconfigs/wpa_supplicant.conf"
    local backup_file="${wpa_supplicant_conf}.bak"

    if [ -f "$backup_file" ]; then
        rm "$backup_file" >/dev/null 2>&1
		longdialoginfo "Removing old backup"
		sleep 1
    fi

    if [ -f "$wpa_supplicant_conf" ]; then
        cp "$wpa_supplicant_conf" "$backup_file" >/dev/null 2>&1

        if [ $? -eq 0 ]; then
            longdialoginfo "Backup created: $backup_file"
            sleep 1
        else
            longdialoginfo "Failed to create backup."
            sleep 1
        fi
    else
        longdialoginfo "wpa_supplicant.conf file not found."
        sleep 2
    fi
    sleep 1
}

restore_backup() {
    local wpa_supplicant_conf="/appconfigs/wpa_supplicant.conf"
    local backup_file="${wpa_supplicant_conf}.bak"
	local default_file="/mnt/SDCARD/Apps/BetterWifi.pak/script/wpa_supplicant.conf"

    if [ -f "$backup_file" ]; then
        rm "$wpa_supplicant_conf" >/dev/null 2>&1

        cp "$backup_file" "$wpa_supplicant_conf" >/dev/null 2>&1

        if [ $? -eq 0 ]; then
            longdialoginfo "Backup restored successfully."
            sleep 1
			restart_wifi
        else
            longdialoginfo "Failed to restore backup."
            sleep 1
        fi
    else
        longdialoginfo "Backup file not found."
		sleep 1
		$DIALOG --no-lines --yesno "Info: Do you want to restore a default wpa_supplicant.conf?" 0 0
		response=$?
		if [ $response -eq 0 ]; then
			cp "$default_file" "$wpa_supplicant_conf" >/dev/null 2>&1 
			longdialoginfo "Default file restored"
			sleep 1
			restart_wifi
		else
			return
		fi
    fi
}

reset_wpa_supplicant() {
    $DIALOG --no-lines --yesno "Warning: This will delete ALL currently stored WiFi networks. Are you sure you want to do this?" 0 0
    response=$?
    if [ $response -eq 0 ]; then
        pkill -9 wpa_supplicant
        $WPACLI -i wlan0 remove_network all >/dev/null 2>&1
        $WPACLI -i wlan0 save_config >/dev/null 2>&1
        rm -f /appconfigs/wpa_supplicant.conf >/dev/null 2>&1
        cat <<-EOF > /appconfigs/wpa_supplicant.conf
		ctrl_interface=/var/run/wpa_supplicant
		update_config=1
		EOF
        $WPACLI -i wlan0 reconfigure >/dev/null 2>&1
        longdialoginfo "Done, your wpa_supplicant.conf file has been reset"
        sleep 1
        longdialoginfo "WiFi has been shut down"
        sleep 1
    else
        longdialoginfo "Aborted. Your WiFi networks were not deleted."
        sleep 1
    fi
    sleep 1
}

main
