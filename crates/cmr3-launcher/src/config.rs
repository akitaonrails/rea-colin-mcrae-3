//! Command line and the settings file (`launcher.cfg`).
//!
//! Every option is declared once, in [`OPTIONS`] (the SDK's
//! `rea_launcher::cfg::spec`): its key, flag, values, label and a one-line
//! description. `--help`, the commented template written on a first run,
//! the file and flag parsing and the settings window's rows all come from
//! that table, so the window and the command line can't drift apart.
//!
//! The file is plain `key = value` lines with `#` comments. The launcher
//! later changes only the lines it owns (`game`), keeping everything else
//! as the user left it. Unknown keys are reported, ignored and kept, so
//! older launchers can read newer files; values it cannot read are
//! reported and ignored.

use std::path::PathBuf;

use rea_launcher::cfg::spec::{self, Kind, OptionSpec, Value};
use rea_launcher::cfg::{self, ArgError, KeyStyle, Rejected, Resolution};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    /// Where the player's profile (`CMR3` file) is kept (see `TEMPLATE`).
    pub saves: Option<PathBuf>,
    /// A look's name ([`LOOKS`]): its picture values for every picture
    /// setting not given (`form::with_look`).
    pub look: Option<&'static str>,
    /// Race resolution written to SilentPatchCMR3.ini; `None` keeps the
    /// file's (SilentPatch itself defaults to the desktop size).
    pub resolution: Option<(u32, u32)>,
    /// `resolution = original`: the 640×480 of 2003, replacing a file's
    /// wider setting from the command line.
    pub original_resolution: bool,
    pub display_mode: Option<DisplayMode>,
    /// Unset keeps the game's own choice.
    pub vsync: Option<bool>,
    /// Anisotropic filtering level; `None` keeps the game's.
    pub anisotropic: Option<u32>,
    /// Anti-aliasing (SilentPatch fixed its rendering bugs).
    pub fsaa: Option<bool>,
    /// Field of view in degrees, 30 to 150 in steps of 5; both cameras.
    pub fov: Option<u32>,
    /// The HD UI overlay pack (by Bekoha) for 4K-sharp menus.
    pub hd_ui: Option<bool>,
    /// Car shadows: `sharp` (2005-style, SilentPatch's default) or `soft`
    /// (the 2003 blur). Unset keeps the game's.
    pub shadows: Option<ShadowStyle>,
    /// Xidi's own log (`gamepad_log`): what the game asks of DirectInput
    /// and what every pad control sent — the file to read when a button
    /// doesn't arrive.
    pub gamepad_log: Option<bool>,
    /// The Xidi gamepad mapping (see `xidi`).
    pub gamepad: Option<Gamepad>,
    /// Linux only: put DXVK's d3d9.dll next to the game (Vulkan instead of
    /// Wine's OpenGL translator). Ignored on Windows.
    pub dxvk: Option<bool>,
    /// Linux only: how `cmr3-play`'s gamescope enlarges the picture.
    pub scaler: Option<Scaler>,
    /// Post-processing over the whole frame (vkBasalt under Wine):
    /// colour grading and sharpening, or the same with depth-based
    /// ambient occlusion when the shaders are available. Opt-in.
    pub postfx: Option<Postfx>,
}

/// Post-processing over the finished frame (opt-in; Linux, via
/// `cmr3-play`'s vkBasalt layer).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Postfx {
    /// The game's own picture.
    #[default]
    Off,
    /// Real blacks and fuller contrast (a colour lookup table) and CAS
    /// sharpening — no depth buffer needed.
    Color,
    /// A stronger grade, plus depth-based ambient occlusion (needs the AO
    /// shaders; see docs/ENHANCEMENTS.md).
    Cinematic,
}

impl Postfx {
    pub const ALL: [Postfx; 3] = [Postfx::Off, Postfx::Color, Postfx::Cinematic];

    pub fn key(self) -> &'static str {
        match self {
            Postfx::Off => "off",
            Postfx::Color => "color",
            Postfx::Cinematic => "cinematic",
        }
    }

    fn from_key(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == v)
    }
}

/// The car-shadow style (SilentPatch's `SHARPER_SHADOWS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowStyle {
    Sharp,
    Soft,
}

impl ShadowStyle {
    pub const ALL: [ShadowStyle; 2] = [ShadowStyle::Sharp, ShadowStyle::Soft];

