//! The wheel settings: the SDK's [`Setup`] for the in-game DirectInput
//! layer (`rea_launcher::wheel::dinput`), kept in `launcher.cfg` as the
//! SDK's option rows (`dinput::options`) like every other option. The
//! pedal calibration stays in `rea_wheel.ini` beside `launcher.cfg`; at
//! each start the launcher writes the layer's own `rea_wheel.ini` from
//! both ([`runtime_ini`]).

use rea_launcher::cfg::spec::Value;
use rea_launcher::wheel::SteeringLock;
use rea_launcher::wheel::dinput::options::OPTIONS;
use rea_launcher::wheel::dinput::{HEADER, Profile, Setup};
use rea_launcher::wheel::rig;

pub use rea_launcher::wheel::dinput::FILE;

/// The rally games' setup before anything is set: the SDK's, with a
/// 540° lock (the same in CMR3, CMR04 and CMR2005).
pub fn default_setup() -> Setup {
    let mut setup = Setup::default();
    setup.wheel.lock = SteeringLock::Degrees(540);
    setup
}

/// The wheel options a file or a command line set: the setup, and which
/// keys were given (so the command line overrides `launcher.cfg` key by
/// key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wheel {
    pub setup: Setup,
    given: Vec<&'static str>,
}

impl Default for Wheel {
    fn default() -> Self {
        Wheel {
            setup: default_setup(),
            given: Vec::new(),
        }
    }
}

impl Wheel {
    /// Takes a checked value; false when `key` isn't a wheel key.
    pub fn apply(&mut self, key: &str, value: Value) -> bool {
        let Some(row) = OPTIONS.iter().find(|r| r.key == key) else {
            return false;
        };
        if self.setup.apply(key, value) && !self.given.contains(&row.key) {
            self.given.push(row.key);
        }
        true
    }

    /// These options with `over`'s given keys on top.
    pub fn merged(mut self, over: &Wheel) -> Wheel {
        for row in OPTIONS.iter().filter(|r| over.given.contains(&r.key)) {
            let text = over.setup.value(row.key).unwrap_or_default();
            if let Ok(value) = row.parse(&text) {
                self.apply(row.key, value);
            }
        }
        self
    }
}

/// The `rea_wheel.ini` the in-game layer reads this run: the setup, and
/// the calibration from the config folder's file (`None`: none yet).
pub fn runtime_ini(setup: &Setup, calibration: Option<&str>) -> String {
    let profile = calibration.and_then(Profile::from_cfg);
    rig::save("", setup, profile.as_ref(), HEADER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use rea_launcher::wheel::dinput::Mode;

    fn flags(f: &[&str]) -> Settings {
        Settings::parse_args(f.iter().map(|f| f.to_string()))
            .unwrap()
            .0
    }

    #[test]
    fn the_rally_default_is_a_540_degree_lock() {
        let s = Settings::default();
        assert_eq!(s.wheel.setup.wheel.lock, SteeringLock::Degrees(540));
        assert_eq!(s.wheel.setup.mode, Mode::Auto);
    }

    #[test]
    fn flags_override_the_file_key_by_key() {
        let file = Settings::parse_file("wheel = off\nwheel_lock = 900\nwheel_deadzone = 4\n").0;
        let run = file.merged(flags(&["--wheel", "on", "--wheel-ffb", "30"]));
        let s = &run.wheel.setup;
        assert_eq!(s.mode, Mode::On);
        assert_eq!(s.wheel.lock, SteeringLock::Degrees(900), "the file's");
        assert_eq!((s.deadzone, s.ffb.strength), (4, 30));
    }

    #[test]
    fn the_runtime_file_holds_the_setup_and_the_calibration() {
        let wheel = rea_launcher::wheel::DeviceId {
            name: "FANATEC CSL DD".into(),
            vid: 0x0eb7,
            pid: 0x0020,
            guid: "{csl}".into(),
        };
        let resting = rea_launcher::wheel::DeviceState {
            axes: vec![0, 32_767, 32_767],
            buttons: vec![false; 8],
        };
        let calibration = Profile::guess(wheel, &resting, None).to_cfg("", "");
        let calibration = calibration.as_str();
        assert!(Profile::from_cfg(calibration).is_some());
        let run = flags(&["--wheel", "on", "--wheel-lock", "270"]);
        let text = runtime_ini(&run.wheel.setup, Some(calibration));
        assert!(text.starts_with(&format!("# {HEADER}\n")), "{text}");
        assert_eq!(Setup::from_cfg(&text), run.wheel.setup);
        assert_eq!(
            Profile::from_cfg(&text),
            Profile::from_cfg(calibration),
            "the calibration, when there is one"
        );
        let plain = runtime_ini(&default_setup(), None);
        assert!(plain.contains("lock = 540") && !plain.contains("wheel_vid"));
    }
}
