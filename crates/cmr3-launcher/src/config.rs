//! Command line and the settings file (`launcher.cfg`).
//!
//! The file is plain `key = value` lines with `#` comments, written once from
//! a commented template so users can edit it. The launcher later changes
//! only the lines it owns (`game`), keeping everything else as the user left
//! it. Unknown keys are ignored so older launchers can read newer files;
//! values it cannot read are reported and ignored.

use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    /// Where the player's profile (`CMR3` file) is kept (see `TEMPLATE`).
    pub saves: Option<PathBuf>,
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

    fn parse(v: &str) -> Option<Postfx> {
        let v = v.trim().to_ascii_lowercase();
        Postfx::ALL.into_iter().find(|m| m.key() == v)
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

    fn parse(v: &str) -> Option<ShadowStyle> {
        let v = v.trim().to_ascii_lowercase();
        ShadowStyle::ALL.into_iter().find(|m| m.key() == v)
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

    fn parse(v: &str) -> Option<DisplayMode> {
        let v = v.trim().to_ascii_lowercase();
        DisplayMode::ALL.into_iter().find(|m| m.key() == v)
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

    fn parse(v: &str) -> Option<Gamepad> {
        let v = v.trim().to_ascii_lowercase();
        Gamepad::ALL.into_iter().find(|g| g.key() == v)
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

    fn parse(v: &str) -> Option<Scaler> {
        let v = v.trim().to_ascii_lowercase();
        Scaler::ALL.into_iter().find(|s| s.key() == v)
    }
}

pub const FILE_NAME: &str = "launcher.cfg";

/// The file written on first run.
pub const TEMPLATE: &str = r"# Colin McRae Rally 3 launcher settings.
#
# One setting per line as key = value; lines starting with # are comments.
# Command-line options (--game, --runtime) override this file for one run.
# A launcher.cfg next to CMR3Launcher.exe is used instead of this one when
# it exists (for portable installs).

# The folder with your installed Rally_3PC.exe. Filled in by --game; the
# game folder is only read, never changed.
game =

# Where the launcher keeps its private, writable copy of the game (about
# 3 GB the first time; hard-linked instead of copied when that is
# possible). Default: %LOCALAPPDATA%\CMR3Launcher\Rally3
#runtime =

# Where your profile (the game's CMR3 file) is kept. It is copied into the
# game before it starts and back when it exits.
# Default: %USERPROFILE%\Saved Games\Colin McRae Rally 3
#saves =

# Display resolution, e.g. 1920x1080, or original for the 640x480 of 2003.
# Unset: the desktop's size. The launcher's SilentPatch adds every other
# aspect ratio, so 16:9 shows more to the sides, unstretched.
#resolution = 3840x2160

# Fullscreen (the default), windowed or borderless.
#display_mode = fullscreen

# true waits for the screen's refresh before showing each frame (no
# tearing); false doesn't. Unset keeps the game's own choice.
#vsync = true

# Anisotropic filtering: off, 2, 4, 8 or 16. Sharpens the road surface at
# a distance. Unset keeps the game's.
#anisotropic = 16

# Anti-aliasing. SilentPatch fixed its broken sun and shadows, so it is
# safe at any size; it costs speed at 4K.
#fsaa = off

# Field of view in degrees for both cameras, 30 to 150 in steps of 5. The
# game's own is 75; wider screens already show more to the sides.
#fov = 75

# The HD UI pack (by Bekoha): fonts, banners and race UI redrawn for 4K.
#hd_ui = on

# Car shadows: sharp (the sharper 2005 look, SilentPatch's default) or
# soft (the blurrier 2003 one).
#shadows = sharp

# Xidi's log: what the game asks of DirectInput and what every pad
# control sent — turn it on when a button doesn't arrive.
#gamepad_log = off

# Gamepad through Xidi, as a game controller: rally puts the triggers on
# the game's pedal axis (both at once), ignores the resting left-stick Y,
# and has the d-pad and A also press the arrow keys and Enter for the
# menus (which answer only the keyboard); standard passes the pad through as it
# reports itself. With either, a new profile starts with the pad chosen
# in the game's Options > Controls. off: no Xidi, the game's own
# DirectInput.
#gamepad = rally

# Linux only: play through DXVK (Vulkan) instead of Wine's OpenGL
# translator — the difference between native speed and ~10 FPS at 4K.
#dxvk = on


# Linux (cmr3-play) only: how gamescope enlarges the picture. nearest
# (sharp pixels), pixel (sharp, all pixels the same size), fsr (smooth,
# then sharpened) or linear (smooth).
#scaler = linear

# Linux (cmr3-play) only: post-processing over the finished frame, on
# top of the game's own picture. off (the default): the faithful,
# washed-out picture. color: real blacks, fuller contrast and CAS
# sharpening. cinematic: a stronger grade, plus depth-based ambient
# occlusion, which lifts the flat look (needs the AO
# shaders; docs/ENHANCEMENTS.md). Needs dxvk = on: it works on the
# game's Vulkan.
#postfx = cinematic
";

pub const USAGE: &str = concat!(
    "CMR3Launcher ",
    env!("CARGO_PKG_VERSION"),
    "\n\n",
    "\
CMR3Launcher [options]

  --game <folder>          the folder with your installed Rally_3PC.exe (remembered)
  --saves <folder>         where your profile is kept
  --runtime <folder>       where the launcher keeps its private copy
  --resolution <WxH>       display resolution, or original (640x480)
  --display-mode <mode>    fullscreen, windowed or borderless
  --vsync on|off           wait for the screen's refresh
  --anisotropic <n>        off, 2, 4, 8 or 16
  --fsaa on|off            anti-aliasing
  --fov <30-150>           field of view for both cameras
  --hd-ui on|off           the HD UI pack
  --shadows <style>        sharp or soft car shadows
  --postfx <style>         off, color or cinematic (Linux)
  --gamepad <preset>       rally, standard (Xidi controller) or off
  --gamepad-log on|off     Xidi's log of what the game and the pad exchange
  --dxvk on|off            DXVK under Wine (Linux only)
  --scaler <filter>        nearest, pixel, fsr or linear: gamescope's (Linux)

These override launcher.cfg for one run (only --game is remembered).
Without options the settings window opens first.

  --no-window              play with the saved settings, no window
  --settings [--linux]     only the settings window (exit 0: play, 3: closed;
                           --linux adds the gamescope scaler, for cmr3-play)

CMR3Launcher --check [--game <folder>]
  checks the game folder, its Rally_3PC.exe version and the bundled parts

Other settings are in launcher.cfg (see the comments in it)."
);

fn parse_bool(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// `WxH` within what a modern screen can hold; `original` is 640×480, the
/// size the game shipped with.
fn parse_resolution(v: &str) -> Option<(u32, u32)> {
    let lower = v.trim().to_ascii_lowercase();
    if lower == "original" {
        return Some((640, 480));
    }
    let (w, h) = lower.split_once('x')?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    ((640..=7680).contains(&w) && (480..=4320).contains(&h)).then_some((w, h))
}

/// An anisotropic level: `off`, or 2, 4, 8, 16.
fn parse_anisotropic(v: &str) -> Option<u32> {
    let v = v.trim().to_ascii_lowercase();
    if v == "off" {
        return Some(0);
    }
    let n: u32 = v.parse().ok()?;
    [2, 4, 8, 16].contains(&n).then_some(n)
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

fn parse_fov(v: &str) -> Option<u32> {
    let n: u32 = v.trim().parse().ok()?;
    (30..=150).contains(&n).then_some(n - n % 5)
}

/// Splits `key = value`, ignoring comments and blank values.
fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    let value = value.trim();
    (!value.is_empty()).then_some((key.trim(), value))
}

/// Why a setting was not used.
enum Rejected {
    /// Not a key this launcher knows.
    Unknown,
    /// The values the key takes, for the message.
    Needs(&'static str),
}

impl Settings {
    /// Sets one value by its `launcher.cfg` key. The command line's
    /// `--some-key` is `some_key`, so the file and the flags read alike.
    fn apply(&mut self, key: &str, value: &str) -> Result<(), Rejected> {
        fn need<T>(v: Option<T>, what: &'static str) -> Result<T, Rejected> {
            v.ok_or(Rejected::Needs(what))
        }
        const ON_OFF: &str = "on or off";
        match key {
            "game" => self.game = Some(value.into()),
            "runtime" => self.runtime = Some(value.into()),
            "saves" => self.saves = Some(value.into()),
            "resolution" => {
                let r = need(parse_resolution(value), "WIDTHxHEIGHT or original")?;
                self.resolution = Some(r);
                self.original_resolution = r == (640, 480);
            }
            "display_mode" => {
                let m = DisplayMode::parse(value);
                self.display_mode = Some(need(m, "fullscreen, windowed or borderless")?);
            }
            "vsync" => self.vsync = Some(need(parse_bool(value), ON_OFF)?),
            "anisotropic" => {
                let a = parse_anisotropic(value);
                self.anisotropic = Some(need(a, "off, 2, 4, 8 or 16")?);
            }
            "fsaa" => self.fsaa = Some(need(parse_bool(value), ON_OFF)?),
            "fov" => self.fov = Some(need(parse_fov(value), "a number from 30 to 150")?),
            "shadows" => self.shadows = Some(need(ShadowStyle::parse(value), "sharp or soft")?),
            "hd_ui" => self.hd_ui = Some(need(parse_bool(value), ON_OFF)?),
            "gamepad_log" => self.gamepad_log = Some(need(parse_bool(value), ON_OFF)?),
            "gamepad" => {
                let g = Gamepad::parse(value);
                self.gamepad = Some(need(g, "rally, standard or off")?);
            }
            "dxvk" => self.dxvk = Some(need(parse_bool(value), ON_OFF)?),
            "postfx" => {
                let p = Postfx::parse(value);
                self.postfx = Some(need(p, "off, color or cinematic")?);
            }
            "scaler" => {
                let f = Scaler::parse(value);
                self.scaler = Some(need(f, "nearest, pixel, fsr or linear")?);
            }
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// Reads a settings file; the second value lists lines it could not use.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut s = Settings::default();
        let mut warnings = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            if let Err(Rejected::Needs(what)) = s.apply(key, value) {
                warnings.push(format!(
                    "{FILE_NAME} line {}: {key} needs {what}, not {value:?}",
                    n + 1
                ));
            }
        }
        (s, warnings)
    }

    /// Command-line values override the file's.
    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            saves: over.saves.or(self.saves),
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

    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Settings, String> {
        let mut s = Settings::default();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            if matches!(flag.as_str(), "--help" | "-h" | "/?") {
                return Err(USAGE.into());
            }
            let unknown = || format!("unknown argument {flag}\n\n{USAGE}");
            let key = flag
                .strip_prefix("--")
                .ok_or_else(unknown)?
                .replace('-', "_");
            let Some(value) = args.next() else {
                // A known key with its value missing, or no such key.
                return Err(match Settings::default().apply(&key, "") {
                    Err(Rejected::Unknown) => unknown(),
                    _ => format!("{flag} needs a value\n\n{USAGE}"),
                });
            };
            match s.apply(&key, &value) {
                Ok(()) => {}
                Err(Rejected::Unknown) => return Err(unknown()),
                Err(Rejected::Needs(what)) => {
                    return Err(format!("{flag} needs {what}, not {value}\n\n{USAGE}"));
                }
            }
        }
        Ok(s)
    }
}

/// Sets `key` in a settings file, keeping every other line (and comments)
/// as it was: replaces the first active or empty `key =` line, else appends.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let is_key = |line: &str| {
        let line = line.trim();
        !line.starts_with('#') && line.split_once('=').is_some_and(|(k, _)| k.trim() == key)
    };
    let mut out = String::with_capacity(text.len() + key.len() + value.len() + 4);
    let mut done = false;
    for line in text.lines() {
        if !done && is_key(line) {
            out.push_str(&format!("{key} = {value}"));
            done = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    if !done {
        out.push_str(&format!("{key} = {value}\n"));
    }
    out
}

/// Turns `key` off in a settings file: its active line becomes a comment
/// (`#key = `), so the default applies; other lines are kept.
pub fn clear_key(text: &str, key: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with('#') && t.split_once('=').is_some_and(|(k, _)| k.trim() == key) {
            out.push_str(&format!("#{key} ="));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Result<Settings, String> {
        Settings::parse_args(a.iter().map(|s| s.to_string()))
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

    #[test]
    fn parses_every_setting() {
        let (s, warnings) = Settings::parse_file(
            "game = C:\\G\nresolution = 3840x2160\ndisplay_mode = borderless\nvsync = off\nanisotropic = 16\nfsaa = on\nfov = 90\nhd_ui = off\ngamepad = standard\ndxvk = off\nscaler = fsr\n",
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s.game, Some(PathBuf::from(r"C:\G")));
        assert_eq!(s.resolution, Some((3840, 2160)));
        assert_eq!(s.display_mode, Some(DisplayMode::Borderless));
        assert_eq!(s.vsync, Some(false));
        assert_eq!(s.anisotropic, Some(16));
        assert_eq!(s.fsaa, Some(true));
        assert_eq!(s.fov, Some(90));
        assert_eq!(s.hd_ui, Some(false));
        assert_eq!(s.gamepad, Some(Gamepad::Standard));
        assert_eq!(s.dxvk, Some(false));
        assert_eq!(s.scaler, Some(Scaler::Fsr));
    }

    #[test]
    fn template_parses_clean() {
        let (s, warnings) = Settings::parse_file(TEMPLATE);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(s, Settings::default(), "every setting is off by default");
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
        // 45 degrees is in range (it snaps to the step), so no warning.
        let (_, warnings) = Settings::parse_file("fov = 45\n");
        assert!(warnings.is_empty());
    }

    #[test]
    fn fov_snaps_to_steps_of_five() {
        assert_eq!(parse_fov("77"), Some(75));
        assert_eq!(parse_fov("78"), Some(75));
        assert_eq!(parse_fov("80"), Some(80));
        assert_eq!(parse_fov("29"), None);
    }

    #[test]
    fn anisotropic_maps_to_the_ini_index() {
        assert_eq!(anisotropic_ini(Some(0)), Some(0));
        assert_eq!(anisotropic_ini(Some(2)), Some(1));
        assert_eq!(anisotropic_ini(Some(16)), Some(4));
        assert_eq!(anisotropic_ini(None), None);
    }

    #[test]
    fn every_flag_parses() {
        let s = args(&[
            "--game",
            "D:\\G",
            "--resolution",
            "2560x1440",
            "--display-mode",
            "windowed",
            "--vsync",
            "on",
            "--anisotropic",
            "8",
            "--fsaa",
            "off",
            "--fov",
            "90",
            "--hd-ui",
            "off",
            "--gamepad",
            "off",
            "--dxvk",
            "off",
        ])
        .unwrap();
        assert_eq!(s.game, Some(PathBuf::from(r"D:\G")));
        assert_eq!(s.resolution, Some((2560, 1440)));
        assert_eq!(s.display_mode, Some(DisplayMode::Windowed));
        assert_eq!(s.vsync, Some(true));
        assert_eq!(s.anisotropic, Some(8));
        assert_eq!(s.fsaa, Some(false));
        assert_eq!(s.fov, Some(90));
        assert_eq!(s.hd_ui, Some(false));
        assert_eq!(s.gamepad, Some(Gamepad::Off));
        assert_eq!(s.dxvk, Some(false));
        assert!(args(&["--vsync", "maybe"]).is_err());
        assert!(args(&["--fov"]).is_err());
        assert!(args(&["--bogus"]).is_err());
        assert!(args(&["--help"]).unwrap_err().contains("CMR3Launcher"));
    }

    #[test]
    fn every_documented_flag_is_accepted() {
        // Each `--flag` in USAGE parses (they share launcher.cfg's keys).
        for line in USAGE.lines() {
            let Some(flag) = line.trim().strip_prefix("--") else {
                continue;
            };
            let flag = flag.split_whitespace().next().unwrap();
            if matches!(flag, "no-window" | "settings" | "linux" | "check") {
                continue; // taken by main before parse_args
            }
            if let Err(e) = args(&[&format!("--{flag}"), "?"]) {
                assert!(!e.starts_with("unknown"), "--{flag}: {e}");
            }
        }
        let s = args(&[
            "--shadows",
            "soft",
            "--postfx",
            "color",
            "--gamepad-log",
            "on",
        ])
        .unwrap();
        assert_eq!(s.shadows, Some(ShadowStyle::Soft));
        assert_eq!(s.postfx, Some(Postfx::Color));
        assert_eq!(s.gamepad_log, Some(true));
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
        let filled = set_key(TEMPLATE, "game", r"D:\CMR3");
        assert!(filled.contains("\ngame = D:\\CMR3\n"));
        assert!(filled.contains("#runtime =\n"));
        let again = set_key(&filled, "game", r"E:\CMR3");
        assert_eq!(again.matches("game =").count(), 1);
        let cleared = clear_key(&filled, "game");
        assert!(cleared.contains("#game ="));
        assert!(!cleared.contains("\ngame ="));
    }
}