    /// The value in launcher.cfg.
    pub fn key(self) -> &'static str {
        match self {
            ShadowStyle::Sharp => "sharp",
            ShadowStyle::Soft => "soft",
        }
    }

    /// The SilentPatchCMR3.ini `[Advanced]` `SHARPER_SHADOWS` value.
    pub fn ini(self) -> u8 {
        match self {
            ShadowStyle::Sharp => 1,
            ShadowStyle::Soft => 0,
        }
    }

    fn from_key(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == v)
    }
}

/// The game's display mode (SilentPatch's Advanced Graphics option).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Fullscreen,
    Windowed,
    Borderless,
}

impl DisplayMode {
    pub const ALL: [DisplayMode; 3] = [
        DisplayMode::Fullscreen,
        DisplayMode::Windowed,
        DisplayMode::Borderless,
    ];

    /// The value in launcher.cfg.
    pub fn key(self) -> &'static str {
        match self {
            DisplayMode::Fullscreen => "fullscreen",
            DisplayMode::Windowed => "windowed",
            DisplayMode::Borderless => "borderless",
        }
    }

    /// The SilentPatchCMR3.ini `DISPLAY_MODE` value.
    pub fn ini(self) -> u32 {
        match self {
            DisplayMode::Fullscreen => 0,
            DisplayMode::Windowed => 1,
            DisplayMode::Borderless => 2,
        }
    }

    /// The mode to actually run: exclusive fullscreen under Wine (outside
    /// gamescope's single-screen world) picks its display by enumerating
    /// the desktop's monitors, and whatever that finds first — a rotated
    /// or portrait neighbour included — is what the game renders into,
    /// cropped, on every compositor that scales or reorders screens
    /// (measured: a portrait monitor first in the list got the game a
    /// 2160×3840 swapchain inside a landscape window). Borderless sizes
    /// the game's own window to the chosen resolution instead: no
    /// enumeration, the right pixels, everywhere. Inside gamescope there
    /// is exactly one screen, so fullscreen stays fullscreen; on Windows
    /// the game's own exclusive mode is the right thing.
    pub fn effective(self, wine: bool, gamescope: bool) -> DisplayMode {
        match (self, wine && !gamescope) {
            (DisplayMode::Fullscreen, true) => DisplayMode::Borderless,
            _ => self,
        }
    }

    fn from_key(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == v)
    }
}

/// The Xidi mapper that shapes how a modern pad reaches the game.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gamepad {
    /// Triggers on the game's pedal axis, d-pad and Start on the keyboard
    /// keys the game knows, physical left-stick Y ignored (no accidental
    /// throttle). The mapping validated in the distrobox-gaming setup.
    #[default]
    Rally,
    /// Xidi's plain StandardGamepad: every pad axis and button passes
    /// through as it reports itself.
    Standard,
    /// No Xidi: the game reads the pad straight through DirectInput.
    Off,
}

impl Gamepad {
    pub const ALL: [Gamepad; 3] = [Gamepad::Rally, Gamepad::Standard, Gamepad::Off];

    pub fn key(self) -> &'static str {
        match self {
            Gamepad::Rally => "rally",
            Gamepad::Standard => "standard",
            Gamepad::Off => "off",
        }
    }

    fn from_key(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == v)
    }
}

/// `scaler` (Linux, gamescope's filter): sharp pixels, sharp pixels of
/// even size, FSR, or plain smoothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaler {
    Nearest,
    Pixel,
    Fsr,
    Linear,
}

impl Scaler {
    pub const ALL: [Scaler; 4] = [Scaler::Nearest, Scaler::Pixel, Scaler::Fsr, Scaler::Linear];

    /// The value in launcher.cfg (and gamescope's `-F` name).
    pub fn key(self) -> &'static str {
        match self {
            Scaler::Nearest => "nearest",
            Scaler::Pixel => "pixel",
            Scaler::Fsr => "fsr",
            Scaler::Linear => "linear",
        }
    }

    fn from_key(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == v)
    }
}

pub const FILE_NAME: &str = "launcher.cfg";

/// `display_mode` values and their window labels.
pub const DISPLAY_MODES: &[(&str, &str)] = &[
    ("fullscreen", "Fullscreen"),
    ("windowed", "Windowed"),
    ("borderless", "Borderless"),
];
/// `anisotropic` values: off or the level in texels.
pub const ANISOTROPICS: &[(&str, &str)] = &[
    ("off", "Off"),
    ("2", "2×"),
    ("4", "4×"),
    ("8", "8×"),
    ("16", "16×"),
];
pub const SHADOWS: &[(&str, &str)] = &[("sharp", "Sharp"), ("soft", "Soft")];
pub const GAMEPADS: &[(&str, &str)] =
    &[("rally", "Rally"), ("standard", "Standard"), ("off", "Off")];
