//! The runtime folder: a private, writable copy of the user's game folder.
//!
//! The user's folder is never written. The game is about 3 GB, most of it
//! static data the game only reads (`Data\`, `fonts\`, `shaders\`, the math
//! DLLs), so those files are hard-linked into the runtime folder when the
//! two folders sit on one volume — near-instant setup — and copied
//! otherwise. Everything the game or the launcher writes into the runtime
//! folder goes through `install`/`replace`, which swap the file in whole,
//! never writing through a link back to the original.
//!
//! A manifest records what each file looked like in the source, so later
//! launches bring in only what changed there. The game's profile (`CMR3`)
//! is copied once and then belongs to the game; the launcher keeps it in
//! step with the player's saves folder instead.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST: &str = ".cmr3-manifest";

/// The game's own record of the player's progress, in the game's root.
pub const PROFILE: &str = "CMR3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: u64,
}

/// Relative path (with `/` separators) to its stamp.
pub type Listing = BTreeMap<String, Stamp>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub copy: Vec<String>,
    /// The new manifest, written after the copies succeed.
    pub manifest: Listing,
}

/// True for trees the game only reads, which may be hard-linked instead of
/// copied (and fall back to a copy when linking is not possible).
fn is_static(rel: &str) -> bool {
    let top = rel.split('/').next().unwrap_or_default();
    if matches!(top, "Data" | "fonts" | "shaders") {
        return true;
    }
    // No folder part: static root files.
    !rel.contains('/') && {
        let lower = rel.to_ascii_lowercase();
        lower.ends_with(".dll")
            || lower.ends_with(".ico")
            || lower.ends_with(".exe")
            || lower.ends_with(".bin")
    }
}

/// True for files the game writes while playing: never linked, never
/// overwritten by a sync once they exist in the runtime folder.
fn is_game_written(rel: &str) -> bool {
    rel.eq_ignore_ascii_case(PROFILE)
}

/// Decides which files to bring over.
///
/// - missing in the runtime folder: link or copy;
/// - changed in the source since the last sync (per the manifest): bring
///   it in again, except files the game writes, which the game owns once
///   they exist.
pub fn plan(source: &Listing, runtime_has: &dyn Fn(&str) -> bool, manifest: &Listing) -> Plan {
    let mut copy = Vec::new();
    let mut next = Listing::new();
    for (rel, stamp) in source {
        let present = runtime_has(rel);
        let changed = manifest.get(rel) != Some(stamp);
        if !present || (changed && !is_game_written(rel)) {
            copy.push(rel.clone());
            next.insert(rel.clone(), *stamp);
        } else {
            next.insert(rel.clone(), *manifest.get(rel).unwrap_or(stamp));
        }
    }
    Plan {
        copy,
        manifest: next,
    }
}

pub fn parse_manifest(text: &str) -> Listing {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let size = parts.next()?.parse().ok()?;
            let mtime = parts.next()?.parse().ok()?;
            let rel = parts.next()?.to_owned();
            Some((rel, Stamp { size, mtime }))
        })
        .collect()
}

pub fn format_manifest(listing: &Listing) -> String {
    listing
        .iter()
        .map(|(rel, s)| format!("{}\t{}\t{}\n", s.size, s.mtime, rel))
        .collect()
}

fn mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

/// Lists every file below `root`.
pub fn list(root: &Path) -> io::Result<Listing> {
    fn walk(root: &Path, dir: &Path, out: &mut Listing) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                walk(root, &path, out)?;
            } else if meta.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .expect("walk stays below root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(
                    rel,
                    Stamp {
                        size: meta.len(),
                        mtime: mtime_secs(&meta),
                    },
                );
            }
        }
        Ok(())
    }
    let mut out = Listing::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

pub fn native(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(root.to_path_buf(), |p, part| p.join(part))
}

/// Refuses runtime folders that overlap the game folder.
pub fn check_separate(game: &Path, runtime: &Path) -> Result<(), String> {
    let g = fs::canonicalize(game).map_err(|e| format!("game folder {}: {e}", game.display()))?;
    let r = fs::canonicalize(runtime).unwrap_or_else(|_| runtime.to_path_buf());
    if r.starts_with(&g) || g.starts_with(&r) {
        return Err(format!(
            "the runtime folder {} must be outside the game folder {}",
            runtime.display(),
            game.display()
        ));
    }
    Ok(())
}

