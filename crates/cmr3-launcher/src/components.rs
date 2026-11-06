//! The bundled components and how they go into the runtime folder.
//!
//! Everything the game needs travels with the launcher in a `thirdparty/`
//! folder next to its executable (fetched and checksummed when the package
//! is built; see `packaging/third-party.txt`):
//!
//! ```text
//! thirdparty/silentpatch/SilentPatchCMR3.asi   the fixes (MIT)
//! thirdparty/silentpatch/dinput8.dll           the ASI loader; installed as dsound.dll
//! thirdparty/xidi/dinput8.dll + Xidi.32.dll    the pad bridge (BSD-3-Clause)
//! thirdparty/patcher/xdelta.exe + *.xdelta     the DRM-free patcher (Apache-2.0)
//! thirdparty/dxvk/d3d9.dll                     DXVK for Wine (Zlib)
//! thirdparty/hd-ui/…                           the HD UI overlay (Bekoha)
//! ```
//!
//! Components are installed by whole-file swaps (`rea_launcher::fsx`), so a
//! hard-linked original is never written through, and reinstalls are
//! idempotent. What the launcher installed is recorded by checksum in the
//! SDK's ownership ledger (`rea_launcher::ledger`, `.cmr3-launcher-components`
//! in the runtime): only a file still holding those bytes is the launcher's
//! to replace or take out, and taking it out brings back the game folder's
//! own file. Missing components disable their feature with a log line,
//! never a failure.

use std::fs;
use std::io;
use std::path::Path;

use rea_launcher::fsx;
use rea_launcher::ledger::{self, Ledger, Outcome};

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

/// The bundled parts `--check` looks for: name, the files (a path ending
/// in `/` is a folder) under the bundle, and whether play works without
/// it (the HD UI is an extra).
pub const PARTS: [(&str, &[&str], bool); 6] = [
    ("SilentPatch", &["silentpatch/SilentPatchCMR3.asi"], false),
    ("ASI loader", &["silentpatch/dinput8.dll"], false),
    (
        "Xidi",
        &[
            "xidi/dinput8.dll",
            "xidi/Xidi.32.dll",
            "xidi/msvcp140.dll",
            "xidi/msvcp140_atomic_wait.dll",
            "xidi/concrt140.dll",
            "xidi/vcruntime140.dll",
        ],
        false,
    ),
    (
        "DRM-free patcher",
        &["patcher/xdelta.exe", "patcher/efigs.xdelta"],
        false,
    ),
    ("DXVK (Wine)", &["dxvk/d3d9.dll"], false),
    ("HD UI", &["hd-ui/"], true),
];

/// Which bundled parts are present, for `--check` and the log.
pub fn present(bundle: &Path) -> Vec<crate::check::Part> {
    let there = |rel: &str| match rel.strip_suffix('/') {
        Some(dir) => bundle.join(dir).is_dir(),
        None => bundle.join(rel).is_file(),
    };
    PARTS
        .iter()
        .map(|&(name, files, optional)| crate::check::Part {
            name,
            there: files.iter().all(|f| there(f)),
            optional,
        })
        .collect()
}

/// The runtime's ledger of the files the launcher installed (the SDK's
/// format: `sha256\tname` lines).
const OWNERSHIP: &str = ".cmr3-launcher-components";

fn ledger(game: &Path, runtime: &Path) -> Ledger {
    Ledger::load(game, runtime, OWNERSHIP)
}

/// How a component install went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// In place (or, for an "off" choice, taken out).
    Done,
    /// Not in the bundle: the feature is off.
    NotBundled,
    /// The runtime already holds this file, and it is neither the
    /// launcher's nor the game folder's own (something the player put
    /// there): left alone, the component left out.
    NotOurs(&'static str),
}

/// Installs `bytes` as the runtime's `name` unless someone else's file
/// is there. `known` lists the bundle bytes a launcher from before the
/// ledger may have written.
fn put(
    ledger: &mut Ledger,
    name: &'static str,
    bytes: &[u8],
    known: &[&[u8]],
) -> io::Result<Option<Installed>> {
    Ok(match ledger.install(name, bytes, known)? {
        Outcome::NotOurs => Some(Installed::NotOurs(name)),
        _ => None,
    })
}

