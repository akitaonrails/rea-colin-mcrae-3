//! `--check`'s report: what it says about the game folder, its executable
//! and the bundled parts, and whether that is ready to play. The binary
//! gathers the facts (reading files, trying the DRM-free patch on a
//! scratch copy); this module turns them into lines and the verdict.

use crate::edition::{self, Edition};

/// What the executable is, with what the bundled patcher made of it when
/// it is not a known DRM-free build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exe {
    /// A known DRM-free build.
    DrmFree(&'static str),
    /// Not DRM-free; the bundled patcher turned a scratch copy into a
    /// known DRM-free build with this patch set (`efigs`, `polish`):
    /// the launcher converts it at Play.
    ConvertsAtPlay(&'static str),
    /// Not DRM-free, and the bundled patcher refused it (or gave an
    /// unknown result): refused at Play.
    Refused,
    /// Not DRM-free, and no patcher is bundled to try it.
    NoPatcher,
    /// The patcher could not be run (the reason).
    PatcherFailed(String),
}

impl Exe {
    /// From the executable's [`Edition`], trying the patcher (`try_patch`:
    /// `None` when it isn't bundled, else the patch set that converted it
    /// or why it couldn't run) only when needed.
    pub fn from_edition(
        edition: Edition,
        try_patch: impl FnOnce() -> Option<Result<Option<&'static str>, String>>,
    ) -> Exe {
        match edition {
            Edition::DrmFree(name) => Exe::DrmFree(name),
            Edition::NeedsPatching => match try_patch() {
                None => Exe::NoPatcher,
                Some(Ok(Some(variant))) => Exe::ConvertsAtPlay(variant),
                Some(Ok(None)) => Exe::Refused,
                Some(Err(e)) => Exe::PatcherFailed(e),
            },
        }
    }

    fn ok(&self) -> bool {
        matches!(self, Exe::DrmFree(_) | Exe::ConvertsAtPlay(_))
    }

    fn line(&self) -> String {
        match self {
            Exe::DrmFree(name) => (*name).to_string(),
            Exe::ConvertsAtPlay(variant) => format!(
                "official 1.1 with disc protection; converted at Play to the publisher's \
                 DRM-free 1.1 (patch set {variant})"
            ),
            Exe::Refused => format!(
                "not a version the launcher knows, and the DRM-free patcher refuses it \
                 (an unofficial no-CD, or still v1.0?). Install the official 1.1 patch. \
                 Report: {}",
                edition::ISSUE_URL
            ),
            Exe::NoPatcher => format!(
                "{} The DRM-free patcher is not bundled, so it cannot be converted. \
                 Report: {}",
                edition::UNKNOWN_NOTE,
                edition::ISSUE_URL
            ),
            Exe::PatcherFailed(e) => {
                format!("not a known DRM-free build, and the DRM-free patcher could not run ({e})")
            }
        }
    }
}

/// One bundled part and whether it is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub name: &'static str,
    pub there: bool,
    /// Play works without it (the HD UI is an extra).
    pub optional: bool,
}

/// The executable as `--check` found it.
pub type Found = Result<(Exe, String), String>;

/// The game folder holds a known installer instead of an installed game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unpack {
    /// Play unpacks it, once, into `dest`.
    Pending {
        /// The setup program's file name, the installer's name and the
        /// setup program's SHA-256.
        setup: String,
        name: &'static str,
        sha256: String,
        dest: String,
        /// Bytes the unpack needs free (`installer::space_needed`), and
        /// the free bytes there (`None`: the system can't say).
        need: u64,
        free: Option<u64>,
    },
    /// Unpacked already: Play uses `dest` (the executable line is about
    /// the game there).
    Done {
        setup: String,
        name: &'static str,
        dest: String,
    },
}

impl Unpack {
    /// Whether Play still has to run innoextract.
    pub fn pending(&self) -> bool {
        matches!(self, Unpack::Pending { .. })
    }

