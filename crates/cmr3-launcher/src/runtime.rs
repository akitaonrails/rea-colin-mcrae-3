//! The runtime folder: a private, writable copy of the user's game folder,
//! kept by the SDK's `rea_launcher::runtime` with CMR3's knowledge of its
//! files ([`Cmr3`]).
//!
//! The user's folder is never written. The game is about 3 GB, most of it
//! static data the game only reads (`Data\`, `fonts\`, `shaders\`, the math
//! DLLs), so those files are hard-linked into the runtime folder when the
//! two folders sit on one volume — near-instant setup — and copied
//! otherwise. Everything the game or the launcher writes into the runtime
//! folder is swapped in whole (`rea_launcher::fsx`), never written through
//! a link back to the original.
//!
//! A manifest records what each file looked like in the source, so later
//! launches bring in only what changed there. The game's profile (`CMR3`)
//! is copied once and then belongs to the game; the launcher keeps it in
//! step with the player's saves folder instead ([`profile_mirror`]).

use std::io;
use std::path::Path;

use rea_launcher::fsx;
use rea_launcher::runtime::{Policy, Stamp, SyncReport};
use rea_launcher::saves::{FirstRun, Mirror};

pub use rea_launcher::paths::native;
pub use rea_launcher::runtime::list;

pub const MANIFEST: &str = ".cmr3-manifest";

/// The game's own record of the player's progress, in the game's root.
pub const PROFILE: &str = "CMR3";

/// The profile's state as of the last mirror, kept in the runtime folder:
/// it tells which copy changed since.
pub const PROFILE_STATE: &str = ".cmr3-profile-state";

/// The temp-file suffix launchers before the SDK used (`.name.cmr3-new`).
pub const LEGACY_TEMP: &str = ".cmr3-new";

/// What CMR3 knows about its files.
pub struct Cmr3;

impl Policy for Cmr3 {
    /// Trees the game only reads, and the static root files: hard-linked
    /// when possible.
    fn link(&self, rel: &str) -> bool {
        let top = rel.split('/').next().unwrap_or_default();
        if matches!(top, "Data" | "fonts" | "shaders") {
            return true;
        }
        // No folder part: static root files.
        !rel.contains('/') && {
            let lower = rel.to_ascii_lowercase();
            [".dll", ".ico", ".exe", ".bin"]
                .iter()
                .any(|ext| lower.ends_with(ext))
        }
    }

    /// The profile: copied once, then the game's.
    fn game_written(&self, rel: &str, _source: Stamp) -> bool {
        rel.eq_ignore_ascii_case(PROFILE)
    }
}

/// Brings `runtime` up to date with `game`. Temp files an older launcher
/// left behind (`.cmr3-new`) are unlinked first, best effort.
pub fn sync(game: &Path, runtime: &Path) -> io::Result<SyncReport> {
    if runtime.is_dir() {
        let _ = fsx::remove_stale_temps(runtime, &[LEGACY_TEMP]);
    }
    rea_launcher::runtime::sync(game, runtime, &Cmr3, MANIFEST)
}

/// Puts the game's own `rel` back in the runtime folder (linked or copied,
/// as a sync would), or removes it when the game has none.
pub fn restore(game: &Path, runtime: &Path, rel: &str) -> io::Result<()> {
    rea_launcher::runtime::restore(game, runtime, rel, &Cmr3)
}

