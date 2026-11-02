//! Identifies the user's `Rally_3PC.exe` and what the launcher must do to
//! make it run everywhere.
//!
//! Colin McRae Rally 3 shipped with SecuROM or SafeDisc, both now disabled
//! by Windows and unsupported under Wine. CookiePLMonster's DRM-free
//! patcher (bundled, MIT) converts an official 1.1 executable into the
//! identical DRM-free one the publisher's later re-releases used (owner
//! decision of 2026-11-02, recorded in AGENTS.md and THIRD_PARTY.md), so
//! the launcher checks for those first and applies the patch only when
//! needed. Builds are identified by SHA-256 where a copy was measured and
//! by the patcher's own MD5 list otherwise; anything else the patcher
//! refuses is refused with its SHA-256 and the issue link ([`refusal`]).

use md5::Digest as _;
use md5::Md5;
use rea_launcher::identify::{self, Known};

/// Where players report a build the launcher doesn't know (the
/// unknown-edition issue template asks for the SHA-256, where the copy
/// comes from and `launcher.log`).
pub const ISSUE_URL: &str =
    "https://github.com/akitaonrails/rea-colin-mcrae-3/issues/new?template=unknown-edition.yml";

/// What the executable is, and how to make it run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Already DRM-free (a late re-release, or the patcher's output).
    DrmFree(&'static str),
    /// Not one of the known DRM-free builds: the bundled xdelta patch
    /// decides — it succeeds on official 1.1 executables with DRM and on
    /// nothing else (xdelta checks its source). Anything it refuses is
    /// refused with [`refusal`].
    NeedsPatching,
}

/// The DRM-free 1.1 executables by SHA-256, measured on real copies.
static DRM_FREE_SHA256: [Known<&str>; 1] = [Known::new(
    "c56344b4c2a08844cbe8d80cdbcc7f3c22ad1f5eb515474dba7845268812811c",
    "DRM-free v1.1 (variant B, also the community repack)",
)];

/// The same builds by MD5, from the DRM-free patcher's setup.bat: the
/// cross-reference for the variants no copy here has been measured from
/// (A and C).
static DRM_FREE: [([u8; 16], &str); 3] = [
    (
        hex16("9f5bb2b0983b49cbb4fc6f45d22afd75"),
        "DRM-free v1.1 (variant A)",
    ),
    (
        hex16("2744404bfabf1e64ea365ae9b9b89ef6"),
        "DRM-free v1.1 (variant B, also the community repack)",
    ),
    (
        hex16("8bcc13512a783f8fcbbeef90fe6e977d"),
        "DRM-free v1.1 (variant C)",
    ),
];

pub fn md5(bytes: &[u8]) -> [u8; 16] {
    Md5::digest(bytes).into()
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("lowercase hex expected"),
    }
}

const fn hex16(s: &str) -> [u8; 16] {
    let b = s.as_bytes();
    assert!(b.len() == 32);
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = (nibble(b[2 * i]) << 4) | nibble(b[2 * i + 1]);
        i += 1;
    }
    out
}

/// The edition of these executable bytes: SHA-256 first, then the
/// patcher's MD5 list.
pub fn identify(exe: &[u8]) -> Edition {
    if let Ok(name) = identify::identify(exe, &DRM_FREE_SHA256) {
        return Edition::DrmFree(name);
    }
    let digest = md5(exe);
    match DRM_FREE.iter().find(|(known, _)| *known == digest) {
        Some((_, name)) => Edition::DrmFree(name),
        None => Edition::NeedsPatching,
    }
}

/// The SHA-256 of an executable, as hex (for the log and reports).
pub fn sha256_hex(exe: &[u8]) -> String {
    identify::hex(&identify::sha256(exe))
}

/// What the window's note and `--check` say about an executable that
/// is not a known DRM-free build, before the patcher has tried it.
pub const UNKNOWN_NOTE: &str = "Not a known DRM-free build. If it is the official 1.1 update \
     with its disc protection, the launcher converts it to the publisher's DRM-free 1.1; any \
     other build is refused at Play with its SHA-256 and a link to report it.";

/// The message for a build the DRM-free patcher refused: never patched
/// by guesswork, never run with its disc protection.
pub fn refusal(sha256_hex: &str) -> String {
    format!(
        "This Rally_3PC.exe is not a version the launcher knows \
         (SHA-256 {sha256_hex}).\n\n\
         The launcher runs the DRM-free 1.1 executables and converts the official 1.1 \
         update; an unpatched 1.0 or a no-CD executable is not one of them. Install the \
         official 1.1 patch over the game (see the README), then try again.\n\n\
         If your copy is a 1.1 build anyway, please report it with launcher.log:\n{ISSUE_URL}"
    )
}

/// True when these bytes are one of the known DRM-free executables; the
/// patched runtime copy is checked with it to skip repeated patching.
pub fn is_drm_free(exe: &[u8]) -> bool {
    matches!(identify(exe), Edition::DrmFree(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::identify::hex as to_hex;

    #[test]
    fn hex_round_trips() {
        let h = "9f5bb2b0983b49cbb4fc6f45d22afd75";
        assert_eq!(to_hex(&hex16(h)), h);
    }

    #[test]
    fn known_hashes_are_distinct_and_md5_is_wired_up() {
        for (i, (a, _)) in DRM_FREE.iter().enumerate() {
            for (b, _) in &DRM_FREE[i + 1..] {
                assert_ne!(a, b);
            }
        }
        // MD5 of the empty input, a fixed vector.
        assert_eq!(to_hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn unknown_executables_need_patching() {
        assert_eq!(identify(b"not a game"), Edition::NeedsPatching);
        assert!(!is_drm_free(b"not a game"));
    }

    #[test]
    fn a_refusal_names_the_digest_and_where_to_report() {
        let digest = sha256_hex(b"not a game");
        let text = refusal(&digest);
        assert!(text.contains(&digest));
        assert!(text.contains(ISSUE_URL));
        assert!(UNKNOWN_NOTE.contains("SHA-256"));
    }

    #[test]
    fn the_sha256_and_md5_tables_agree() {
        // Every SHA-256 entry names a build of the MD5 list.
        for known in &DRM_FREE_SHA256 {
            assert!(DRM_FREE.iter().any(|(_, name)| *name == known.value));
        }
    }
}
