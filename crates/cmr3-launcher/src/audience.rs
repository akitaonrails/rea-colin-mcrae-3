//! Who sees what the launcher says (`sdk/docs/STANDARDS.md`, "Messages
//! reach the player"). The rule is the SDK's (`report::Audience`); this
//! module only says which family flags a command line holds, read from the
//! parsed switches so a value spelled like a flag (`--game --check`) is
//! never taken for one.

use rea_launcher_win32::report::Run;

use crate::config::Settings;

/// The run `args` ask for (the launcher's arguments, program name
/// excluded). `--help` and a command line the launcher can't read answer
/// like a command: the terminal, or a dialog when a person started it and
/// sees no terminal.
pub fn run(args: &[String]) -> Run {
    match Settings::parse_args(args.iter().cloned()) {
        Ok((_, switches)) => Run::from_args(&switches, &[]),
        Err(_) => Run {
            command: true,
            ..Run::from_args(args, &[])
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::report::{Audience, Mode, Route, Terminal, route};

    fn mode(values: &[&str], wine: bool) -> Mode {
        let args: Vec<String> = values.iter().map(|value| (*value).to_string()).collect();
        Audience::of(run(&args), wine).mode()
    }

    /// `cmr3-play`, the AppImage and Proton: no console, stdout to a log.
    const UNDER_HELPER: Terminal = Terminal {
        attached: false,
        redirected: true,
        wine: true,
    };

    const BOTH: Route = Route {
        print: true,
        dialog: true,
    };

    #[test]
    fn a_play_error_under_the_linux_helper_is_a_dialog() {
        // cmr3-play's game run (`--no-window`, one-run flags too) and a
        // Proton or AppImage start without arguments.
        for args in [
            &["--no-window"][..],
            &["--no-window", "--scaler", "fsr"],
            &[],
        ] {
            let mode = mode(args, true);
            assert_eq!(mode, Mode::Play { interactive: true }, "{args:?}");
            assert_eq!(route(mode, UNDER_HELPER), BOTH, "{args:?}");
        }
    }

    /// cmr3-play's window run (`--settings --linux`): the window shows its
    /// own messages, and when it can't open the helper reads exit code 4
    /// and starts the game run, which reports.
    #[test]
    fn the_helpers_settings_window_shows_no_dialog() {
        let mode = mode(&["--settings", "--linux"], true);
        assert_eq!(mode, Mode::Play { interactive: false });
        assert!(!route(mode, UNDER_HELPER).dialog);
    }

    #[test]
    fn check_and_help_are_commands_with_a_dialog_only_when_unseen() {
        for args in [&["--check"][..], &["--help"], &["-h"], &["/?"]] {
            assert!(
                matches!(mode(args, false), Mode::Command { .. }),
                "{args:?}"
            );
        }
        // Proton: stdout is a log, so the player gets the dialog too.
        assert_eq!(route(mode(&["--check"], true), UNDER_HELPER), BOTH);
        // From a terminal: printed, no dialog.
        let terminal = Terminal {
            attached: true,
            redirected: false,
            wine: false,
        };
        assert_eq!(
            route(mode(&["--check"], false), terminal),
            Route {
                print: true,
                dialog: false
            }
        );
        // The helper reads `--check --linux` itself.
        assert_eq!(
            mode(&["--check", "--linux"], true),
            Mode::Command { interactive: false }
        );
    }

    #[test]
    fn a_bad_command_line_answers_like_a_command() {
        assert!(matches!(
            mode(&["--no-such-flag"], false),
            Mode::Command { interactive: true }
        ));
        assert!(matches!(
            mode(&["--no-such-flag", "--no-window"], false),
            Mode::Command { interactive: false }
        ));
    }

    #[test]
    fn a_value_spelled_like_a_flag_is_a_value() {
        // `--game --check` names a folder called `--check`: a play run.
        assert_eq!(
            mode(&["--game", "--check"], false),
            Mode::Play { interactive: true }
        );
    }

    #[test]
    fn a_windows_script_with_no_window_never_gets_a_dialog() {
        let redirected = Terminal {
            attached: false,
            redirected: true,
            wine: false,
        };
        let script = mode(&["--no-window", "--game", "C:\\CMR3"], false);
        assert_eq!(script, Mode::Play { interactive: false });
        assert!(!route(script, redirected).dialog);
        // `--check` from a Windows script: the terminal only.
        assert!(!route(mode(&["--check", "--no-window"], false), redirected).dialog);
        // A double-clicked shortcut is a person.
        assert_eq!(mode(&[], false), Mode::Play { interactive: true });
    }

    /// Under Wine (`sdk/scripts/wine-test` pipes stdout, as cmr3-play
    /// does): the game run's error reaches the player as a dialog.
    #[cfg(windows)]
    #[test]
    fn under_wine_the_game_run_error_is_a_dialog() {
        use rea_launcher_win32::{report, wine};
        if !wine::is_wine() {
            return;
        }
        let args = vec!["--no-window".to_string()];
        assert!(report::route_here(report::audience(run(&args)).mode()).dialog);
    }
}
