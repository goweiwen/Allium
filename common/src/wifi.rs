use std::fs::{self, File};
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::constants::{ALLIUM_SCRIPTS_DIR, ALLIUM_WIFI_SETTINGS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiFiSettings {
    pub wifi: bool,
    pub ssid: String,
    pub password: String,
    pub telnet: bool,
    pub ftp: bool,
}

impl WiFiSettings {
    pub fn new() -> Self {
        Self {
            wifi: false,
            ssid: String::new(),
            password: String::new(),
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
            enable_wifi()?;
            if self.telnet {
                enable_telnet()?;
            }
            if self.ftp {
                enable_ftp()?;
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        File::create(ALLIUM_WIFI_SETTINGS.as_path())?.write_all(json.as_bytes())?;
        self.update_wpa_supplicant_conf()?;
        Ok(())
    }

    fn load_wpa_supplicant_conf() -> Option<Self> {
        let data = fs::read_to_string("/appconfigs/wpa_supplicant.conf").ok()?;

        let ssid_index = data.find("ssid=\"").unwrap();
        let ssid = &data[ssid_index + 6..];
        let ssid_end = ssid.find('"').unwrap();
        let ssid = &ssid[..ssid_end];

        let psk_index = data.find("psk=\"").unwrap();
        let psk = &data[psk_index + 5..];
        let psk_end = psk.find('"').unwrap();
        let psk = &psk[..psk_end];

        Some(Self {
            wifi: false,
            ssid: ssid.to_string(),
            password: psk.to_string(),
            telnet: false,
            ftp: false,
        })
    }

    fn update_wpa_supplicant_conf(&self) -> Result<()> {
        let mut file = File::create("/appconfigs/wpa_supplicant.conf")?;
        write!(
            file,
            r#"ctrl_interface=/var/run/wpa_supplicant.conf
update_config=1

network={{
	ssid="{ssid}"
	psk="{password}"
}}"#,
            ssid = self.ssid,
            password = self.password
        )?;
        Ok(())
    }

    pub fn toggle_wifi(&mut self) -> Result<()> {
        self.wifi = !self.wifi;
        if self.wifi {
            enable_wifi()?;
            if self.telnet {
                enable_telnet()?;
            }
            if self.ftp {
                enable_ftp()?;
            }
        } else {
            disable_wifi()?;
        }
        Ok(())
    }

    pub fn toggle_telnet(&mut self) -> Result<()> {
        self.telnet = !self.telnet;
        if self.telnet {
            enable_telnet()?;
        } else {
            disable_telnet()?;
        }
        Ok(())
    }

    pub fn toggle_ftp(&mut self) -> Result<()> {
        self.ftp = !self.ftp;
        if self.ftp {
            enable_ftp()?;
        } else {
            disable_ftp()?;
        }
        Ok(())
    }
}

impl Default for WiFiSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub fn enable_wifi() -> Result<()> {
    Command::new(ALLIUM_SCRIPTS_DIR.join("wifi-on.sh")).spawn()?;
    Ok(())
}

pub fn disable_wifi() -> Result<()> {
    Command::new(ALLIUM_SCRIPTS_DIR.join("wifi-off.sh")).spawn()?;
    Ok(())
}

pub fn enable_telnet() -> Result<()> {
    Command::new("telnetd")
        .args(["-l", "sh"])
        .stdout(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn disable_telnet() -> Result<()> {
    Command::new("killall")
        .arg("telnetd")
        .stdout(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn enable_ftp() -> Result<()> {
    Command::new("tcpsvd")
        .args(["-E", "0.0.0.0", "21", "ftpd", "-w", "/mnt/SDCARD"])
        .stdout(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn disable_ftp() -> Result<()> {
    Command::new("killall")
        .arg("ftpd")
        .stdout(Stdio::null())
        .spawn()?;
    Command::new("killall")
        .arg("tcpsvd")
        .stdout(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn ip_address() -> Option<String> {
    let output = Command::new("ip")
        .args(["route", "get", "255.255.255.255"])
        .output()
        .ok()?;
    let output = String::from_utf8(output.stdout).ok()?;
    let ip_address = output.split_whitespace().last().map(|s| s.to_string());

    ip_address.and_then(|addr| {
        addr.split('.')
            .all(|octet| octet.parse::<u8>().is_ok())
            .then_some(addr)
    })
}