    fn lines(&self) -> (Vec<String>, bool) {
        let gb = |bytes: u64| format!("{:.1} GB", bytes as f64 / 1e9);
        match self {
            Unpack::Pending {
                setup,
                name,
                sha256,
                dest,
                need,
                free,
            } => {
                let mut lines = vec![
                    format!("Installer: {setup}, {name}"),
                    format!("SHA-256: {sha256}"),
                    format!(
                        "Play unpacks it once into {dest}, then plays from there; the installer \
                         is never run and its files are only read."
                    ),
                ];
                let ok = free.is_none_or(|free| free >= *need);
                lines.push(match free {
                    Some(free) if !ok => format!(
                        "Free space: {} needed in {dest}, only {} free. Free some space, or \
                         move the launcher's copy (runtime) to another drive.",
                        gb(*need),
                        gb(*free)
                    ),
                    Some(free) => format!("Free space: {} needed, {} free.", gb(*need), gb(*free)),
                    None => format!(
                        "Free space: {} needed (the free space is unknown).",
                        gb(*need)
                    ),
                });
                (lines, ok)
            }
            Unpack::Done { setup, name, dest } => (
                vec![
                    format!("Installer: {setup}, {name}"),
                    format!(
                        "Already unpacked into {dest}; Play uses that. The installer files are \
                         no longer needed."
                    ),
                ],
                true,
            ),
        }
    }
}