pub const SCALERS: &[(&str, &str)] = &[
    ("nearest", "Sharp"),
    ("pixel", "Even pixels"),
    ("fsr", "FSR"),
    ("linear", "Smooth"),
];
/// The looks (`look = ` / `--look`): the game as it shipped, the screen's
/// own size with every improvement (the recommended look), the Steam
/// Deck's screen. The window's fourth card, Custom, is any other mix.
pub const LOOKS: &[(&str, &str)] = &[
    ("original", "Original 2003"),
    ("enhanced", "Enhanced"),
    ("steam_deck", "Steam Deck"),
];
pub const POSTFXS: &[(&str, &str)] = &[
    ("off", "Off"),
    ("color", "Colour"),
    ("cinematic", "Cinematic"),
];

/// The window's chip for an unset picture option: the game keeps its own value.
pub const AS_THE_GAME: &str = "As the game";

/// Every option: one row each for the file, the flags, `--help`, the
/// template and the window.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "game",
        Kind::Path,
        "Game folder",
        "The folder with your installed Rally_3PC.exe. It is only read, never changed. \
         Remembered once given.",
    ),
    OptionSpec::new(
        "saves",
        Kind::Path,
        "Profile folder",
        "Where your profile (the game's CMR3 file) is kept: copied into the game before it \
         starts and back when it ends. Default: Saved Games\\Colin McRae Rally 3.",
    ),
    OptionSpec::new(
        "runtime",
        Kind::Path,
        "Runtime folder",
        "Where the launcher keeps its private, writable copy of the game (about 3 GB; \
         hard-linked when it can be). Default: %LOCALAPPDATA%\\CMR3Launcher\\Rally3.",
    )
    .launcher_only(),
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "Every picture setting at once: the game as it shipped in 2003, your screen's own size \
         with every improvement (recommended), or the Steam Deck's screen. Picture settings \
         given on their own win over it.",
    ),
    OptionSpec::new(
        "resolution",
        Kind::Size {
            min: (640, 480),
            max: (7680, 4320),
            aliases: cfg::ORIGINAL,
        },
        "Resolution",
        "The game's screen size; original is the 640×480 of 2003. Unset: your desktop's size. \
         Wider screens show more to the sides, unstretched.",
    )
    .with_example("1920x1080"),
    OptionSpec::new(
        "display_mode",
        Kind::Choice(DISPLAY_MODES),
        "Display mode",
        "Fullscreen fills the screen; windowed and borderless play in a window on the desktop.",
    ),
    OptionSpec::new(
        "vsync",
        Kind::Toggle,
        "VSync",
        "Waits for the screen's refresh before showing each frame: no tearing. \
         Unset keeps the game's own choice.",
    )
    .unset_label(AS_THE_GAME),
    OptionSpec::new(
        "anisotropic",
        Kind::Choice(ANISOTROPICS),
        "Texture sharpness",
        "Anisotropic filtering: keeps the road surface sharp into the distance. \
         Unset keeps the game's.",
    )
    .unset_label(AS_THE_GAME),
    OptionSpec::new(
        "fsaa",
        Kind::Toggle,
        "Smooth edges",
        "Anti-aliasing: smooths the jagged edges of cars and scenery; costs speed at 4K. \
         Unset keeps the game's.",
    )
    .unset_label(AS_THE_GAME),
    OptionSpec::new(
        "fov",
        Kind::Range { min: 30, max: 150 },
        "Field of view",
        "How wide the camera sees, in degrees, for both cameras (rounded down to a step of 5). \
         The game's own is 75; wider screens already show more to the sides.",
    )
    .with_example("75"),
    OptionSpec::new(
        "shadows",
        Kind::Choice(SHADOWS),
        "Car shadows",
        "Sharp: the crisper 2005 shadows. Soft: the blurrier 2003 ones. \
         Unset keeps the game's.",
    )
    .unset_label(AS_THE_GAME),
    OptionSpec::new(
        "hd_ui",
        Kind::Toggle,
        "HD menus",
        "Bekoha's HD UI: fonts, menus and race graphics redrawn for 4K.",
    ),
    OptionSpec::new(
        "gamepad",
        Kind::Choice(GAMEPADS),
        "Gamepad",
        "Rally: the triggers are the pedals, and the d-pad and A also work the menus. \
         Standard: the pad as it reports itself. Off: the game's own DirectInput.",
    ),
    OptionSpec::new(
        "gamepad_log",
        Kind::Toggle,
        "Gamepad log",
        "Records what the game asks of the pad and what every button and stick sent \
         (Xidi's log, beside the game in the runtime folder); for reporting a button \
         that doesn't arrive.",
    ),
    OptionSpec::new(
        "dxvk",
        Kind::Toggle,
        "Vulkan (DXVK)",
        "Linux and Wine only: draws through Vulkan instead of Wine's OpenGL translator, \
         full speed instead of about 10 FPS at 4K. On unless set off; ignored on Windows.",
    ),
    OptionSpec::new(
        "scaler",
        Kind::Choice(SCALERS),
        "Upscaling",
        "Linux only: how gamescope enlarges the picture to your screen. Sharp: crisp blocks. \
         Even pixels: crisp, all the same size. FSR: smoothed, then sharpened. Smooth: the softest.",
    )
    .helper_owned(),
    OptionSpec::new(
        "postfx",
        Kind::Choice(POSTFXS),
        "Post-processing",
        "Linux only, needs DXVK: a colour grade over the finished frame. Colour: real blacks, \
         fuller contrast and sharpening. Cinematic: a stronger grade.",
    )
    .helper_owned(),
];

