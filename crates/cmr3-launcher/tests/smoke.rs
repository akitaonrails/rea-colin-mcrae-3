//! Opt-in smoke tests against real data on this machine. Skipped unless
//! the environment names it:
//!
//! ```sh
//! CMR3_SMOKE_GAME=/path/to/game CMR3_SMOKE_BUNDLE=/path/to/thirdparty \
//!   cargo test --test smoke -- --ignored --nocapture
//! ```

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn game() -> PathBuf {
    std::env::var("CMR3_SMOKE_GAME")
        .expect("CMR3_SMOKE_GAME names the game folder")
        .into()
}

fn bundle() -> PathBuf {
    std::env::var("CMR3_SMOKE_BUNDLE")
        .expect("CMR3_SMOKE_BUNDLE names the thirdparty folder")
        .into()
}

#[test]
#[ignore]
fn identifies_the_real_executable() {
    let exe = std::fs::read(game().join("Rally_3PC.exe")).unwrap();
    println!("{:?}", cmr3_launcher::edition::identify(&exe));
    println!(
        "sha256 {}",
        cmr3_launcher::edition::to_hex(&cmr3_launcher::edition::sha256(&exe))
    );
}

#[test]
#[ignore]
fn syncs_a_runtime_and_installs_everything() {
    let game = game();
    let bundle = bundle();
    // First on a different volume (the copy path), then next to the game
    // (the hard-link path). Both stay outside the game folder.
    let rt = std::env::temp_dir().join(format!("cmr3-smoke-{}", std::process::id()));
    let rt_link = game
        .parent()
        .unwrap_or(&game)
        .join(format!(".cmr3-smoke-{}", std::process::id()));
    for dir in [&rt, &rt_link] {
        let _ = std::fs::remove_dir_all(dir);
    }
    // The invariant that matters: every file in the game folder keeps its
    // exact bytes (it may already hold an older Xidi.ini or loader from a
    // previous setup).
    let snapshot: Vec<_> = cmr3_launcher::runtime::list(&game)
        .unwrap()
        .into_iter()
        .map(|(rel, stamp)| {
            let bytes = std::fs::read(cmr3_launcher::runtime::native(&game, &rel)).unwrap();
            (rel, stamp, cmr3_launcher::edition::md5(&bytes))
        })
        .collect();

    let (brought, linked) = cmr3_launcher::runtime::sync(&game, &rt).unwrap();
    println!("copy run: brought {brought} files ({linked} linked)");
    assert!(brought > 1000, "a full game");
    assert_eq!(linked, 0, "across volumes everything is copied");
    assert!(!game.join(cmr3_launcher::runtime::MANIFEST).is_file());

    let (brought, linked) = cmr3_launcher::runtime::sync(&game, &rt_link).unwrap();
    println!("link run: brought {brought} files ({linked} linked)");
    assert!(linked > 1000, "on one volume the data trees hard-link");
    assert!(brought > 1000);

    let rt = rt_link;
    assert!(cmr3_launcher::components::install_silentpatch(&rt, &bundle).unwrap());
    assert!(
        cmr3_launcher::components::install_xidi(
            &rt,
            &bundle,
            cmr3_launcher::config::Gamepad::Rally,
            false
        )
        .unwrap()
    );
    assert!(cmr3_launcher::components::install_dxvk(&rt, &bundle, true).unwrap());
    let hd = cmr3_launcher::components::install_hd_ui(&rt, &bundle).unwrap();
    println!("hd ui: {hd} files");
    assert!(hd > 20);
    let settings = cmr3_launcher::config::Settings {
        resolution: Some((3840, 2160)),
        vsync: Some(true),
        anisotropic: Some(16),
        fov: Some(75),
        ..Default::default()
    };
    let log =
        cmr3_launcher::components::apply_settings(&rt, &settings, Some((3840, 2160))).unwrap();
    println!("ini: {log:?}");
    let ini = std::fs::read_to_string(rt.join("SilentPatchCMR3.ini")).unwrap();
    assert!(ini.contains("WIDTH=3840"));
    for name in [
        "SilentPatchCMR3.asi",
        "dsound.dll",
        "dinput8.dll",
        "Xidi.ini",
        "d3d9.dll",
    ] {
        assert!(rt.join(name).is_file(), "{name} installed");
    }
    // The user's folder never changed, byte for byte.
    let after = cmr3_launcher::runtime::list(&game).unwrap();
    assert_eq!(
        after.len(),
        snapshot.len(),
        "no file appeared in or vanished from the game folder"
    );
    for (rel, stamp, digest) in &snapshot {
        let now_stamp = after.get(rel).expect("still there");
        assert_eq!(now_stamp, stamp, "{rel} untouched");
        let bytes = std::fs::read(cmr3_launcher::runtime::native(&game, rel)).unwrap();
        assert_eq!(
            &cmr3_launcher::edition::md5(&bytes),
            digest,
            "{rel} unchanged"
        );
    }

    // A second run brings nothing over and stays fast.
    let (again, _) = cmr3_launcher::runtime::sync(&game, &rt).unwrap();
    assert_eq!(again, 0);
    std::fs::remove_dir_all(&rt).unwrap();
    std::fs::remove_dir_all(
        std::env::temp_dir().join(format!("cmr3-smoke-{}", std::process::id())),
    )
    .unwrap();
}

/// The bundle the package builds: verify the checksums the way CI does,
/// then that the patcher runs on this host (it is a Windows exe: skipped
/// unless wine is on the PATH or CMR3_SMOKE_WINE names one).
#[test]
#[ignore]
fn bundle_layout_is_complete() {
    let bundle = bundle();
    for rel in [
        "silentpatch/SilentPatchCMR3.asi",
        "silentpatch/dinput8.dll",
        "xidi/dinput8.dll",
        "patcher/xdelta.exe",
        "patcher/efigs.xdelta",
        "patcher/polish.xdelta",
        "dxvk/d3d9.dll",
    ] {
        assert!(bundle.join(rel).is_file(), "{rel} missing");
    }
    assert!(bundle.join("hd-ui").is_dir());
    assert_eq!(
        cmr3_launcher::edition::to_hex(&file_md5(&bundle.join("xidi/dinput8.dll"))),
        // The Xidi v4.3.1 Win32 build this setup was validated with.
        "ac44d28829d1aa81d1dec4246e2139c4"
    );

    let wine = std::env::var("CMR3_SMOKE_WINE").unwrap_or_else(|_| "wine".into());
    if Command::new(&wine).arg("--version").output().is_err() {
        println!("no wine ({wine}); skipping the patcher run");
        return;
    }
    // xdelta on a nonsense source must refuse (exit != 0): the patcher's
    // version check.
    let tmp = std::env::temp_dir().join("cmr3-smoke-xd");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("src.exe"), b"not a game").unwrap();
    let out = Command::new(&wine)
        .arg(bundle.join("patcher/xdelta.exe"))
        .args([
            "-d",
            "-s",
            tmp.join("src.exe").as_os_str().to_string_lossy().as_ref(),
            bundle
                .join("patcher/efigs.xdelta")
                .as_os_str()
                .to_string_lossy()
                .as_ref(),
            tmp.join("out.exe").as_os_str().to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a mismatched source is refused");
    assert!(!tmp.join("out.exe").is_file());
    let _ = std::fs::remove_dir_all(&tmp);
}

fn file_md5(path: &Path) -> [u8; 16] {
    cmr3_launcher::edition::md5(&std::fs::read(path).unwrap())
}
