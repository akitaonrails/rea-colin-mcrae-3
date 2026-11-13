//! The wheel settings: `rea_wheel.ini` (the Wheel tab's file, read by the
//! SDK's in-game DirectInput layer, `rea_launcher::wheel::dinput`), kept
//! next to `launcher.cfg`, with `launcher.cfg`'s and the command line's
//! `wheel`, `wheel_lock` and `wheel_ffb` on top. The tab's other settings
//! (which wheel and pedals, deadzone, curve, pedal calibration) live only
//! in `rea_wheel.ini`.

use rea_launcher::cfg::{clear_key, set_key};
use rea_launcher::ffb::FfbSettings;
use rea_launcher::wheel::SteeringLock;
use rea_launcher::wheel::dinput::{Mode, Setup};

use crate::config::{self, Settings};

pub use rea_launcher::wheel::dinput::FILE;

/// The comment a new `rea_wheel.ini` starts with (the SDK tab's own).
pub const HEADER: &str = "rea_wheel.ini - the wheel settings, written by the launcher";

/// The wheel settings a run uses: `rea_wheel.ini`'s text (empty when
/// there is none), with the options `settings` sets.
pub fn setup(ini: &str, settings: &Settings) -> Setup {
    let mut setup = Setup::from_cfg(ini);
    if let Some(mode) = settings.wheel {
        setup.mode = mode;
    }
    if let Some(lock) = settings.wheel_lock {
        setup.wheel.lock = lock;
    }
    if let Some(strength) = settings.wheel_ffb {
        setup.ffb.strength = strength;
    }
    setup
}

/// The `rea_wheel.ini` the in-game layer reads this run: `ini` with the
/// run's settings, every other line (the calibration) kept.
pub fn runtime_ini(ini: &str, settings: &Settings) -> String {
    setup(ini, settings).to_cfg(ini, HEADER)
}

/// `launcher.cfg`'s text with the tab's `wheel`, `wheel_lock` and
/// `wheel_ffb`, as the window saves them: a default value clears its key
/// (`rea_wheel.ini` holds every value as well).
pub fn save_keys(text: &str, setup: &Setup) -> String {
    let put = |text: &str, key: &str, value: Option<String>| match value {
        Some(v) => set_key(text, key, &v),
        None => clear_key(text, key),
    };
    let lock = setup.wheel.lock;
    let strength = setup.ffb.strength;
    let text = put(
        text,
        "wheel",
        (setup.mode != Mode::Auto).then(|| setup.mode.key().into()),
    );
    let text = put(
        &text,
        "wheel_lock",
        (lock != SteeringLock::DEFAULT).then(|| config::lock_key(lock)),
    );
    put(
        &text,
        "wheel_ffb",
        (strength != FfbSettings::default().strength).then(|| strength.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(flags: &[&str]) -> Settings {
        Settings::parse_args(flags.iter().map(|f| f.to_string()))
            .unwrap()
            .0
    }

    #[test]
    fn the_options_win_over_the_tabs_file() {
        let ini = "mode = off\nlock = 900\nffb_strength = 20\ndeadzone = 4\n";
        let s = setup(ini, &settings(&[]));
        assert_eq!(s, Setup::from_cfg(ini), "nothing set: the file's");
        let s = setup(
            ini,
            &settings(&["--wheel", "on", "--wheel-lock", "540", "--wheel-ffb", "70"]),
        );
        assert_eq!(s.mode, Mode::On);
        assert_eq!(s.wheel.lock, SteeringLock::Degrees(540));
        assert_eq!(s.ffb.strength, 70);
        assert_eq!(s.deadzone, 4, "tab-only settings stay the file's");
    }

    #[test]
    fn the_runtime_file_keeps_the_calibration() {
        let ini = "# mine\nmode = auto\nwheel_vid = 0x0eb7\ngas_axis = 2\n";
        let text = runtime_ini(ini, &settings(&["--wheel", "off", "--wheel-ffb", "0"]));
        assert!(text.starts_with("# mine\n"), "{text}");
        assert!(text.contains("wheel_vid = 0x0eb7") && text.contains("gas_axis = 2"));
        assert!(text.contains("mode = off") && text.contains("ffb_strength = 0"));
        let fresh = runtime_ini("", &Settings::default());
        assert!(fresh.starts_with(&format!("# {HEADER}\n")), "{fresh}");
        assert_eq!(Setup::from_cfg(&fresh), Setup::default());
    }

    #[test]
    fn the_window_saves_what_differs_from_the_defaults() {
        let mut s = Setup::default();
        let text = save_keys("wheel = on\nvsync = on\n", &s);
        assert!(!text.lines().any(|l| l.starts_with("wheel")), "{text}");
        assert!(text.contains("vsync = on"));
        s.mode = Mode::Off;
        s.wheel.lock = SteeringLock::Full;
        s.ffb.strength = 75;
        let (saved, warnings) = Settings::parse_file(&save_keys("", &s));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(setup("", &saved), s);
    }
}
