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

/// The runtime's ledger of the files the launcher installed, by name,
/// with the SHA-256 of what it wrote: only a file that still matches is
/// the launcher's to take out.
const OWNERSHIP: &str = ".cmr3-launcher-components";
type Ownership = std::collections::BTreeMap<String, String>;

fn digest(bytes: &[u8]) -> String {
    edition::to_hex(&edition::sha256(bytes))
}

fn read_ownership(runtime: &Path) -> Ownership {
    fs::read_to_string(runtime.join(OWNERSHIP))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (digest, name) = line.split_once('\t')?;
            Some((name.to_owned(), digest.to_owned()))
        })
        .collect()
}

fn write_ownership(runtime: &Path, ownership: &Ownership) -> io::Result<()> {
    let text: String = ownership
        .iter()
        .map(|(name, digest)| format!("{digest}\t{name}\n"))
        .collect();
    runtime::install(&runtime.join(OWNERSHIP), text.as_bytes())
}

/// Installs `bytes` as the runtime's `name` and records it as ours.
fn put_owned(
    runtime: &Path,
    name: &str,
    bytes: &[u8],
    ownership: &mut Ownership,
) -> io::Result<()> {
    runtime::install(&runtime.join(name), bytes)?;
    ownership.insert(name.to_owned(), digest(bytes));
    write_ownership(runtime, ownership)
}

/// Takes the runtime's `name` out when the launcher put it there (per the
/// ledger, or byte-equal to one of `ours`, for runtimes from before the
/// ledger): the game's own file comes back, or none. Anything else (the
/// game's own file, a player's mod) stays.
fn take_out(
    game: &Path,
    runtime: &Path,
    name: &str,
    ours: &[Vec<u8>],
    ownership: &mut Ownership,
) -> io::Result<()> {
    if let Ok(bytes) = fs::read(runtime.join(name))
        && (ownership.get(name) == Some(&digest(&bytes)) || ours.contains(&bytes))
    {
        runtime::restore(game, runtime, name)?;
    }
    if ownership.remove(name).is_some() {
        write_ownership(runtime, ownership)?;
    }
    Ok(())
}

/// Installs SilentPatch and its ASI loader (as `dsound.dll`, leaving the
/// `dinput8.dll` name free for Xidi). Returns false when not bundled.
pub fn install_silentpatch(runtime: &Path, bundle: &Path) -> io::Result<bool> {
    let asi = bundle.join("silentpatch/SilentPatchCMR3.asi");
    let loader = bundle.join("silentpatch/dinput8.dll");
    if !asi.is_file() || !loader.is_file() {
        return Ok(false);
    }
    let mut ownership = read_ownership(runtime);
    put_owned(
        runtime,
        "SilentPatchCMR3.asi",
        &fs::read(&asi)?,
        &mut ownership,
    )?;
    // The loader hooks the first of dsound/dinput8/winmm it is named as;
    // dsound is free here and the game imports it.
    put_owned(runtime, "dsound.dll", &fs::read(&loader)?, &mut ownership)?;
    Ok(true)
}

/// Installs Xidi as `dinput8.dll` with the preset's `Xidi.ini` (its
/// logger on when `log`); the `off` preset takes out what the launcher
/// put in, bringing back a `dinput8.dll` of the game's own (the game then
/// loads it, or the system DirectInput).
pub fn install_xidi(
    game: &Path,
    runtime: &Path,
    bundle: &Path,
    preset: Gamepad,
    log: bool,
) -> io::Result<bool> {
    let dll = bundle.join("xidi/dinput8.dll");
    let mut ownership = read_ownership(runtime);
    match xidi::ini(preset, log) {
        Some(ini) => {
            if !dll.is_file() {
                return Ok(false);
            }
            put_owned(runtime, "dinput8.dll", &fs::read(&dll)?, &mut ownership)?;
            put_owned(runtime, "Xidi.ini", ini.as_bytes(), &mut ownership)?;
            Ok(true)
        }
        None => {
            let ours: Vec<Vec<u8>> = fs::read(&dll).into_iter().collect();
            take_out(game, runtime, "dinput8.dll", &ours, &mut ownership)?;
            let inis: Vec<Vec<u8>> = Gamepad::ALL
                .into_iter()
                .flat_map(|p| [xidi::ini(p, false), xidi::ini(p, true)])
                .flatten()
                .map(String::into_bytes)
                .collect();
            take_out(game, runtime, "Xidi.ini", &inis, &mut ownership)?;
            Ok(true)
        }
    }
}

