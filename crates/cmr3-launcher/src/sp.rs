//! `SilentPatchCMR3.ini`: the game's settings, which SilentPatch redirects
//! from the registry into this portable file in the game folder.
//!
//! The launcher writes the keys below only while their option is set and
//! leaves every other line as the game wrote it, so the player's own
//! choices (gamma, detail levels, controls) survive. What it wrote is
//! recorded ([`OWNED`], the SDK's `rea_launcher::ini::Owned`): when an
//! option goes back to unset, the key returns to its earlier value (or
//! goes) only while it still holds the launcher's value — a choice the
//! player made since in the game's own menu stays. The editing itself is
//! the SDK's `rea_launcher::ini` (byte-faithful, `KEY=value` as
//! SilentPatch writes).

use rea_launcher::ini::{self, Ini, Owned};

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

/// The record of the values the launcher wrote, in the runtime folder.
pub const OWNED: &str = ".cmr3-ini-owned";

/// Writes the launcher's picture settings into the ini file's bytes,
/// recording them in `owned`. Returns the new bytes and a log line for
/// each key set or given back.
pub fn apply_picture(
    bytes: &[u8],
    settings: &Settings,
    resolution: Option<(u32, u32)>,
    owned: &mut Owned,
) -> (Vec<u8>, Vec<String>) {
    let mut ini = Ini::from_bytes(bytes).tight();
    let mut log = Vec::new();
    let mut put = |section: &str, key: &str, value: Option<String>| match value {
        Some(v) => {
            ini::set_owned(&mut ini, owned, section, key, &v);
            log.push(format!("{key}={v}"));
        }
        None => match ini::release(&mut ini, owned, section, key) {
            ini::Released::Restored => log.push(format!("{key} back to the game's")),
            ini::Released::PlayerChanged => log.push(format!("{key} kept (changed in the game)")),
            ini::Released::NotOurs => {}
        },
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
        apply_with(text, settings, res, &mut Owned::default())
    }

    fn apply_with(
        text: &str,
        settings: &Settings,
        res: Option<(u32, u32)>,
        owned: &mut Owned,
    ) -> (String, Vec<String>) {
        let (bytes, log) = apply_picture(text.as_bytes(), settings, res, owned);
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
        let mut owned = Owned::default();
        let (t, log) = apply_with(GAME_WRITTEN, &sharp, None, &mut owned);
        assert!(t.contains("[Advanced]\nSHARPER_SHADOWS=1"));
        assert!(log.iter().any(|l| l == "SHARPER_SHADOWS=1"));
        let soft = Settings {
            shadows: Some(ShadowStyle::Soft),
            ..Settings::default()
        };
        let (t, log) = apply_with(&t, &soft, None, &mut owned);
        assert!(t.contains("SHARPER_SHADOWS=0"));
        assert!(log.iter().any(|l| l == "SHARPER_SHADOWS=0"));
        // Unset: the key the launcher added goes again.
        let (t, _) = apply_with(&t, &Settings::default(), None, &mut owned);
        assert!(!t.contains("SHARPER_SHADOWS"));
    }

    #[test]
    fn unset_gives_back_only_what_the_launcher_wrote() {
        // The game's own file: the player chose these in its menu.
        let game = "[Registry]\nWIDTH=1024\nHEIGHT=768\nVSYNC=0\nFSAA=1\nDISPLAY_MODE=1\n[Graphics]\nEXTERIOR_FOV=80\n";
        let set = Settings {
            vsync: Some(true),
            fsaa: Some(false),
            fov: Some(90),
            display_mode: Some(DisplayMode::Fullscreen),
            ..Settings::default()
        };
        let mut owned = Owned::default();
        let (t, _) = apply_with(game, &set, Some((1920, 1080)), &mut owned);
        assert!(t.contains("VSYNC=1") && t.contains("FSAA=0") && t.contains("WIDTH=1920"));
        // The player then changes vsync in the game's own menu.
        let t = t.replace("VSYNC=1", "VSYNC=0");
        // Every option back to unset: the launcher's values go back to the
        // game's earlier ones; the player's new vsync stays.
        let (t, log) = apply_with(&t, &Settings::default(), None, &mut owned);
        assert_eq!(t, game, "{log:?}");
        assert!(log.contains(&"VSYNC kept (changed in the game)".to_string()));
        assert!(log.contains(&"FSAA back to the game's".to_string()));
        // A key the launcher never wrote is never touched.
        let (again, log) = apply_with(&t, &Settings::default(), None, &mut owned);
        assert_eq!(again, game);
        assert!(log.is_empty(), "{log:?}");
    }

    #[test]
    fn values_from_before_the_record_are_left_alone() {
        // A runtime from release 0.1 holds the old launcher's values but
        // no record: unset keeps them rather than guessing.
        let with = "[Registry]\nWIDTH=3840\nHEIGHT=2160\nVSYNC=1\n[Graphics]\nEXTERIOR_FOV=90\n";
        let (t, _) = apply(with, &Settings::default(), None);
        assert_eq!(t, with);
    }

    #[test]
    fn a_file_in_another_encoding_keeps_its_bytes() {
        // A Windows-1252 value the game wrote, CRLF lines.
        let bytes = b"[Registry]\r\nPLAYER=Jos\xE9\r\nWIDTH=640\r\n";
        let settings = Settings::default();
        let (out, _) = apply_picture(bytes, &settings, Some((1920, 1080)), &mut Owned::default());
        assert_eq!(
            out,
            b"[Registry]\r\nPLAYER=Jos\xE9\r\nWIDTH=1920\r\nHEIGHT=1080\r\n"
        );
    }
}
