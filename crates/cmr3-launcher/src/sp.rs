//! `SilentPatchCMR3.ini`: the game's settings, which SilentPatch redirects
//! from the registry into this portable file in the game folder.
//!
//! The launcher owns the keys below and leaves every other line as the
//! game wrote it, so the player's own choices (gamma, detail levels,
//! controls) survive. Keys the launcher no longer sets are removed, so the
//! game's defaults apply again.

use crate::config::Settings;

/// Sets `key = value` under `[section]`, keeping every other line as it
/// was. The section is created (at the end) when missing.
pub fn set(text: &str, section: &str, key: &str, value: &str) -> String {
    match section_range(text, section) {
        Some((start, end)) => insert_in_section(text, key, value, start, end),
        None => format!("{text}[{section}]\n{key}={value}\n"),
    }
}

/// Removes `key` from `[section]`; other lines are kept.
pub fn remove(text: &str, section: &str, key: &str) -> String {
    let Some((start, end)) = section_range(text, section) else {
        return text.to_string();
    };
    let mut out = text[..start].to_string();
    for line in text[start..end].lines() {
        let t = line.trim();
        let is_key = t
            .split_once('=')
            .is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
        if !is_key {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str(&text[end..]);
    out
}

/// Where `[section]`'s body starts and the whole file ends: the byte
/// offsets of the first line after the header and of the next header (or
/// the end of the file). Sections match without regard to case or spaces.
fn section_range(text: &str, section: &str) -> Option<(usize, usize)> {
    let same = |header: &str| {
        let inner = header.trim_start_matches('[').trim_end_matches(']');
        inner.trim().eq_ignore_ascii_case(section)
    };
    // Lines that keep their line endings, so offsets add up.
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut start = None;
    let mut offset = 0usize;
    for (n, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if start.is_some() {
                return start.map(|s| (s, offset));
            }
            if same(t) {
                start = Some(offset + line.len());
            }
        }
        let _ = n;
        offset += line.len();
    }
    start.map(|s| (s, text.len()))
}

fn insert_in_section(text: &str, key: &str, value: &str, start: usize, end: usize) -> String {
    let mut out = text[..start].to_string();
    let mut done = false;
    for line in text[start..end].lines() {
        let t = line.trim();
        let is_key = t
            .split_once('=')
            .is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
        if is_key && !done {
            out.push_str(&format!("{key}={value}\n"));
            done = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !done {
        out.push_str(&format!("{key}={value}\n"));
    }
    out.push_str(&text[end..]);
    out
}

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

/// Writes the launcher's picture settings into the ini text. Returns the
/// new text and a log line for each key.
pub fn apply_picture(
    text: &str,
    settings: &Settings,
    resolution: Option<(u32, u32)>,
) -> (String, Vec<String>) {
    let mut t = text.to_string();
    let mut log = Vec::new();
    let put = |t: &mut String, key: &str, value: Option<String>, log: &mut Vec<String>| match value
    {
        Some(v) => {
            *t = set(t, REGISTRY, key, &v);
            log.push(format!("{key}={v}"));
        }
        None => {
            *t = remove(t, REGISTRY, key);
        }
    };
    match resolution {
        Some((w, h)) => {
            put(&mut t, registry::WIDTH, Some(w.to_string()), &mut log);
            put(&mut t, registry::HEIGHT, Some(h.to_string()), &mut log);
        }
        None => {
            put(&mut t, registry::WIDTH, None, &mut log);
            put(&mut t, registry::HEIGHT, None, &mut log);
        }
    }
    put(
        &mut t,
        registry::DISPLAY_MODE,
        settings.display_mode.map(|m| m.ini().to_string()),
        &mut log,
    );
    put(
        &mut t,
        registry::VSYNC,
        settings.vsync.map(|b| u8::from(b).to_string()),
        &mut log,
    );
    put(
        &mut t,
        registry::ANISOTROPIC,
        crate::config::anisotropic_ini(settings.anisotropic).map(|v| v.to_string()),
        &mut log,
    );
    put(
        &mut t,
        registry::FSAA,
        settings.fsaa.map(|b| u8::from(b).to_string()),
        &mut log,
    );
    if let Some(fov) = settings.fov {
        t = set(&t, GRAPHICS, "EXTERIOR_FOV", &fov.to_string());
        t = set(&t, GRAPHICS, "INTERIOR_FOV", &fov.to_string());
        log.push(format!("EXTERIOR_FOV={fov}"));
        log.push(format!("INTERIOR_FOV={fov}"));
    } else {
        t = remove(&t, GRAPHICS, "EXTERIOR_FOV");
        t = remove(&t, GRAPHICS, "INTERIOR_FOV");
    }
    (t, log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayMode, Settings};

    const GAME_WRITTEN: &str = "[Registry]\n\
LANGUAGE=E\n\
GAMMA=1\n\
WIDTH=640\n\
HEIGHT=480\n\
[Graphics]\n\
DIGITAL_TACHO=0\n";

    #[test]
    fn set_replaces_only_its_key() {
        let t = set(GAME_WRITTEN, REGISTRY, "WIDTH", "3840");
        assert!(t.contains("WIDTH=3840\n"));
        assert!(!t.contains("WIDTH=640"));
        assert!(t.contains("GAMMA=1"));
        assert!(t.contains("LANGUAGE=E"));
        assert!(t.contains("DIGITAL_TACHO=0"));
    }

    #[test]
    fn set_creates_missing_sections_and_keys() {
        let t = set("", REGISTRY, "VSYNC", "1");
        assert_eq!(t, "[Registry]\nVSYNC=1\n");
        let t = set(&t, GRAPHICS, "EXTERIOR_FOV", "75");
        assert!(t.contains("[Graphics]\nEXTERIOR_FOV=75\n"));
    }

    #[test]
    fn set_matches_sections_without_regard_to_case() {
        let t = set("[registry]\nVSync=x\n", REGISTRY, "VSYNC", "1");
        assert!(t.contains("VSYNC=1\n"));
    }

    #[test]
    fn remove_takes_the_key_out() {
        let t = remove(GAME_WRITTEN, REGISTRY, "WIDTH");
        assert!(!t.contains("WIDTH"));
        assert!(t.contains("HEIGHT=480"));
        let t = remove(&t, REGISTRY, "nothing");
        assert_eq!(t, t);
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
        let (t, log) = apply_picture(GAME_WRITTEN, &settings, Some((3840, 2160)));
        assert!(t.contains("WIDTH=3840\nHEIGHT=2160\n"));
        assert!(t.contains("DISPLAY_MODE=0\n"));
        assert!(t.contains("VSYNC=1\n"));
        assert!(t.contains("ANISOTROPIC=4\n"));
        assert!(t.contains("FSAA=0\n"));
        assert!(t.contains("EXTERIOR_FOV=75\n"));
        assert!(t.contains("GAMMA=1"), "the game's keys stay");
        assert!(log.contains(&"WIDTH=3840".to_string()));
    }

    #[test]
    fn apply_picture_without_settings_restores_the_defaults() {
        let with = "[Registry]\nWIDTH=3840\nHEIGHT=2160\nVSYNC=1\nANISOTROPIC=4\n[Graphics]\nEXTERIOR_FOV=90\nINTERIOR_FOV=90\n";
        let (t, _) = apply_picture(with, &Settings::default(), None);
        assert!(!t.contains("WIDTH"));
        assert!(!t.contains("VSYNC"));
        assert!(!t.contains("ANISOTROPIC"));
        assert!(!t.contains("FOV"));
    }
}