/// DXVK's d3d9.dll next to the game under Wine (Vulkan) — or taken out
/// again (a d3d9.dll of the game's own comes back). On Windows this is
/// never installed: the system's own Direct3D is the right one there.
pub fn install_dxvk(game: &Path, runtime: &Path, bundle: &Path, on: bool) -> io::Result<bool> {
    let from = bundle.join("dxvk/d3d9.dll");
    let mut ownership = read_ownership(runtime);
    if !on {
        let ours: Vec<Vec<u8>> = fs::read(&from).into_iter().collect();
        take_out(game, runtime, "d3d9.dll", &ours, &mut ownership)?;
        return Ok(true);
    }
    if !from.is_file() {
        return Ok(false);
    }
    put_owned(runtime, "d3d9.dll", &fs::read(&from)?, &mut ownership)?;
    Ok(true)
}

/// `WINEDLLOVERRIDES` for the game under Wine: `natives` (the DLL names
/// the launcher installed beside it: dsound for the ASI loader, dinput8
/// for Xidi, d3d9 for DXVK) load from the game folder first. The player's
/// own rules (`existing`) are kept, except for those names, so ours apply.
pub fn wine_overrides(existing: Option<&str>, natives: &[&str]) -> String {
    let mut rules = Vec::new();
    if !natives.is_empty() {
        rules.push(format!("{}=n,b", natives.join(",")));
    }
    for rule in existing.into_iter().flat_map(|r| r.split(';')) {
        if rule.trim().is_empty() {
            continue;
        }
        let Some((names, action)) = rule.split_once('=') else {
            rules.push(rule.to_owned());
            continue;
        };
        let kept: Vec<_> = names
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty() && !natives.iter().any(|o| n.eq_ignore_ascii_case(o)))
            .collect();
        if !kept.is_empty() {
            rules.push(format!("{}={action}", kept.join(",")));
        }
    }
    rules.join(";")
}

/// The runtime's record of the files the HD UI overlay put in, so
/// switching it off can put the game's own back.
const HD_UI_FILES: &str = ".cmr3-hd-ui";

/// Overlays the HD UI pack (Data/, fonts/) into the runtime folder.
/// Returns how many files went in; 0 when not bundled.
pub fn install_hd_ui(runtime: &Path, bundle: &Path) -> io::Result<usize> {
    let from = bundle.join("hd-ui");
    if !from.is_dir() {
        return Ok(0);
    }
    let rels: Vec<String> = runtime::list(&from)?.into_keys().collect();
    // Recorded first, so an interrupted overlay can still be undone.
    let mut owned = hd_ui_files(runtime);
    owned.extend(rels.iter().cloned());
    let text: String = owned.iter().map(|r| format!("{r}\n")).collect();
    runtime::install(&runtime.join(HD_UI_FILES), text.as_bytes())?;
    for rel in &rels {
        runtime::install_copy(&runtime::native(&from, rel), &runtime::native(runtime, rel))?;
    }
    Ok(rels.len())
}

fn hd_ui_files(runtime: &Path) -> std::collections::BTreeSet<String> {
    fs::read_to_string(runtime.join(HD_UI_FILES))
        .unwrap_or_default()
        .lines()
        // Only plain relative paths below the runtime folder.
        .filter(|r| !r.is_empty() && !r.starts_with('/') && !r.split('/').any(|p| p == ".."))
        .map(str::to_owned)
        .collect()
}

