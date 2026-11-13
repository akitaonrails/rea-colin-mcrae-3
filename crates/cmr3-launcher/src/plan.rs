//! What a launch will do, worked out from the settings and the machine
//! before anything is written: which components go in, the display mode
//! the game can honour, the `SilentPatchCMR3.ini` keys, Xidi's settings
//! and Wine's DLL overrides. `main` carries the plan out; tests and the
//! golden snapshots (`tests/snapshots/`) read it, so a change in what the
//! launcher produces shows up as a reviewed diff.

use crate::config::{DisplayMode, Gamepad, Postfx, Scaler, Settings};
use crate::{components, sp, xidi};

/// The machine the launch happens on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    /// Running under Wine or Proton.
    pub wine: bool,
    /// Inside a gamescope session (one screen).
    pub gamescope: bool,
    /// The desktop's size, when known.
    pub screen: Option<(u32, u32)>,
    /// `WINEDLLOVERRIDES` inherited from the caller.
    pub overrides: Option<String>,
}

/// One launch, decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The display mode chosen, and the one the game runs (fullscreen
    /// plays borderless under raw Wine: [`DisplayMode::effective`]).
    pub display_mode: DisplayMode,
    pub effective_mode: DisplayMode,
    /// Written as WIDTH/HEIGHT: the setting, else the desktop's size.
    pub resolution: Option<(u32, u32)>,
    /// The settings the ini is written from (with the effective mode).
    pub ini_settings: Settings,
    pub gamepad: Gamepad,
    pub gamepad_log: bool,
    /// `Xidi.ini`, or `None` when Xidi is taken out.
    pub xidi_ini: Option<String>,
    /// A new profile gets the pad as its controller.
    pub choose_controller: bool,
    /// DXVK's d3d9.dll beside the game (Wine only).
    pub dxvk: bool,
    pub hd_ui: bool,
    /// The DLLs Wine must load from the game folder, when every bundled
    /// part installs.
    pub natives: Vec<&'static str>,
    /// `WINEDLLOVERRIDES` for the game (Wine only). Kept for `cmr3-play`
    /// and the log; Wine's loader ignores it in the game's environment
    /// block, so the registry overrides below are what count.
    pub wine_overrides: Option<String>,
    /// Wine only: the per-program overrides (native, then builtin) set for
    /// `Rally_3PC.exe`, and the bundled DLLs whose override is removed
    /// ([`components::app_overrides`]).
    pub app_overrides: Vec<&'static str>,
    pub app_overrides_off: Vec<&'static str>,
    /// Read by `cmr3-play` on Linux, not the game: gamescope's filter and
    /// the post-processing layer.
    pub scaler: Scaler,
    pub postfx: Postfx,
}

/// Decides a launch.
pub fn plan(settings: &Settings, env: &Env) -> Plan {
    let display_mode = settings.display_mode.unwrap_or(DisplayMode::Fullscreen);
    let effective_mode = display_mode.effective(env.wine, env.gamescope);
    let mut ini_settings = settings.clone();
    ini_settings.display_mode = Some(effective_mode);
    let gamepad = settings.gamepad.unwrap_or_default();
    let gamepad_log = settings.gamepad_log == Some(true);
    let dxvk = env.wine && settings.dxvk != Some(false);
    let mut natives = vec!["dsound"];
    if gamepad != Gamepad::Off {
        natives.extend_from_slice(&components::XIDI_DLLS);
    }
    if dxvk {
        natives.push("d3d9");
    }
    let wine_overrides = env
        .wine
        .then(|| components::wine_overrides(env.overrides.as_deref(), &natives));
    let (app_overrides, app_overrides_off) = if env.wine {
        components::app_overrides(&natives)
    } else {
        (Vec::new(), Vec::new())
    };
    Plan {
        display_mode,
        effective_mode,
        resolution: settings.resolution.or(env.screen),
        ini_settings,
        gamepad,
        gamepad_log,
        xidi_ini: xidi::ini(gamepad, gamepad_log),
        choose_controller: gamepad != Gamepad::Off,
        dxvk,
        hd_ui: settings.hd_ui == Some(true),
        natives,
        wine_overrides,
        app_overrides,
        app_overrides_off,
        scaler: settings.scaler.unwrap_or(Scaler::Linear),
        postfx: settings.postfx.unwrap_or_default(),
    }
}

impl Plan {
    /// The `SilentPatchCMR3.ini` bytes this launch writes over `existing`
    /// (the runtime's file, empty on a first run), and the log lines.
    pub fn ini(
        &self,
        existing: &[u8],
        owned: &mut rea_launcher::ini::Owned,
    ) -> (Vec<u8>, Vec<String>) {
        sp::apply_picture(existing, &self.ini_settings, self.resolution, owned)
    }

