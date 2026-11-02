//! `Xidi.ini`: the DirectInput-to-XInput bridge the launcher installs as
//! `dinput8.dll` (Xidi by samuelgr, BSD-3-Clause).
//!
//! Colin McRae Rally 3 reads its pad through DirectInput with the
//! accelerator and brake bound to the left stick's Y axis. The `rally`
//! mapping puts the triggers on that pedal axis (both at once: left-foot
//! braking) and ignores the physical left-stick Y so resting thumbs can't
//! open the throttle. The game's menus answer only the keyboard, so the
//! d-pad is the arrow keys (no hat: the Controls menu would read both and
//! move twice) and A presses Enter as well as its button. The game stores
//! pad bindings by the controller's layout; `profile` writes the ones
//! validated on this layout. Xidi also lets pads be plugged in and
//! swapped while the game runs.

use crate::config::Gamepad;

/// The Xidi.ini text for a preset, or `None` when Xidi is off. `log`
/// turns Xidi's own logger on (`gamepad_log`): it writes what the game
/// asks of DirectInput and what every pad control sent — the file to read
/// when an axis or a button doesn't arrive.
pub fn ini(preset: Gamepad, log: bool) -> Option<String> {
    let mapper = match preset {
        Gamepad::Off => return None,
        // The plain wrapper: every axis and button passes through as the
        // pad reports it.
        Gamepad::Standard => "[Mapper]\nType = StandardGamepad\n",
        Gamepad::Rally => {
            "[Mapper]\n\
Type = CMR3Gamepad\n\
\n\
[CustomMapper:CMR3Gamepad]\n\
Template = StandardGamepad\n\
StickLeftX = Axis(X)\n\
StickLeftY = Null\n\
TriggerRT = Axis(Y, -)\n\
TriggerLT = Axis(Y, +)\n\
DpadUp = Keyboard(UpArrow)\n\
DpadDown = Keyboard(DownArrow)\n\
DpadLeft = Keyboard(LeftArrow)\n\
DpadRight = Keyboard(RightArrow)\n\
ButtonA = Compound(Button(4), Keyboard(Enter))\n"
        }
    };
    Some(format!(
        "{mapper}\n\
[Properties]\n\
UseBuiltInProperties = yes\n\
DeadzonePercentStickLeft = 5\n\
DeadzonePercentStickRight = 5\n\
DeadzonePercentTriggerLT = 0\n\
DeadzonePercentTriggerRT = 0\n\
SaturationPercentStickLeft = 100\n\
SaturationPercentStickRight = 100\n\
SaturationPercentTriggerLT = 100\n\
SaturationPercentTriggerRT = 100\n\
\n\
[Log]\n\
Enabled = {log_enabled}\n\
Level = 3\n\
\n\
[Import]\n\
dinput8.dll = C:\\Windows\\system32\\dinput8.dll\n\
\n\
[Workarounds]\n\
ActiveVirtualControllerMask = 0x01\n",
        log_enabled = if log { "yes" } else { "no" },
    ))
}

/// What each preset does, for the settings window.
pub fn note(preset: Gamepad) -> &'static str {
    match preset {
        Gamepad::Rally => {
            "Left stick steers, the triggers are the pedals (both at once: \
             left-foot braking) and a resting thumb can't open the throttle. \
             The d-pad and A also work the menus, as the arrow keys and \
             Enter."
        }
        Gamepad::Standard => {
            "Xidi passes the pad through as it reports itself; bind steering \
             and the pedals in the game's Options > Controls."
        }
        Gamepad::Off => {
            "No Xidi: the game reads the pad through DirectInput, where the \
             triggers may share one axis and new profiles start on the keyboard."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rally_keeps_the_validated_mapping() {
        let t = ini(Gamepad::Rally, false).unwrap();
        assert!(t.contains("Type = CMR3Gamepad"));
        assert!(t.contains("StickLeftY = Null"));
        assert!(t.contains("TriggerRT = Axis(Y, -)"));
        assert!(t.contains("TriggerLT = Axis(Y, +)"));
        // The layout `profile`'s bindings were validated on: the d-pad is
        // keys only, A keeps its button and adds Enter, nothing else is a key.
        let keys: Vec<&str> = t.lines().filter(|l| l.contains("Keyboard(")).collect();
        assert_eq!(
            keys,
            [
                "DpadUp = Keyboard(UpArrow)",
                "DpadDown = Keyboard(DownArrow)",
                "DpadLeft = Keyboard(LeftArrow)",
                "DpadRight = Keyboard(RightArrow)",
                "ButtonA = Compound(Button(4), Keyboard(Enter))",
            ]
        );
        assert!(t.contains("DeadzonePercentStickLeft = 5"));
        assert!(t.contains("dinput8.dll = C:\\Windows\\system32\\dinput8.dll"));
        assert!(t.contains("Enabled = no"));
        assert!(ini(Gamepad::Rally, true).unwrap().contains("Enabled = yes"));
    }

    #[test]
    fn standard_has_no_custom_mapper() {
        let t = ini(Gamepad::Standard, false).unwrap();
        assert!(t.contains("Type = StandardGamepad"));
        assert!(!t.contains("CustomMapper"));
        assert!(!t.contains("Keyboard"));
    }

    #[test]
    fn off_installs_nothing() {
        assert!(ini(Gamepad::Off, false).is_none());
    }

    #[test]
    fn every_preset_has_a_note() {
        for preset in Gamepad::ALL {
            assert!(!note(preset).is_empty());
        }
    }
}

#[cfg(test)]
mod golden {
    use super::*;

    /// The rally mapping, byte for byte as validated on real hardware
    /// (the distrobox-gaming setup, carried into this launcher). If this
    /// test fails, the mapping changed — play a stage with a pad before
    /// shipping the change, and update the fixture deliberately.
    #[test]
    fn rally_matches_the_validated_mapping_byte_for_byte() {
        let expected = include_str!("../tests/fixtures/xidi-rally.ini");
        assert_eq!(ini(Gamepad::Rally, false).unwrap(), expected);
    }
}
