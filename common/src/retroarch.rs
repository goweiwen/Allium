use std::{borrow::Cow, time::Duration};

use anyhow::Result;
use log::{debug, error, trace};
use tokio::{net::UdpSocket, select};

use crate::constants::RETROARCH_UDP_SOCKET;

#[allow(unused)]
#[derive(Debug)]
pub enum RetroArchCommand {
    FastForward,
    FastForwardHold,
    LoadState,
    SaveState,
    FullscreenToggle,
    Quit,
    StateSlotPlus,
    StateSlotMinus,
    Rewind,
    MovieRecordToggle,
    PauseToggle,
    FrameAdvance,
    Reset,
    ShaderNext,
    ShaderPrev,
    CheatIndexPlus,
    CheatIndexMinus,
    CheatToggle,
    Screenshot,
    Mute,
    NetplayFlip,
    SlowMotion,
    VolumeUp,
    VolumeDown,
    OverlayNext,
    DiskEjectToggle,
    DiskNext,
    DiskPrev,
    GrabMouseToggle,
    MenuToggle,
    GetDiskSlotCount,
    GetDiskSlot,
    SetDiskSlot(u8),
    GetStateSlot,
    SetStateSlot(u8),
    SaveStateSlot(u8),
    LoadStateSlot(u8),
}

impl RetroArchCommand {
    pub async fn send(&self) -> Result<()> {
        debug!("Sending RetroArch command: {}", self.as_str());
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        trace!("Bound UDP socket: {}", socket.local_addr()?);
        socket.connect(RETROARCH_UDP_SOCKET).await?;
        trace!(
            "Connecting to RetroArch UDP socket: {}",
            RETROARCH_UDP_SOCKET
        );
        socket.send(self.as_str().as_bytes()).await?;
        Ok(())
    }

    pub async fn send_recv(&self) -> Result<Option<String>> {
        debug!("Sending and awaiting RetroArch command: {}", self.as_str(),);
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        trace!("Bound UDP socket: {}", socket.local_addr()?);
        socket.connect(RETROARCH_UDP_SOCKET).await?;
        trace!(
            "Connecting to RetroArch UDP socket: {}",
            RETROARCH_UDP_SOCKET
        );
        socket.send(self.as_str().as_bytes()).await?;
        let mut reply = Vec::with_capacity(128);
        select! {
            ret = socket.recv(&mut reply) => if let Err(e) = ret {
                error!("Failed to receive reply from RetroArch: {}", e);
            },
            _ = tokio::time::sleep(Duration::from_millis(100)) => {},
        }
        Ok(Some(String::from_utf8(reply)?))
    }

    fn as_str(&self) -> Cow<'static, str> {
        match self {
            RetroArchCommand::FastForward => Cow::Borrowed("FAST_FORWARD"),
            RetroArchCommand::FastForwardHold => Cow::Borrowed("FAST_FORWARD_HOLD"),
            RetroArchCommand::LoadState => Cow::Borrowed("LOAD_STATE"),
            RetroArchCommand::SaveState => Cow::Borrowed("SAVE_STATE"),
            RetroArchCommand::FullscreenToggle => Cow::Borrowed("FULLSCREEN_TOGGLE"),
            RetroArchCommand::Quit => Cow::Borrowed("QUIT"),
            RetroArchCommand::StateSlotPlus => Cow::Borrowed("STATE_SLOT_PLUS"),
            RetroArchCommand::StateSlotMinus => Cow::Borrowed("STATE_SLOT_MINUS"),
            RetroArchCommand::Rewind => Cow::Borrowed("REWIND"),
            RetroArchCommand::MovieRecordToggle => Cow::Borrowed("MOVIE_RECORD_TOGGLE"),
            RetroArchCommand::PauseToggle => Cow::Borrowed("PAUSE_TOGGLE"),
            RetroArchCommand::FrameAdvance => Cow::Borrowed("FRAMEADVANCE"),
            RetroArchCommand::Reset => Cow::Borrowed("RESET"),
            RetroArchCommand::ShaderNext => Cow::Borrowed("SHADER_NEXT"),
            RetroArchCommand::ShaderPrev => Cow::Borrowed("SHADER_PREV"),
            RetroArchCommand::CheatIndexPlus => Cow::Borrowed("CHEAT_INDEX_PLUS"),
            RetroArchCommand::CheatIndexMinus => Cow::Borrowed("CHEAT_INDEX_MINUS"),
            RetroArchCommand::CheatToggle => Cow::Borrowed("CHEAT_TOGGLE"),
            RetroArchCommand::Screenshot => Cow::Borrowed("SCREENSHOT"),
            RetroArchCommand::Mute => Cow::Borrowed("MUTE"),
            RetroArchCommand::NetplayFlip => Cow::Borrowed("NETPLAY_FLIP"),
            RetroArchCommand::SlowMotion => Cow::Borrowed("SLOWMOTION"),
            RetroArchCommand::VolumeUp => Cow::Borrowed("VOLUME_UP"),
            RetroArchCommand::VolumeDown => Cow::Borrowed("VOLUME_DOWN"),
            RetroArchCommand::OverlayNext => Cow::Borrowed("OVERLAY_NEXT"),
            RetroArchCommand::DiskEjectToggle => Cow::Borrowed("DISK_EJECT_TOGGLE"),
            RetroArchCommand::DiskNext => Cow::Borrowed("DISK_NEXT"),
            RetroArchCommand::DiskPrev => Cow::Borrowed("DISK_PREV"),
            RetroArchCommand::GrabMouseToggle => Cow::Borrowed("GRAB_MOUSE_TOGGLE"),
            RetroArchCommand::MenuToggle => Cow::Borrowed("MENU_TOGGLE"),
            RetroArchCommand::GetDiskSlotCount => Cow::Borrowed("GET_DISK_SLOT_COUNT"),
            RetroArchCommand::GetDiskSlot => Cow::Borrowed("GET_DISK_SLOT"),
            RetroArchCommand::SetDiskSlot(slot) => Cow::Owned(format!("SET_DISK_SLOT {slot}")),
            RetroArchCommand::GetStateSlot => Cow::Borrowed("GET_STATE_SLOT"),
            RetroArchCommand::SetStateSlot(slot) => Cow::Owned(format!("SET_STATE_SLOT {slot}")),
            RetroArchCommand::SaveStateSlot(slot) => Cow::Owned(format!("SAVE_STATE_SLOT {slot}")),
            RetroArchCommand::LoadStateSlot(slot) => Cow::Owned(format!("LOAD_STATE_SLOT {slot}")),
        }
    }
}
