//! The SDK's in-game DirectInput wheel layer, `rea_dinput.dll`
//! (`rea-launcher-dinput`): shipped in the package's `components/` and put
//! beside the runtime copy's `Rally_3PC.exe` as [`ASI`], with the layer's
//! `rea_wheel.ini` next to it and the launcher's decision in
//! `REA_WHEEL_MODE`. It wraps the `DirectInput8Create` of whatever
//! `dinput8.dll` the game loads (Xidi's), so pads keep Xidi.
//!
//! SilentPatch's ASI loader (the bundled `dsound.dll`) loads it at start,
//! long before the game creates DirectInput (after the title screen).
//! The SDK's other way in, adding the DLL to the executable's imports
//! (`rea_dinput::import_into`), is not used: SilentPatch checks the
//! executable and refuses a changed one ("An unsupported game version was
//! detected!"), and the game then crashes without its fixes
//! (docs/RUNTIME_NOTES.md, 2026-11-13). The executable stays the
//! publisher's DRM-free build.

pub use rea_dinput::{DLL, MODE_ENV};

/// The layer's file name in the runtime folder: an ASI plugin.
pub const ASI: &str = "rea_dinput.asi";

/// The value of [`MODE_ENV`] for the launcher's decision.
pub fn mode_value(shown: bool) -> &'static str {
    if shown { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_and_the_mode() {
        assert_eq!((mode_value(true), mode_value(false)), ("on", "off"));
        assert_eq!(MODE_ENV, "REA_WHEEL_MODE");
        assert_eq!(DLL, "rea_dinput.dll");
        assert!(ASI.ends_with(".asi") && ASI.starts_with("rea_dinput"));
    }
}
