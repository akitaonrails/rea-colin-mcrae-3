//! One launcher at a time (SDK STANDARDS, "Clean exit, single instance").
//!
//! The launcher holds a named mutex (`rea_launcher_win32::instance`) from
//! the moment it starts until the game it started has ended, so a second
//! start can neither open a second window nor start a second game from
//! the same runtime copy: it brings the first launcher's window forward,
//! or says the game is running and exits 5 (`instance::ALREADY_RUNNING`):
//! the SDK's `AlreadyRunning::second_start` with `Window::Own`, which only
//! finds a window of this launcher's own exe. Before Play the launcher
//! also refuses while the runtime's game still runs (`process::find_running`):
//! a launcher killed mid-game released the instance, its game did not.
//! `--check` takes the instance like any action (SDK STANDARDS, "Action
//! flags and one instance"): with the window open it says to use the
//! window and exits 5; `--help` runs alongside. On Linux, `cmr3-play`'s
//! session lock does the same job.

/// The instance name (`Local\rea-launcher-rea-colin-mcrae-3`).
pub const NAME: &str = "rea-colin-mcrae-3";

/// The settings window's title, which a second start looks for to bring
/// the window forward. Not the game's title: FindWindow would find the
/// game's window instead.
pub const WINDOW_TITLE: &str = "Colin McRae Rally 3 launcher";

/// The game's own window title (Rally_3PC.exe 1.1, seen under Wine). A
/// second start that found this window would raise the game and say
/// nothing.
pub const GAME_WINDOW_TITLE: &str = "Colin McRae Rally 3";

/// The game's name in the SDK's second-start messages
/// (`instance::SecondStart::message`).
pub const GAME: &str = "Colin McRae Rally 3";

/// Whether a run does work without the window (`--check`; the SDK's
/// `instance::is_action`): the second start's `action`. `switches` are the
/// parsed ones (`Settings::parse_args`), so `--game --check`, a folder
/// called `--check`, is no action. This launcher has no actions of its own.
#[cfg(any(windows, test))]
pub fn is_action(switches: &[String]) -> bool {
    rea_launcher_win32::instance::is_action(switches, &[])
}

/// The exit code of a second start (`SecondStart::exit_code`: 0 when the
/// open window came forward, else 5). Under `--settings` cmr3-play reads
/// 0 as "Play pressed", so a window brought forward is "closed without
/// Play" (3) there and starts nothing.
pub fn second_start_exit(exit_code: i32, settings_only: bool) -> i32 {
    if settings_only && exit_code == 0 {
        3
    } else {
        exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Measured 2026-11-02 in the real game: with the settings window
    /// titled like the game, a second start found the game's window,
    /// raised it and never said the game was running.
    #[test]
    fn the_launcher_window_is_never_mistaken_for_the_game() {
        assert_ne!(WINDOW_TITLE, GAME_WINDOW_TITLE);
    }

    #[test]
    fn a_second_settings_window_start_never_reads_as_play() {
        use rea_launcher_win32::instance::{ALREADY_RUNNING, SecondStart};
        let exit = |second: SecondStart, settings_only| {
            second_start_exit(second.exit_code(), settings_only)
        };
        assert_eq!(exit(SecondStart::Focused, false), 0);
        assert_eq!(exit(SecondStart::Focused, true), 3);
        assert_eq!(exit(SecondStart::Busy, false), ALREADY_RUNNING);
        assert_eq!(exit(SecondStart::Busy, true), ALREADY_RUNNING);
        assert_eq!(
            SecondStart::Busy.message(GAME).as_deref(),
            Some(
                "Colin McRae Rally 3 is already running from this launcher. \
                 Quit the game first, then start the launcher again."
            )
        );
    }

    /// Windows 10 test (rea-launcher-sdk#1): `--check` with the window
    /// open ran the check anyway. It is an action now: refused with the
    /// family's message and exit 5, the window left where it is.
    #[test]
    fn check_with_the_window_open_is_refused_with_exit_5() {
        use crate::config::Settings;
        use rea_launcher_win32::instance::{ALREADY_RUNNING, SecondStart};
        let switches = |args: &[&str]| {
            Settings::parse_args(args.iter().map(|a| a.to_string()))
                .unwrap()
                .1
        };
        let check = is_action(&switches(&["--check"]));
        assert!(check);
        let second = SecondStart::decide(check, || true, || panic!("an action never raises"));
        assert_eq!(second, SecondStart::WindowOpen);
        assert_eq!(
            second_start_exit(second.exit_code(), false),
            ALREADY_RUNNING
        );
        assert_eq!(
            second.message(GAME).as_deref(),
            Some(
                "The Colin McRae Rally 3 launcher window is open. Use it there, \
                 or close it and try again."
            )
        );
        // While the game runs (no window): the game is running, exit 5.
        let second = SecondStart::decide(check, || false, || panic!("an action never raises"));
        assert_eq!(second, SecondStart::Busy);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        // A plain start, a play run and a folder called `--check` are no
        // action: the open window comes forward.
        for args in [&[][..], &["--no-window"], &["--game", "--check"]] {
            let action = is_action(&switches(args));
            assert!(!action, "{args:?}");
            let second = SecondStart::decide(action, || unreachable!(), || true);
            assert_eq!(second, SecondStart::Focused, "{args:?}");
        }
    }

    /// Under Wine (`sdk/scripts/wine-test`): while one launcher holds the
    /// instance, a second start is refused, finds no window of its own
    /// exe to bring forward (headless: no settings window), and so says
    /// the game is running and exits 5; once the first ends, the next
    /// start gets it.
    #[cfg(windows)]
    #[test]
    fn a_second_launcher_is_refused_while_the_first_runs() {
        use rea_launcher_win32::instance::{self, ALREADY_RUNNING, SecondStart, Window};
        let name = format!("{NAME}-test-{}", std::process::id());
        let first = instance::single(&name).expect("the first launcher");
        let second = instance::single(&name).expect_err("a second launcher");
        let second = second.second_start(false, Window::Own(WINDOW_TITLE));
        assert_eq!(second, SecondStart::Busy);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        drop(first);
        drop(instance::single(&name).expect("free once the first ended"));
    }

    /// Under Wine: the check before Play sees a running copy of an exe
    /// (here the test program itself), and nothing for an exe not running.
    #[cfg(windows)]
    #[test]
    fn play_sees_a_game_still_running_from_the_runtime() {
        use rea_launcher_win32::process;
        let me = std::env::current_exe().unwrap();
        assert!(process::find_running(&me).contains(&std::process::id()));
        let elsewhere = me.with_file_name("Rally_3PC-not-running.exe");
        assert!(process::find_running(&elsewhere).is_empty());
    }
}
