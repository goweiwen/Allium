use log::debug;

pub fn load_fonts() {
    debug!("loading fonts");
    let mut font_system = iced_graphics::text::font_system().write().unwrap();
    font_system
        .raw()
        .db_mut()
        .load_fonts_dir("/mnt/SDCARD/.allium/fonts");
    font_system.raw().db_mut().set_sans_serif_family("Nunito");
}
