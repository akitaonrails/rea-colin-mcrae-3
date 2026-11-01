//! Identifies the user's `Rally_3PC.exe` and what the launcher must do to
//! make it run everywhere.
//!
//! Colin McRae Rally 3 shipped with SecuROM or SafeDisc, both now disabled
//! by Windows and unsupported under Wine. CookiePLMonster's DRM-free
//! patcher (bundled, MIT) converts an official 1.1 executable into the
//! identical DRM-free one the later re-releases used, so the launcher
//! checks for those first and applies the patch only when needed.

use md5::Digest as _;
use md5::Md5;
use sha2::Sha256;

/// What the executable is, and how to make it run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Already DRM-free (a late re-release, or the patcher's output).
    /// The names match the known MD5s in the DRM-free patcher's setup.bat.
    DrmFree(&'static str),
    /// Not one of the known DRM-free builds: the bundled xdelta patch
    /// decides — it succeeds on official 1.1 executables with DRM and on
    /// nothing else.
    NeedsPatching,
}

/// The DRM-free 1.1 executables (MD5, from the DRM-free patcher).
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

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn to_hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
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

pub fn identify(exe: &[u8]) -> Edition {
    let digest = md5(exe);
    match DRM_FREE.iter().find(|(known, _)| *known == digest) {
        Some((_, name)) => Edition::DrmFree(name),
        None => Edition::NeedsPatching,
    }
}

/// True when these bytes are one of the known DRM-free executables; the
/// patched runtime copy is checked with it to skip repeated patching.
pub fn is_drm_free(exe: &[u8]) -> bool {
    matches!(identify(exe), Edition::DrmFree(_))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
