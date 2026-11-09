//! What the game folder the player chose holds: an installed game, or an
//! installer the launcher knows, which it unpacks once into a folder of
//! its own (the SDK's `rea_launcher::installer`; the Windows half,
//! innoextract and the progress window, is in the binary).
//!
//! The installer is never run: the bundled innoextract only reads its
//! files. The one installer supported is the Magipack repack v4, a
//! third-party build the owner chose to support (2026-11-09,
//! `docs/RESEARCH.md`): unpacked, it is the publisher's DRM-free 1.1
//! (variant B) plus the repack's own SilentPatch, which the launcher
//! replaces with the one it bundles ([`InstallerBuild::drop`]).

use std::path::{Path, PathBuf};

use rea_launcher::identify::{self, Known};
use rea_launcher::installer::{self, Installer, InstallerBuild};
use rea_launcher::paths;

use crate::edition;

/// The game's executable.
pub const EXE: &str = "Rally_3PC.exe";

/// The installers the launcher unpacks, by their setup program's SHA-256.
pub static INSTALLERS: &[Known<InstallerBuild>] = &[Known::new(
    "0966e300f69b0ea0c564fd6626c4147c2691a3fb8e2393d53a8948370dbd1166",
    InstallerBuild {
        name: "the Magipack repack (v4)",
        setup: "Colin McRae Rally 3_Repack_v4_Setup.exe",
        parts: &[
            ("Colin McRae Rally 3_Repack_v4_Setup-1.bin", 735_350_784),
            ("Colin McRae Rally 3_Repack_v4_Setup-2.bin", 736_000_000),
            ("Colin McRae Rally 3_Repack_v4_Setup-3.bin", 330_889_825),
        ],
        unpacked: 3_398_338_055,
        root: "app",
        // No two components write the same file.
        prefer: &[],
        // The repack's own SilentPatch and its ASI loader: the launcher
        // installs the ones it bundles into the runtime copy, and would
        // otherwise keep these older copies as the player's own files.
        drop: &["dinput8.dll", "SilentPatchCMR3.asi"],
    },
)];

/// What a game folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// An installed game: the executable, spelled as on disk.
    Game(PathBuf),
    /// A known installer, every data part there at its size.
    Installer(Installer),
    /// Neither.
    Nothing,
}

/// Looks at `dir`: an installed game wins over an installer beside it.
/// An installer the launcher doesn't know, or a known one with a part
/// missing or short, is an error for the player (left alone, reported
/// with its SHA-256 and [`edition::ISSUE_URL`]).
pub fn find(dir: &Path) -> Result<Found, String> {
    if let Some(exe) = paths::find_ignoring_case(dir, EXE).filter(|p| p.is_file()) {
        return Ok(Found::Game(exe));
    }
    Ok(
        match installer::detect(dir, INSTALLERS, edition::ISSUE_URL)? {
            Some(found) => Found::Installer(found),
            None => Found::Nothing,
        },
    )
}

/// The message for a folder with neither the game nor an installer.
pub fn nothing_here(dir: &Path) -> String {
    format!(
        "There is no {EXE} in {}, and no installer the launcher knows.\n\n\
         Choose the folder the game is installed in, or the folder with the game's installer \
         (the Magipack repack v4's setup program and its .bin files).",
        dir.display()
    )
}

/// Where the launcher unpacks an installer: beside the runtime copy (it
/// is as big as the game, so it follows a runtime moved to another
/// drive), named after it: `Rally3` → `Rally3-game`.
pub fn unpacked_dir(runtime: &Path) -> PathBuf {
    let name = runtime.file_name().unwrap_or_default().to_string_lossy();
    runtime.with_file_name(format!("{name}-game"))
}

