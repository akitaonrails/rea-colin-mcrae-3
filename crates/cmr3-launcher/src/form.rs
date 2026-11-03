//! What the settings window shows and saves, kept apart from the drawing
//! so it can be tested on any host.
//!
//! The window offers one choice first, the look: the original 2003 game,
//! an enhanced preset that fits the player's screen, the Steam Deck's, or
//! custom values. The window never stores the preset: the file keeps the
//! individual settings (as older launchers read them) and the look is
//! worked out from them when the window opens. A `look =` line or a
//! `--look` flag names one for scripts and the command line
//! ([`with_look`]).

use rea_launcher::cfg::on_off;
use rea_launcher::display::{STEAM_DECK_SCREEN, is_steam_deck, landscape};

use crate::config::{self, DisplayMode, Gamepad, Postfx, Scaler, Settings, ShadowStyle};

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
    /// Unset keeps the game's shadow style.
    pub shadows: Option<ShadowStyle>,
    /// Linux only, opt-in: the post-processing pass.
    pub postfx: Postfx,
}

impl Picture {
    /// The picture values in settings (unset display mode: fullscreen).
    pub fn of(s: &Settings) -> Picture {
        Picture {
            resolution: s.resolution,
            original: s.original_resolution,
            display_mode: s.display_mode.unwrap_or(DisplayMode::Fullscreen),
            vsync: s.vsync,
            anisotropic: s.anisotropic,
            fsaa: s.fsaa,
            fov: s.fov,
            hd_ui: s.hd_ui == Some(true),
            shadows: s.shadows,
            postfx: s.postfx.unwrap_or_default(),
        }
    }

    /// Sets every picture value of `s` to this picture's, as the window
    /// saves them: an unset value is unset (the game's own).
    pub fn apply(&self, s: &mut Settings) {
        s.resolution = self.resolution;
        s.original_resolution = self.original;
        s.display_mode =
            (self.display_mode != DisplayMode::Fullscreen).then_some(self.display_mode);
        s.vsync = self.vsync;
        s.anisotropic = self.anisotropic;
        s.fsaa = self.fsaa;
        s.fov = self.fov;
        s.hd_ui = self.hd_ui.then_some(true);
        s.shadows = self.shadows;
        s.postfx = (self.postfx != Postfx::Off).then_some(self.postfx);
    }

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
        shadows: None,
        postfx: Postfx::Off,
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
            shadows: None,
            postfx: Postfx::Off,
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
        shadows: None,
        postfx: Postfx::Off,
    };
}

/// The look to recommend on a screen: Steam Deck on a Deck, else Enhanced.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if is_steam_deck(screen) {
        Look::SteamDeck
    } else {
        Look::Enhanced
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Original,
    Enhanced,
    SteamDeck,
    Custom,
}

impl Look {
    /// The look a `look` value (`config::LOOKS`) names.
    pub fn from_key(key: &str) -> Option<Look> {
        match key {
            "original" => Some(Look::Original),
            "enhanced" => Some(Look::Enhanced),
            "steam_deck" => Some(Look::SteamDeck),
            _ => None,
        }
    }

    /// Its `look` value; Custom has none.
    pub fn key(self) -> Option<&'static str> {
        match self {
            Look::Original => Some("original"),
            Look::Enhanced => Some("enhanced"),
            Look::SteamDeck => Some("steam_deck"),
            Look::Custom => None,
        }
    }

    /// The look's picture on a screen; `None` for Custom.
    pub fn picture(self, screen: Option<(u32, u32)>) -> Option<Picture> {
        match self {
            Look::Original => Some(Picture::ORIGINAL),
            Look::Enhanced => Some(Picture::enhanced(screen)),
            Look::SteamDeck => Some(Picture::STEAM_DECK),
            Look::Custom => None,
        }
    }
}