/// Takes the HD UI overlay back out: every file it put in becomes the
/// game's own again, or goes when the game has none. Returns how many
/// files were restored; 0 when the overlay is not installed.
pub fn remove_hd_ui(game: &Path, runtime: &Path) -> io::Result<usize> {
    let owned = hd_ui_files(runtime);
    for rel in &owned {
        runtime::restore(game, runtime, rel)?;
    }
    runtime::remove(&runtime.join(HD_UI_FILES))?;
    Ok(owned.len())
}

/// The xdelta arguments (after the program itself, which the process
/// start adds) that turn the DRM `from` exe into the DRM-free exe of
/// `variant` ("efigs" or "polish"), written to `to`. The variants are
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
        let game = temp("game-install");
        let rt = temp("rt-install");
        // A pre-existing (hard-linked-style) file is replaced, not written
        // through: removing the bundle copy later cannot matter.
        fs::write(rt.join("SilentPatchCMR3.asi"), b"old").unwrap();
        assert!(install_silentpatch(&rt, &bundle).unwrap());
        assert_eq!(fs::read(rt.join("SilentPatchCMR3.asi")).unwrap(), b"asi");
        assert_eq!(fs::read(rt.join("dsound.dll")).unwrap(), b"loader");
        assert!(!rt.join("dinput8.dll").is_file());

        assert!(install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap());
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"xidi");
        let ini = fs::read_to_string(rt.join("Xidi.ini")).unwrap();
        assert!(ini.contains("CMR3Gamepad"));

        assert!(install_dxvk(&game, &rt, &bundle, true).unwrap());
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"dxvk9");

        // Off takes Xidi back out.
        assert!(install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap());
        assert!(!rt.join("dinput8.dll").is_file());
        assert!(!rt.join("Xidi.ini").is_file());
        assert!(install_dxvk(&game, &rt, &bundle, false).unwrap());
        assert!(!rt.join("d3d9.dll").is_file());

        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn off_takes_out_only_what_the_launcher_put_in() {
        let bundle = bundle_with("own-bundle");
        let game = temp("own-game");
        let rt = temp("own-rt");
        // The game ships its own dinput8.dll (a mod of the player's).
        fs::write(game.join("dinput8.dll"), b"game's own").unwrap();
        runtime::sync(&game, &rt).unwrap();
        assert!(install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap());
        assert!(install_dxvk(&game, &rt, &bundle, true).unwrap());
        assert!(install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap());
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"game's own");
        assert!(!rt.join("Xidi.ini").exists());
        // A d3d9.dll the player dropped in after ours is not ours to remove.
        fs::write(rt.join("d3d9.dll"), b"player's d3d9").unwrap();
        assert!(install_dxvk(&game, &rt, &bundle, false).unwrap());
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"player's d3d9");
        // Off again leaves the game's own file alone.
        assert!(install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap());
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"game's own");
        assert_eq!(fs::read(game.join("dinput8.dll")).unwrap(), b"game's own");
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn files_from_before_the_ledger_are_still_taken_out() {
        let bundle = bundle_with("legacy-bundle");
        let game = temp("legacy-game");
        let rt = temp("legacy-rt");
        // An older launcher installed these without recording them.
        fs::write(rt.join("dinput8.dll"), b"xidi").unwrap();
        let ini = xidi::ini(Gamepad::Rally, true).unwrap();
        fs::write(rt.join("Xidi.ini"), ini).unwrap();
        fs::write(rt.join("d3d9.dll"), b"dxvk9").unwrap();
        install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap();
        install_dxvk(&game, &rt, &bundle, false).unwrap();
        for name in ["dinput8.dll", "Xidi.ini", "d3d9.dll"] {
            assert!(!rt.join(name).exists(), "{name}");
        }
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn wine_overrides_put_ours_first_and_keep_the_players() {
        assert_eq!(wine_overrides(None, &[]), "");
        assert_eq!(
            wine_overrides(None, &["dsound", "dinput8", "d3d9"]),
            "dsound,dinput8,d3d9=n,b"
        );
        // cmr3-play's own rules: ours replace the names we install, the
        // rest (winedbg, the AppImage's mscoree) stays.
        assert_eq!(
            wine_overrides(
                Some("dinput8,dsound,d3d9=n,b;winedbg=d;mscoree,mshtml="),
                &["dsound", "d3d9"]
            ),
            "dsound,d3d9=n,b;dinput8=n,b;winedbg=d;mscoree,mshtml="
        );
        assert_eq!(
            wine_overrides(Some("D3D9=b;;xinput1_3"), &["d3d9"]),
            "d3d9=n,b;xinput1_3"
        );
    }

    #[test]
    fn missing_parts_disable_their_feature() {
        let bundle = temp("empty-bundle");
        let game = temp("game-missing");
        let rt = temp("rt-missing");
        assert!(!install_silentpatch(&rt, &bundle).unwrap());
        assert!(!install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap());
        assert!(!install_dxvk(&game, &rt, &bundle, true).unwrap());
        assert_eq!(install_hd_ui(&rt, &bundle).unwrap(), 0);
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
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
    fn hd_ui_off_puts_the_games_files_back() {
        let bundle = temp("hd-off-bundle");
        let game = temp("hd-off-game");
        let rt = temp("hd-off-rt");
        fs::create_dir_all(bundle.join("hd-ui/Data/Boot")).unwrap();
        fs::create_dir_all(bundle.join("hd-ui/fonts")).unwrap();
        fs::write(bundle.join("hd-ui/Data/Boot/colin3.dds"), b"high").unwrap();
        fs::write(bundle.join("hd-ui/fonts/hel_18pt.dds"), b"hd").unwrap();
        fs::create_dir_all(game.join("Data/Boot")).unwrap();
        fs::write(game.join("Data/Boot/colin3.dds"), b"low").unwrap();
        runtime::sync(&game, &rt).unwrap();

        assert_eq!(install_hd_ui(&rt, &bundle).unwrap(), 2);
        assert_eq!(remove_hd_ui(&game, &rt).unwrap(), 2);
        assert_eq!(fs::read(rt.join("Data/Boot/colin3.dds")).unwrap(), b"low");
        assert!(!rt.join("fonts/hel_18pt.dds").exists(), "overlay-only file");
        assert_eq!(fs::read(game.join("Data/Boot/colin3.dds")).unwrap(), b"low");
        // Off again: nothing recorded, nothing to do.
        assert_eq!(remove_hd_ui(&game, &rt).unwrap(), 0);
        // And a sync afterwards brings nothing (the manifest still holds).
        assert_eq!(runtime::sync(&game, &rt).unwrap().0, 0);
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
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
                "-f".to_string(),
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

    /// The arguments work with a real xdelta3 (skipped when none is on
    /// PATH): encode a tiny patch, then decode it the launcher's way.
    #[test]
    fn xdelta_args_decode_with_a_real_xdelta() {
        use std::process::Command;
        if Command::new("xdelta3").arg("-V").output().is_err() {
            eprintln!("xdelta3 not on PATH; skipped");
            return;
        }
        let d = temp("xd-real");
        fs::create_dir_all(d.join("patcher")).unwrap();
        fs::write(d.join("old.exe"), b"the DRM executable bytes").unwrap();
        fs::write(d.join("want.exe"), b"the DRM-free executable bytes").unwrap();
        let encoded = Command::new("xdelta3")
            .args(["-f", "-e", "-s"])
            .arg(d.join("old.exe"))
            .arg(d.join("want.exe"))
            .arg(d.join("patcher/efigs.xdelta"))
            .status()
            .unwrap();
        assert!(encoded.success());
        let xdelta = d.join("patcher/xdelta.exe");
        let args = xdelta_args(&xdelta, &d.join("old.exe"), "efigs", &d.join("new.exe"));
        let decoded = Command::new("xdelta3").args(&args).status().unwrap();
        assert!(decoded.success());
        assert_eq!(
            fs::read(d.join("new.exe")).unwrap(),
            b"the DRM-free executable bytes"
        );
        fs::remove_dir_all(&d).unwrap();
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
