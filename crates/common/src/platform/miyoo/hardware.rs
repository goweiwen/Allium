use std::{fmt, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiyooDeviceModel {
    Miyoo283,
    Miyoo354,
}

impl fmt::Display for MiyooDeviceModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MiyooDeviceModel::Miyoo283 => write!(f, "Miyoo Mini (MY283)"),
            MiyooDeviceModel::Miyoo354 => write!(f, "Miyoo Mini+ (MY354)"),
        }
    }
}

pub fn detect_model() -> MiyooDeviceModel {
    if std::path::Path::new("/customer/app/axp_test").exists() {
        MiyooDeviceModel::Miyoo354
    } else {
        MiyooDeviceModel::Miyoo283
    }
}

pub fn detect_firmware() -> String {
    let stdout = Command::new("/etc/fw_printenv").output().unwrap().stdout;
    let stdout = std::str::from_utf8(&stdout).unwrap();
    parse_firmware(stdout).to_string()
}

fn parse_firmware(data: &str) -> &str {
    for line in data.lines() {
        if let Some(firmware) = line.strip_prefix("miyoo_version=") {
            return firmware;
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_firmware() {
        let data = r#"SdUpgradeImage=miyoo354_fw.img
baudrate=115200
bootargs=console=ttyS0,115200 root=/dev/mtdblock4 rootfstype=squashfs ro init=/linuxrc LX_MEM=0x7f00000 mma_heap=mma_heap_name0,miu=0,sz=0x1500000 mma_memblock_remove=1 highres=off mmap_reserved=fb,miu=0,sz=0x300000,max_start_off=0x7C00000,max_end_off=0x7F00000
bootcmd=gpio output 85 1; bootlogo 0 0 0 0 0; mw 1f001cc0 11; gpio out 8 0; sf probe 0;sf read 0x22000000 ${sf_kernel_start} ${sf_kernel_size}; gpio out 8 1; sleepms 1000; gpio output 4 1; bootm 0x22000000
bootdelay=0
cpu_part_start=14270000
dispout=K101_IM2BVL
ethact=sstar_emac
ethaddr=00:30:1b:ba:02:db
filesize=1774c
miyoo_version=202303262339
sf_kernel_size=200000
sf_kernel_start=60000
sf_part_size=20000
sf_part_start=270000
stderr=serial
stdin=serial
stdout=serial
usb_folder=images
"#;
        assert_eq!(parse_firmware(data), "202303262339");
    }
}