/// Installs SilentPatch and its ASI loader (as `dsound.dll`, leaving the
/// `dinput8.dll` name free for Xidi).
pub fn install_silentpatch(game: &Path, runtime: &Path, bundle: &Path) -> io::Result<Installed> {
    let asi = bundle.join("silentpatch/SilentPatchCMR3.asi");
    let loader = bundle.join("silentpatch/dinput8.dll");
    if !asi.is_file() || !loader.is_file() {
        return Ok(Installed::NotBundled);
    }
    let mut ledger = ledger(game, runtime);
    let asi = fs::read(&asi)?;
    if let Some(refused) = put(&mut ledger, "SilentPatchCMR3.asi", &asi, &[&asi])? {
        return Ok(refused);
    }
    // The loader hooks the first of dsound/dinput8/winmm it is named as;
    // dsound is free here and the game imports it.
    let loader = fs::read(&loader)?;
    if let Some(refused) = put(&mut ledger, "dsound.dll", &loader, &[&loader])? {
        return Ok(refused);
    }
    Ok(Installed::Done)
}

/// Every `Xidi.ini` this launcher writes (each preset, log on and off):
/// how a pre-ledger file of ours is recognised.
fn xidi_inis() -> Vec<Vec<u8>> {
    Gamepad::ALL
        .into_iter()
        .flat_map(|p| [xidi::ini(p, false), xidi::ini(p, true)])
        .flatten()
        .map(String::into_bytes)
        .collect()
}

/// Xidi files earlier releases installed, by sha256: Xidi 4.3.1's Win32
/// `dinput8.dll` and the `Xidi.ini` texts of 0.2.0–0.3.0 (rally and
/// standard, log off and on). A runtime from before the ledger may hold
/// them; they are the launcher's and get replaced.
const EARLIER_XIDI: [&str; 5] = [
    "dedbf4aa0d1cc3b0c036feb1a089d181345c03ef07a118886b66654f04395efd",
    "1bbf46f735f129ea058204295475a8f1a95383c27667a40cc04ddda2dcfc9fc1",
    "7a510df3a9fa10a332da185c1e057c620a8e206e89862c1eadbca6f766eb5c82",
    "9868dfaa075b30e5999a6a1efda39fe77b3b2836713581da57590545c42f8486",
    "a7e88ecf8bbf65d48f9511f593a74ad1c99b2a913c864a015a81e5fffceaeed6",
];

/// Installs Xidi — its `dinput8.dll` wrapper and the `Xidi.32.dll` main
/// binary the wrapper loads — with the preset's `Xidi.ini` (its logger on
/// when `log`); the `off` preset takes out what the launcher put in,
/// bringing back a `dinput8.dll` of the game's own (the game then loads
/// it, or the system DirectInput).
pub fn install_xidi(
    game: &Path,
    runtime: &Path,
    bundle: &Path,
    preset: Gamepad,
    log: bool,
) -> io::Result<Installed> {
    install_xidi_over(game, runtime, bundle, preset, log, &EARLIER_XIDI)
}

/// [`install_xidi`], with the digests of the files earlier releases
/// installed (`earlier`) given.
fn install_xidi_over(
    game: &Path,
    runtime: &Path,
    bundle: &Path,
    preset: Gamepad,
    log: bool,
    earlier: &[&str],
) -> io::Result<Installed> {
    const FILES: [&str; 6] = [
        "dinput8.dll",
        "Xidi.32.dll",
        "msvcp140.dll",
        "msvcp140_atomic_wait.dll",
        "concrt140.dll",
        "vcruntime140.dll",
    ];
    let from = |name: &str| bundle.join("xidi").join(name);
    let mut ledger = ledger(game, runtime);
    // What counts as ours in the runtime: what this bundle holds, plus a
    // file an earlier release installed there before the ledger (the
    // SDK's `Ledger::known_with_earlier`).
    let mut known = Vec::new();
    for name in FILES {
        let current = fs::read(from(name)).into_iter().collect();
        known.push(ledger.known_with_earlier(name, current, earlier)?);
    }
    let inis = ledger.known_with_earlier("Xidi.ini", xidi_inis(), earlier)?;
    let inis: Vec<&[u8]> = inis.iter().map(Vec::as_slice).collect();
    let ours = |name: &str| -> Vec<&[u8]> {
        let i = FILES.iter().position(|f| *f == name).expect("a Xidi file");
        known[i].iter().map(Vec::as_slice).collect()
    };
    match xidi::ini(preset, log) {
        Some(ini) => {
            if !FILES.iter().all(|name| from(name).is_file()) {
                return Ok(Installed::NotBundled);
            }
            // The settings first: Xidi without them would map nothing.
            if let Some(refused) = put(&mut ledger, "Xidi.ini", ini.as_bytes(), &inis)? {
                return Ok(refused);
            }
            for name in FILES {
                let bytes = fs::read(from(name))?;
                let ours = ours(name);
                if let Some(refused) = put(&mut ledger, name, &bytes, &ours)? {
                    return Ok(refused);
                }
            }
            Ok(Installed::Done)
        }
        None => {
            for name in FILES {
                ledger.take_out(name, &ours(name))?;
            }
            ledger.take_out("Xidi.ini", &inis)?;
            Ok(Installed::Done)
        }
    }
}

