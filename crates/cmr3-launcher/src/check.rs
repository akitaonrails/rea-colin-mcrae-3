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

/// The report's lines and whether everything needed is there. `game`:
/// the game folder's line, or why there is none; `exe`: the executable
/// and its SHA-256, or why it could not be read (`None` without a game
/// folder).
pub fn report(
    game: Result<String, String>,
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
        let (lines, ok) = report(Ok("C:\\CMR3".into()), found(exe), &parts(&[]));
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
            let (lines, ok) = report(Ok("C:\\CMR3".into()), found(exe.clone()), &parts(&[]));
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
        let (lines, ok) = report(Ok("C:\\CMR3".into()), exe(), &parts(&["HD UI"]));
        assert!(ok, "{lines:?}");
        assert!(lines.contains(&"  HD UI: missing (optional)".to_string()));
        let (lines, ok) = report(Ok("C:\\CMR3".into()), exe(), &parts(&["SilentPatch"]));
        assert!(!ok, "{lines:?}");
        assert!(lines.contains(&"  SilentPatch: missing".to_string()));
    }

    #[test]
    fn no_game_folder_is_not_ready() {
        let (lines, ok) = report(Err("Tell the launcher where".into()), None, &parts(&[]));
        assert!(!ok);
        assert_eq!(lines[0], "Tell the launcher where");
    }
}
