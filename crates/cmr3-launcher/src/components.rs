//! The bundled components and how they go into the runtime folder.
//!
//! Everything the game needs travels with the launcher in a `thirdparty/`
//! folder next to its executable (fetched and checksummed when the package
//! is built; see `packaging/third-party.txt`):
//!
//! ```text
//! thirdparty/silentpatch/SilentPatchCMR3.asi   the fixes (MIT)
//! thirdparty/silentpatch/dinput8.dll           the ASI loader; installed as dsound.dll
//! thirdparty/xidi/dinput8.dll                  the pad bridge (BSD-3-Clause)
//! thirdparty/patcher/xdelta.exe + *.xdelta     the DRM-free patcher (Apache-2.0)
//! thirdparty/dxvk/d3d9.dll                     DXVK for Wine (Zlib)
//! thirdparty/hd-ui/…                           the HD UI overlay (Bekoha)
//! ```
//!
//! Components are installed by whole-file swaps (`runtime::install*`), so a
//! hard-linked original is never written through, and reinstalls are
//! idempotent. Missing components disable their feature with a log line,
//! never a failure.

use std::fs;
use std::io;
use std::path::Path;

use crate::config::{Gamepad, Settings};
use crate::edition;
use crate::runtime;
use crate::xidi;

/// The folder of bundled components next to the launcher (or
/// `CMR3_THIRDPARTY`).
pub fn bundle_dir() -> std::path::PathBuf {
    if let Some(set) = std::env::var_os("CMR3_THIRDPARTY") {
        return std::path::PathBuf::from(set);
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("thirdparty")))
        .unwrap_or_else(|| std::path::PathBuf::from("thirdparty"))
}

/// Which bundled parts are present, for `--check` and the log.
pub fn present(bundle: &Path) -> Vec<(&'static str, bool)> {
    let file = |rel: &[&str]| {
        bundle
            .join(rel.iter().collect::<std::path::PathBuf>())
            .is_file()
    };
    vec![
        ("SilentPatch", file(&["silentpatch", "SilentPatchCMR3.asi"])),
        ("ASI loader", file(&["silentpatch", "dinput8.dll"])),
        ("Xidi", file(&["xidi", "dinput8.dll"])),
        (
            "DRM-free patcher",
            file(&["patcher", "xdelta.exe"]) && file(&["patcher", "efigs.xdelta"]),
        ),
        ("DXVK (Wine)", file(&["dxvk", "d3d9.dll"])),
        ("HD UI", bundle.join("hd-ui").is_dir()),
    ]
}

/// Installs SilentPatch and its ASI loader (as `dsound.dll`, leaving the
/// `dinput8.dll` name free for Xidi). Returns false when not bundled.
pub fn install_silentpatch(runtime: &Path, bundle: &Path) -> io::Result<bool> {
    let asi = bundle.join("silentpatch/SilentPatchCMR3.asi");
    let loader = bundle.join("silentpatch/dinput8.dll");
    if !asi.is_file() || !loader.is_file() {
        return Ok(false);
    }
    runtime::install_copy(&asi, &runtime.join("SilentPatchCMR3.asi"))?;
    // The loader hooks the first of dsound/dinput8/winmm it is named as;
    // dsound is free here and the game imports it.
    runtime::install_copy(&loader, &runtime.join("dsound.dll"))?;
    Ok(true)
}

/// Installs Xidi as `dinput8.dll` with the preset's `Xidi.ini` (its
/// logger on when `log`); the `off` preset removes both (the game then
/// loads the system DirectInput).
pub fn install_xidi(runtime: &Path, bundle: &Path, preset: Gamepad, log: bool) -> io::Result<bool> {
    let dll = bundle.join("xidi/dinput8.dll");
    match xidi::ini(preset, log) {
        Some(ini) => {
            if !dll.is_file() {
                return Ok(false);
            }
            runtime::install_copy(&dll, &runtime.join("dinput8.dll"))?;
            runtime::install(&runtime.join("Xidi.ini"), ini.as_bytes())?;
            Ok(true)
        }
        None => {
            for name in ["dinput8.dll", "Xidi.ini"] {
                runtime::remove(&runtime.join(name))?;
            }
            Ok(true)
        }
    }
}

