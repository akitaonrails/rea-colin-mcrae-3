//! What the settings window shows and saves, kept apart from the drawing
//! so it can be tested on any host.
//!
//! The window offers one choice first, the look: the original 2003 game,
//! an enhanced preset that fits the player's screen, or custom values. The
//! preset is never stored: the file keeps the individual settings (as the
//! command line and older launchers read them) and the look is worked out
//! from them when the window opens.

use crate::config::{self, DisplayMode, Gamepad, Scaler, Settings};

/// The picture settings the look presets cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    /// `None`: SilentPatch's default, the desktop's size.
    pub resolution: Option<(u32, u32)>,
    /// True when the stored resolution is the keyword `original`
    /// (640×480); kept so saving writes the keyword, not numbers.
    pub original: bool,
    pub display_mode: DisplayMode,
    /// Unset keeps the game's own choice.
    pub vsync: Option<bool>,
    /// Unset keeps the game's own.
    pub anisotropic: Option<u32>,
    pub fsaa: Option<bool>,
    pub fov: Option<u32>,
    pub hd_ui: bool,
}

impl Picture {
    /// The game as it was in 2003: 640×480, everything else the game's.
    pub const ORIGINAL: Picture = Picture {
        resolution: Some((640, 480)),
        original: true,
        display_mode: DisplayMode::Fullscreen,
        vsync: None,
        anisotropic: None,
        fsaa: None,
        fov: None,
        hd_ui: false,
    };

    /// The recommended look for a screen: its own size, fullscreen,
    /// vsync on, 16× anisotropic filtering and the HD UI. Field of view
    /// stays at the game's 75: widescreen already shows more to the sides.
    pub fn enhanced(screen: Option<(u32, u32)>) -> Picture {
        Picture {
            resolution: screen.and_then(landscape),
            original: false,
            display_mode: DisplayMode::Fullscreen,
            vsync: Some(true),
            anisotropic: Some(16),
            fsaa: Some(false),
            fov: Some(75),
            hd_ui: true,
        }
    }

    /// The Steam Deck's screen (1280×800, 16:10), easy on the battery.
    pub const STEAM_DECK: Picture = Picture {
        resolution: Some(STEAM_DECK_SCREEN),
        original: false,
        display_mode: DisplayMode::Fullscreen,
        vsync: Some(true),
        anisotropic: Some(16),
        fsaa: Some(false),
        fov: Some(75),
        hd_ui: true,
    };
}

pub const STEAM_DECK_SCREEN: (u32, u32) = (1280, 800);

/// The look to recommend on a screen: Steam Deck on a Deck, else Enhanced.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if screen.and_then(landscape) == Some(STEAM_DECK_SCREEN) {
        Look::SteamDeck
    } else {
        Look::Enhanced
    }
}

/// A screen size as a landscape display resolution the game can use, or
/// `None` when it is too small to be worth it.
fn landscape((a, b): (u32, u32)) -> Option<(u32, u32)> {
    let (w, h) = (a.max(b) & !1, a.min(b) & !1);
    (w > 640 && h > 480 && w <= 7680 && h <= 4320).then_some((w, h))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Original,
    Enhanced,
    SteamDeck,
    Custom,
}

/// Everything the window edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub saves: String,
    pub picture: Picture,
    /// Remembered so a look picked by name stays picked even when its
    /// values match another (and "Custom" stays open while editing).
    pub look: Look,
    pub gamepad: Gamepad,
    pub dxvk: bool,
    pub scaler: Scaler,
}

/// Field-of-view choices offered (the game's own is 75).
pub const FOVS: [(Option<u32>, &str); 4] = [
    (None, "75 (the game's)"),
    (Some(60), "60"),
    (Some(90), "90"),
    (Some(110), "110"),
];
pub const DISPLAY_MODES: [(DisplayMode, &str); 3] = [
    (DisplayMode::Fullscreen, "Fullscreen"),
    (DisplayMode::Windowed, "Windowed"),
    (DisplayMode::Borderless, "Borderless"),
];
pub const VSYNCS: [(Option<bool>, &str); 3] = [
    (None, "As the game"),
    (Some(true), "On"),
    (Some(false), "Off"),
];
pub const ANISOTROPICS: [(Option<u32>, &str); 5] = [
    (None, "As the game"),
    (Some(0), "Off"),
    (Some(2), "2×"),
    (Some(8), "8×"),
    (Some(16), "16×"),
];
pub const GAMEPADS: [(Gamepad, &str); 3] = [
    (Gamepad::Rally, "Rally"),
    (Gamepad::Standard, "Standard"),
    (Gamepad::Off, "Off"),
];
pub const SCALERS: [(Scaler, &str, &str); 4] = [
    (
        Scaler::Nearest,
        "Sharp",
        "Every game pixel becomes a crisp block.",
    ),
    (
        Scaler::Pixel,
        "Even pixels",
        "Crisp blocks, with only their edges blended so all are the same size.",
    ),
    (
        Scaler::Fsr,
        "FSR",
        "AMD FSR: smooth, then sharpened. Softer than Sharp.",
    ),
    (
        Scaler::Linear,
        "Smooth",
        "Plain smoothing. The softest look.",
    ),
];