/// The report's lines and whether everything needed is there. `game`:
/// the game folder's line, or why there is none; `unpack`: the installer
/// found there instead of an installed game; `exe`: the executable and its
/// SHA-256, or why it could not be read (`None` without a game folder, or
/// before an installer is unpacked).
pub fn report(
    game: Result<String, String>,
    unpack: Option<&Unpack>,
    exe: Option<Found>,
    parts: &[Part],
) -> (Vec<String>, bool) {
    let mut lines = Vec::new();
    let mut ok = true;
    match game {
        Ok(folder) => lines.push(format!("Game folder: {folder}")),
        Err(e) => {
            lines.push(e);
            ok = false;
        }
    }
    if let Some(unpack) = unpack {
        let (more, fine) = unpack.lines();
        lines.extend(more);
        ok &= fine;
    }
    match exe {
        Some(Ok((exe, sha))) => {
            lines.push(format!("Rally_3PC.exe: {}", exe.line()));
            lines.push(format!("SHA-256: {sha}"));
            ok &= exe.ok();
        }
        Some(Err(e)) => {
            lines.push(format!("Rally_3PC.exe: cannot read it ({e})"));
            ok = false;
        }
        None => {}
    }
    lines.push("Bundled parts:".into());
    for part in parts {
        let state = match (part.there, part.optional) {
            (true, _) => "yes",
            (false, false) => "missing",
            (false, true) => "missing (optional)",
        };
        lines.push(format!("  {}: {state}", part.name));
        ok &= part.there || part.optional;
    }
    lines.push(if ok {
        "Ready to play.".into()
    } else {
        "Not ready: see above.".into()
    });
    (lines, ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(missing: &[&str]) -> Vec<Part> {
        crate::components::PARTS
            .iter()
            .map(|&(name, _, optional)| Part {
                name,
                there: !missing.contains(&name),
                optional,
            })
            .collect()
    }

    fn found(exe: Exe) -> Option<Found> {
        Some(Ok((exe, "ab12".into())))
    }

    #[test]
    fn an_official_1_1_the_patcher_converts_is_ready() {
        let exe = Exe::from_edition(Edition::NeedsPatching, || Some(Ok(Some("efigs"))));
        assert_eq!(exe, Exe::ConvertsAtPlay("efigs"));
        let (lines, ok) = report(Ok("C:\\CMR3".into()), None, found(exe), &parts(&[]));
        assert!(ok, "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("converted at Play")),
            "{lines:?}"
        );
        assert_eq!(lines.last().unwrap(), "Ready to play.");
    }

    #[test]
    fn a_build_the_patcher_refuses_is_not_ready() {
        let refused = Exe::from_edition(Edition::NeedsPatching, || Some(Ok(None)));
        assert_eq!(refused, Exe::Refused);
        let none = Exe::from_edition(Edition::NeedsPatching, || None);
        let failed = Exe::from_edition(Edition::NeedsPatching, || Some(Err("gone".into())));
        for exe in [refused, none, failed] {
            let (lines, ok) = report(Ok("C:\\CMR3".into()), None, found(exe.clone()), &parts(&[]));
            assert!(!ok, "{exe:?}");
            assert!(lines.iter().any(|l| l.contains("Rally_3PC.exe: ")));
        }
    }

    #[test]
    fn a_drm_free_build_is_never_patched_by_check() {
        let exe = Exe::from_edition(Edition::DrmFree("DRM-free v1.1"), || {
            panic!("the patcher must not run for a DRM-free build")
        });
        assert_eq!(exe, Exe::DrmFree("DRM-free v1.1"));
    }

    #[test]
    fn only_required_parts_count() {
        let exe = || found(Exe::DrmFree("DRM-free v1.1"));
        let (lines, ok) = report(Ok("C:\\CMR3".into()), None, exe(), &parts(&["HD UI"]));
        assert!(ok, "{lines:?}");
        assert!(lines.contains(&"  HD UI: missing (optional)".to_string()));
        let (lines, ok) = report(Ok("C:\\CMR3".into()), None, exe(), &parts(&["SilentPatch"]));
        assert!(!ok, "{lines:?}");
        assert!(lines.contains(&"  SilentPatch: missing".to_string()));
    }

    #[test]
    fn no_game_folder_is_not_ready() {
        let (lines, ok) = report(
            Err("Tell the launcher where".into()),
            None,
            None,
            &parts(&[]),
        );
        assert!(!ok);
        assert_eq!(lines[0], "Tell the launcher where");
    }

    fn pending(free: Option<u64>) -> Unpack {
        Unpack::Pending {
            setup: "Colin McRae Rally 3_Repack_v4_Setup.exe".into(),
            name: "the Magipack repack (v4)",
            sha256: "0966e3".into(),
            dest: r"C:\L\Rally3-game".into(),
            need: 3_738_171_860,
            free,
        }
    }

    fn innoextract(there: bool, needed: bool) -> Vec<Part> {
        let mut all = parts(&[]);
        all.push(crate::components::innoextract_part(there, needed));
        all
    }

    #[test]
    fn an_installer_says_what_play_does_and_the_space_it_needs() {
        let unpack = pending(Some(100_000_000_000));
        let (lines, ok) = report(
            Ok(r"D:\Repack".into()),
            Some(&unpack),
            None,
            &innoextract(true, unpack.pending()),
        );
        assert!(ok, "{lines:?}");
        assert_eq!(
            lines[1],
            "Installer: Colin McRae Rally 3_Repack_v4_Setup.exe, the Magipack repack (v4)"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains(r"Play unpacks it once into C:\L\Rally3-game"))
        );
        assert!(lines.iter().any(|l| l.contains("never run")));
        assert!(lines.contains(&"Free space: 3.7 GB needed, 100.0 GB free.".to_string()));
        assert!(lines.contains(&"  innoextract (unpacks the installer): yes".to_string()));
        assert_eq!(lines.last().unwrap(), "Ready to play.");
    }

    #[test]
    fn an_installer_without_the_space_or_innoextract_is_not_ready() {
        let (lines, ok) = report(
            Ok(r"D:\Repack".into()),
            Some(&pending(Some(1_000_000_000))),
            None,
            &innoextract(true, true),
        );
        assert!(!ok, "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("only 1.0 GB free")),
            "{lines:?}"
        );
        let (lines, ok) = report(
            Ok(r"D:\Repack".into()),
            Some(&pending(None)),
            None,
            &innoextract(false, true),
        );
        assert!(!ok, "{lines:?}");
        assert!(lines.contains(&"  innoextract (unpacks the installer): missing".to_string()));
        // An installed game doesn't need it.
        let (lines, ok) = report(
            Ok(r"C:\CMR3".into()),
            None,
            found(Exe::DrmFree("DRM-free v1.1")),
            &innoextract(false, false),
        );
        assert!(ok, "{lines:?}");
    }

    #[test]
    fn an_unpacked_installer_reports_the_game_there() {
        let done = Unpack::Done {
            setup: "Colin McRae Rally 3_Repack_v4_Setup.exe".into(),
            name: "the Magipack repack (v4)",
            dest: r"C:\L\Rally3-game".into(),
        };
        assert!(!done.pending());
        let (lines, ok) = report(
            Ok(r"D:\Repack".into()),
            Some(&done),
            found(Exe::DrmFree("DRM-free v1.1")),
            &innoextract(false, false),
        );
        assert!(ok, "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with(r"Already unpacked into C:\L\Rally3-game"))
        );
        assert!(lines.contains(&"Rally_3PC.exe: DRM-free v1.1".to_string()));
    }
}
