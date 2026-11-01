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

/// Anisotropic filtering levels offered, with their SilentPatch ini value
/// (the menu index: 0 off, then 2×, 4×, 8×, 16×).
pub const ANISOTROPIC: [(u32, &str); 5] = [(0, "Off"), (1, "2×"), (2, "4×"), (3, "8×"), (4, "16×")];

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

# Xidi's log: what the game asks of DirectInput and what every pad
# control sent — turn it on when a button doesn't arrive.
#gamepad_log = off

# Gamepad through Xidi: rally maps the triggers to the game's pedal axis
# and the d-pad and Start to the menu keys; standard passes the pad through
# as it reports itself; off runs the game's own DirectInput handling.
#gamepad = rally

# Linux only: play through DXVK (Vulkan) instead of Wine's OpenGL
# translator — the difference between native speed and ~10 FPS at 4K.
#dxvk = on


# Linux (cmr3-play) only: how gamescope enlarges the picture. nearest
# (sharp pixels), pixel (sharp, all pixels the same size), fsr (smooth,
# then sharpened) or linear (smooth).
#scaler = linear
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
  --gamepad <preset>       rally, standard or off
  --gamepad-log on|off     Xidi's log of what the game and the pad exchange
  --dxvk on|off            DXVK under Wine (Linux only)

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

