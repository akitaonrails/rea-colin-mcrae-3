//! Golden snapshots of everything the launcher produces (SDK STANDARDS,
//! "Golden snapshots"): `--help`, the `launcher.cfg` template, and for each
//! look and a few option combinations the launch plan — components,
//! `SilentPatchCMR3.ini` bytes, `Xidi.ini`, `WINEDLLOVERRIDES`, game
//! arguments, patch sets and the Linux helper's values. No game bytes:
//! only what the launcher generates.
//!
//! A diff is reviewed like code. To accept an intended change:
//! `CMR3_UPDATE_SNAPSHOTS=1 cargo test --test snapshots` (or the SDK's
//! `REA_UPDATE_SNAPSHOTS=1`), then read the diff.

use cmr3_launcher::config::{self, Settings};
use cmr3_launcher::form::{self, Form, Look};
use cmr3_launcher::plan::{self, Env};

/// Compares with `tests/snapshots/NAME` through the SDK helper: the crate
/// version is stored as `<version>`, line endings don't matter, and a
/// difference names its first line. `CMR3_UPDATE_SNAPSHOTS=1` (or the
/// SDK's `REA_UPDATE_SNAPSHOTS=1`) writes them.
fn check(name: &str, actual: &str) {
    rea_launcher::snapshots!()
        .update_var("CMR3_UPDATE_SNAPSHOTS")
        .assert(name, actual);
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
    check("help.txt", &format!("{}\n", config::usage()));
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

/// `--look` end to end: a `--no-window` run with the flag (as
/// `cmr3-play --play --look steam_deck` starts it) plays what the window
/// saves for that look, over a file the player customised.
#[test]
fn the_look_flag_plays_what_the_window_saves() {
    let file = Settings::parse_file(
        "resolution = 1600x900\ndisplay_mode = windowed\nvsync = off\nanisotropic = 2\n\
         fsaa = on\nfov = 110\nhd_ui = off\nshadows = soft\n",
    )
    .0;
    for (key, chosen, screen) in [
        ("original", Look::Original, SCREEN),
        ("enhanced", Look::Enhanced, SCREEN),
        ("steam_deck", Look::SteamDeck, DECK),
    ] {
        let (cli, _) = Settings::parse_args(["--look", key].map(String::from)).unwrap();
        let run = form::with_look(file.clone(), cli, screen);
        for env in [windows(screen), wine(screen, true), wine(screen, false)] {
            assert_eq!(
                plan::plan(&run, &env).describe(GAME_INI.as_bytes()),
                plan::plan(&look(chosen, screen), &env).describe(GAME_INI.as_bytes()),
                "--look {key}"
            );
        }
    }
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