fn make_writable(path: &Path) -> io::Result<()> {
    let mut perm = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    fs::set_permissions(path, perm)
}

/// Brings one file from the game into the runtime folder: a hard link for
/// static data when both sit on one volume (falling back to a copy), a
/// plain copy otherwise. Returns how the file arrived.
fn bring_over(from: &Path, to: &Path, rel: &str) -> io::Result<&'static str> {
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir)?;
    }
    if to.exists() {
        make_writable(to)?;
        fs::remove_file(to)?;
    }
    if is_static(rel) && fs::hard_link(from, to).is_ok() {
        return Ok("linked");
    }
    fs::copy(from, to)?;
    make_writable(to)?;
    Ok("copied")
}

/// Writes `bytes` to `path` as a fresh file, never through a hard link to
/// the original: an existing file is removed first.
pub fn install(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if path.exists() {
        make_writable(path)?;
        fs::remove_file(path)?;
    }
    fs::write(path, bytes)
}

/// Copies `from` over `path` as a fresh file (never through a link).
pub fn install_copy(from: &Path, path: &Path) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if path.exists() {
        make_writable(path)?;
        fs::remove_file(path)?;
    }
    fs::copy(from, path)?;
    make_writable(path)
}

/// Removes a file the launcher put in the runtime folder, writable or not.
pub fn remove(path: &Path) -> io::Result<()> {
    if path.exists() {
        make_writable(path)?;
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Brings `runtime` up to date with `game`. Returns how many files were
/// brought over and how many of those were hard-linked.
pub fn sync(game: &Path, runtime: &Path) -> io::Result<(usize, usize)> {
    let source = list(game)?;
    fs::create_dir_all(runtime)?;
    let manifest_path = runtime.join(MANIFEST);
    let manifest = fs::read_to_string(&manifest_path)
        .map(|t| parse_manifest(&t))
        .unwrap_or_default();
    let plan = plan(&source, &|rel| native(runtime, rel).is_file(), &manifest);
    let mut linked = 0;
    for rel in &plan.copy {
        let from = native(game, rel);
        let to = native(runtime, rel);
        if bring_over(&from, &to, rel)? == "linked" {
            linked += 1;
        }
        // The manifest compares source stamps; the runtime copy keeps the
        // source's modification time so saves mirroring can tell them
        // apart later.
        if let Some(stamp) = source.get(rel)
            && let Ok(file) = fs::OpenOptions::new().write(true).open(&to)
        {
            let _ = file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.mtime));
        }
    }
    fs::write(&manifest_path, format_manifest(&plan.manifest))?;
    Ok((plan.copy.len(), linked))
}

