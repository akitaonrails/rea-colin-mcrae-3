//! `SilentPatchCMR3.ini`: the game's settings, which SilentPatch redirects
//! from the registry into this portable file in the game folder.
//!
//! The launcher owns the keys below and leaves every other line as the
//! game wrote it, so the player's own choices (gamma, detail levels,
//! controls) survive. Keys the launcher no longer sets are removed, so the
//! game's defaults apply again. The editing itself is the SDK's
//! `rea_launcher::ini` (byte-faithful, `KEY=value` as SilentPatch writes).

use rea_launcher::ini::Ini;

use crate::config::Settings;

/// The keys under `[Registry]` the launcher owns.
pub mod registry {
    pub const WIDTH: &str = "WIDTH";
    pub const HEIGHT: &str = "HEIGHT";
    pub const FSAA: &str = "FSAA";
    pub const VSYNC: &str = "VSYNC";
    pub const ANISOTROPIC: &str = "ANISOTROPIC";
    pub const DISPLAY_MODE: &str = "DISPLAY_MODE";
}

pub const REGISTRY: &str = "Registry";
pub const GRAPHICS: &str = "Graphics";
pub const ADVANCED: &str = "Advanced";

/// Writes the launcher's picture settings into the ini file's bytes.
/// Returns the new bytes and a log line for each key set.
pub fn apply_picture(
    bytes: &[u8],
    settings: &Settings,
    resolution: Option<(u32, u32)>,
) -> (Vec<u8>, Vec<String>) {
    let mut ini = Ini::from_bytes(bytes).tight();
    let mut log = Vec::new();
    let mut put = |section: &str, key: &str, value: Option<String>| match value {
        Some(v) => {
            ini.set(section, key, &v);
            log.push(format!("{key}={v}"));
        }
        None => {
            ini.remove(section, key);
        }
    };
    put(
        REGISTRY,
        registry::WIDTH,
        resolution.map(|(w, _)| w.to_string()),
    );
    put(
        REGISTRY,
        registry::HEIGHT,
        resolution.map(|(_, h)| h.to_string()),
    );
    put(
        REGISTRY,
        registry::DISPLAY_MODE,
        settings.display_mode.map(|m| m.ini().to_string()),
    );
    put(
        REGISTRY,
        registry::VSYNC,
        settings.vsync.map(|b| u8::from(b).to_string()),
    );
    put(
        REGISTRY,
        registry::ANISOTROPIC,
        crate::config::anisotropic_ini(settings.anisotropic).map(|v| v.to_string()),
    );
    put(
        REGISTRY,
        registry::FSAA,
        settings.fsaa.map(|b| u8::from(b).to_string()),
    );
    put(
        ADVANCED,
        "SHARPER_SHADOWS",
        settings.shadows.map(|style| style.ini().to_string()),
    );
    let fov = settings.fov.map(|f| f.to_string());
    put(GRAPHICS, "EXTERIOR_FOV", fov.clone());
    put(GRAPHICS, "INTERIOR_FOV", fov);
    (ini.to_bytes(), log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayMode, Settings, ShadowStyle};

    const GAME_WRITTEN: &str = "[Registry]\n\
LANGUAGE=E\n\
GAMMA=1\n\
WIDTH=640\n\
HEIGHT=480\n\
[Graphics]\n\
DIGITAL_TACHO=0\n";

    fn apply(text: &str, settings: &Settings, res: Option<(u32, u32)>) -> (String, Vec<String>) {
        let (bytes, log) = apply_picture(text.as_bytes(), settings, res);
        (String::from_utf8(bytes).unwrap(), log)
    }

    #[test]
    fn apply_picture_writes_the_launcher_keys() {
        let settings = Settings {
            resolution: Some((3840, 2160)),
            display_mode: Some(DisplayMode::Fullscreen),
            vsync: Some(true),
            anisotropic: Some(16),
            fsaa: Some(false),
            fov: Some(75),
            ..Settings::default()
        };
        let (t, log) = apply(GAME_WRITTEN, &settings, Some((3840, 2160)));
        assert!(t.contains("WIDTH=3840\nHEIGHT=2160\n"), "{t}");
        assert!(!t.contains("WIDTH=640"));
        assert!(t.contains("DISPLAY_MODE=0\n"));
        assert!(t.contains("VSYNC=1\n"));
        assert!(t.contains("ANISOTROPIC=4\n"));
        assert!(t.contains("FSAA=0\n"));
        assert!(t.contains("EXTERIOR_FOV=75\n"));
        assert!(t.contains("INTERIOR_FOV=75\n"));
        assert!(t.contains("GAMMA=1"), "the game's keys stay");
        assert!(t.contains("LANGUAGE=E"));
        assert!(t.contains("DIGITAL_TACHO=0"));
        assert!(log.contains(&"WIDTH=3840".to_string()));
    }

    #[test]
    fn an_empty_file_gets_its_sections() {
        let settings = Settings {
            vsync: Some(true),
            fov: Some(90),
            ..Settings::default()
        };
        let (t, _) = apply("", &settings, None);
        assert!(t.starts_with("[Registry]\nVSYNC=1\n"), "{t}");
        assert!(t.contains("[Graphics]\nEXTERIOR_FOV=90\nINTERIOR_FOV=90\n"));
        // Sections and keys match without regard to case; a last line
        // without its newline never glues a new section on.
        let (t, _) = apply("[registry]\nVSync=x", &settings, None);
        assert!(t.starts_with("[registry]\nVSYNC=1\n"), "{t}");
        assert!(t.contains("\n[Graphics]\n"));
    }

    #[test]
    fn shadows_switch_writes_the_advanced_key() {
        let sharp = Settings {
            shadows: Some(ShadowStyle::Sharp),
            ..Settings::default()
        };
        let (t, log) = apply(GAME_WRITTEN, &sharp, None);
        assert!(t.contains("[Advanced]\nSHARPER_SHADOWS=1"));
        assert!(log.iter().any(|l| l == "SHARPER_SHADOWS=1"));
        let soft = Settings {
            shadows: Some(ShadowStyle::Soft),
            ..Settings::default()
        };
        let (t, log) = apply(&t, &soft, None);
        assert!(t.contains("SHARPER_SHADOWS=0"));
        assert!(log.iter().any(|l| l == "SHARPER_SHADOWS=0"));
        // Unset: the game's choice again.
        let (t, _) = apply(&t, &Settings::default(), None);
        assert!(!t.contains("SHARPER_SHADOWS"));
    }

    #[test]
    fn apply_picture_without_settings_restores_the_defaults() {
        let with = "[Registry]\nWIDTH=3840\nHEIGHT=2160\nVSYNC=1\nANISOTROPIC=4\n[Graphics]\nEXTERIOR_FOV=90\nINTERIOR_FOV=90\n";
        let (t, _) = apply(with, &Settings::default(), None);
        assert!(!t.contains("WIDTH"));
        assert!(!t.contains("VSYNC"));
        assert!(!t.contains("ANISOTROPIC"));
        assert!(!t.contains("FOV"));
    }

    #[test]
    fn a_file_in_another_encoding_keeps_its_bytes() {
        // A Windows-1252 value the game wrote, CRLF lines.
        let bytes = b"[Registry]\r\nPLAYER=Jos\xE9\r\nWIDTH=640\r\n";
        let settings = Settings::default();
        let (out, _) = apply_picture(bytes, &settings, Some((1920, 1080)));
        assert_eq!(
            out,
            b"[Registry]\r\nPLAYER=Jos\xE9\r\nWIDTH=1920\r\nHEIGHT=1080\r\n"
        );
    }
}
