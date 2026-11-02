//! The settings window's look: CMR3's colours for the SDK's colour roles,
//! its design size and its decal. The drawing itself is
//! `rea_launcher_ui`; this is game data, kept here so it is tested on the
//! host.
//!
//! The CMR3-era crimson branding — this launcher's own colour next to the
//! 1998 game's rally blue, both with the one gold accent for whatever is
//! chosen and a condensed italic title like a door decal, so the two read
//! as one collection. Gold is both the action colour (Play, the tab
//! underline) and the selection colour.

use eframe::egui::{Color32, Vec2, vec2};
use rea_launcher_ui::theme::Palette;

pub const BASE: Color32 = Color32::from_rgb(0x54, 0x10, 0x1c);
pub const PANEL: Color32 = Color32::from_rgb(0x69, 0x18, 0x2a);
pub const LINE: Color32 = Color32::from_rgb(0x8a, 0x2a, 0x42);
pub const GOLD: Color32 = Color32::from_rgb(0xf5, 0xc4, 0x00);
pub const GOLD_HOVER: Color32 = Color32::from_rgb(0xff, 0xd3, 0x2a);
pub const INK: Color32 = Color32::from_rgb(0xf7, 0xf0, 0xf2);
pub const MUTED: Color32 = Color32::from_rgb(0xd6, 0xaa, 0xb6);
pub const ON_GOLD: Color32 = Color32::from_rgb(0x2b, 0x0d, 0x18);
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x8a, 0x80);

pub const PALETTE: Palette = Palette {
    base: BASE,
    panel: PANEL,
    line: LINE,
    ink: INK,
    muted: MUTED,
    action: GOLD,
    action_hover: GOLD_HOVER,
    on_action: ON_GOLD,
    select: GOLD,
    on_select: ON_GOLD,
    error: ERROR,
    // Nothing in this window is ever disabled; dimmed crimson all the same.
    disabled_ink: Color32::from_rgb(0x9a, 0x62, 0x72),
    disabled_line: Color32::from_rgb(0x6a, 0x22, 0x34),
    focus: INK,
};

/// The settings window's design size in points.
pub const SIZE: Vec2 = vec2(880.0, 704.0);

/// The width of the portrait strip on the left.
pub const PHOTO_W: f32 = 300.0;

/// The two slanted gold livery stripes in the top right corner.
pub fn stripes() -> [(f32, Color32); 2] {
    [(30.0, GOLD), (16.0, GOLD.gamma_multiply(0.65))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_ui::window::creation_zoom;

    #[test]
    fn the_palette_reads_well() {
        assert_eq!(PALETTE.contrast_problems(), Vec::<String>::new());
    }

    #[test]
    fn under_wine_the_window_is_created_at_its_final_size() {
        // Wine's default 96 DPI: 1.5x at 4K, 1x at 1080p.
        assert_eq!(creation_zoom(SIZE, true, Some((3840, 2160)), 1.0), 1.5);
        assert_eq!(creation_zoom(SIZE, true, Some((1920, 1080)), 1.0), 1.0);
        // Wine at 144 or 192 DPI already scales the window: never zoomed
        // on top of that, and never taller than the screen.
        assert_eq!(creation_zoom(SIZE, true, Some((3840, 2160)), 1.5), 1.0);
        for dpi in [96.0, 120.0, 144.0, 192.0] {
            let native = dpi / 96.0;
            let zoom = creation_zoom(SIZE, true, Some((3840, 2160)), native);
            assert!(SIZE.y * zoom * native <= 2160.0, "{dpi} DPI");
        }
        // Windows: created at 1x, fitted on the first frame.
        assert_eq!(creation_zoom(SIZE, false, Some((3840, 2160)), 1.5), 1.0);
    }
}