/// The settings a run uses (`sdk/docs/STANDARDS.md`: "Looks have a
/// `--look` flag that sets all of the look's values"): the file's look
/// under the picture values the file sets itself; then a `--look` from
/// the command line replaces every picture value; then the other flags on
/// top.
pub fn with_look(file: Settings, args: Settings, screen: Option<(u32, u32)>) -> Settings {
    let picture = |look: Option<&str>| look.and_then(Look::from_key)?.picture(screen);
    let mut base = match picture(file.look) {
        Some(look) => {
            let mut from_look = Settings::default();
            look.apply(&mut from_look);
            from_look.merged(file)
        }
        None => file,
    };
    if let Some(look) = picture(args.look) {
        look.apply(&mut base);
    }
    base.merged(args)
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
    pub gamepad_log: bool,
}

/// Field-of-view choices offered (the game's own is 75); a value saved
/// from the file or a flag outside these gets a chip of its own
/// ([`with_saved`]).
pub const FOVS: [(Option<u32>, &str); 5] = [
    (None, config::AS_THE_GAME),
    (Some(60), "60°"),
    (Some(75), "75°"),
    (Some(90), "90°"),
    (Some(110), "110°"),
];
pub const DISPLAY_MODES: [(DisplayMode, &str); 3] = [
    (DisplayMode::Fullscreen, "Fullscreen"),
    (DisplayMode::Windowed, "Windowed"),
    (DisplayMode::Borderless, "Borderless"),
];
pub const POSTFX: [(Postfx, &str); 3] = [
    (Postfx::Off, "Off"),
    (Postfx::Color, "Colour"),
    (Postfx::Cinematic, "Cinematic"),
];
pub const GAMEPADS: [(Gamepad, &str); 3] = [
    (Gamepad::Rally, "Rally"),
    (Gamepad::Standard, "Standard"),
    (Gamepad::Off, "Off"),
];

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