/// The profile mirror between the runtime's `CMR3` (`runtime_profile`) and
/// the player's copy (`saved_profile`), recorded in `state`
/// ([`PROFILE_STATE`] in the runtime folder). A copy changed since the last
/// mirror wins (a backup the player restored, even an older one; or the
/// runtime's when a session ended before it was saved); without a record
/// the newer copy wins.
pub fn profile_mirror<'a>(
    runtime_profile: &'a Path,
    saved_profile: &'a Path,
    state: &'a Path,
) -> Mirror<'a> {
    Mirror::file(runtime_profile, saved_profile, state, FirstRun::NewerWins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn temp(name: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("cmr3-launcher-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn static_and_written_files_are_classified() {
        let s = Stamp { size: 1, mtime: 1 };
        for rel in [
            "Data/Tracks/x",
            "fonts/hel_18pt.dds",
            "shaders/water.pso",
            "MathCPU.dll",
            "Rally_3PC.exe",
            "PATCHINFO.BIN",
        ] {
            assert!(Cmr3.link(rel), "{rel}");
        }
        for rel in ["CMR3", "SilentPatchCMR3.ini", "unins000.exe.log"] {
            assert!(!Cmr3.link(rel), "{rel}");
        }
        assert!(Cmr3.game_written("CMR3", s));
        assert!(Cmr3.game_written("cmr3", s));
        assert!(!Cmr3.game_written("CMR3.bak", s));
    }

    #[test]
    fn sync_links_data_and_keeps_the_original_safe() {
        let game = temp("game");
        let rt = temp("rt");
        fs::create_dir_all(game.join("Data/Tracks")).unwrap();
        fs::write(game.join("Rally_3PC.exe"), b"exe").unwrap();
        fs::write(game.join("Data/Tracks/a.dat"), b"data").unwrap();
        fs::write(game.join("CMR3"), b"profile").unwrap();

        let r = sync(&game, &rt).unwrap();
        assert_eq!((r.brought, r.linked), (3, 2), "the exe and the track file");
        assert_eq!(fs::read(rt.join("CMR3")).unwrap(), b"profile");
        assert!(rt.join(MANIFEST).is_file());
        assert!(!game.join(MANIFEST).exists());
        let again = sync(&game, &rt).unwrap();
        assert_eq!((again.brought, again.linked), (0, 0));

        // Installing over a linked file must not change the original.
        fsx::install(&rt.join("Data/Tracks/a.dat"), b"replaced").unwrap();
        assert_eq!(fs::read(game.join("Data/Tracks/a.dat")).unwrap(), b"data");

        // The game writes its profile; a changed source file comes in, the
        // profile does not.
        fs::write(rt.join("CMR3"), b"played").unwrap();
        fs::write(game.join("CMR3"), b"other-machine").unwrap();
        let f = fs::OpenOptions::new()
            .write(true)
            .open(game.join("Rally_3PC.exe"))
            .unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();
        drop(f);
        assert_eq!(sync(&game, &rt).unwrap().brought, 1);
        assert_eq!(fs::read(rt.join("CMR3")).unwrap(), b"played");

        // Restoring the game's own file after the launcher replaced it.
        fsx::install(&rt.join("Rally_3PC.exe"), b"patched").unwrap();
        restore(&game, &rt, "Rally_3PC.exe").unwrap();
        assert_eq!(fs::read(rt.join("Rally_3PC.exe")).unwrap(), b"exe");
        for d in [game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn temp_files_of_older_launchers_are_cleared() {
        let game = temp("legacy-temp-game");
        let rt = temp("legacy-temp-rt");
        fs::create_dir_all(game.join("Data")).unwrap();
        fs::create_dir_all(rt.join("Data")).unwrap();
        fs::write(game.join("Data/a.dat"), b"data").unwrap();
        // An interrupted pre-SDK install left its temp beside the target.
        fs::write(rt.join(".SilentPatchCMR3.ini.cmr3-new"), b"half").unwrap();
        fs::write(rt.join("Data/.a.dat.cmr3-new"), b"half").unwrap();
        sync(&game, &rt).unwrap();
        assert!(!rt.join(".SilentPatchCMR3.ini.cmr3-new").exists());
        assert!(!rt.join("Data/.a.dat.cmr3-new").exists());
        assert_eq!(fs::read(rt.join("Data/a.dat")).unwrap(), b"data");
        for d in [game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn the_runtime_must_be_outside_the_game() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(rea_launcher::paths::check_separate(&game, &game.join("rt")).is_err());
        assert!(rea_launcher::paths::check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(&game).unwrap();
    }

    /// A profile file with `bytes` and a modification time `secs` ago.
    fn put_profile(path: &Path, bytes: &[u8], secs_ago: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs_ago))
            .unwrap();
    }

    fn profile_dirs(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let rt = temp(&format!("{name}-rt"));
        let saves = temp(&format!("{name}-saves"));
        fs::create_dir_all(&rt).unwrap();
        let state = rt.join(PROFILE_STATE);
        (rt.join(PROFILE), saves.join(PROFILE), state, saves)
    }

    #[test]
    fn a_normal_session_saves_the_profile_at_exit() {
        let (rt, saves, state, saves_dir) = profile_dirs("profile-exit");
        let mirror = profile_mirror(&rt, &saves, &state);
        put_profile(&saves, b"start", 100);
        assert!(!mirror.before().unwrap().is_empty());
        assert_eq!(fs::read(&rt).unwrap(), b"start");
        assert!(mirror.before().unwrap().is_empty());
        put_profile(&rt, b"played", 0);
        assert_eq!(mirror.after().unwrap(), [PROFILE]);
        assert_eq!(fs::read(&saves).unwrap(), b"played");
        assert!(mirror.after().unwrap().is_empty());
        assert!(mirror.before().unwrap().is_empty());
        for d in [rt.parent().unwrap(), &saves_dir] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_restored_older_backup_wins() {
        let (rt, saves, state, saves_dir) = profile_dirs("profile-backup");
        let mirror = profile_mirror(&rt, &saves, &state);
        put_profile(&saves, b"start", 1000);
        mirror.before().unwrap();
        put_profile(&rt, b"latest", 10);
        mirror.after().unwrap();
        // The player copies an older backup into the saves folder.
        put_profile(&saves, b"backup", 5000);
        assert_eq!(mirror.before().unwrap().to_runtime, [PROFILE]);
        assert_eq!(fs::read(&rt).unwrap(), b"backup");
        assert!(mirror.after().unwrap().is_empty());
        assert_eq!(fs::read(&saves).unwrap(), b"backup");
        for d in [rt.parent().unwrap(), &saves_dir] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_session_killed_before_saving_is_recovered() {
        let (rt, saves, state, saves_dir) = profile_dirs("profile-crash");
        let mirror = profile_mirror(&rt, &saves, &state);
        put_profile(&saves, b"start", 1000);
        mirror.before().unwrap();
        // The game saved, then the launcher was killed: no after().
        put_profile(&rt, b"progress", 10);
        assert_eq!(mirror.before().unwrap().to_saves, [PROFILE]);
        assert_eq!(fs::read(&saves).unwrap(), b"progress");
        assert_eq!(fs::read(&rt).unwrap(), b"progress");
        for d in [rt.parent().unwrap(), &saves_dir] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn without_a_record_the_newer_copy_wins() {
        let (rt, saves, state, saves_dir) = profile_dirs("profile-first");
        put_profile(&saves, b"old", 1000);
        put_profile(&rt, b"new", 10);
        profile_mirror(&rt, &saves, &state).before().unwrap();
        assert_eq!(fs::read(&saves).unwrap(), b"new");
        assert!(state.is_file(), "recorded from now on");
        for d in [rt.parent().unwrap(), &saves_dir] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn the_state_file_of_older_launchers_is_read() {
        let (rt, saves, state, saves_dir) = profile_dirs("profile-legacy");
        put_profile(&saves, b"same", 1000);
        put_profile(&rt, b"same", 1000);
        // Pre-SDK record: the runtime profile's `size\tmtime` (seconds).
        let mtime = fs::metadata(&rt)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fs::write(&state, format!("4\t{mtime}\n")).unwrap();
        let mirror = profile_mirror(&rt, &saves, &state);
        assert!(mirror.before().unwrap().is_empty(), "both as recorded");
        let upgraded = fs::read_to_string(&state).unwrap();
        assert!(upgraded.ends_with(&format!("\t{}\n", PROFILE.to_lowercase())));
        // Then a backup restored with an older time still wins.
        put_profile(&saves, b"back", 5000);
        assert_eq!(mirror.before().unwrap().to_runtime, [PROFILE]);
        assert_eq!(fs::read(&rt).unwrap(), b"back");
        for d in [rt.parent().unwrap(), &saves_dir] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn hostile_paths_sync_and_mirror() {
        // Spaces, apostrophes and accents in every folder the player picks.
        let root = temp("José");
        let game = root.join("Jogos d'Ação").join("Colin McRae Rally 3");
        let rt = root.join("Runtime d'Ação");
        let saves = root.join("Saved Games").join("Colin McRae Rally 3");
        fs::create_dir_all(game.join("Data")).unwrap();
        fs::write(game.join("Rally_3PC.exe"), b"exe").unwrap();
        fs::write(game.join("Data/ação.dat"), b"data").unwrap();
        rea_launcher::paths::check_separate(&game, &rt).unwrap();
        let r = sync(&game, &rt).unwrap();
        assert_eq!(r.brought, 2);
        assert_eq!(fs::read(rt.join("Data/ação.dat")).unwrap(), b"data");
        let (rt_profile, saved) = (rt.join(PROFILE), saves.join(PROFILE));
        let state = rt.join(PROFILE_STATE);
        let mirror = profile_mirror(&rt_profile, &saved, &state);
        mirror.before().unwrap();
        put_profile(&rt_profile, b"played", 0);
        mirror.after().unwrap();
        assert_eq!(fs::read(&saved).unwrap(), b"played");
        fs::remove_dir_all(root).unwrap();
    }
}