/// The row for `key`; every key the launcher uses has one.
pub fn option(key: &str) -> &'static OptionSpec {
    spec::find(OPTIONS, key).unwrap_or_else(|| panic!("no option row for {key}"))
}

/// The file written on first run: a commented line for every option.
pub fn template() -> String {
    spec::template(
        "Colin McRae Rally 3 launcher settings.\n\
         \n\
         One setting per line as key = value; lines starting with # are comments. \
         A setting left unset keeps the game's own value (or the default named). \
         Command-line options (--resolution 1920x1080, ...) override this file for one run. \
         A launcher.cfg next to CMR3Launcher.exe is used instead of this one when it exists \
         (portable installs).",
        OPTIONS,
    )
}

/// The launcher's own switches, which take no value.
pub const SWITCHES: [&str; 4] = ["--settings", "--linux", "--no-window", "--check"];

/// `--help`: the options from [`OPTIONS`], the switches and exit codes.
pub fn usage() -> String {
    format!(
        "CMR3Launcher {}\n\n\
         CMR3Launcher [options]\n\n\
         {}\n\n\
         These override launcher.cfg for one run (only --game is remembered);\n\
         every one is also a launcher.cfg key (--hd-ui is hd_ui) and a control\n\
         in the settings window. Without options the settings window opens first.\n\n\
         \x20 --no-window              play with the saved settings, no window\n\
         \x20 --settings [--linux]     only the settings window (exit 0: Play pressed,\n\
         \x20                          3: closed without Play, 4: no window possible);\n\
         \x20                          --linux adds the Linux-only rows, for cmr3-play\n\
         \x20 --check                  checks the game folder, its Rally_3PC.exe and the\n\
         \x20                          bundled parts without playing; the report goes to\n\
         \x20                          the terminal and launcher.log (exit 0: ready,\n\
         \x20                          1: something is missing or unknown)\n\
         \x20 --help                   this text\n\n\
         The log is %LOCALAPPDATA%\\CMR3Launcher\\launcher.log.",
        env!("CARGO_PKG_VERSION"),
        spec::usage(OPTIONS, KeyStyle::Underscore).trim_end(),
    )
}

/// The SilentPatch ini value for an anisotropic level in texels.
pub fn anisotropic_ini(level: Option<u32>) -> Option<u32> {
    // The ini stores the menu index: 0 off, 1 = 2×, 2 = 4×, 3 = 8×, 4 = 16×.
    level.map(|n| match n {
        0 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => 4,
    })
}

/// Why a parsed value can't be used: only an option row whose kind and
/// key disagree (a bug in [`OPTIONS`]).
fn mismatch() -> Rejected {
    Rejected::Unknown
}

fn path(v: Value) -> Result<PathBuf, Rejected> {
    match v {
        Value::Text(t) => Ok(t.into()),
        _ => Err(mismatch()),
    }
}

fn toggle(v: Value) -> Result<bool, Rejected> {
    match v {
        Value::Toggle(on) => Ok(on),
        _ => Err(mismatch()),
    }
}