/// The launcher's check of an unpacked game, run before it takes its
/// place: the executable must be there and readable. Its build is named
/// for the log; a build that is not DRM-free is left to the DRM-free
/// patcher at Play, as for an installed game.
pub fn check_game(dir: &Path) -> Result<String, String> {
    let exe = paths::find_ignoring_case(dir, EXE)
        .filter(|p| p.is_file())
        .ok_or_else(|| format!("the installer did not give a {EXE}"))?;
    let bytes = std::fs::read(&exe).map_err(|e| format!("cannot read {}: {e}", exe.display()))?;
    Ok(match edition::identify(&bytes) {
        edition::Edition::DrmFree(name) => name.to_string(),
        edition::Edition::NeedsPatching => {
            "not a known DRM-free build; the DRM-free patcher will try it".into()
        }
    })
}

/// Whether `dir` is a folder the launcher unpacked from an installer (it
/// holds the SDK's marker).
pub fn is_unpacked_by_launcher(dir: &Path) -> bool {
    dir.join(installer::MARKER).is_file()
}

/// What the settings window and `--check` say about a known installer.
pub fn installer_note(found: &Installer) -> String {
    format!(
        "Found: the installer of {}. Play unpacks it once into the launcher's own folder \
         (about {:.1} GB free needed). The installer is never run; its files are only read.",
        found.build.name,
        installer::space_needed(found.build) as f64 / 1e9
    )
}

/// What the settings window says about a game the launcher unpacked.
pub fn unpacked_note(build: &str) -> String {
    format!(
        "Found: {build}, unpacked by the launcher from your installer. You no longer need \
         the installer files."
    )
}

/// The game folder setting to save after a run, if it changes: the
/// folder played from (the unpacked one, after an unpack), or, when the
/// folder came from the command line (`--game`), that folder as given;
/// the launcher never swaps a folder the player typed for its own.
/// `saved` is the file's current value (`None` also when the file is
/// new), `chosen` the folder the run started from, `played` the one the
/// game ran from.
pub fn game_to_save(
    saved: Option<&Path>,
    from_command_line: bool,
    chosen: &Path,
    played: &Path,
) -> Option<PathBuf> {
    let keep = if from_command_line { chosen } else { played };
    (saved != Some(keep)).then(|| keep.to_path_buf())
}

/// What became of the game folder setting after an unpack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// It now names the unpacked folder.
    Saved,
    /// The folder came from the command line (`--game`): left as given.
    CommandLine,
    /// The settings file could not be written (the log says why).
    NotSaved,
}

/// What the player is told after an unpack (launcher.log, the terminal).
pub fn unpacked_message(dest: &Path, setting: Setting) -> String {
    let dest = dest.display();
    match setting {
        Setting::Saved => format!(
            "The game was unpacked into {dest}. The game folder setting now points there; the \
             installer files are no longer needed."
        ),
        Setting::CommandLine => format!(
            "The game was unpacked into {dest}. The game folder came from the command line, \
             so the setting was left as given; play with --game \"{dest}\" and the installer \
             files are no longer needed."
        ),
        Setting::NotSaved => format!(
            "The game was unpacked into {dest}, but the game folder setting could not be \
             saved: choose that folder in the settings window."
        ),
    }
}

