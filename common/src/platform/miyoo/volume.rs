// Adapted from OnionOS: https://github.com/OnionUI/Onion/blob/main/src/common/system/volume.h

use std::ffi::c_ulong;
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::{ffi::c_int, fs::File};

use anyhow::Result;
use nix::{ioctl_readwrite, ioctl_write_ptr};
use tracing::debug;

const MAX_VOLUME: i32 = 20;
const MIN_RAW_VALUE: i32 = -60;
const MAX_RAW_VALUE: i32 = 30;

// const MI_AO_GETVOLUME: u32 = 0xc008690c;
// const MI_AO_SETVOLUME: u32 = 0x4008690b;
// const MI_AO_SETMUTE: u32 = 0x4008690d;

const MI_IOC_MAGIC: u8 = b'i';

#[allow(unused)]
#[derive(Debug)]
enum MiAoFunction {
    SetPubAttr = 0,
    GetPubAttr = 1,
    Enable = 2,
    Disable = 3,
    EnableChn = 4,
    DisableChn = 5,
    PauseChn = 7,
    ResumeChn = 8,
    ClearChnBuf = 9,
    QueryChnStat = 10,
    SetVolume = 11,
    GetVolume = 12,
    SetMute = 13,
    GetMute = 14,
    ClearPubAttr = 15,
    Init = 20,
    Deinit = 21,
}

ioctl_readwrite!(
    mi_ao_getvolume,
    MI_IOC_MAGIC,
    MiAoFunction::GetVolume as c_ulong,
    MiPrm
);
ioctl_write_ptr!(
    mi_ao_setvolume,
    MI_IOC_MAGIC,
    MiAoFunction::SetVolume as c_ulong,
    MiPrm
);
ioctl_write_ptr!(
    mi_ao_setmute,
    MI_IOC_MAGIC,
    MiAoFunction::SetMute as c_ulong,
    MiPrm
);

#[repr(C)]
#[derive(Debug)]
pub struct MiPrm {
    size: c_ulong,
    offset: c_ulong,
}

#[repr(C)]
#[derive(Debug)]
pub struct MiAoVol {
    dev: c_int,
    volume: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct MiAoMute {
    dev: c_int,
    enable: c_int,
}

/// Set volume output between 0 and 60, use `add` for boosting
fn set_volume_raw(mut volume: i32, add: i32) -> Result<()> {
    let mi_ao = File::open("/dev/mi_ao")?;
    let mi_ao_fd = mi_ao.as_raw_fd();

    let mut mi_ao_vol = MiAoVol { dev: 0, volume: 0 };
    let mut mi_prm_vol = MiPrm {
        size: size_of::<MiAoVol>() as c_ulong,
        offset: &mi_ao_vol as *const _ as c_ulong,
    };
    // TODO: figure out why these traces are needed... something to do IO on stdout or some delay?
    tracing::debug!(
        "miao_vol (size: {}): {:?}",
        size_of::<MiAoVol>(),
        &mi_ao_vol
    );
    tracing::debug!(
        "miao_prm_vol (size: {}): {:?}",
        size_of::<MiPrm>(),
        &mi_prm_vol
    );
    unsafe {
        mi_ao_getvolume(mi_ao_fd, &mut mi_prm_vol)?;
    }
    let prev_volume = mi_ao_vol.volume;
    tracing::debug!("prev volume: {:?}", mi_ao_vol);

    volume = if add != 0 {
        prev_volume + add
    } else {
        volume + MIN_RAW_VALUE
    };

    volume = volume.clamp(MIN_RAW_VALUE, MAX_RAW_VALUE);

    if volume == prev_volume {
        return Ok(());
    }

    mi_ao_vol.volume = volume;
    unsafe {
        mi_ao_setvolume(mi_ao_fd, &mi_prm_vol as *const _)?;
    }
    tracing::debug!("set raw volume: {}", mi_ao_vol.volume);

    if prev_volume <= MIN_RAW_VALUE && volume > MIN_RAW_VALUE {
        let mi_ao_mute = MiAoMute { dev: 0, enable: 0 };
        let mi_prm_mute = MiPrm {
            size: size_of::<MiAoMute>() as c_ulong,
            offset: &mi_ao_mute as *const _ as c_ulong,
        };
        unsafe {
            mi_ao_setmute(mi_ao_fd, &mi_prm_mute)?;
        }
        debug!("mute: OFF");
    } else if prev_volume > MIN_RAW_VALUE && volume <= MIN_RAW_VALUE {
        let mi_ao_mute = MiAoMute { dev: 0, enable: 1 };
        let mi_prm_mute = MiPrm {
            size: size_of::<MiAoMute>() as c_ulong,
            offset: &mi_ao_mute as *const _ as c_ulong,
        };
        unsafe {
            mi_ao_setmute(mi_ao_fd, &mi_prm_mute)?;
        }
        debug!("mute: ON");
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