fn choice<T>(v: Value, from: fn(&str) -> Option<T>) -> Result<T, Rejected> {
    match v {
        Value::Choice(c) => from(c).ok_or_else(mismatch),
        _ => Err(mismatch()),
    }
}

impl Settings {
    /// Sets one value by its `launcher.cfg` key, already checked against
    /// its row in [`OPTIONS`]. The command line's `--some-key` is
    /// `some_key`, so the file and the flags read alike.
    fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        match key {
            "game" => self.game = Some(path(value)?),
            "runtime" => self.runtime = Some(path(value)?),
            "saves" => self.saves = Some(path(value)?),
            "look" => {
                self.look = Some(choice(value, |c| {
                    LOOKS.iter().find(|(v, _)| *v == c).map(|(v, _)| *v)
                })?);
            }
            "resolution" => {
                let Value::Size(r) = value else {
                    return Err(mismatch());
                };
                let r = match r {
                    Resolution::Size(w, h) => (w, h),
                    Resolution::Original | Resolution::Fullscreen => (640, 480),
                };
                self.resolution = Some(r);
                self.original_resolution = r == (640, 480);
            }
            "display_mode" => self.display_mode = Some(choice(value, DisplayMode::from_key)?),
            "vsync" => self.vsync = Some(toggle(value)?),
            "anisotropic" => {
                self.anisotropic = Some(choice(value, |c| match c {
                    "off" => Some(0),
                    n => n.parse().ok(),
                })?);
            }
            "fsaa" => self.fsaa = Some(toggle(value)?),
            "fov" => {
                let Value::Number(n) = value else {
                    return Err(mismatch());
                };
                self.fov = Some(n - n % 5);
            }
            "shadows" => self.shadows = Some(choice(value, ShadowStyle::from_key)?),
            "hd_ui" => self.hd_ui = Some(toggle(value)?),
            "gamepad_log" => self.gamepad_log = Some(toggle(value)?),
            "gamepad" => self.gamepad = Some(choice(value, Gamepad::from_key)?),
            "dxvk" => self.dxvk = Some(toggle(value)?),
            "postfx" => self.postfx = Some(choice(value, Postfx::from_key)?),
            "scaler" => self.scaler = Some(choice(value, Scaler::from_key)?),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// Reads a settings file; the second value lists lines it could not use.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut s = Settings::default();
        let warnings = spec::parse_file(text, FILE_NAME, OPTIONS, |k, v| s.apply(k, v));
        (s, warnings)
    }

    /// Command-line values override the file's.
    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            saves: over.saves.or(self.saves),
            look: over.look.or(self.look),
            resolution: over.resolution.or(self.resolution),
            // Follows whichever resolution won.
            original_resolution: if over.resolution.is_some() {
                over.original_resolution
            } else {
                self.original_resolution
            },
            display_mode: over.display_mode.or(self.display_mode),
            vsync: over.vsync.or(self.vsync),
            anisotropic: over.anisotropic.or(self.anisotropic),
            fsaa: over.fsaa.or(self.fsaa),
            fov: over.fov.or(self.fov),
            hd_ui: over.hd_ui.or(self.hd_ui),
            shadows: over.shadows.or(self.shadows),
            gamepad: over.gamepad.or(self.gamepad),
            gamepad_log: over.gamepad_log.or(self.gamepad_log),
            dxvk: over.dxvk.or(self.dxvk),
            scaler: over.scaler.or(self.scaler),
            postfx: over.postfx.or(self.postfx),
        }
    }

    /// The command line: `--key value` settings and the launcher's own
    /// [`SWITCHES`] (returned in order), in one pass.
    pub fn parse_args<I: IntoIterator<Item = String>>(
        args: I,
    ) -> Result<(Settings, Vec<String>), ArgError> {
        let mut s = Settings::default();
        let switches = spec::parse_args(args, KeyStyle::Underscore, OPTIONS, &SWITCHES, |k, v| {
            s.apply(k, v)
        })?;
        Ok((s, switches))
    }
}

