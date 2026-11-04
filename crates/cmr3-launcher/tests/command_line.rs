//! The built `CMR3Launcher.exe` on a bad command line (Windows target,
//! `sdk/scripts/wine-test`): it prints the error and the usage and exits
//! 2 (`report::BAD_COMMAND_LINE`), the family's code for a bad command
//! line (Windows 10 test, rea-launcher-sdk#1).
//!
//! Under Wine stdout is never a terminal, so any run a person may watch
//! (`--no-window` included) asks for a dialog, which nobody answers in a
//! headless run. `--linux` (cmr3-play reads the exit code itself) never
//! does; the routing from a Windows terminal is tested as a pure function
//! in `audience`. Each run is also bounded: a hang fails the test instead
//! of blocking it.
#![cfg(windows)]

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(60);

/// Runs the launcher with `args`, its home folder in a scratch folder of
/// its own (never the prefix's real `%LOCALAPPDATA%\CMR3Launcher`): its
/// exit code and stdout. Ends the run (this test's own child) after
/// [`LIMIT`].
fn launcher(name: &str, args: &[&str]) -> (Option<i32>, String) {
    let home = std::env::temp_dir().join(format!("cmr3-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mut child = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_CMR3Launcher")))
        .args(args)
        .env("LOCALAPPDATA", &home)
        .env("USERPROFILE", &home)
        .env_remove("CMR3_GAME")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the launcher starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&home);
            panic!("{args:?}: still running after {LIMIT:?} (a dialog?)");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    (status.code(), out)
}

#[test]
fn a_bad_command_line_prints_the_usage_and_exits_2() {
    for (name, args) in [
        ("bogus", &["--bogus-flag", "--linux"][..]),
        ("look", &["--look", "invalid", "--linux"]),
        ("check", &["--check", "--vsync", "maybe", "--linux"]),
    ] {
        let (code, text) = launcher(name, args);
        assert_eq!(code, Some(2), "{args:?}: {text}");
        assert!(
            text.contains("CMR3Launcher [options]"),
            "{args:?}: the usage, got {text}"
        );
    }
}
