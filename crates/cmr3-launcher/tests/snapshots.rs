//! Golden snapshots of everything the launcher produces (SDK STANDARDS,
//! "Golden snapshots"): `--help`, the `launcher.cfg` template, and for each
//! look and a few option combinations the launch plan — components,
//! `SilentPatchCMR3.ini` bytes, `Xidi.ini`, `WINEDLLOVERRIDES`, game
//! arguments, patch sets and the Linux helper's values. No game bytes:
//! only what the launcher generates.
//!
//! A diff is reviewed like code. To accept an intended change:
//! `CMR3_UPDATE_SNAPSHOTS=1 cargo test --test snapshots`, then read the diff.

use std::path::PathBuf;

use cmr3_launcher::config::{self, Settings};
use cmr3_launcher::form::{Form, Look};
use cmr3_launcher::plan::{self, Env};

fn check(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("CMR3_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e} (CMR3_UPDATE_SNAPSHOTS=1 writes it)",
            path.display()
        )
    });
    // Compared without regard to line endings (a Windows checkout).
    assert_eq!(
        expected.replace("\r\n", "\n"),
        actual,
        "{name} changed: review the difference, then update it on purpose"
    );
}

const SCREEN: Option<(u32, u32)> = Some((2560, 1440));
const DECK: Option<(u32, u32)> = Some((1280, 800));

fn windows(screen: Option<(u32, u32)>) -> Env {
    Env {
        screen,
        ..Env::default()
    }
}

fn wine(screen: Option<(u32, u32)>, gamescope: bool) -> Env {
    Env {
        wine: true,
        gamescope,
        screen,
        overrides: Some("winedbg=d".into()),
    }
}

/// The settings a look saves, as the window writes them.
fn look(look: Look, screen: Option<(u32, u32)>) -> Settings {
    let template = config::template();
    let mut form = Form::first_run(&Settings::parse_file(&template).0, screen);
    form.set_look(look, screen);
    let (s, warnings) = Settings::parse_file(&form.save(&template));
    assert!(warnings.is_empty(), "{warnings:?}");
    s
}

/// A game-written ini the launcher writes over (synthetic, not the game's).
const GAME_INI: &str = "[Registry]\nLANGUAGE=E\nGAMMA=1\nWIDTH=1024\nHEIGHT=768\nVSYNC=0\n[Graphics]\nDIGITAL_TACHO=0\n";

fn describe(title: &str, settings: &Settings, envs: &[(&str, Env)]) -> String {
    let mut out = format!("# {title}\n");
    for (name, env) in envs {
        out.push_str(&format!("\n## {name}\n"));
        out.push_str(&plan::plan(settings, env).describe(GAME_INI.as_bytes()));
    }
    out
}

#[test]
fn help_and_template() {
    // The version changes every release; the snapshot keeps a placeholder.
    check(
        "help.txt",
        &format!("{}\n", config::usage()).replace(env!("CARGO_PKG_VERSION"), "<version>"),
    );
    check("launcher.cfg", &config::template());
}

#[test]
fn looks() {
    check(
        "look-original.txt",
        &describe(
            "Original 2003",
            &look(Look::Original, SCREEN),
            &[
                ("Windows, 2560x1440", windows(SCREEN)),
                ("Wine in gamescope", wine(SCREEN, true)),
            ],
        ),
    );
    check(
        "look-enhanced.txt",
        &describe(
            "Enhanced (recommended) on a 2560x1440 screen",
            &look(Look::Enhanced, SCREEN),
            &[
                ("Windows", windows(SCREEN)),
                ("Wine, no gamescope", wine(SCREEN, false)),
            ],
        ),
    );
    check(
        "look-steam-deck.txt",
        &describe(
            "Steam Deck",
            &look(Look::SteamDeck, DECK),
            &[(
                "Steam Deck Game Mode (Proton in gamescope)",
                wine(DECK, true),
            )],
        ),
    );
}

#[test]
fn option_combinations() {
    let parse = |text: &str| {
        let (s, warnings) = Settings::parse_file(text);
        assert!(warnings.is_empty(), "{warnings:?}");
        s
    };
    check(
        "combo-custom-linux.txt",
        &describe(
            "Custom: windowed 1920x1080, fov 90, 4x, soft shadows, FSAA, standard pad with its log, postfx cinematic, fsr",
            &parse(
                "resolution = 1920x1080\ndisplay_mode = windowed\nvsync = off\nanisotropic = 4\n\
                 fsaa = on\nfov = 90\nshadows = soft\nhd_ui = on\ngamepad = standard\n\
                 gamepad_log = on\npostfx = cinematic\nscaler = fsr\n",
            ),
            &[("Wine in gamescope", wine(SCREEN, true))],
        ),
    );
    check(
        "combo-no-extras.txt",
        &describe(
            "No pad bridge, no DXVK, borderless, everything else the game's",
            &parse("gamepad = off\ndxvk = off\ndisplay_mode = borderless\n"),
            &[
                ("Windows", windows(SCREEN)),
                ("Wine, no gamescope", wine(SCREEN, false)),
            ],
        ),
    );
}