/// What a gamepad does in the settings window, from the XInput buttons
/// pressed since the last look (`wButtons` bits): Start plays, the d-pad
/// moves between controls (as Tab and Shift+Tab do), A presses the one in
/// focus (as Space does), LB and RB change tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadAction {
    Play,
    Next,
    Previous,
    Press,
    NextTab,
    PreviousTab,
}

pub const PAD_UP: u16 = 0x0001;
pub const PAD_DOWN: u16 = 0x0002;
pub const PAD_LEFT: u16 = 0x0004;
pub const PAD_RIGHT: u16 = 0x0008;
pub const PAD_START: u16 = 0x0010;
pub const PAD_LB: u16 = 0x0100;
pub const PAD_RB: u16 = 0x0200;
pub const PAD_A: u16 = 0x1000;

/// The actions of the buttons in `now` that were not held in `before`.
pub fn pad_actions(before: u16, now: u16) -> Vec<PadAction> {
    let pressed = now & !before;
    [
        (PAD_START, PadAction::Play),
        (PAD_DOWN, PadAction::Next),
        (PAD_RIGHT, PadAction::Next),
        (PAD_UP, PadAction::Previous),
        (PAD_LEFT, PadAction::Previous),
        (PAD_A, PadAction::Press),
        (PAD_RB, PadAction::NextTab),
        (PAD_LB, PadAction::PreviousTab),
    ]
    .into_iter()
    .filter(|(bit, _)| pressed & bit != 0)
    .map(|(_, action)| action)
    .collect()
}

/// The settings window's design size in points.
pub const WINDOW: (f32, f32) = (880.0, 640.0);

/// How much larger to draw the window on a screen (`screen` in pixels,
/// `native` the system's own scale, 1.0 under Wine unless its DPI is set):
/// 1x up to 1440 pixels on the shorter side, then in step with it (1.5x at
/// 4K); never below the system's scale and never larger than 90 % of the
/// screen. In steps of 0.05.
pub fn window_zoom(screen: Option<(u32, u32)>, native: f32) -> f32 {
    let native = if native > 0.0 { native } else { 1.0 };
    let Some((w, h)) = screen else {
        return 1.0;
    };
    let (w, h) = (w as f32, h as f32);
    let fit = 0.9 * (w / WINDOW.0).min(h / WINDOW.1);
    let target = (w.min(h) / 1440.0).max(1.0).min(fit);
    let zoom = (target / native).max(1.0);
    (zoom * 20.0).round() / 20.0
}

/// Display resolutions offered: the screen's, the common sizes, and the
/// original 640×480.
pub fn resolutions(screen: Option<(u32, u32)>) -> Vec<(Option<(u32, u32)>, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut out = Vec::new();
    if let Some(s) = screen.and_then(landscape)
        && !common.contains(&s)
        && s != (640, 480)
    {
        out.push((Some(s), format!("Screen {}×{}", s.0, s.1)));
    }
    for (w, h) in common {
        let label = match h {
            1080 => "1080p",
            1440 => "1440p",
            _ => "4K",
        };
        out.push((Some((w, h)), label.to_string()));
    }
    out.push((Some((640, 480)), "Original 640×480".to_string()));
    out
}