/// `choices` with `current` added as its own chip (labelled by `label`)
/// when it is a value the curated list lacks: a size or angle saved from
/// `launcher.cfg` or a flag is shown and kept, never lost.
pub fn with_saved<T: PartialEq + Copy>(
    choices: &[(Option<T>, String)],
    current: Option<T>,
    label: impl Fn(T) -> String,
) -> Vec<(Option<T>, String)> {
    let mut out = choices.to_vec();
    if let Some(value) = current
        && !out.iter().any(|(v, _)| *v == Some(value))
    {
        out.push((Some(value), label(value)));
    }
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
        let picture = Picture::of(s);
        Form {
            game: path(&s.game),
            saves: path(&s.saves),
            picture,
            look: Form::look_of(&picture, screen),
            gamepad: s.gamepad.unwrap_or(Gamepad::Rally),
            dxvk: s.dxvk != Some(false),
            scaler: s.scaler.unwrap_or(Scaler::Linear),
            gamepad_log: s.gamepad_log == Some(true),
        }
    }

    /// True until the window has saved once: it always writes the scaler,
    /// and a first run has no picture value either. The launcher may
    /// already have stored `game` (`--game`, a `--no-window` run); that
    /// does not count.
    pub fn is_first_run(s: &Settings) -> bool {
        s.resolution.is_none()
            && s.display_mode.is_none()
            && s.vsync.is_none()
            && s.anisotropic.is_none()
            && s.fsaa.is_none()
            && s.fov.is_none()
            && s.hd_ui.is_none()
            && s.shadows.is_none()
            && s.postfx.is_none()
            && s.scaler.is_none()
    }

    /// The form for a first run: the stored settings (the game folder,
    /// say) with the Enhanced look fitted to the screen, the Steam Deck
    /// look on a Deck (owner decision — the goal is a modern, widescreen,
    /// HD experience with no settings touched).
    pub fn first_run(settings: &Settings, screen: Option<(u32, u32)>) -> Form {
        let mut form = Form::new(settings, screen);
        form.set_look(recommended(screen), screen);
        form
    }

    /// The look these picture values make on a screen.
    /// On a Steam Deck the Enhanced look fitted to its screen holds the
    /// same values as the Steam Deck look, which is the one shown.
    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        if *picture == Picture::ORIGINAL {
            Look::Original
        } else if *picture == Picture::STEAM_DECK && is_steam_deck(screen) {
            Look::SteamDeck
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
        // The window saves every value on its own; a `look =` line would
        // fill the values left unset on the next read.
        let mut t = config::clear_key(text, "look");
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
        put("vsync", p.vsync.map(|v| on_off(v).into()));
        put(
            "anisotropic",
            p.anisotropic.map(|a| match a {
                0 => "off".into(),
                n => n.to_string(),
            }),
        );
        put("fsaa", p.fsaa.map(|b| on_off(b).into()));
        put("shadows", p.shadows.map(|m| m.key().into()));
        put(
            "postfx",
            (p.postfx != Postfx::Off).then(|| p.postfx.key().into()),
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
        put("gamepad_log", self.gamepad_log.then(|| "on".into()));
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

    fn first_run(screen: Option<(u32, u32)>) -> Form {
        Form::first_run(&Settings::parse_file(&config::template()).0, screen)
    }

    #[test]
    fn a_first_run_is_any_file_the_window_never_saved() {
        let first = |text: &str| Form::is_first_run(&Settings::parse_file(text).0);
        assert!(first(""));
        assert!(first(&config::template()));
        // `--game` (or a --no-window run) stored the folder first.
        let with_game = config::set_key(&config::template(), "game", r"D:\CMR3");
        assert!(first(&with_game));
        let f = Form::first_run(&Settings::parse_file(&with_game).0, SCREEN);
        assert_eq!(f.look, Look::Enhanced);
        assert_eq!(f.game, r"D:\CMR3", "the stored folder is kept");
        // Once the window saved, even all-default values, it is not.
        let saved = form(&with_game).save(&with_game);
        assert!(!first(&saved));
        let mut original = form(&with_game);
        original.set_look(Look::Original, SCREEN);
        assert!(!first(&original.save(&with_game)));
    }

    #[test]
    fn a_first_run_starts_with_the_enhanced_look() {
        let f = first_run(SCREEN);
        assert_eq!(f.look, Look::Enhanced);
        assert_eq!(f.picture.resolution, Some((2560, 1440)));
        assert_eq!(f.picture.anisotropic, Some(16));
        assert!(f.picture.hd_ui);
        assert_eq!(f.gamepad, Gamepad::Rally);
        assert_eq!(first_run(Some(STEAM_DECK_SCREEN)).look, Look::SteamDeck);
    }

    #[test]
    fn the_look_is_worked_out_from_the_saved_values() {
        let mut f = form(&config::template());
        f.set_look(Look::Enhanced, SCREEN);
        let saved = f.save(&config::template());
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
    fn on_a_steam_deck_its_own_look_is_the_one_chosen() {
        let deck = Some(STEAM_DECK_SCREEN);
        let f = first_run(deck);
        assert_eq!(f.look, Look::SteamDeck);
        // Saved and opened again on the Deck: still the Steam Deck look,
        // although Enhanced fitted to 1280×800 has the same values.
        let saved = f.save(&config::template());
        assert_eq!(
            Form::new(&Settings::parse_file(&saved).0, deck).look,
            Look::SteamDeck
        );
        // Elsewhere those values are still the Steam Deck look.
        assert_eq!(form(&saved).look, Look::SteamDeck);
    }

    /// Every picture value set to something no look uses.
    const CUSTOM: &str = "resolution = 1600x900\ndisplay_mode = windowed\nvsync = off\n\
        anisotropic = 2\nfsaa = on\nfov = 110\nhd_ui = on\nshadows = soft\npostfx = color\n\
        gamepad = standard\n";

    fn cli(flags: &[&str]) -> Settings {
        Settings::parse_args(flags.iter().map(|f| f.to_string()))
            .unwrap()
            .0
    }

    /// What the window saves for a look.
    fn window_saves(look: Look) -> Settings {
        let mut f = form("");
        f.set_look(look, SCREEN);
        Settings::parse_file(&f.save("")).0
    }

    /// `--look` (STANDARDS: "Looks have a --look flag that sets all of
    /// the look's values"): a run with the flag plays exactly what the
    /// window saves for that look, whatever the file held.
    #[test]
    fn the_look_flag_sets_every_picture_value() {
        let file = Settings::parse_file(CUSTOM).0;
        for (key, look) in [
            ("original", Look::Original),
            ("enhanced", Look::Enhanced),
            ("steam_deck", Look::SteamDeck),
        ] {
            assert_eq!(Look::from_key(key), Some(look));
            assert_eq!(look.key(), Some(key));
            let run = with_look(file.clone(), cli(&["--look", key]), SCREEN);
            assert_eq!(Picture::of(&run), Picture::of(&window_saves(look)), "{key}");
            assert_eq!(Form::look_of(&Picture::of(&run), SCREEN), look, "{key}");
            // Settings that aren't picture values stay the file's.
            assert_eq!(run.gamepad, Some(Gamepad::Standard), "{key}");
        }
        assert_eq!(Look::Custom.key(), None);
        assert_eq!(Look::from_key("custom"), None);
    }

    #[test]
    fn picture_flags_given_with_the_look_win_over_it() {
        for flags in [
            &["--look", "enhanced", "--fov", "90"][..],
            &["--fov", "90", "--look", "enhanced"],
        ] {
            let run = with_look(Settings::default(), cli(flags), SCREEN);
            assert_eq!(
                Picture::of(&run),
                Picture {
                    fov: Some(90),
                    ..Picture::enhanced(SCREEN)
                },
                "{flags:?}"
            );
        }
    }

    /// `look = ` in launcher.cfg: the look's values for every picture key
    /// the file doesn't set; the window opens on that look and saves the
    /// values themselves.
    #[test]
    fn a_look_in_the_file_fills_the_keys_it_leaves_out() {
        let run = with_look(
            Settings::parse_file("look = steam_deck\nfov = 90\n").0,
            Settings::default(),
            SCREEN,
        );
        assert_eq!(
            Picture::of(&run),
            Picture {
                fov: Some(90),
                ..Picture::STEAM_DECK
            }
        );
        let file = "look = steam_deck\n";
        let settings = with_look(Settings::parse_file(file).0, Settings::default(), SCREEN);
        assert!(!Form::is_first_run(&settings), "a look is a choice");
        let f = Form::new(&settings, SCREEN);
        assert_eq!(f.look, Look::SteamDeck);
        let saved = f.save(file);
        assert!(
            !saved.lines().any(|l| l.trim_start().starts_with("look")),
            "{saved}"
        );
        assert_eq!(Settings::parse_file(&saved).0.resolution, Some((1280, 800)));
        // An unknown look name is reported by the parser and changes nothing.
        let (s, warnings) = Settings::parse_file("look = modern\n");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(
            with_look(s, Settings::default(), SCREEN),
            Settings::default()
        );
    }

    #[test]
    fn a_saved_value_outside_the_chips_gets_its_own() {
        let fovs: Vec<_> = FOVS.iter().map(|(v, l)| (*v, l.to_string())).collect();
        let shown = with_saved(&fovs, Some(45), |f| format!("{f}°"));
        assert_eq!(shown.last(), Some(&(Some(45), "45°".to_string())));
        assert_eq!(with_saved(&fovs, Some(90), |f| format!("{f}°")), fovs);
        assert_eq!(with_saved(&fovs, None, |f| format!("{f}°")), fovs);
        // And the value is kept when saving.
        let f = form("fov = 45\nresolution = 1600x900\n");
        let (s, _) = Settings::parse_file(&f.save(""));
        assert_eq!((s.fov, s.resolution), (Some(45), Some((1600, 900))));
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
        let mut f = form(&config::template());
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
            shadows: Some(crate::config::ShadowStyle::Soft),
            postfx: Postfx::Cinematic,
        };
        f.gamepad = Gamepad::Standard;
        f.dxvk = false;
        f.scaler = Scaler::Fsr;
        f.gamepad_log = true;
        let saved = f.save(&config::template());
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
        let f = form(&config::template());
        let saved = f.save(&config::template());
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
}
