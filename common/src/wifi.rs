use std::fs::{self, File};
use std::io::Write;
#[cfg(feature = "miyoo")]
use std::process::Command;

use anyhow::Result;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::constants::ALLIUM_WIFI_SETTINGS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiFiSettings {
    pub wifi: bool,
    pub ssid: String,
    pub password: String,
    pub ntp: bool,
    pub telnet: bool,
    pub ftp: bool,
}

impl WiFiSettings {
    pub fn new() -> Self {
        Self {
            wifi: false,
            ssid: String::new(),
            password: String::new(),
            ntp: false,
            telnet: false,
            ftp: false,
        }
    }

    pub fn load() -> Result<Self> {
        if ALLIUM_WIFI_SETTINGS.exists() {
            debug!("found state, loading from file");
            if let Ok(json) = fs::read_to_string(ALLIUM_WIFI_SETTINGS.as_path()) {
                if let Ok(json) = serde_json::from_str(&json) {
                    return Ok(json);
                }
            }
            warn!("failed to read state file, removing");
            fs::remove_file(ALLIUM_WIFI_SETTINGS.as_path())?;
        }
        Ok(Self::load_wpa_supplicant_conf().unwrap_or_default())
    }

    pub fn init(&self) -> Result<()> {
        if self.wifi {
            wifi_on()?;
            telnet_off()?;
            if self.ntp {
                ntp_sync()?;
            }
            if self.telnet {
                telnet_on()?;
            }
            if self.ftp {
                ftp_on()?;
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        File::create(ALLIUM_WIFI_SETTINGS.as_path())?.write_all(json.as_bytes())?;
        if let Err(e) = self.update_wpa_supplicant_conf() {
            warn!("failed to update wpa_supplicant.conf: {}", e);
        }
        Ok(())
    }

    fn load_wpa_supplicant_conf() -> Option<Self> {
        #[cfg(feature = "miyoo")]
        {
            let data = fs::read_to_string("/appconfigs/wpa_supplicant.conf").ok()?;

            let ssid_index = data.find("ssid=\"")?;
            let ssid = &data[ssid_index + 6..];
            let ssid_end = ssid.find('"')?;
            let ssid = &ssid[..ssid_end];

            let psk_index = data.find("psk=\"")?;
            let psk = &data[psk_index + 5..];
            let psk_end = psk.find('"')?;
            let psk = &psk[..psk_end];

            Some(Self {
                ssid: ssid.to_string(),
                password: psk.to_string(),
                ..Default::default()
            })
        }

        #[cfg(not(feature = "miyoo"))]
        Some(Self::new())
    }

    fn update_wpa_supplicant_conf(&self) -> Result<()> {
        #[cfg(feature = "miyoo")]
        {
            let mut file = File::create("/appconfigs/wpa_supplicant.conf")?;
            write!(
                file,
                r#"ctrl_interface=/var/run/wpa_supplicant
update_config=1

network={{
	ssid="{ssid}"
	psk="{password}"
}}"#,
                ssid = self.ssid.replace('"', "\\\""),
                password = self.password.replace('"', "\\\""),
            )?;
        }
        Ok(())
    }

    pub fn toggle_wifi(&mut self, enabled: bool) -> Result<()> {
        self.wifi = enabled;
        if self.wifi {
            wifi_on()?;
            if self.telnet {
                telnet_on()?;
            }
            if self.ftp {
                ftp_on()?;
            }
        } else {
            wifi_off()?;
        }
        Ok(())
    }

    pub fn toggle_ntp(&mut self, enabled: bool) -> Result<()> {
        self.ntp = enabled;
        if self.ntp {
            ntp_sync()?;
        }
        Ok(())
    }

    pub fn toggle_telnet(&mut self, enabled: bool) -> Result<()> {
        self.telnet = enabled;
        if self.telnet {
            telnet_on()?;
        } else {
            telnet_off()?;
        }
        Ok(())
    }

    pub fn toggle_ftp(&mut self, enabled: bool) -> Result<()> {
        self.ftp = enabled;
        if self.ftp {
            ftp_on()?;
        } else {
            ftp_off()?;
        }
        Ok(())
    }
}

impl Default for WiFiSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub fn wifi_on() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("wifi-on.sh")).spawn()?;
    Ok(())
}

pub fn wifi_off() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("wifi-off.sh")).spawn()?;
    Ok(())
}

pub fn telnet_on() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("telnet-on.sh")).spawn()?;
    Ok(())
}

pub fn telnet_off() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("telnet-off.sh")).spawn()?;
    Ok(())
}

pub fn ftp_on() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("ftp-on.sh")).spawn()?;
    Ok(())
}

pub fn ftp_off() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("ftp-off.sh")).spawn()?;
    Ok(())
}

pub fn ntp_sync() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("ntp-sync.sh")).spawn()?;
    Ok(())
}

pub fn wait_for_wifi() -> Result<()> {
    #[cfg(feature = "miyoo")]
    Command::new(crate::constants::ALLIUM_SCRIPTS_DIR.join("wait-for-wifi.sh")).spawn()?;
    Ok(())
}

pub fn ip_address() -> Option<String> {
    #[cfg(feature = "miyoo")]
    {
        let output = Command::new("ip")
            .args(["route", "get", "255.255.255.255"])
            .output()
            .ok()?;
        let output = String::from_utf8(output.stdout).ok()?;
        let ip_address = output.split_whitespace().last().map(|s| s.to_string());

        return ip_address.and_then(|addr| {
            addr.split('.')
                .all(|octet| octet.parse::<u8>().is_ok())
                .then_some(addr)
        });
    }

    #[cfg(feature = "simulator")]
    {
        return Some("127.0.0.1".to_string());
    }

    #[cfg(not(any(feature = "miyoo", feature = "simulator")))]
    return None;
}

struct ByteBuf<'a>(&'a [u8]);

impl<'a> std::fmt::LowerHex for ByteBuf<'a> {
    fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        for byte in self.0 {
            fmtr.write_fmt(format_args!("{:02x}", byte))?;
        }
        Ok(())
    }
}