impl Form {
    /// The window's starting state for saved settings on a screen.
    pub fn new(s: &Settings, screen: Option<(u32, u32)>) -> Form {
        let path = |p: &Option<std::path::PathBuf>| {
            p.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        };
        let picture = Picture {
            resolution: s.resolution,
            original: s.original_resolution,
            display_mode: s.display_mode.unwrap_or(DisplayMode::Fullscreen),
            vsync: s.vsync,
            anisotropic: s.anisotropic,
            fsaa: s.fsaa,
            fov: s.fov,
            hd_ui: s.hd_ui == Some(true),
        };
        Form {
            game: path(&s.game),
            saves: path(&s.saves),
            picture,
            look: Form::look_of(&picture, screen),
            gamepad: s.gamepad.unwrap_or(Gamepad::Rally),
            dxvk: s.dxvk != Some(false),
            scaler: s.scaler.unwrap_or(Scaler::Linear),
        }
    }

    /// The form for a first run (no settings file yet): the Enhanced look
    /// fitted to the screen, with the Steam Deck look on a Deck (owner
    /// decision — the goal is a modern, widescreen, HD experience with no
    /// settings touched).
    pub fn first_run(screen: Option<(u32, u32)>) -> Form {
        let (settings, _) = Settings::parse_file(config::TEMPLATE);
        let mut form = Form::new(&settings, screen);
        form.set_look(recommended(screen), screen);
        form
    }

