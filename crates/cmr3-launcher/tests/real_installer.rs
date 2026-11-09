//! The owner's real installer, unpacked through the launcher's own code
//! (`source::find`, then `unpack::unpack`, which Play runs) with no
//! window, under `sdk/scripts/wine-test`. Skipped unless the environment
//! names everything; the installer folder is only read, and the game is
//! unpacked beside `CMR3_REAL_RUNTIME` (its `-game` sibling):
//!
//! ```sh
//! CMR3_REAL_INSTALLER='Z:\path\to\repack' \
//! CMR3_REAL_RUNTIME='Z:\repo\.work\installer\home\Rally3' \
//! CMR3_REAL_COMPONENTS='Z:\repo\.work\installer\pkg\components' \
//!   sdk/scripts/wine-test --locked --test real_installer -- --nocapture
//! ```
//!
//! Then compare the result with an installed copy (`docs/RUNTIME_NOTES.md`).
#![cfg(windows)]

use std::path::PathBuf;
use std::time::Instant;

use cmr3_launcher::source::{self, Found};
use cmr3_launcher::unpack;

fn var(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

#[test]
fn unpacks_the_real_repack_once() {
    let (Some(dir), Some(runtime), Some(components)) = (
        var("CMR3_REAL_INSTALLER"),
        var("CMR3_REAL_RUNTIME"),
        var("CMR3_REAL_COMPONENTS"),
    ) else {
        println!("skipped: CMR3_REAL_INSTALLER, CMR3_REAL_RUNTIME, CMR3_REAL_COMPONENTS unset");
        return;
    };
    let Found::Installer(found) = source::find(&dir).expect("the folder is readable") else {
        panic!("{} holds no installer the launcher knows", dir.display());
    };
    println!(
        "found {} ({})",
        found.build.name,
        source::sha256_hex(&found)
    );
    let mut log = |line: &str| println!("{line}");
    let started = Instant::now();
    let dest = unpack::unpack(&found, &dir, &runtime, &components, &mut log, |_| {
        Err("no window in this test".to_string())
    })
    .expect("the unpack succeeds")
    .expect("nobody cancels it");
    println!("unpacked in {:.0} s", started.elapsed().as_secs_f64());
    assert_eq!(dest, source::unpacked_dir(&runtime));
    assert_eq!(
        source::check_game(&dest).unwrap(),
        "DRM-free v1.1 (variant B, also the community repack)"
    );
    for dropped in found.build.drop {
        assert!(!dest.join(dropped).exists(), "{dropped} was not dropped");
    }
    assert!(source::is_unpacked_by_launcher(&dest));
    // A second Play reuses it: no innoextract, no window.
    let started = Instant::now();
    let again = unpack::unpack(&found, &dir, &runtime, &components, &mut log, |_| {
        panic!("a second unpack must not run")
    })
    .unwrap()
    .unwrap();
    assert_eq!(again, dest);
    println!("reused in {:.1} s", started.elapsed().as_secs_f64());
}