impl Settings {
    /// Reads a settings file; the second value lists lines it could not use.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut s = Settings::default();
        let mut warnings = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            let mut bad = |what: &str| {
                warnings.push(format!("{FILE_NAME} line {}: {what} {value:?}", n + 1));
            };
            match key {
                "game" => s.game = Some(value.into()),
                "runtime" => s.runtime = Some(value.into()),
                "saves" => s.saves = Some(value.into()),
                "resolution" => match parse_resolution(value) {
                    Some((640, 480)) => {
                        s.resolution = Some((640, 480));
                        s.original_resolution = true;
                    }
                    Some(r) => s.resolution = Some(r),
                    None => bad("resolution needs WIDTHxHEIGHT or original, not"),
                },
                "display_mode" => match DisplayMode::parse(value) {
                    Some(m) => s.display_mode = Some(m),
                    None => bad("display_mode needs fullscreen, windowed or borderless, not"),
                },
                "vsync" => match parse_bool(value) {
                    Some(b) => s.vsync = Some(b),
                    None => bad("vsync needs true or false, not"),
                },
                "anisotropic" => match parse_anisotropic(value) {
                    Some(a) => s.anisotropic = Some(a),
                    None => bad("anisotropic needs off, 2, 4, 8 or 16, not"),
                },
                "fsaa" => match parse_bool(value) {
                    Some(b) => s.fsaa = Some(b),
                    None => bad("fsaa needs on or off, not"),
                },
                "fov" => match parse_fov(value) {
                    Some(f) => s.fov = Some(f),
                    None => bad("fov needs a number from 30 to 150, not"),
                },
                "hd_ui" => match parse_bool(value) {
                    Some(b) => s.hd_ui = Some(b),
                    None => bad("hd_ui needs on or off, not"),
                },
                "gamepad_log" => match parse_bool(value) {
                    Some(b) => s.gamepad_log = Some(b),
                    None => bad("gamepad_log needs on or off, not"),
                },
                "gamepad" => match Gamepad::parse(value) {
                    Some(g) => s.gamepad = Some(g),
                    None => bad("gamepad needs rally, standard or off, not"),
                },
                "dxvk" => match parse_bool(value) {
                    Some(b) => s.dxvk = Some(b),
                    None => bad("dxvk needs on or off, not"),
                },
                "scaler" => match Scaler::parse(value) {
                    Some(f) => s.scaler = Some(f),
                    None => bad("scaler needs nearest, pixel, fsr or linear, not"),
                },
                _ => {}
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
            original_resolution: over.original_resolution || self.original_resolution,
            display_mode: over.display_mode.or(self.display_mode),
            vsync: over.vsync.or(self.vsync),
            anisotropic: over.anisotropic.or(self.anisotropic),
            fsaa: over.fsaa.or(self.fsaa),
            fov: over.fov.or(self.fov),
            hd_ui: over.hd_ui.or(self.hd_ui),
            gamepad: over.gamepad.or(self.gamepad),
            gamepad_log: over.gamepad_log.or(self.gamepad_log),
            dxvk: over.dxvk.or(self.dxvk),
            scaler: over.scaler.or(self.scaler),
        }
    }

    /// The display resolution to write, if any; `DESKTOP` (in `form`)
    /// resolves to the screen's size at launch.
    pub fn display_resolution(&self) -> Option<(u32, u32)> {
        self.resolution.filter(|&r| r != DESKTOP)
    }

    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Settings, String> {
        let mut s = Settings::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .ok_or_else(|| format!("{name} needs a value\n\n{USAGE}"))
            };
            match arg.as_str() {
                "--game" => s.game = Some(value("--game")?.into()),
                "--saves" => s.saves = Some(value("--saves")?.into()),
                "--runtime" => s.runtime = Some(value("--runtime")?.into()),
                "--resolution" => {
                    let v = value("--resolution")?;
                    let r = parse_resolution(&v).ok_or_else(|| {
                        format!("--resolution needs WIDTHxHEIGHT or original, not {v}\n\n{USAGE}")
                    })?;
                    s.resolution = Some(r);
                    s.original_resolution = r == (640, 480);
                }
                "--display-mode" => {
                    let v = value("--display-mode")?;
                    s.display_mode = Some(DisplayMode::parse(&v).ok_or_else(|| {
                        format!(
                            "--display-mode needs fullscreen, windowed or borderless, not {v}\n\n{USAGE}"
                        )
                    })?);
                }
                "--anisotropic" => {
                    let v = value("--anisotropic")?;
                    s.anisotropic = Some(parse_anisotropic(&v).ok_or_else(|| {
                        format!("--anisotropic needs off, 2, 4, 8 or 16, not {v}\n\n{USAGE}")
                    })?);
                }
                "--fov" => {
                    let v = value("--fov")?;
                    s.fov = Some(parse_fov(&v).ok_or_else(|| {
                        format!("--fov needs a number from 30 to 150, not {v}\n\n{USAGE}")
                    })?);
                }
                "--gamepad" => {
                    let v = value("--gamepad")?;
                    s.gamepad = Some(Gamepad::parse(&v).ok_or_else(|| {
                        format!("--gamepad needs rally, standard or off, not {v}\n\n{USAGE}")
                    })?);
                }
                flag @ ("--vsync" | "--fsaa" | "--hd-ui" | "--dxvk") => {
                    let v = value(flag)?;
                    let b = parse_bool(&v)
                        .ok_or_else(|| format!("{flag} needs on or off, not {v}\n\n{USAGE}"))?;
                    match flag {
                        "--vsync" => s.vsync = Some(b),
                        "--fsaa" => s.fsaa = Some(b),
                        "--hd-ui" => s.hd_ui = Some(b),
                        _ => s.dxvk = Some(b),
                    }
                }
                "--help" | "-h" | "/?" => return Err(USAGE.into()),
                other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
            }
        }
        Ok(s)
    }
}

/// `resolution = desktop`: the launcher's keyword meaning "the screen's
/// size"; replaced by real numbers before anything is written.
pub const DESKTOP: (u32, u32) = (0, 0);

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
        assert_eq!(s.display_resolution(), Some((640, 480)));
        assert!(s.original_resolution);
        // And it overrides a wider file setting from the command line.
        let (file, _) = Settings::parse_file("resolution = 1920x1080\n");
        let m = file.merged(args(&["--resolution", "original"]).unwrap());
        assert_eq!(m.display_resolution(), Some((640, 480)));
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