/// Copies `from` over `to` when `to` is missing or older (by size and
/// modification time). Returns true when a copy was made. Used to keep the
/// player's profile (`saves` setting) and the runtime's `CMR3` file in
/// step, in either direction.
pub fn mirror_file(from: &Path, to: &Path) -> io::Result<bool> {
    let Ok(meta) = fs::metadata(from) else {
        return Ok(false);
    };
    let stamp = Stamp {
        size: meta.len(),
        mtime: mtime_secs(&meta),
    };
    if let Ok(dst) = fs::metadata(to)
        && dst.len() == stamp.size
        && mtime_secs(&dst) == stamp.mtime
    {
        return Ok(false);
    }
    install_copy(from, to)?;
    let file = fs::OpenOptions::new().write(true).open(to)?;
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.mtime))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(size: u64, mtime: u64) -> Stamp {
        Stamp { size, mtime }
    }

    fn listing(items: &[(&str, Stamp)]) -> Listing {
        items.iter().map(|(r, s)| (r.to_string(), *s)).collect()
    }

    #[test]
    fn first_sync_bring_everything_over() {
        let src = listing(&[
            ("Rally_3PC.exe", stamp(1, 1)),
            ("Data/Tracks/a.dat", stamp(2, 2)),
            ("CMR3", stamp(3, 3)),
        ]);
        let p = plan(&src, &|_| false, &Listing::new());
        assert_eq!(p.copy, ["CMR3", "Data/Tracks/a.dat", "Rally_3PC.exe"]);
        assert_eq!(p.manifest, src);
    }

    #[test]
    fn unchanged_source_bring_nothing() {
        let src = listing(&[("Rally_3PC.exe", stamp(1, 1))]);
        let p = plan(&src, &|_| true, &src);
        assert!(p.copy.is_empty());
    }

    #[test]
    fn changed_data_is_brought_again_but_the_profile_is_kept() {
        let old = listing(&[("Data/a.dat", stamp(1, 1)), ("CMR3", stamp(1, 1))]);
        let new = listing(&[("Data/a.dat", stamp(1, 9)), ("CMR3", stamp(5, 9))]);
        let p = plan(&new, &|_| true, &old);
        assert_eq!(p.copy, ["Data/a.dat"]);
        assert_eq!(p.manifest["CMR3"], stamp(1, 1));
    }

    #[test]
    fn static_and_written_files_are_classified() {
        assert!(is_static("Data/Tracks/x"));
        assert!(is_static("fonts/hel_18pt.dds"));
        assert!(is_static("shaders/water.pso"));
        assert!(is_static("MathCPU.dll"));
        assert!(is_static("Rally_3PC.exe"));
        assert!(is_static("PATCHINFO.BIN"));
        assert!(!is_static("CMR3"));
        assert!(!is_static("SilentPatchCMR3.ini"));
        assert!(!is_static("unins000.exe.log"));
        assert!(is_game_written("CMR3"));
        assert!(is_game_written("cmr3"));
        assert!(!is_game_written("CMR3.bak"));
    }

    fn temp(name: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("cmr3-launcher-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn sync_links_data_and_keeps_the_original_safe() {
        let game = temp("game");
        let rt = temp("rt");
        fs::create_dir_all(game.join("Data/Tracks")).unwrap();
        fs::write(game.join("Rally_3PC.exe"), b"exe").unwrap();
        fs::write(game.join("Data/Tracks/a.dat"), b"data").unwrap();
        fs::write(game.join("CMR3"), b"profile").unwrap();

        let (brought, linked) = sync(&game, &rt).unwrap();
        assert_eq!(brought, 3);
        assert_eq!(linked, 2, "the exe and the track file, not the profile");
        assert_eq!(fs::read(rt.join("CMR3")).unwrap(), b"profile");
        assert_eq!(sync(&game, &rt).unwrap(), (0, 0));

        // Installing over a linked file must not change the original.
        install(&rt.join("Data/Tracks/a.dat"), b"replaced").unwrap();
        assert_eq!(fs::read(rt.join("Data/Tracks/a.dat")).unwrap(), b"replaced");
        assert_eq!(
            fs::read(game.join("Data/Tracks/a.dat")).unwrap(),
            b"data",
            "the user's folder is never written"
        );

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
        assert_eq!(sync(&game, &rt).unwrap().0, 1);
        assert_eq!(fs::read(rt.join("CMR3")).unwrap(), b"played");

        for d in [game, rt] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn mirror_file_copies_only_what_changed() {
        let a = temp("mirror-a");
        let b = temp("mirror-b");
        fs::create_dir_all(&a).unwrap();
        fs::write(a.join("CMR3"), b"one").unwrap();
        assert!(mirror_file(&a.join("CMR3"), &b.join("CMR3")).unwrap());
        assert!(!mirror_file(&a.join("CMR3"), &b.join("CMR3")).unwrap());
        assert!(!mirror_file(&a.join("missing"), &b.join("CMR3")).unwrap());
        fs::write(a.join("CMR3"), b"two!").unwrap();
        assert!(mirror_file(&a.join("CMR3"), &b.join("CMR3")).unwrap());
        assert_eq!(fs::read(b.join("CMR3")).unwrap(), b"two!");
        for d in [a, b] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn overlapping_folders_are_refused() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(check_separate(&game, &game.join("rt")).is_err());
        assert!(check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn manifest_round_trips() {
        let l = listing(&[("Data/a b.dat", stamp(10, 20))]);
        assert_eq!(parse_manifest(&format_manifest(&l)), l);
    }
}