/// DXVK's d3d9.dll next to the game under Wine (Vulkan) — or taken out
/// again (a d3d9.dll of the game's own comes back). On Windows this is
/// never installed: the system's own Direct3D is the right one there.
pub fn install_dxvk(game: &Path, runtime: &Path, bundle: &Path, on: bool) -> io::Result<Installed> {
    let from = bundle.join("dxvk/d3d9.dll");
    let mut ledger = ledger(game, runtime);
    let ours: Vec<Vec<u8>> = fs::read(&from).into_iter().collect();
    let ours: Vec<&[u8]> = ours.iter().map(Vec::as_slice).collect();
    if !on {
        ledger.take_out("d3d9.dll", &ours)?;
        return Ok(Installed::Done);
    }
    let Some(bytes) = ours.first() else {
        return Ok(Installed::NotBundled);
    };
    Ok(put(&mut ledger, "d3d9.dll", bytes, &ours)?.unwrap_or(Installed::Done))
}

/// The DLLs Xidi installs beside the game that Wine also ships: its
/// `dinput8` wrapper and the 32-bit VC++ 2015–2022 runtime DLLs it imports.
pub const XIDI_DLLS: [&str; 5] = [
    "dinput8",
    "msvcp140",
    "msvcp140_atomic_wait",
    "concrt140",
    "vcruntime140",
];

/// The DLLs the launcher puts beside the game that Wine also ships: dsound
/// (the ASI loader), Xidi (`dinput8` + its VC++ runtime DLLs), d3d9 (DXVK).
pub const BUNDLED_DLLS: [&str; 7] = [
    "dsound",
    "dinput8",
    "msvcp140",
    "msvcp140_atomic_wait",
    "concrt140",
    "vcruntime140",
    "d3d9",
];