/// DXVK's d3d9.dll next to the game under Wine (Vulkan) — or removed. On
/// Windows this is never installed: the system's own Direct3D is the right
/// one there.
pub fn install_dxvk(runtime: &Path, bundle: &Path, on: bool) -> io::Result<bool> {
    let dll = runtime.join("d3d9.dll");
    if !on {
        runtime::remove(&dll)?;
        return Ok(true);
    }
    let from = bundle.join("dxvk/d3d9.dll");
    if !from.is_file() {
        return Ok(false);
    }
    runtime::install_copy(&from, &dll)?;
    Ok(true)
}

/// Overlays the HD UI pack (Data/, fonts/) into the runtime folder.
/// Returns how many files went in; 0 when not bundled.
pub fn install_hd_ui(runtime: &Path, bundle: &Path) -> io::Result<usize> {
    let from = bundle.join("hd-ui");
    if !from.is_dir() {
        return Ok(0);
    }
    let mut n = 0;
    for rel in runtime::list(&from)?.keys() {
        let src = runtime::native(&from, rel);
        let dst = runtime::native(runtime, rel);
        runtime::install_copy(&src, &dst)?;
        n += 1;
    }
    Ok(n)
}

/// The xdelta command that turns the DRM `from` exe into the DRM-free exe
/// of `variant` ("efigs" or "polish"), written to `to`. The variants are
/// tried in order; xdelta fails on a mismatched source, which is how the
/// patcher tells the game versions apart.
pub fn xdelta_args(
    xdelta: &Path,
    from: &Path,
    variant: &str,
    to: &Path,
) -> Vec<std::ffi::OsString> {
    let patch = xdelta
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{variant}.xdelta"));
    vec![
        xdelta.as_os_str().into(),
        "-f".into(),
        "-d".into(),
        "-s".into(),
        from.as_os_str().into(),
        patch.into(),
        to.as_os_str().into(),
    ]
}

/// Checks whether the runtime exe is already DRM-free, so the patcher runs
/// once, not every launch.
pub fn runtime_needs_patch(runtime_exe: &Path) -> bool {
    fs::read(runtime_exe)
        .map(|bytes| !edition::is_drm_free(&bytes))
        .unwrap_or(true)
}

