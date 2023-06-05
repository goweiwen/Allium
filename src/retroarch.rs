use anyhow::Result;
use tokio::net::UdpSocket;

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
}

impl RetroArchCommand {
    pub async fn send(&self) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(RETROARCH_UDP_SOCKET).await?;
        socket.send(self.as_bytes()).await?;
        Ok(())
    }

    fn as_bytes(&self) -> &[u8] {
        match self {
            RetroArchCommand::FastForward => b"FAST_FORWARD",
            RetroArchCommand::FastForwardHold => b"FAST_FORWARD_HOLD",
            RetroArchCommand::LoadState => b"LOAD_STATE",
            RetroArchCommand::SaveState => b"SAVE_STATE",
            RetroArchCommand::FullscreenToggle => b"FULLSCREEN_TOGGLE",
            RetroArchCommand::Quit => b"QUIT",
            RetroArchCommand::StateSlotPlus => b"STATE_SLOT_PLUS",
            RetroArchCommand::StateSlotMinus => b"STATE_SLOT_MINUS",
            RetroArchCommand::Rewind => b"REWIND",
            RetroArchCommand::MovieRecordToggle => b"MOVIE_RECORD_TOGGLE",
            RetroArchCommand::PauseToggle => b"PAUSE_TOGGLE",
            RetroArchCommand::FrameAdvance => b"FRAMEADVANCE",
            RetroArchCommand::Reset => b"RESET",
            RetroArchCommand::ShaderNext => b"SHADER_NEXT",
            RetroArchCommand::ShaderPrev => b"SHADER_PREV",
            RetroArchCommand::CheatIndexPlus => b"CHEAT_INDEX_PLUS",
            RetroArchCommand::CheatIndexMinus => b"CHEAT_INDEX_MINUS",
            RetroArchCommand::CheatToggle => b"CHEAT_TOGGLE",
            RetroArchCommand::Screenshot => b"SCREENSHOT",
            RetroArchCommand::Mute => b"MUTE",
            RetroArchCommand::NetplayFlip => b"NETPLAY_FLIP",
            RetroArchCommand::SlowMotion => b"SLOWMOTION",
            RetroArchCommand::VolumeUp => b"VOLUME_UP",
            RetroArchCommand::VolumeDown => b"VOLUME_DOWN",
            RetroArchCommand::OverlayNext => b"OVERLAY_NEXT",
            RetroArchCommand::DiskEjectToggle => b"DISK_EJECT_TOGGLE",
            RetroArchCommand::DiskNext => b"DISK_NEXT",
            RetroArchCommand::DiskPrev => b"DISK_PREV",
            RetroArchCommand::GrabMouseToggle => b"GRAB_MOUSE_TOGGLE",
            RetroArchCommand::MenuToggle => b"MENU_TOGGLE",
        }
    }
}