/// Wine's per-program DLL overrides for the game (`HKCU\Software\Wine\
/// AppDefaults\Rally_3PC.exe\DllOverrides`, native then builtin), which
/// Wine's loader reads even when the launcher was started without
/// `cmr3-play` (it ignores `WINEDLLOVERRIDES` in the game's environment
/// block): the installed `natives` to set, and the other bundled DLLs
/// whose override of ours goes.
pub fn app_overrides(natives: &[&'static str]) -> (Vec<&'static str>, Vec<&'static str>) {
    let off = BUNDLED_DLLS
        .into_iter()
        .filter(|dll| !natives.contains(dll))
        .collect();
    (natives.to_vec(), off)
}

/// `WINEDLLOVERRIDES` for the game under Wine: `natives` (the DLL names
/// the launcher installed beside it: dsound for the ASI loader, dinput8
/// for Xidi, d3d9 for DXVK) load from the game folder first. The player's
/// own rules (`existing`) are kept, except for those names, so ours apply.
pub fn wine_overrides(existing: Option<&str>, natives: &[&str]) -> String {
    let ours = if natives.is_empty() {
        String::new()
    } else {
        format!("{}=n,b", natives.join(","))
    };
    rea_launcher::wine::merge_dll_overrides(&ours, existing)
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
    fsx::write_atomic(&runtime.join(HD_UI_FILES), text)?;
    for rel in &rels {
        fsx::install_copy(&runtime::native(&from, rel), &runtime::native(runtime, rel))?;
    }
    Ok(rels.len())
}

fn hd_ui_files(runtime: &Path) -> std::collections::BTreeSet<String> {
    fs::read_to_string(runtime.join(HD_UI_FILES))
        .unwrap_or_default()
        .lines()
        .filter(|r| ledger::is_plain_relative(r))
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
    fsx::remove(&runtime.join(HD_UI_FILES))?;
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
/// detail, its video quality, adapters) stays as the player left it, and
/// an unset option gives a key back only while it holds the launcher's
/// value (recorded in [`crate::sp::OWNED`]).
pub fn apply_settings(
    runtime: &Path,
    settings: &Settings,
    resolution: Option<(u32, u32)>,
) -> io::Result<Vec<String>> {
    let ini = runtime.join("SilentPatchCMR3.ini");
    let bytes = match fs::read(&ini) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    let record = runtime.join(crate::sp::OWNED);
    let mut owned = rea_launcher::ini::Owned::load(&record)?;
    let (bytes, log) = crate::sp::apply_picture(&bytes, settings, resolution, &mut owned);
    fsx::install(&ini, bytes)?;
    owned.save(&record)?;
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
        fs::write(b.join("xidi/Xidi.32.dll"), b"xidi-main").unwrap();
        fs::write(b.join("xidi/msvcp140.dll"), b"msvcp140").unwrap();
        fs::write(b.join("xidi/msvcp140_atomic_wait.dll"), b"msvcp140_aw").unwrap();
        fs::write(b.join("xidi/concrt140.dll"), b"concrt140").unwrap();
        fs::write(b.join("xidi/vcruntime140.dll"), b"vcruntime140").unwrap();
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
        // A copy of the bundle's file from a launcher before the ledger is
        // ours: replaced as a whole file, never written through.
        fs::write(rt.join("SilentPatchCMR3.asi"), b"asi").unwrap();
        let done = Installed::Done;
        assert_eq!(install_silentpatch(&game, &rt, &bundle).unwrap(), done);
        assert_eq!(fs::read(rt.join("SilentPatchCMR3.asi")).unwrap(), b"asi");
        assert_eq!(fs::read(rt.join("dsound.dll")).unwrap(), b"loader");
        assert!(!rt.join("dinput8.dll").is_file());
        assert!(
            fs::read_to_string(rt.join(OWNERSHIP))
                .unwrap()
                .contains("\tdsound.dll\n"),
            "recorded in the ledger"
        );

        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap(),
            done
        );
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"xidi");
        let ini = fs::read_to_string(rt.join("Xidi.ini")).unwrap();
        assert!(ini.contains("CMR3Gamepad"));
        // Another preset rewrites our own Xidi.ini.
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Standard, true).unwrap(),
            done
        );
        assert!(
            fs::read_to_string(rt.join("Xidi.ini"))
                .unwrap()
                .contains("StandardGamepad")
        );

        assert_eq!(install_dxvk(&game, &rt, &bundle, true).unwrap(), done);
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"dxvk9");

        // Off takes Xidi back out.
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap(),
            done
        );
        assert!(!rt.join("dinput8.dll").is_file());
        assert!(!rt.join("Xidi.ini").is_file());
        assert_eq!(install_dxvk(&game, &rt, &bundle, false).unwrap(), done);
        assert!(!rt.join("d3d9.dll").is_file());

        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_players_own_file_in_the_runtime_is_left_alone() {
        let bundle = bundle_with("players-bundle");
        let game = temp("players-game");
        let rt = temp("players-rt");
        // Dropped into the runtime by hand: not the game's, not ours.
        fs::write(rt.join("d3d9.dll"), b"reshade").unwrap();
        assert_eq!(
            install_dxvk(&game, &rt, &bundle, true).unwrap(),
            Installed::NotOurs("d3d9.dll")
        );
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"reshade");
        // The game folder's own copy, untouched in the runtime, is replaced.
        fs::write(game.join("dsound.dll"), b"game's dsound").unwrap();
        runtime::sync(&game, &rt).unwrap();
        assert_eq!(
            install_silentpatch(&game, &rt, &bundle).unwrap(),
            Installed::Done
        );
        assert_eq!(fs::read(rt.join("dsound.dll")).unwrap(), b"loader");
        assert_eq!(fs::read(game.join("dsound.dll")).unwrap(), b"game's dsound");
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
        let done = Installed::Done;
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap(),
            done
        );
        assert_eq!(install_dxvk(&game, &rt, &bundle, true).unwrap(), done);
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap(),
            done
        );
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"game's own");
        assert!(!rt.join("Xidi.ini").exists());
        // A d3d9.dll the player dropped in after ours is not ours to remove.
        fs::write(rt.join("d3d9.dll"), b"player's d3d9").unwrap();
        assert_eq!(install_dxvk(&game, &rt, &bundle, false).unwrap(), done);
        assert_eq!(fs::read(rt.join("d3d9.dll")).unwrap(), b"player's d3d9");
        // Off again leaves the game's own file alone.
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap(),
            done
        );
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"game's own");
        assert_eq!(fs::read(game.join("dinput8.dll")).unwrap(), b"game's own");
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn xidi_brings_its_main_binary_and_takes_it_out_again() {
        let bundle = bundle_with("xidi5-bundle");
        let game = temp("xidi5-game");
        let rt = temp("xidi5-rt");
        install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap();
        // Xidi 5's wrapper loads its main binary from beside the game.
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"xidi");
        assert_eq!(fs::read(rt.join("Xidi.32.dll")).unwrap(), b"xidi-main");
        install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap();
        for name in ["dinput8.dll", "Xidi.32.dll", "Xidi.ini"] {
            assert!(!rt.join(name).exists(), "{name}");
        }
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn an_earlier_releases_file_is_ours_before_the_ledger() {
        let bundle = bundle_with("earlier-bundle");
        let game = temp("earlier-game");
        let rt = temp("earlier-rt");
        let digest = rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"xidi-4"));
        // An earlier release's dinput8.dll, from before the ledger: listed
        // by digest, so it is ours and gets replaced.
        fs::write(rt.join("dinput8.dll"), b"xidi-4").unwrap();
        let done = install_xidi_over(&game, &rt, &bundle, Gamepad::Rally, false, &[&digest]);
        assert_eq!(done.unwrap(), Installed::Done);
        assert_eq!(fs::read(rt.join("dinput8.dll")).unwrap(), b"xidi");
        // Anything else stays the player's.
        let rt2 = temp("earlier-rt-players");
        fs::write(rt2.join("dinput8.dll"), b"players").unwrap();
        let refused = install_xidi_over(&game, &rt2, &bundle, Gamepad::Rally, false, &[&digest]);
        assert_eq!(refused.unwrap(), Installed::NotOurs("dinput8.dll"));
        assert_eq!(fs::read(rt2.join("dinput8.dll")).unwrap(), b"players");
        for d in [bundle, game, rt, rt2] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn earlier_xidi_files_are_the_ones_releases_installed() {
        // Xidi 4.3.1's Win32 dinput8.dll (from the pinned zip, sha256
        // 78120f44…) and the four Xidi.ini texts 0.2.0–0.3.0 wrote.
        assert!(
            EARLIER_XIDI
                .contains(&"dedbf4aa0d1cc3b0c036feb1a089d181345c03ef07a118886b66654f04395efd")
        );
        assert_eq!(EARLIER_XIDI.len(), 5);
        // None of them is a file this release writes.
        for ini in xidi_inis() {
            let d = rea_launcher::identify::hex(&rea_launcher::identify::sha256(&ini));
            assert!(!EARLIER_XIDI.contains(&d.as_str()));
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
    fn a_ledger_written_before_the_sdk_is_read() {
        let bundle = bundle_with("old-ledger-bundle");
        let game = temp("old-ledger-game");
        let rt = temp("old-ledger-rt");
        // The pre-SDK launcher's own ledger line for a Xidi DLL it wrote,
        // and a hostile line that must never be followed.
        let digest = rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"xidi-old"));
        fs::write(rt.join("dinput8.dll"), b"xidi-old").unwrap();
        fs::write(
            rt.join(OWNERSHIP),
            format!("{digest}\tdinput8.dll\n{digest}\t../outside.dll\n"),
        )
        .unwrap();
        install_xidi(&game, &rt, &bundle, Gamepad::Off, false).unwrap();
        assert!(!rt.join("dinput8.dll").exists(), "recognised by its record");
        for d in [bundle, game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn app_overrides_cover_every_bundled_dll_once() {
        assert_eq!(
            app_overrides(&BUNDLED_DLLS),
            (BUNDLED_DLLS.to_vec(), vec![])
        );
        // A part not installed (turned off, or the game's own DLL kept)
        // loses the override we wrote for it.
        assert_eq!(
            app_overrides(&["dsound"]),
            (
                vec!["dsound"],
                vec![
                    "dinput8",
                    "msvcp140",
                    "msvcp140_atomic_wait",
                    "concrt140",
                    "vcruntime140",
                    "d3d9",
                ]
            )
        );
        assert_eq!(app_overrides(&[]), (vec![], BUNDLED_DLLS.to_vec()));
    }

    /// Under Wine (`sdk/scripts/wine-test`): the overrides reach the key
    /// Wine's loader reads for the game, a part turned off loses ours, and
    /// a player's own value stays.
    #[cfg(windows)]
    #[test]
    fn app_overrides_reach_wines_registry() {
        use rea_launcher_win32::reg::{Hive, Key};
        use rea_launcher_win32::wine::{self, DllMode, app_defaults_key};
        let exe = format!("cmr3-test-{}.exe", std::process::id());
        let modes = |dlls: &[&'static str]| {
            dlls.iter()
                .map(|&d| (d, DllMode::NativeThenBuiltin))
                .collect::<Vec<_>>()
        };
        let read = |dll: &str| {
            Key::open_read(Hive::CurrentUser, &app_defaults_key(&exe))
                .unwrap()
                .and_then(|k| k.string(dll).unwrap())
        };
        assert!(wine::is_wine(), "run under Wine");
        let (on, _) = app_overrides(&BUNDLED_DLLS);
        wine::app_dll_overrides(&exe, &modes(&on)).unwrap();
        for dll in BUNDLED_DLLS {
            assert_eq!(read(dll).as_deref(), Some("native,builtin"), "{dll}");
        }
        // The player sets d3d9 to builtin; then the gamepad and DXVK go off.
        Key::create(Hive::CurrentUser, &app_defaults_key(&exe))
            .unwrap()
            .set_string("d3d9", "builtin")
            .unwrap();
        let (on, off) = app_overrides(&["dsound"]);
        wine::app_dll_overrides(&exe, &modes(&on)).unwrap();
        let removed = wine::remove_app_dll_overrides(&exe, &modes(&off)).unwrap();
        assert_eq!(removed, XIDI_DLLS);
        assert_eq!(read("dsound").as_deref(), Some("native,builtin"));
        assert_eq!(read("dinput8"), None);
        assert_eq!(read("d3d9").as_deref(), Some("builtin"));
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
        assert_eq!(
            wine_overrides(Some("DSound.DLL,winmm.dll=b;é=n"), &["dsound"]),
            "dsound=n,b;winmm.dll=b;é=n"
        );
    }

    #[test]
    fn missing_parts_disable_their_feature() {
        let bundle = temp("empty-bundle");
        let game = temp("game-missing");
        let rt = temp("rt-missing");
        let missing = Installed::NotBundled;
        assert_eq!(install_silentpatch(&game, &rt, &bundle).unwrap(), missing);
        assert_eq!(
            install_xidi(&game, &rt, &bundle, Gamepad::Rally, false).unwrap(),
            missing
        );
        assert_eq!(install_dxvk(&game, &rt, &bundle, true).unwrap(), missing);
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
        // A record naming a path outside the runtime is never followed.
        fs::write(
            rt.join(HD_UI_FILES),
            "../outside.dds\n/etc/x\nData/Boot/colin3.dds\n",
        )
        .unwrap();
        assert_eq!(remove_hd_ui(&game, &rt).unwrap(), 1);
        // And a sync afterwards brings nothing (the manifest still holds).
        assert_eq!(runtime::sync(&game, &rt).unwrap().brought, 0);
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
        // Compared as paths, not text: `join` spells the separator its
        // own way on each platform.
        let want: Vec<std::ffi::OsString> = vec![
            "-f".into(),
            "-d".into(),
            "-s".into(),
            "C:\\rt\\Rally_3PC.exe".into(),
            bundle.join("patcher").join("efigs.xdelta").into(),
            "C:\\rt\\Rally_3PC.exe.new".into(),
        ];
        assert_eq!(args.len(), want.len());
        for (got, want) in args.iter().zip(&want) {
            assert_eq!(Path::new(got), Path::new(want));
        }
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