    /// The look these picture values make on a screen.
    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        if *picture == Picture::ORIGINAL {
            Look::Original
        } else if *picture == Picture::enhanced(screen) {
            Look::Enhanced
        } else if *picture == Picture::STEAM_DECK {
            Look::SteamDeck
        } else {
            Look::Custom
        }
    }

    /// Picks a look; Original, Enhanced and Steam Deck set the picture,
    /// Custom keeps the current values to edit.
    pub fn set_look(&mut self, look: Look, screen: Option<(u32, u32)>) {
        self.look = look;
        match look {
            Look::Original => self.picture = Picture::ORIGINAL,
            Look::Enhanced => self.picture = Picture::enhanced(screen),
            Look::SteamDeck => self.picture = Picture::STEAM_DECK,
            Look::Custom => {}
        }
    }

    /// The settings file with this form's values written in; every other
    /// line (comments, keys the window doesn't show) is kept.
    pub fn save(&self, text: &str) -> String {
        let p = &self.picture;
        let mut t = text.to_string();
        let mut put = |key: &str, value: Option<String>| {
            t = match &value {
                Some(v) => config::set_key(&t, key, v),
                None => config::clear_key(&t, key),
            };
        };
        let path = |s: &str| Some(s.trim().to_string()).filter(|s| !s.is_empty());
        put("game", path(&self.game));
        put("saves", path(&self.saves));
        put(
            "resolution",
            p.resolution.map(|(w, h)| {
                if p.original {
                    "original".into()
                } else {
                    format!("{w}x{h}")
                }
            }),
        );
        put(
            "display_mode",
            (p.display_mode != DisplayMode::Fullscreen).then(|| p.display_mode.key().into()),
        );
        put("vsync", p.vsync.map(|v| v.to_string()));
        put(
            "anisotropic",
            p.anisotropic.map(|a| match a {
                0 => "off".into(),
                n => n.to_string(),
            }),
        );
        put(
            "fsaa",
            p.fsaa.map(|b| if b { "on".into() } else { "off".into() }),
        );
        put("fov", p.fov.map(|f| f.to_string()));
        put(
            "hd_ui",
            p.hd_ui.then(|| "on".into()), // off by default
        );
        put(
            "gamepad",
            (self.gamepad != Gamepad::Rally).then(|| self.gamepad.key().into()),
        );
        put("dxvk", (!self.dxvk).then(|| "off".into()));
        put("scaler", Some(self.scaler.key().into()));
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(u32, u32)> = Some((2560, 1440));

    fn form(text: &str) -> Form {
        Form::new(&Settings::parse_file(text).0, SCREEN)
    }

    #[test]
    fn a_first_run_starts_with_the_enhanced_look() {
        let f = Form::first_run(SCREEN);
        assert_eq!(f.look, Look::Enhanced);
        assert_eq!(f.picture.resolution, Some((2560, 1440)));
        assert_eq!(f.picture.anisotropic, Some(16));
        assert!(f.picture.hd_ui);
        assert_eq!(f.gamepad, Gamepad::Rally);
        assert_eq!(
            Form::first_run(Some(STEAM_DECK_SCREEN)).look,
            Look::SteamDeck
        );
    }

    #[test]
    fn the_look_is_worked_out_from_the_saved_values() {
        let mut f = form(config::TEMPLATE);
        f.set_look(Look::Enhanced, SCREEN);
        let saved = f.save(config::TEMPLATE);
        assert_eq!(form(&saved).look, Look::Enhanced);
        let tweaked = config::set_key(&saved, "anisotropic", "8");
        assert_eq!(form(&tweaked).look, Look::Custom);
        let mut g = form(&saved);
        g.set_look(Look::Original, SCREEN);
        let back = g.save(&saved);
        assert_eq!(form(&back).look, Look::Original);
        let (s, warnings) = Settings::parse_file(&back);
        assert!(warnings.is_empty());
        assert_eq!(s.resolution, Some((640, 480)));
        assert!(s.original_resolution);
        assert_eq!(s.hd_ui, None, "off by default");
    }

    #[test]
    fn enhanced_fits_the_screen() {
        assert_eq!(
            Picture::enhanced(Some((2560, 1440))).resolution,
            Some((2560, 1440))
        );
        assert_eq!(
            Picture::enhanced(Some((1440, 2560))).resolution,
            Some((2560, 1440)),
            "a rotated monitor still gives a landscape display"
        );
        assert_eq!(Picture::enhanced(None).resolution, None);
        assert_eq!(Picture::enhanced(Some((640, 480))).resolution, None);
        assert_eq!(
            Picture::enhanced(Some((1366, 769))).resolution,
            Some((1366, 768))
        );
    }

    #[test]
    fn saving_round_trips_every_field() {
        let mut f = form(config::TEMPLATE);
        f.game = r"D:\Games\CMR3".into();
        f.saves = r"Z:\home\me\saves".into();
        f.picture = Picture {
            resolution: Some((3840, 2160)),
            original: false,
            display_mode: DisplayMode::Borderless,
            vsync: Some(false),
            anisotropic: Some(8),
            fsaa: Some(true),
            fov: Some(90),
            hd_ui: false,
        };
        f.gamepad = Gamepad::Standard;
        f.dxvk = false;
        f.scaler = Scaler::Fsr;
        let saved = f.save(config::TEMPLATE);
        assert!(saved.contains("# Colin McRae Rally 3 launcher settings."));
        let mut back = form(&saved);
        assert_eq!(back.look, Look::Custom);
        back.look = f.look;
        assert_eq!(back, f);
        let (_, warnings) = Settings::parse_file(&saved);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn defaults_round_trip_to_an_empty_file() {
        let f = form(config::TEMPLATE);
        let saved = f.save(config::TEMPLATE);
        // An empty file is neither preset (the window's first run picks
        // one); it keeps SilentPatch's own defaults, which start at the
        // desktop's size.
        assert_eq!(form(&saved).look, Look::Custom);
        let (_, warnings) = Settings::parse_file(&saved);
        assert!(warnings.is_empty());
        // Every default writes as little as possible: only active line is
        // the scaler.
        let active: Vec<_> = saved
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && l.contains('='))
            .collect();
        assert_eq!(active, ["scaler = linear"]);
    }

    #[test]
    fn the_window_grows_with_the_screen() {
        assert_eq!(window_zoom(Some((1920, 1080)), 1.0), 1.0);
        assert_eq!(window_zoom(Some((3840, 2160)), 1.0), 1.5);
        assert_eq!(window_zoom(Some((3840, 2160)), 1.5), 1.0);
        assert_eq!(window_zoom(Some((1280, 800)), 1.0), 1.0);
        assert_eq!(window_zoom(None, 1.0), 1.0);
    }

    #[test]
    fn resolutions_list_the_screen_once() {
        let r = resolutions(Some((2560, 1600)));
        assert_eq!(r[0], (Some((2560, 1600)), "Screen 2560×1600".to_string()));
        assert!(
            resolutions(None)
                .iter()
                .any(|(r, _)| *r == Some((640, 480)))
        );
        let listed = resolutions(Some((1920, 1080)));
        assert_eq!(
            listed
                .iter()
                .filter(|(r, _)| *r == Some((1920, 1080)))
                .count(),
            1
        );
    }

    #[test]
    fn gamepad_buttons_act_once_per_press() {
        assert_eq!(pad_actions(0, PAD_START), [PadAction::Play]);
        assert_eq!(pad_actions(PAD_START, PAD_START), []);
        assert_eq!(
            pad_actions(0, PAD_DOWN | PAD_A),
            [PadAction::Next, PadAction::Press]
        );
        assert_eq!(pad_actions(0, PAD_RB), [PadAction::NextTab]);
        assert_eq!(pad_actions(PAD_A, 0), []);
    }
}
