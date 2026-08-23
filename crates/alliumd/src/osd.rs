use std::time::Duration;

use anyhow::Result;
use common::constants::UI_FRAME_INTERVAL;
use common::display::{Display, RectHold, draw_speaker_icon, draw_sun_icon, fill_rounded_rect};
use common::geom::Rect;
use common::platform::Platform;
use common::stylesheet::Stylesheet;
use tokio::time::Instant;

/// How long the indicator stays on screen after the last change
const HIDE_TIMEOUT: Duration = Duration::from_millis(1000);
/// Twice per UI frame, since a launcher redraw flushes the whole screen
const UI_REDRAW_PERIOD: Duration = Duration::from_micros(UI_FRAME_INTERVAL.as_micros() as u64 / 2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdKind {
    Volume,
    Brightness,
}

/// The plate and its contents, placed from the framebuffer size and the theme
#[derive(Clone, Copy)]
struct Plate {
    rect: Rect,
    icon: Rect,
    bar: Rect,
    /// Corner rounding, kept equal to the horizontal padding
    radius: u32,
}

impl Plate {
    fn new<D: Display>(display: &D, styles: &Stylesheet) -> Self {
        let font_size = styles.ui.ui_font.size;
        // Same theme-driven paddings as the launcher's toast popups
        let padding_x = styles.ui.margin_x.max(0) as u32;
        let padding_y = styles.ui.margin_y.max(0) as u32;
        let icon_side = font_size * 2 / 3;

        let bar_w = display.width() / 4;
        let bar_h = font_size / 3;
        let plate_w = padding_x + icon_side + padding_x + bar_w + padding_x;
        let plate_h = font_size + padding_y * 2;

        // Clear the button hint row, positioned by ButtonHints::ensure_layout
        let hint_h = styles.button_size().max(styles.button_hint_font_size()) as u32;
        let bottom_margin = padding_x + hint_h + padding_y;

        let rect = Rect::new(
            (display.width() as i32 - plate_w as i32) / 2,
            display.height() as i32 - (plate_h + bottom_margin) as i32,
            plate_w,
            plate_h,
        );
        Self {
            rect,
            icon: Rect::new(
                rect.x + padding_x as i32,
                rect.y + ((plate_h - icon_side) / 2) as i32,
                icon_side,
                icon_side,
            ),
            bar: Rect::new(
                rect.x + (padding_x + icon_side + padding_x) as i32,
                rect.y + ((plate_h - bar_h) / 2) as i32,
                bar_w,
                bar_h,
            ),
            radius: padding_x,
        }
    }
}

/// How often the plate has to be rewritten to stay on screen
enum Refresh {
    /// Nothing repaints under us: re-flush periodically, restore the background on hide
    Periodic { next_redraw: Instant },
    /// The app rewrites the whole frame ~60/s, so a stamper thread owns the rect
    Continuous(RectHold),
}

/// The framebuffer we draw on, and the plate placed on it
struct Surface<P: Platform> {
    display: P::Display,
    plate: Plate,
}

impl<P: Platform> Surface<P> {
    fn new(platform: &mut P, styles: &Stylesheet) -> Result<Self> {
        // The constructor reads the current frame, so save() keeps the pre-OSD background
        let mut display = platform.display()?;
        display.save()?;
        let plate = Plate::new(&display, styles);
        Ok(Self { display, plate })
    }

    /// Hands the plate to a stamper thread, on platforms that have one
    fn hold_plate(&mut self) -> Result<Option<RectHold>> {
        self.display.hold_rect(self.plate.rect, self.plate.radius)
    }

    fn flush_plate(&mut self) -> Result<()> {
        self.display.flush_rect(self.plate.rect)
    }

    /// Puts the pre-OSD background back where the plate was
    fn restore_plate(&mut self) -> Result<()> {
        self.display.load(self.plate.rect)?;
        self.display.flush_rect(self.plate.rect)
    }

    fn draw(&mut self, styles: &Stylesheet, kind: OsdKind, fraction: f32) -> Result<()> {
        let Plate {
            rect,
            icon,
            bar,
            radius,
        } = self.plate;

        // Repaint over the saved background so a shrinking bar leaves no residue
        self.display.load(rect)?;

        // Unlike ui.background_color, this one is opaque in every bundled theme
        fill_rounded_rect(
            &mut self.display.pixmap_mut(),
            rect,
            radius,
            styles.menu.background_color,
        );

        match kind {
            OsdKind::Volume => {
                draw_speaker_icon(&mut self.display.pixmap_mut(), icon, styles.ui.text_color)
            }
            OsdKind::Brightness => {
                draw_sun_icon(&mut self.display.pixmap_mut(), icon, styles.ui.text_color)
            }
        }

        let bar_radius = bar.h / 2;
        fill_rounded_rect(
            &mut self.display.pixmap_mut(),
            bar,
            bar_radius,
            styles.ui.disabled_color,
        );
        let fill_w = (bar.w as f32 * fraction.clamp(0.0, 1.0)).round() as u32;
        if fill_w > 0 {
            let fill = Rect::new(bar.x, bar.y, fill_w, bar.h);
            fill_rounded_rect(
                &mut self.display.pixmap_mut(),
                fill,
                bar_radius.min(fill_w / 2),
                styles.ui.highlight_color,
            );
        }

        self.display.flush_rect(rect)
    }
}

/// State that exists only while the indicator is on screen
struct Shown<P: Platform> {
    surface: Surface<P>,
    hide_at: Instant,
    refresh: Refresh,
}

/// On-screen indicator drawn over whatever app owns the framebuffer.
pub struct Osd<P: Platform> {
    styles: Stylesheet,
    shown: Option<Shown<P>>,
}

impl<P: Platform> Osd<P> {
    pub fn new(styles: Stylesheet) -> Self {
        Self {
            styles,
            shown: None,
        }
    }

    /// When `tick()` is next due, or `None` while hidden — nothing to wake for.
    pub fn next_wake(&self) -> Option<Instant> {
        self.shown.as_ref().map(|shown| match shown.refresh {
            Refresh::Periodic { next_redraw } => shown.hide_at.min(next_redraw),
            Refresh::Continuous(_) => shown.hide_at,
        })
    }

    pub fn show(
        &mut self,
        platform: &mut P,
        kind: OsdKind,
        fraction: f32,
        repainting: bool,
    ) -> Result<()> {
        // Consuming the old state stops its stamper before the new content is drawn
        let mut surface = match self.shown.take() {
            Some(shown) => shown.surface,
            None => Surface::new(platform, &self.styles)?,
        };

        surface.draw(&self.styles, kind, fraction)?;

        let now = Instant::now();
        let periodic = Refresh::Periodic {
            next_redraw: now + UI_REDRAW_PERIOD,
        };
        self.shown = Some(Shown {
            // Must precede the `surface` move: hold_plate takes it by &mut
            refresh: if repainting {
                match surface.hold_plate()? {
                    Some(hold) => Refresh::Continuous(hold),
                    // Nothing stamps here, so fall back to the static-UI cadence
                    None => periodic,
                }
            } else {
                periodic
            },
            surface,
            hide_at: now + HIDE_TIMEOUT,
        });
        Ok(())
    }

    pub fn tick(&mut self) -> Result<()> {
        let Some(shown) = self.shown.as_mut() else {
            return Ok(());
        };
        let now = Instant::now();
        if now >= shown.hide_at {
            return self.hide();
        }
        if let Refresh::Periodic { next_redraw } = &mut shown.refresh {
            *next_redraw = now + UI_REDRAW_PERIOD;
            shown.surface.flush_plate()?;
        }
        Ok(())
    }

    pub fn hide(&mut self) -> Result<()> {
        let Some(mut shown) = self.shown.take() else {
            return Ok(());
        };
        match shown.refresh {
            // The app repaints the frame itself; only static UI needs the background back
            Refresh::Periodic { .. } => shown.surface.restore_plate()?,
            Refresh::Continuous(hold) => drop(hold),
        }
        Ok(())
    }
}
