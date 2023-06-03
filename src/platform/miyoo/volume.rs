// Adapted from OnionOS: https://github.com/OnionUI/Onion/blob/main/src/common/system/volume.h

use std::ffi::c_ulong;
use std::mem::{self, size_of};
use std::os::fd::AsRawFd;
use std::{ffi::c_int, fs::File};

use anyhow::Result;
use nix::{ioctl_read, ioctl_readwrite, ioctl_write_ptr};
use tracing::{debug, trace};

const MAX_VOLUME: i32 = 20;
const MIN_RAW_VALUE: i32 = -60;
const MAX_RAW_VALUE: i32 = 30;

// const MI_AO_GETVOLUME: u32 = 0xc008690c;
// const MI_AO_SETVOLUME: u32 = 0x4008690b;
// const MI_AO_SETMUTE: u32 = 0x4008690d;

const MI_IOC_MAGIC: u8 = b'i';
const MI_AO_SETPUBATTR: c_ulong = 0;
const MI_AO_GETPUBATTR: c_ulong = 1;
const MI_AO_ENABLE: c_ulong = 2;
const MI_AO_DISABLE: c_ulong = 3;
const MI_AO_ENABLECHN: c_ulong = 4;
const MI_AO_DISABLECHN: c_ulong = 5;
const MI_AO_PAUSECHN: c_ulong = 7;
const MI_AO_RESUMECHN: c_ulong = 8;
const MI_AO_CLEARCHNBUF: c_ulong = 9;
const MI_AO_QUERYCHNSTAT: c_ulong = 10;
const MI_AO_SETVOLUME: c_ulong = 11;
const MI_AO_GETVOLUME: c_ulong = 12;
const MI_AO_SETMUTE: c_ulong = 13;
const MI_AO_GETMUTE: c_ulong = 14;
const MI_AO_CLEARPUBATTR: c_ulong = 15;
const MI_AO_INIT: c_ulong = 20;
const MI_AO_DEINIT: c_ulong = 21;

ioctl_readwrite!(mi_ao_getvolume, MI_IOC_MAGIC, MI_AO_GETVOLUME, MiPrm);
ioctl_write_ptr!(mi_ao_setvolume, MI_IOC_MAGIC, MI_AO_SETVOLUME, MiPrm);
ioctl_write_ptr!(mi_ao_setmute, MI_IOC_MAGIC, MI_AO_SETMUTE, MiPrm);

#[repr(C)]
#[derive(Debug)]
pub struct MiPrm {
    size: c_ulong,
    offset: c_ulong,
}

#[repr(C)]
#[derive(Debug)]
pub struct MiaoVol {
    dev: c_int,
    volume: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct MiaoMute {
    dev: c_int,
    enable: c_int,
}

/// Set volume output between 0 and 60, use `add` for boosting
fn set_volume_raw(mut volume: i32, add: i32) -> Result<()> {
    let miao = File::open("/dev/mi_ao")?;

    let mut miao_vol = MiaoVol { dev: 0, volume: 0 };
    let mut miao_prm_vol = MiPrm {
        size: size_of::<MiaoVol>() as c_ulong,
        offset: &miao_vol as *const _ as c_ulong,
    };
    unsafe {
        mi_ao_getvolume(miao.as_raw_fd(), &mut miao_prm_vol)?;
    }
    let prev_volume = miao_vol.volume;
    trace!("prev volume: {:?}", miao_vol);

    volume = if add != 0 {
        prev_volume + add
    } else {
        volume + MIN_RAW_VALUE
    };

    volume = volume.clamp(MIN_RAW_VALUE, MAX_RAW_VALUE);

    if volume == prev_volume {
        return Ok(());
    }

    miao_vol.volume = volume;
    unsafe {
        mi_ao_setvolume(miao.as_raw_fd(), &miao_prm_vol as *const _)?;
    }
    debug!("set raw volume: {}", miao_vol.volume);

    // Ensure these are not dropped earlier
    mem::drop(miao_vol);
    mem::drop(miao_prm_vol);

    if prev_volume <= MIN_RAW_VALUE && volume > MIN_RAW_VALUE {
        let miao_mute = MiaoMute { dev: 0, enable: 0 };
        let miao_prm_mute = MiPrm {
            size: size_of::<MiaoMute>() as c_ulong,
            offset: &miao_mute as *const _ as c_ulong,
        };
        debug!("mute: OFF");
        unsafe {
            mi_ao_setmute(miao.as_raw_fd(), &miao_prm_mute)?;
        }
        mem::drop(miao_mute);
        mem::drop(miao_prm_mute);
    } else if prev_volume > MIN_RAW_VALUE && volume <= MIN_RAW_VALUE {
        let miao_mute = MiaoMute { dev: 0, enable: 1 };
        let miao_prm_mute = MiPrm {
            size: size_of::<MiaoMute>() as c_ulong,
            offset: &miao_mute as *const _ as c_ulong,
        };
        debug!("mute: ON");
        unsafe {
            mi_ao_setmute(miao.as_raw_fd(), &miao_prm_mute)?;
        }
        mem::drop(miao_mute);
        mem::drop(miao_prm_mute);
    }

    Ok(())
}

pub fn set_volume(volume: i32) -> Result<()> {
    let mut volume_raw = 0;
    let volume = volume.clamp(0, MAX_VOLUME);
    if volume != 0 {
        volume_raw = (48.0 * (1.0 + volume as f32).log10()).round() as i32; // see volume curve below
    }
    debug!("set volume: {}", volume_raw);
    set_volume_raw(volume_raw, 0)?;
    Ok(())
}

/*
VOLUME CURVE (later corrected to rise to 63)

         60 :                    .                    .                    .       ..............
            .                                                             .............
            .                                                    ............
            .                                             ..........
            .                                      ...::....
            .                                 ....... .
            .                            .......
         40 :                    .  ..::...           .                    .                    .
            .                    .:...
    RAW     .                 .:...
   VALUE    .              .:..
            .           .::.
            .         .::
            .       .:.
         20 :      ::.           .                    .                    .                    .
            .     :.
            .   .:
            .  .:
            .  :
            ..:.
            ...
          0 :....................:....................:....................:....................:.
            0                    5                   10                   15                   20

                                                   VOLUME
*/