/// The setup program's SHA-256 as hex, for messages.
pub fn sha256_hex(found: &Installer) -> String {
    identify::hex(&found.sha256)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmr3-source-{}-{name}-Jogos d'Ação",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A sparse file of `len` bytes (no disk space used).
    fn sized(path: &Path, len: u64) {
        fs::File::create(path).unwrap().set_len(len).unwrap();
    }

    #[test]
    fn the_magipack_repack_is_the_one_known_installer() {
        assert_eq!(INSTALLERS.len(), 1);
        let known = &INSTALLERS[0];
        assert_eq!(
            identify::hex(&known.sha256),
            "0966e300f69b0ea0c564fd6626c4147c2691a3fb8e2393d53a8948370dbd1166"
        );
        let build = known.value;
        assert_eq!(build.setup, "Colin McRae Rally 3_Repack_v4_Setup.exe");
        assert_eq!(
            build.parts,
            &[
                ("Colin McRae Rally 3_Repack_v4_Setup-1.bin", 735_350_784),
                ("Colin McRae Rally 3_Repack_v4_Setup-2.bin", 736_000_000),
                ("Colin McRae Rally 3_Repack_v4_Setup-3.bin", 330_889_825),
            ]
        );
        assert_eq!((build.unpacked, build.root), (3_398_338_055, "app"));
        assert!(build.prefer.is_empty());
        assert_eq!(build.drop, &["dinput8.dll", "SilentPatchCMR3.asi"]);
        // Every part beside the setup program, named after it.
        for (part, _) in build.parts {
            assert!(part.starts_with("Colin McRae Rally 3_Repack_v4_Setup-"));
        }
    }

    #[test]
    fn the_dropped_files_are_the_ones_the_launcher_bundles() {
        let bundled: Vec<&str> = crate::components::PARTS
            .iter()
            .flat_map(|(_, files, _)| files.iter())
            .filter_map(|f| f.rsplit('/').next())
            .collect();
        for dropped in INSTALLERS[0].value.drop {
            assert!(bundled.contains(dropped), "{dropped} is not bundled");
        }
    }

    #[test]
    fn an_installed_game_wins_over_an_installer() {
        let dir = temp("installed");
        fs::write(dir.join("RALLY_3PC.EXE"), b"x").unwrap();
        fs::write(dir.join("setup.exe"), b"Inno Setup Setup Data (5.5.0)").unwrap();
        assert_eq!(find(&dir).unwrap(), Found::Game(dir.join("RALLY_3PC.EXE")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_empty_folder_holds_nothing() {
        let dir = temp("empty");
        assert_eq!(find(&dir).unwrap(), Found::Nothing);
        let text = nothing_here(&dir);
        assert!(
            text.contains("Rally_3PC.exe") && text.contains("installer"),
            "{text}"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_unknown_inno_installer_is_left_alone_and_reported() {
        let dir = temp("unknown");
        let setup = dir.join("Colin McRae Rally 3_Repack_v4_Setup.exe");
        fs::write(&setup, b"MZ... Inno Setup Setup Data (5.5.0) ...").unwrap();
        let error = find(&dir).unwrap_err();
        assert!(error.contains("left alone"), "{error}");
        assert!(error.contains(edition::ISSUE_URL), "{error}");
        let digest = identify::hex(&identify::sha256_file(&setup).unwrap());
        assert!(error.contains(&digest), "{error}");
        // Another Inno Setup program, any name.
        fs::remove_file(&setup).unwrap();
        fs::write(dir.join("setup_cmr3.exe"), b"Inno Setup Setup Data (6.0.0)").unwrap();
        assert!(find(&dir).unwrap_err().contains("left alone"));
        // Any other .exe is not an installer.
        fs::remove_file(dir.join("setup_cmr3.exe")).unwrap();
        fs::write(dir.join("readme.exe"), b"MZ").unwrap();
        assert_eq!(find(&dir).unwrap(), Found::Nothing);
        fs::remove_dir_all(dir).unwrap();
    }

    /// A stand-in for the repack with the right sizes: the real setup
    /// program's digest can't be made, so it is checked through the SDK
    /// with a table of the same shape.
    #[test]
    fn a_missing_or_short_part_is_named() {
        static STAND_IN: &[Known<InstallerBuild>] = &[Known::new(
            // SHA-256 of b"setup".
            "8fb6d5f37e8055ce720bd0b1d56587f88c0071f285966ba17e72b2b12672aa73",
            InstallerBuild {
                parts: &[("Colin McRae Rally 3_Repack_v4_Setup-1.bin", 1_000)],
                ..INSTALLERS[0].value
            },
        )];
        let dir = temp("parts");
        fs::write(
            dir.join("Colin McRae Rally 3_Repack_v4_Setup.exe"),
            b"setup",
        )
        .unwrap();
        let detect = || installer::detect(&dir, STAND_IN, edition::ISSUE_URL);
        assert!(detect().unwrap_err().contains("is missing"));
        sized(&dir.join("Colin McRae Rally 3_Repack_v4_Setup-1.bin"), 999);
        assert!(
            detect()
                .unwrap_err()
                .contains("has 999 bytes instead of 1000")
        );
        sized(
            &dir.join("Colin McRae Rally 3_Repack_v4_Setup-1.bin"),
            1_000,
        );
        let found = detect().unwrap().expect("the stand-in installer");
        assert_eq!(found.build.name, "the Magipack repack (v4)");
        let note = installer_note(&found);
        assert!(
            note.starts_with(
                "Found: the installer of the Magipack repack (v4). Play unpacks it once"
            ),
            "{note}"
        );
        assert!(
            note.contains("never run") && note.contains("only read"),
            "{note}"
        );
        assert!(note.contains("3.7 GB"), "{note}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_unpacked_game_sits_beside_the_runtime_copy() {
        assert_eq!(
            unpacked_dir(Path::new("/h/CMR3Launcher/Rally3")),
            PathBuf::from("/h/CMR3Launcher/Rally3-game")
        );
        assert_eq!(
            unpacked_dir(Path::new("/d/CMR 3")),
            PathBuf::from("/d/CMR 3-game")
        );
    }

    #[test]
    fn the_game_check_needs_the_executable() {
        let dir = temp("check");
        assert!(check_game(&dir).unwrap_err().contains("Rally_3PC.exe"));
        fs::write(dir.join("rally_3pc.exe"), b"not a known build").unwrap();
        assert!(check_game(&dir).unwrap().contains("DRM-free patcher"));
        assert!(!is_unpacked_by_launcher(&dir));
        fs::write(dir.join(installer::MARKER), "ab").unwrap();
        assert!(is_unpacked_by_launcher(&dir));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_unpacked_folder_is_saved_unless_the_command_line_named_the_folder() {
        let installer = Path::new(r"D:\Repack");
        let unpacked = Path::new(r"C:\L\Rally3-game");
        // From the settings file (the window): the setting moves to the
        // unpacked game.
        assert_eq!(
            game_to_save(Some(installer), false, installer, unpacked),
            Some(unpacked.to_path_buf())
        );
        // Already there: nothing to write.
        assert_eq!(
            game_to_save(Some(unpacked), false, unpacked, unpacked),
            None
        );
        // A first run with no file: saved.
        assert_eq!(
            game_to_save(None, false, installer, unpacked),
            Some(unpacked.to_path_buf())
        );
        // `--game D:\Repack`: remembered as typed, never swapped.
        assert_eq!(
            game_to_save(None, true, installer, unpacked),
            Some(installer.to_path_buf())
        );
        assert_eq!(
            game_to_save(Some(installer), true, installer, unpacked),
            None
        );
        // An installed game: as before, the folder played.
        let game = Path::new(r"D:\CMR3");
        assert_eq!(
            game_to_save(None, false, game, game),
            Some(game.to_path_buf())
        );
        assert_eq!(game_to_save(Some(game), true, game, game), None);
    }

    #[test]
    fn the_player_is_told_where_the_game_went() {
        let dest = Path::new(r"C:\L\Rally3-game");
        let saved = unpacked_message(dest, Setting::Saved);
        assert!(saved.contains(r"C:\L\Rally3-game"), "{saved}");
        assert!(saved.contains("setting now points there"), "{saved}");
        assert!(saved.contains("no longer needed"), "{saved}");
        let given = unpacked_message(dest, Setting::CommandLine);
        assert!(given.contains("command line"), "{given}");
        assert!(given.contains(r#"--game "C:\L\Rally3-game""#), "{given}");
        let failed = unpacked_message(dest, Setting::NotSaved);
        assert!(failed.contains("could not be saved"), "{failed}");
        assert!(unpacked_note("DRM-free v1.1").contains("no longer need"));
    }
}