/// Writes the launcher's picture settings into the runtime folder's
/// SilentPatchCMR3.ini (creating it when the game has none yet). Only the
/// launcher's own keys are touched; everything the game saved (gamma,
/// detail, its video quality, adapters) stays as the player left it.
pub fn apply_settings(
    runtime: &Path,
    settings: &Settings,
    resolution: Option<(u32, u32)>,
) -> io::Result<Vec<String>> {
    let ini = runtime.join("SilentPatchCMR3.ini");
    let text = fs::read_to_string(&ini).unwrap_or_default();
    let (text, log) = crate::sp::apply_picture(&text, settings, resolution);
    runtime::install(&ini, text.as_bytes())?;
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cmr3-parts-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn bundle_with(name: &str) -> PathBuf {
        let b = temp(name);
        for dir in ["silentpatch", "xidi", "patcher", "dxvk"] {
            fs::create_dir_all(b.join(dir)).unwrap();
        }
        fs::write(b.join("silentpatch/SilentPatchCMR3.asi"), b"asi").unwrap();
        fs::write(b.join("silentpatch/dinput8.dll"), b"loader").unwrap();
        fs::write(b.join("xidi/dinput8.dll"), b"xidi").unwrap();
        fs::write(b.join("patcher/xdelta.exe"), b"xd3").unwrap();
        fs::write(b.join("patcher/efigs.xdelta"), b"e").unwrap();
        fs::write(b.join("patcher/polish.xdelta"), b"p").unwrap();
        fs::write(b.join("dxvk/d3d9.dll"), b"dxvk9").unwrap();
        b
    }

    #[test]
    fn installs_overwrite_and_the_bundle_survives() {
        let bundle = bundle_with("bundle");
        let rt = temp("rt-install");
        // A pre-existing (hard-linked-style) file is replaced, not written
        // through: removing the bundle copy later cannot matter.
        fs::write(rt.join("SilentPatchCMR3.asi"), b"old").unwrap();
        assert!(install_silentpatch(&rt, &bundle).unwrap());
        assert_eq!(fs::read(rt.join("SilentPatchCMR3.asi")).unwrap(), b"asi");
        assert_eq!(fs::read(rt.join("dsound.dll")).unwrap(), b"loader");
        assert!(!rt.join("dinput8.dll").is_file());

        assert!(install_xidi(&rt, &bundle, Gamepad::Rally, false).unwrap());
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"xidi");
        let ini = fs::read_to_string(rt.join("Xidi.ini")).unwrap();
        assert!(ini.contains("CMR3Gamepad"));

        assert!(install_dxvk(&rt, &bundle, true).unwrap());
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"dxvk9");

        // Off takes Xidi back out.
        assert!(install_xidi(&rt, &bundle, Gamepad::Off, false).unwrap());
        assert!(!rt.join("dinput8.dll").is_file());
        assert!(!rt.join("Xidi.ini").is_file());
        assert!(install_dxvk(&rt, &bundle, false).unwrap());
        assert!(!rt.join("d3d9.dll").is_file());

        for d in [bundle, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn missing_parts_disable_their_feature() {
        let bundle = temp("empty-bundle");
        let rt = temp("rt-missing");
        assert!(!install_silentpatch(&rt, &bundle).unwrap());
        assert!(!install_xidi(&rt, &bundle, Gamepad::Rally, false).unwrap());
        assert!(!install_dxvk(&rt, &bundle, true).unwrap());
        assert_eq!(install_hd_ui(&rt, &bundle).unwrap(), 0);
        fs::remove_dir_all(&bundle).unwrap();
        fs::remove_dir_all(&rt).unwrap();
    }

    #[test]
    fn hd_ui_overlays_the_trees() {
        let bundle = temp("hd-bundle");
        let rt = temp("rt-hd");
        fs::create_dir_all(bundle.join("hd-ui/Data/Boot")).unwrap();
        fs::create_dir_all(rt.join("Data/Boot")).unwrap();
        fs::write(rt.join("Data/Boot/colin3.dds"), b"low").unwrap();
        fs::write(bundle.join("hd-ui/Data/Boot/colin3.dds"), b"high").unwrap();
        fs::create_dir_all(bundle.join("hd-ui/fonts")).unwrap();
        fs::write(bundle.join("hd-ui/fonts/hel_18pt.dds"), b"hd").unwrap();
        assert_eq!(install_hd_ui(&rt, &bundle).unwrap(), 2);
        assert_eq!(fs::read(rt.join("Data/Boot/colin3.dds")).unwrap(), b"high");
        assert!(rt.join("fonts/hel_18pt.dds").is_file());
        fs::remove_dir_all(&bundle).unwrap();
        fs::remove_dir_all(&rt).unwrap();
    }

    #[test]
    fn xdelta_command_matches_the_patcher() {
        let bundle = bundle_with("xd-bundle");
        let xdelta = bundle.join("patcher/xdelta.exe");
        let args = xdelta_args(
            &xdelta,
            Path::new("C:\\rt\\Rally_3PC.exe"),
            "efigs",
            Path::new("C:\\rt\\Rally_3PC.exe.new"),
        );
        let joined: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            joined,
            vec![
                xdelta.display().to_string(),
                "-f".into(),
                "-d".into(),
                "-s".into(),
                "C:\\rt\\Rally_3PC.exe".into(),
                bundle.join("patcher/efigs.xdelta").display().to_string(),
                "C:\\rt\\Rally_3PC.exe.new".into(),
            ]
        );
        assert!(bundle.join("patcher/polish.xdelta").is_file());
        fs::remove_dir_all(&bundle).unwrap();
    }

    #[test]
    fn settings_go_into_the_ini() {
        let rt = temp("rt-ini");
        let settings = crate::config::Settings {
            vsync: Some(true),
            anisotropic: Some(16),
            ..Settings::default()
        };
        let log = apply_settings(&rt, &settings, Some((1920, 1080))).unwrap();
        let ini = fs::read_to_string(rt.join("SilentPatchCMR3.ini")).unwrap();
        assert!(ini.contains("WIDTH=1920\n"));
        assert!(ini.contains("VSYNC=1\n"));
        assert!(ini.contains("ANISOTROPIC=4\n"));
        assert!(log.iter().any(|l| l == "WIDTH=1920"));
        // The game's own keys, written later by the game, survive.
        fs::write(
            rt.join("SilentPatchCMR3.ini"),
            ini.replace("[Registry]", "[Registry]\nGAMMA=2"),
        )
        .unwrap();
        apply_settings(&rt, &settings, Some((1920, 1080))).unwrap();
        let ini = fs::read_to_string(rt.join("SilentPatchCMR3.ini")).unwrap();
        assert!(ini.contains("GAMMA=2"));
        fs::remove_dir_all(&rt).unwrap();
    }
}