    /// Everything the launch produces, as text: for the golden snapshots
    /// and bug reports. `ini_over` is the game's own ini before the
    /// launch (empty for a first run).
    pub fn describe(&self, ini_over: &[u8]) -> String {
        let mut owned = rea_launcher::ini::Owned::default();
        let (ini, ini_log) = self.ini(ini_over, &mut owned);
        let size = |r: Option<(u32, u32)>| r.map_or("unset".into(), |(w, h)| format!("{w}x{h}"));
        let on = |b: bool| if b { "on" } else { "off" };
        let mut out = String::new();
        out.push_str(&format!(
            "display_mode: {} (plays {})\n",
            self.display_mode.key(),
            self.effective_mode.key()
        ));
        out.push_str(&format!("resolution: {}\n", size(self.resolution)));
        out.push_str("components:\n");
        out.push_str("  SilentPatchCMR3.asi + dsound.dll (ASI loader): on\n");
        out.push_str(&format!(
            "  Xidi dinput8.dll: {} (gamepad {}, log {})\n",
            on(self.xidi_ini.is_some()),
            self.gamepad.key(),
            on(self.gamepad_log)
        ));
        out.push_str(&format!("  DXVK d3d9.dll: {}\n", on(self.dxvk)));
        out.push_str(&format!("  HD UI overlay: {}\n", on(self.hd_ui)));
        out.push_str(&format!(
            "  profile controller set to the pad when new: {}\n",
            on(self.choose_controller)
        ));
        out.push_str("patch sets: DRM-free xdelta (efigs, then polish) only for a protected official 1.1 Rally_3PC.exe\n");
        out.push_str(&format!(
            "WINEDLLOVERRIDES: {}\n",
            self.wine_overrides
                .as_deref()
                .unwrap_or("(not set: Windows)")
        ));
        if !self.app_overrides.is_empty() || !self.app_overrides_off.is_empty() {
            let list = |v: &[&str]| {
                if v.is_empty() {
                    "none".to_string()
                } else {
                    v.join(", ")
                }
            };
            out.push_str(&format!(
                "app DLL overrides (Rally_3PC.exe, native,builtin): {}; removed: {}\n",
                list(&self.app_overrides),
                list(&self.app_overrides_off)
            ));
        }
        let wheel = &self.ini_settings;
        let or_file = |v: Option<String>| v.unwrap_or_else(|| "rea_wheel.ini's".into());
        out.push_str(&format!(
            "wheel: support {}, lock {}, force feedback {}\n",
            or_file(wheel.wheel.map(|m| m.key().into())),
            or_file(wheel.wheel_lock.map(crate::config::lock_key)),
            or_file(wheel.wheel_ffb.map(|f| format!("{f} %"))),
        ));
        out.push_str("game arguments: (none)\n");
        out.push_str(&format!(
            "cmr3-play (Linux): scaler {}, postfx {}\n",
            self.scaler.key(),
            self.postfx.key()
        ));
        out.push_str(&format!("ini log: {}\n", ini_log.join(", ")));
        out.push_str("--- SilentPatchCMR3.ini\n");
        out.push_str(&String::from_utf8_lossy(&ini));
        match &self.xidi_ini {
            Some(x) => {
                out.push_str("--- Xidi.ini\n");
                out.push_str(x);
            }
            None => out.push_str("--- Xidi.ini: (removed)\n"),
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(text: &str) -> Settings {
        let (s, w) = Settings::parse_file(text);
        assert!(w.is_empty(), "{w:?}");
        s
    }

    const WINE: Env = Env {
        wine: true,
        gamescope: false,
        screen: Some((2560, 1440)),
        overrides: None,
    };

    #[test]
    fn windows_has_no_overrides_and_no_dxvk() {
        let p = plan(&settings(""), &Env::default());
        assert!(!p.dxvk);
        assert_eq!(p.wine_overrides, None);
        assert!(p.app_overrides.is_empty() && p.app_overrides_off.is_empty());
        assert_eq!(p.effective_mode, DisplayMode::Fullscreen);
        assert_eq!(p.resolution, None);
    }

    #[test]
    fn wine_gets_dxvk_borderless_and_the_overrides() {
        let p = plan(&settings(""), &WINE);
        assert!(p.dxvk);
        assert_eq!(p.effective_mode, DisplayMode::Borderless);
        assert_eq!(p.resolution, Some((2560, 1440)));
        assert_eq!(
            p.wine_overrides.as_deref(),
            Some("dsound,dinput8,msvcp140,msvcp140_atomic_wait,concrt140,vcruntime140,d3d9=n,b")
        );
        // What Wine's loader reads for a game the launcher starts: the
        // per-program registry key (the environment block is ignored).
        assert_eq!(p.app_overrides, components::BUNDLED_DLLS);
        assert!(p.app_overrides_off.is_empty());
        let p = plan(&settings("dxvk = off\ngamepad = off\n"), &WINE);
        assert_eq!(p.wine_overrides.as_deref(), Some("dsound=n,b"));
        assert_eq!(p.xidi_ini, None);
        assert_eq!(p.app_overrides, ["dsound"]);
        assert_eq!(
            p.app_overrides_off,
            [
                "dinput8",
                "msvcp140",
                "msvcp140_atomic_wait",
                "concrt140",
                "vcruntime140",
                "d3d9"
            ]
        );
        assert!(!p.choose_controller);
    }

    #[test]
    fn every_game_option_reaches_the_ini() {
        let p = plan(
            &settings(
                "resolution = 1920x1080\ndisplay_mode = windowed\nvsync = on\nanisotropic = 8\n\
                 fsaa = on\nfov = 90\nshadows = soft\n",
            ),
            &WINE,
        );
        let (ini, _) = p.ini(b"", &mut rea_launcher::ini::Owned::default());
        let ini = String::from_utf8(ini).unwrap();
        for want in [
            "WIDTH=1920",
            "HEIGHT=1080",
            "DISPLAY_MODE=1",
            "VSYNC=1",
            "ANISOTROPIC=3",
            "FSAA=1",
            "EXTERIOR_FOV=90",
            "INTERIOR_FOV=90",
            "SHARPER_SHADOWS=0",
        ] {
            assert!(ini.contains(want), "{want} in {ini}");
        }
    }

    /// Each flag, through the same parser as launcher.cfg, to what it
    /// changes in the game's files or the launch.
    #[test]
    fn every_flag_reaches_what_it_changes() {
        let run = |flags: &[&str]| {
            let (s, _) = Settings::parse_args(flags.iter().map(|f| f.to_string())).unwrap();
            let p = plan(&s, &WINE);
            let (ini, _) = p.ini(b"", &mut rea_launcher::ini::Owned::default());
            (s, p, String::from_utf8(ini).unwrap())
        };
        let (s, _, _) = run(&["--game", r"C:\G", "--saves", r"D:\S", "--runtime", r"E:\R"]);
        assert_eq!(
            (s.game.unwrap(), s.saves.unwrap(), s.runtime.unwrap()),
            (
                r"C:\G".into(),
                r"D:\S".into(),
                std::path::PathBuf::from(r"E:\R")
            )
        );
        let (_, _, ini) = run(&["--resolution", "original"]);
        assert!(ini.contains("WIDTH=640\nHEIGHT=480\n"), "{ini}");
        let (_, _, ini) = run(&["--display-mode", "windowed"]);
        assert!(ini.contains("DISPLAY_MODE=1"));
        let (_, _, ini) = run(&["--vsync", "off"]);
        assert!(ini.contains("VSYNC=0"));
        let (_, _, ini) = run(&["--anisotropic", "16"]);
        assert!(ini.contains("ANISOTROPIC=4"));
        let (_, _, ini) = run(&["--fsaa", "on"]);
        assert!(ini.contains("FSAA=1"));
        let (_, _, ini) = run(&["--fov", "100"]);
        assert!(ini.contains("EXTERIOR_FOV=100") && ini.contains("INTERIOR_FOV=100"));
        let (_, _, ini) = run(&["--shadows", "sharp"]);
        assert!(ini.contains("[Advanced]\nSHARPER_SHADOWS=1"));
        let (_, p, _) = run(&["--hd-ui", "on"]);
        assert!(p.hd_ui);
        let (_, p, _) = run(&["--gamepad", "standard"]);
        assert!(p.xidi_ini.unwrap().contains("Type = StandardGamepad"));
        let (_, p, _) = run(&["--gamepad", "off"]);
        assert_eq!(p.xidi_ini, None);
        let (_, p, _) = run(&["--gamepad-log", "on"]);
        assert!(p.xidi_ini.unwrap().contains("Enabled = yes"));
        let (_, p, _) = run(&["--dxvk", "off"]);
        assert_eq!(
            p.wine_overrides.as_deref(),
            Some("dsound,dinput8,msvcp140,msvcp140_atomic_wait,concrt140,vcruntime140=n,b")
        );
        let (_, p, _) = run(&["--scaler", "pixel", "--postfx", "cinematic"]);
        assert_eq!((p.scaler, p.postfx), (Scaler::Pixel, Postfx::Cinematic));
        // The wheel options reach the rea_wheel.ini the in-game layer reads.
        let (s, p, _) = run(&["--wheel", "on", "--wheel-lock", "900", "--wheel-ffb", "30"]);
        let ini = crate::wheel::runtime_ini("deadzone = 2\n", &s);
        for want in [
            "mode = on",
            "lock = 900",
            "ffb_strength = 30",
            "deadzone = 2",
        ] {
            assert!(ini.contains(want), "{want} in {ini}");
        }
        assert!(
            p.describe(b"")
                .contains("wheel: support on, lock 900, force feedback 30 %")
        );
    }

    #[test]
    fn the_players_overrides_are_kept_after_ours() {
        let env = Env {
            overrides: Some("winedbg=d;d3d9=b".into()),
            ..WINE
        };
        let p = plan(&settings(""), &env);
        assert_eq!(
            p.wine_overrides.as_deref(),
            Some(
                "dsound,dinput8,msvcp140,msvcp140_atomic_wait,concrt140,vcruntime140,d3d9=n,b;winedbg=d"
            )
        );
    }
}