/// Sets `key` in a settings file, keeping every other line (and comments)
/// as it was.
pub use rea_launcher::cfg::{clear_key, set_key};

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Result<Settings, String> {
        Settings::parse_args(a.iter().map(|s| s.to_string()))
            .map(|(s, _)| s)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn switches_come_back_in_order_and_values_stay_values() {
        let (s, switches) = Settings::parse_args(
            [
                "--settings",
                "--linux",
                "--vsync",
                "on",
                "--no-window",
                "--check",
            ]
            .map(String::from),
        )
        .unwrap();
        assert_eq!(
            switches,
            ["--settings", "--linux", "--no-window", "--check"]
        );
        assert_eq!(s.vsync, Some(true));
        let (s, switches) = Settings::parse_args(["--game", "--check"].map(String::from)).unwrap();
        assert!(switches.is_empty());
        assert_eq!(s.game, Some(PathBuf::from("--check")));
    }

    /// Scripts written for release 0.1 pass `--key=value` and the
    /// underscore spelling of a hyphenated flag; both still read.
    #[test]
    fn equals_and_underscore_flag_spellings_still_read() {
        let (s, switches) = Settings::parse_args(
            ["--scaler=fsr", "--display_mode", "windowed", "--no-window"].map(String::from),
        )
        .unwrap();
        assert_eq!(s.scaler, Some(Scaler::Fsr));
        assert_eq!(s.display_mode, Some(DisplayMode::Windowed));
        assert_eq!(switches, ["--no-window"]);
        assert_eq!(
            args(&["--fov=200"]).unwrap_err(),
            "--fov needs 30 to 150, not 200"
        );
    }

    #[test]
    fn bad_flags_say_what_they_need() {
        assert_eq!(
            args(&["--vsync", "maybe"]).unwrap_err(),
            "--vsync needs on or off, not maybe"
        );
        assert_eq!(args(&["--fov"]).unwrap_err(), "--fov needs a value");
        assert_eq!(
            args(&["--fov", "200"]).unwrap_err(),
            "--fov needs 30 to 150, not 200"
        );
        assert_eq!(
            args(&["--anisotropic", "3"]).unwrap_err(),
            "--anisotropic needs off, 2, 4, 8 or 16, not 3"
        );
        assert_eq!(
            args(&["--resolution", "8000x100"]).unwrap_err(),
            "--resolution needs WIDTHxHEIGHT or original, not 8000x100"
        );
        assert_eq!(
            args(&["--display-mode", "wide"]).unwrap_err(),
            "--display-mode needs fullscreen, windowed or borderless, not wide"
        );
        assert_eq!(
            args(&["--gamepad", "wheel"]).unwrap_err(),
            "--gamepad needs rally, standard or off, not wheel"
        );
        assert_eq!(
            args(&["--scaler", "blurry"]).unwrap_err(),
            "--scaler needs nearest, pixel, fsr or linear, not blurry"
        );
        assert_eq!(args(&["--bogus"]).unwrap_err(), "unknown argument --bogus");
        for help in ["--help", "-h", "/?"] {
            assert!(matches!(
                Settings::parse_args([help.to_string()]),
                Err(ArgError::Help(_))
            ));
        }
    }

    #[test]
    fn the_look_reads_from_the_file_and_the_flag() {
        let (s, warnings) = Settings::parse_file("look = steam_deck\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s.look, Some("steam_deck"));
        assert_eq!(
            args(&["--look", "original"]).unwrap().look,
            Some("original")
        );
        assert_eq!(
            args(&["--look", "modern"]).unwrap_err(),
            "--look needs original, enhanced or steam_deck, not modern"
        );
        // A flag's look wins over the file's when merged.
        let file = Settings::parse_file("look = original\n").0;
        let run = file.merged(args(&["--look", "enhanced"]).unwrap());
        assert_eq!(run.look, Some("enhanced"));
    }

    #[test]
    fn unknown_keys_are_reported_and_kept() {
        let (_, warnings) = Settings::parse_file("future = 1\nvsync = on\n");
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("unknown setting \"future\""),
            "{warnings:?}"
        );
        assert!(set_key("future = 1\n", "vsync", "on").contains("future = 1"));
    }

    #[test]
    fn fullscreen_under_raw_wine_plays_borderless() {
        use DisplayMode as M;
        // Windows native, and Wine inside gamescope's single screen: the
        // choice stands, exclusive fullscreen included.
        assert_eq!(M::Fullscreen.effective(false, false), M::Fullscreen);
        assert_eq!(M::Fullscreen.effective(false, true), M::Fullscreen);
        assert_eq!(M::Fullscreen.effective(true, true), M::Fullscreen);
        // Raw Wine (any compositor): borderless, whose window the game
        // sizes itself — display enumeration never runs.
        assert_eq!(M::Fullscreen.effective(true, false), M::Borderless);
        // The other modes are always fine as they are.
        assert_eq!(M::Windowed.effective(true, false), M::Windowed);
        assert_eq!(M::Borderless.effective(true, false), M::Borderless);
        assert_eq!(M::Windowed.effective(false, false), M::Windowed);
    }

    /// One value per option, as the file and as a flag.
    const EVERY: &[(&str, &str)] = &[
        ("game", r"C:\G"),
        ("saves", r"D:\Saves"),
        ("runtime", r"E:\Rt"),
        ("look", "steam_deck"),
        ("resolution", "3840x2160"),
        ("display_mode", "borderless"),
        ("vsync", "off"),
        ("anisotropic", "4"),
        ("fsaa", "on"),
        ("fov", "90"),
        ("shadows", "soft"),
        ("hd_ui", "off"),
        ("gamepad", "standard"),
        ("gamepad_log", "on"),
        ("dxvk", "off"),
        ("scaler", "fsr"),
        ("postfx", "color"),
    ];

    fn check_every(s: &Settings) {
        assert_eq!(s.game, Some(PathBuf::from(r"C:\G")));
        assert_eq!(s.saves, Some(PathBuf::from(r"D:\Saves")));
        assert_eq!(s.runtime, Some(PathBuf::from(r"E:\Rt")));
        assert_eq!(s.look, Some("steam_deck"));
        assert_eq!(s.resolution, Some((3840, 2160)));
        assert_eq!(s.display_mode, Some(DisplayMode::Borderless));
        assert_eq!(s.vsync, Some(false));
        assert_eq!(s.anisotropic, Some(4));
        assert_eq!(s.fsaa, Some(true));
        assert_eq!(s.fov, Some(90));
        assert_eq!(s.shadows, Some(ShadowStyle::Soft));
        assert_eq!(s.hd_ui, Some(false));
        assert_eq!(s.gamepad, Some(Gamepad::Standard));
        assert_eq!(s.gamepad_log, Some(true));
        assert_eq!(s.dxvk, Some(false));
        assert_eq!(s.scaler, Some(Scaler::Fsr));
        assert_eq!(s.postfx, Some(Postfx::Color));
    }

    #[test]
    fn every_option_parses_from_the_file_and_the_flags() {
        assert_eq!(EVERY.len(), OPTIONS.len(), "one value per option row");
        let file: String = EVERY.iter().map(|(k, v)| format!("{k} = {v}\n")).collect();
        let (s, warnings) = Settings::parse_file(&file);
        assert!(warnings.is_empty(), "{warnings:?}");
        check_every(&s);
        let flags: Vec<String> = EVERY
            .iter()
            .flat_map(|(k, v)| [format!("--{}", k.replace('_', "-")), v.to_string()])
            .collect();
        let (s, switches) = Settings::parse_args(flags).unwrap();
        assert!(switches.is_empty());
        check_every(&s);
    }

    #[test]
    fn help_is_generated_from_the_table() {
        let help = usage();
        for row in OPTIONS {
            let flag = row.flag_name(KeyStyle::Underscore);
            assert!(help.contains(&flag), "{flag} missing from --help");
            assert!(help.contains(row.label), "{} missing", row.label);
        }
        assert!(help.contains("--scaler nearest|pixel|fsr|linear"));
        assert!(help.contains("Upscaling (Linux helper)"));
        assert!(help.contains("4: no window possible"));
        assert!(help.contains("--check"));
        // Every documented flag parses (they share launcher.cfg's keys).
        for line in help.lines() {
            let Some(flag) = line.trim().strip_prefix("--") else {
                continue;
            };
            let flag = flag.split_whitespace().next().unwrap();
            if matches!(flag, "no-window" | "settings" | "linux" | "check" | "help") {
                continue;
            }
            if let Err(e) = args(&[&format!("--{flag}"), "?"]) {
                assert!(!e.starts_with("unknown"), "--{flag}: {e}");
            }
        }
    }

    #[test]
    fn template_parses_clean_and_names_every_key() {
        let t = template();
        let (s, warnings) = Settings::parse_file(&t);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s, Settings::default(), "every setting is unset");
        for row in OPTIONS {
            assert!(t.contains(&format!("\n#{} =", row.key)), "{}", row.key);
        }
    }

    #[test]
    fn a_launcher_cfg_from_release_0_1_still_reads() {
        // The template the 0.1 launchers wrote, then filled in by them.
        let old = include_str!("../tests/fixtures/launcher-0.1.cfg");
        let (s, warnings) = Settings::parse_file(old);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s, Settings::default());
        let filled = set_key(old, "game", r"D:\CMR3");
        let filled = set_key(&filled, "vsync", "true");
        let filled = set_key(&filled, "fov", "77");
        let filled = set_key(&filled, "scaler", "linear");
        let (s, warnings) = Settings::parse_file(&filled);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s.game, Some(PathBuf::from(r"D:\CMR3")));
        assert_eq!(s.vsync, Some(true), "true/false still read");
        assert_eq!(s.fov, Some(75), "snapped to the step");
        assert_eq!(s.scaler, Some(Scaler::Linear));
    }

    #[test]
    fn a_windows_code_page_file_reads() {
        // Saved by Notepad in Windows-1252, CRLF: "José" in the path.
        let bytes = b"game = C:\\Users\\Jos\xe9\\Jogos d'A\xe7\xe3o\\CMR3\r\nvsync = on\r\n";
        let dir = std::env::temp_dir().join(format!("cmr3-cp1252-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::write(&path, bytes).unwrap();
        let text = cfg::read_file(&path).unwrap().unwrap();
        let (s, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            s.game,
            Some(PathBuf::from("C:\\Users\\José\\Jogos d'Ação\\CMR3"))
        );
        assert_eq!(s.vsync, Some(true));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn original_is_the_640x480_of_2003() {
        let (s, _) = Settings::parse_file("resolution = original\n");
        assert_eq!(s.resolution, Some((640, 480)));
        assert!(s.original_resolution);
        // And it overrides a wider file setting from the command line.
        let (file, _) = Settings::parse_file("resolution = 1920x1080\n");
        let m = file.merged(args(&["--resolution", "original"]).unwrap());
        assert_eq!(m.resolution, Some((640, 480)));
        assert!(m.original_resolution);
        // A command-line size overrides a file's `original` too.
        let m = s
            .clone()
            .merged(args(&["--resolution", "1920x1080"]).unwrap());
        assert_eq!(m.resolution, Some((1920, 1080)));
        assert!(!m.original_resolution);
        // No resolution on the command line keeps the file's.
        let m = s.merged(args(&["--vsync", "on"]).unwrap());
        assert!(m.original_resolution);
    }

    #[test]
    fn bad_values_are_reported() {
        let (_, warnings) = Settings::parse_file(
            "resolution = 8000x100\nfov = 200\nanisotropic = 3\ndisplay_mode = wide\ngamepad = wheel\nscaler = blurry\n",
        );
        assert_eq!(warnings.len(), 6, "{warnings:?}");
        assert!(warnings[1].contains("fov needs 30 to 150"), "{warnings:?}");
        // 47 degrees is in range (it snaps to the step), so no warning.
        let (s, warnings) = Settings::parse_file("fov = 47\n");
        assert!(warnings.is_empty());
        assert_eq!(s.fov, Some(45));
    }

    #[test]
    fn anisotropic_maps_to_the_ini_index() {
        assert_eq!(anisotropic_ini(Some(0)), Some(0));
        assert_eq!(anisotropic_ini(Some(2)), Some(1));
        assert_eq!(anisotropic_ini(Some(4)), Some(2));
        assert_eq!(anisotropic_ini(Some(8)), Some(3));
        assert_eq!(anisotropic_ini(Some(16)), Some(4));
        assert_eq!(anisotropic_ini(None), None);
    }

    #[test]
    fn postfx_off_is_a_value_that_overrides() {
        let (file, warnings) = Settings::parse_file("postfx = off\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(file.postfx, Some(Postfx::Off));
        let (cinematic, _) = Settings::parse_file("postfx = cinematic\n");
        let m = cinematic.merged(args(&["--postfx", "off"]).unwrap());
        assert_eq!(m.postfx, Some(Postfx::Off));
    }

    #[test]
    fn set_key_keeps_the_rest_of_the_file() {
        let t = template();
        let filled = set_key(&t, "game", r"D:\CMR3");
        assert!(filled.contains("\ngame = D:\\CMR3\n"));
        assert!(filled.contains("#runtime =\n"));
        let again = set_key(&filled, "game", r"E:\CMR3");
        assert_eq!(again.matches("\ngame =").count(), 1);
        let cleared = clear_key(&filled, "game");
        assert!(cleared.contains("#game ="));
        assert!(!cleared.contains("\ngame ="));
    }
}
