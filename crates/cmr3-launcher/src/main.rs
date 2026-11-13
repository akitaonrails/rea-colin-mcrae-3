#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn main() {
    let code = match app::run() {
        Ok(code) => code,
        Err(err) => {
            app::log_line(&format!("error: {err}"));
            app::fatal(&err);
            1
        }
    };
    std::process::exit(code as i32);
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "CMR3Launcher is a Windows program; on Linux run it with Proton or Wine (or the AppImage)."
    );
    std::process::exit(2);
}

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};

    use cmr3_launcher::audience;
    use cmr3_launcher::check;
    use cmr3_launcher::components::{self, Installed};
    use cmr3_launcher::config::{self, Settings};
    use cmr3_launcher::controllers;
    use cmr3_launcher::edition;
    use cmr3_launcher::form;
    use cmr3_launcher::instance;
    use cmr3_launcher::layer;
    use cmr3_launcher::plan::{self, Env};
    use cmr3_launcher::runtime;
    use cmr3_launcher::source::{self, Found};
    use cmr3_launcher::unpack;
    use cmr3_launcher::wheel;
    use rea_launcher::cfg::ArgError;
    use rea_launcher::installer;
    use rea_launcher::log::Log;
    use rea_launcher::wheel::dinput::{Mode, Setup};
    use rea_launcher::wheel::rig;
    use rea_launcher::{cfg, fsx, paths};
    use rea_launcher_win32::gpu::{self, Glx};
    use rea_launcher_win32::instance::{self as win32_instance, SecondStart, Window};
    use rea_launcher_win32::process::{self, Launch};
    use rea_launcher_win32::report;
    use rea_launcher_win32::wine::DllMode;
    use rea_launcher_win32::{dinput, disk};
    use rea_launcher_win32::{display, wine};

    use crate::ui;

    const EXE: &str = "Rally_3PC.exe";
    const TITLE: &str = instance::WINDOW_TITLE;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR3Launcher")
    }

    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
    }

    /// Who sees this run's messages: the SDK's rule (`report::audience`)
    /// for the family flags on the command line. Under Wine (cmr3-play,
    /// the AppImage, Proton) a person, even with `--no-window`.
    fn mode() -> report::Mode {
        let args: Vec<String> = std::env::args().skip(1).collect();
        report::audience(audience::run(&args)).mode()
    }

    /// An error the launcher stops on: a dialog whenever a person may be
    /// watching, and printed (cmr3-play's log).
    pub fn fatal(text: &str) {
        report::fatal(TITLE, text, mode());
    }

    /// Information or a warning, routed like [`fatal`].
    fn note(text: &str, warning: bool) {
        report::note(TITLE, text, mode(), warning);
    }

    /// A `launcher.cfg` next to the exe wins (portable installs);
    /// otherwise the one in the launcher's home folder.
    fn settings_path() -> PathBuf {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(config::FILE_NAME)));
        match portable {
            Some(p) if p.is_file() => p,
            _ => home().join(config::FILE_NAME),
        }
    }

    /// `%USERPROFILE%\Saved Games\Colin McRae Rally 3`, Windows' usual
    /// place for game saves (inside the prefix under Wine/Proton).
    fn default_saves() -> PathBuf {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(home)
            .join("Saved Games")
            .join("Colin McRae Rally 3")
    }

    fn find_game(settings: &Settings) -> Result<PathBuf, String> {
        if let Some(g) = &settings.game {
            return Ok(g.clone());
        }
        // A launcher placed in the game folder needs no argument, and the
        // Linux helper can name the folder through the environment.
        if let Some(dir) = std::env::var_os("CMR3_GAME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            && dir.join(EXE).is_file()
        {
            return Ok(dir);
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            && dir.join(EXE).is_file()
        {
            return Ok(dir);
        }
        Err(format!(
            "Tell the launcher where the game is: run it once with\n\n  CMR3Launcher --game <folder with {EXE}, or with the game's installer>\n\nThe folder is remembered."
        ))
    }

    /// `--settings` exit codes for cmr3-play: closed without playing, or
    /// no window could be shown (play with the saved settings).
    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;
    /// `--check` found a part missing or a build it doesn't know.
    const CHECK_FAILED: u32 = 1;

    /// The settings file's text: `None` on a first run (no file yet).
    /// A file that cannot be read is never written over.
    fn read_settings(path: &Path) -> Result<Option<String>, String> {
        cfg::read_file(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    }

    pub fn run() -> Result<u32, String> {
        log_line(concat!("CMR3Launcher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let given = args.len();
        let (cli, switches) = match Settings::parse_args(args.iter().cloned()) {
            Ok(parsed) => parsed,
            Err(ArgError::Help(_)) => {
                note(&config::usage(), false);
                return Ok(0);
            }
            Err(e) => {
                // A flag the launcher doesn't know, or a value it can't
                // use: answered like a command (the terminal; a dialog
                // only when nobody sees one), exit 2.
                log_line(&format!("error: {e}"));
                let mode = report::audience(report::Run::bad_command_line(&args)).mode();
                report::fatal(TITLE, &format!("{e}\n\n{}", config::usage()), mode);
                return Ok(report::BAD_COMMAND_LINE as u32);
            }
        };
        let switch = |name: &str| switches.iter().any(|s| s == name);
        // A game folder from the command line is remembered as given;
        // never swapped for the launcher's unpacked copy.
        let game_from_command_line = cli.game.is_some();
        // The settings window: shown when started without options (a
        // double click), or alone with --settings (Linux: cmr3-play shows
        // it, then starts the game in gamescope). Any other option skips
        // it.
        let settings_only = switch("--settings");
        let linux = switch("--linux");
        let no_window = switch("--no-window");
        let check_only = switch("--check");
        let other_options = given > switches.len() || check_only;
        // One launcher at a time, held until the game this run starts has
        // ended: a second start raises the window or says the game runs
        // (`instance`). `--check` takes it like any action.
        let mut held = match win32_instance::single(instance::NAME) {
            Ok(guard) => Some(guard),
            Err(running) => {
                // A plain start brings the open window forward; an action
                // (`--check`) with the window open, or any start while the
                // game (or a run without a window) is in progress, says
                // so and exits 5.
                let action = instance::is_action(&switches);
                let second = running.second_start(action, Window::Own(TITLE));
                log_line(second.log_line());
                // A play-time message: the player started a second copy
                // and must see why nothing happened.
                if let Some(message) = second.message(instance::GAME) {
                    fatal(&message);
                }
                return Ok(instance::second_start_exit(second.exit_code(), settings_only) as u32);
            }
        };
        if settings_only || (!other_options && !no_window) {
            // A window under Wine draws through OpenGL: GLX, not Wine 11's
            // default EGL (software on NVIDIA boxes without egl-x11). On a
            // fresh prefix the setting only takes effect in a new process:
            // the launcher runs itself once more, so this first start
            // already gets its window.
            match gpu::prefer_glx_or_restart(held.take()) {
                Glx::Restarted { exit_code } => {
                    log_line(
                        "Wine: OpenGL through GLX (UseEGL=N) on a fresh prefix, started again",
                    );
                    return Ok(exit_code);
                }
                Glx::Ready {
                    instance: guard,
                    warning,
                } => {
                    held = guard;
                    if let Some(warning) = warning {
                        log_line(&format!("warning: {warning}"));
                    }
                }
            }
            let text = read_settings(&settings_path)
                .ok()
                .flatten()
                .unwrap_or_else(config::template);
            match ui::show(&settings_path, &text, linux) {
                Ok(ui::Outcome::Play) => {
                    log_line("settings window: play");
                    if settings_only {
                        return Ok(0);
                    }
                }
                Ok(ui::Outcome::Closed) => {
                    log_line("settings window: closed");
                    return Ok(if settings_only { WINDOW_CLOSED } else { 0 });
                }
                Err(e) => {
                    // No window possible (no OpenGL): play with the saved
                    // settings.
                    log_line(&format!("settings window unavailable: {e}"));
                    if settings_only {
                        return Ok(WINDOW_UNAVAILABLE);
                    }
                }
            }
        }
        // Unreadable (not missing): played with the defaults, never
        // written over.
        let (existing, writable) = match read_settings(&settings_path) {
            Ok(text) => (text, true),
            Err(e) => {
                log_line(&format!("warning: {e}; left as it is"));
                (None, false)
            }
        };
        let template = config::template();
        let file_text = existing.as_deref().unwrap_or(&template);
        let (saved, warnings) = Settings::parse_file(file_text);
        for w in &warnings {
            log_line(&format!("warning: {w}"));
        }
        // A `look` (the file's, or this run's --look) under the picture
        // values given on their own.
        let settings = form::with_look(saved.clone(), cli, display::screen_size());

        if check_only {
            return Ok(check(&settings));
        }
        let chosen = find_game(&settings)?;
        let rt = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("Rally3"));
        // The folder holds the game, or an installer the launcher unpacks
        // once into its own folder and plays from (`source`).
        let game = match source::find(&chosen)? {
            Found::Game(_) => chosen.clone(),
            Found::Installer(found) => {
                paths::check_separate(&chosen, &rt)?;
                let mut log = |line: &str| log_line(line);
                // The progress window whenever a player may be watching:
                // cmr3-play and the AppImage start this run with
                // --no-window. Only a script waits without one.
                let window = audience::unpack_window(mode());
                let watch = |job: &mut dyn installer::UnpackJob| {
                    if window {
                        ui::show_unpack(&found, job)
                    } else {
                        Err("nobody is watching (--no-window on Windows)".to_string())
                    }
                };
                match unpack::unpack(
                    &found,
                    &chosen,
                    &rt,
                    &components::components_dir(),
                    &mut log,
                    watch,
                )? {
                    Some(dest) => dest,
                    None => return Ok(0),
                }
            }
            Found::Nothing => return Err(source::nothing_here(&chosen)),
        };
        let unpacked = game != chosen;
        let exe_path = game.join(EXE);
        let exe_bytes =
            fs::read(&exe_path).map_err(|e| format!("cannot read {}: {e}", exe_path.display()))?;
        let exe_sha = edition::sha256_hex(&exe_bytes);
        let exe_name = match edition::identify(&exe_bytes) {
            edition::Edition::DrmFree(name) => name.to_string(),
            edition::Edition::NeedsPatching => {
                "not a known DRM-free build; the DRM-free patcher will try it".into()
            }
        };
        log_line(&format!(
            "game: {} ({}) [sha256 {}]",
            game.display(),
            exe_name,
            exe_sha
        ));
        let bundle = components::bundle_dir();

        paths::check_separate(&game, &rt)?;
        // A launcher that was killed released the instance, but its game
        // may still run from this runtime copy: never sync under it.
        if !process::find_running(&rt.join(EXE)).is_empty() {
            let busy = SecondStart::Busy;
            log_line(busy.log_line());
            fatal(&busy.message(instance::GAME).unwrap_or_default());
            return Ok(busy.exit_code() as u32);
        }
        let report = runtime::sync(&game, &rt)
            .map_err(|e| format!("cannot prepare {}: {e}", rt.display()))?;
        log_line(&format!(
            "runtime: {} ({} files, {} linked)",
            rt.display(),
            report.brought,
            report.linked
        ));
        for w in &report.warnings {
            log_line(&format!("warning: {w}"));
        }

        // The player's profile lives outside the runtime copy; the game
        // reads and writes its own `CMR3`, kept in step before and after.
        let saves = settings.saves.clone().unwrap_or_else(default_saves);
        let rt_profile = rt.join(runtime::PROFILE);
        let save_profile = saves.join(runtime::PROFILE);
        // A copy changed since the last mirror wins: one the player put in
        // the saves folder, or else the runtime's when the last session
        // ended (a crash) before the launcher could save it.
        let profile_state = rt.join(runtime::PROFILE_STATE);
        let mirror = runtime::profile_mirror(&rt_profile, &save_profile, &profile_state);
        match mirror.before() {
            Ok(moved) if !moved.to_runtime.is_empty() => {
                log_line("profile: restored from the saves folder")
            }
            Ok(moved) if !moved.to_saves.is_empty() => log_line(
                "profile: saved to the saves folder (the last session ended before it was)",
            ),
            Ok(_) => {}
            Err(e) => return Err(format!("cannot prepare the profile: {e}")),
        }

        // DRM-free: the patcher itself tells the game versions apart (it
        // only applies to a matching file).
        if components::runtime_needs_patch(&rt.join(EXE)) {
            apply_drm_free(&rt, &bundle, &exe_sha)?;
        } else {
            log_line("executable: already DRM-free");
        }

        // What we're running on, decided once: DXVK only helps under Wine,
        // and the display mode the game can honour depends on whether
        // gamescope already gives it a single screen.
        let wine = wine::is_wine();
        let env = Env {
            wine,
            gamescope: std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some(),
            screen: display::screen_size(),
            overrides: std::env::var("WINEDLLOVERRIDES").ok(),
        };
        let plan = plan::plan(&settings, &env);

        // The bundled parts, and the DLL names Wine must then load from
        // the game folder.
        let mut natives = Vec::new();
        // SilentPatch's ASI loader also loads the wheel layer.
        let mut asi_loader = false;
        match components::install_silentpatch(&game, &rt, &bundle) {
            Ok(Installed::Done) => {
                natives.push("dsound");
                asi_loader = true;
                log_line("SilentPatch installed")
            }
            Ok(Installed::NotBundled) => log_line(
                "warning: SilentPatch not bundled; the widescreen and compatibility fixes are off",
            ),
            Ok(Installed::NotOurs(name)) => log_line(&not_ours("SilentPatch", name, &rt)),
            Err(e) => log_line(&format!("warning: SilentPatch: {e}")),
        }
        let gamepad = plan.gamepad;
        let xidi = match components::install_xidi(&game, &rt, &bundle, gamepad, plan.gamepad_log) {
            Ok(Installed::Done) => {
                log_line(&format!("Xidi installed ({gamepad:?})"));
                if plan.xidi_ini.is_some() {
                    natives.extend_from_slice(&components::XIDI_DLLS);
                }
                true
            }
            Ok(Installed::NotBundled) => {
                log_line(
                    "warning: Xidi not bundled; the gamepad runs through the game's own DirectInput",
                );
                false
            }
            Ok(Installed::NotOurs(name)) => {
                log_line(&not_ours("Xidi", name, &rt));
                false
            }
            Err(e) => {
                log_line(&format!("warning: Xidi: {e}"));
                false
            }
        };
        // The wheel layer, in front of Xidi: its settings (launcher.cfg's
        // Wheel tab and the pedal calibration beside it) and the DLL, as
        // an ASI plugin.
        let calibration = fs::read_to_string(settings_path.with_file_name(wheel::FILE)).ok();
        let setup = &settings.wheel.setup;
        let ini = wheel::runtime_ini(setup, calibration.as_deref());
        let components = components::components_dir();
        let layer_on = match components::install_layer(&game, &rt, &components, &ini) {
            Ok(Installed::Done) if asi_loader => true,
            Ok(Installed::Done) => {
                log_line(
                    "warning: the wheel layer needs SilentPatch's ASI loader; wheels as the game sees them",
                );
                false
            }
            Ok(Installed::NotBundled) => {
                log_line("warning: the wheel layer is not bundled; wheels as the game sees them");
                false
            }
            Ok(Installed::NotOurs(name)) => {
                log_line(&not_ours("The wheel layer", name, &rt));
                false
            }
            Err(e) => {
                log_line(&format!("warning: wheel layer: {e}"));
                false
            }
        };
        // Decided now, with the devices attached as the game starts: the
        // layer is told the same (REA_WHEEL_MODE).
        let wheel_shown = layer_on && virtual_wheel(setup);
        log_line(&format!(
            "wheel: {} (wheel support {})",
            if wheel_shown {
                "the game sees the virtual wheel"
            } else {
                "the game sees its controllers as they are"
            },
            setup.mode.key()
        ));
        // The profile's controller records for the devices the game will
        // find: the virtual wheel's, and Xidi's pad as the player would
        // pick it in Options → Controls; records the player set are kept,
        // beside the profile too (`controllers`).
        let stash = saves.join(controllers::STASH_FILE);
        let seats = controllers::lineup(wheel_shown, xidi && plan.choose_controller);
        match controllers::prepare_files(&rt_profile, &stash, &seats) {
            Ok(done) => {
                for line in done {
                    log_line(&format!("profile: {line}"));
                }
            }
            Err(e) => log_line(&format!("warning: profile controllers: {e}")),
        }
        let dxvk_on = plan.dxvk;
        match components::install_dxvk(&game, &rt, &bundle, dxvk_on) {
            Ok(Installed::Done) if dxvk_on => {
                natives.push("d3d9");
                log_line("DXVK installed (d3d9 through Vulkan)")
            }
            Ok(Installed::Done) => log_line("DXVK not active"),
            Ok(Installed::NotOurs(name)) => log_line(&not_ours("DXVK", name, &rt)),
            Ok(Installed::NotBundled) => log_line(
                "warning: DXVK not bundled; Wine's own OpenGL translator will draw (slow at 4K)",
            ),
            Err(e) => log_line(&format!("warning: DXVK: {e}")),
        }
        if plan.hd_ui {
            match components::install_hd_ui(&rt, &bundle) {
                Ok(n) if n > 0 => log_line(&format!("HD UI: {n} files")),
                Ok(_) => log_line("HD UI: not bundled"),
                Err(e) => log_line(&format!("warning: HD UI: {e}")),
            }
        } else {
            match components::remove_hd_ui(&game, &rt) {
                Ok(0) => {}
                Ok(n) => log_line(&format!("HD UI off: {n} files back to the game's")),
                Err(e) => log_line(&format!("warning: HD UI off: {e}")),
            }
        }
        // Unset resolution: SilentPatch's default is the desktop's size;
        // on this screen that is what the settings window would have
        // picked. Exclusive fullscreen under raw Wine chases the desktop's
        // monitor list, so it plays borderless instead (see
        // `DisplayMode::effective`): same size, no mode switch, no wrong
        // display.
        if plan.effective_mode != plan.display_mode {
            log_line(&format!(
                "display mode: {} plays borderless under Wine{} (exclusive fullscreen picks \
                 the wrong display when the desktop has several; install gamescope — cmr3-play \
                 uses it — for true fullscreen)",
                plan.display_mode.key(),
                if env.gamescope {
                    " inside gamescope"
                } else {
                    ""
                },
            ));
        }
        match components::apply_settings(&rt, &plan.ini_settings, plan.resolution) {
            Ok(log) => log_line(&format!("SilentPatchCMR3.ini: {}", log.join(", "))),
            Err(e) => log_line(&format!("warning: cannot write the settings: {e}")),
        }

        // Remember the game folder (after an unpack, the unpacked one,
        // unless --game named the folder); the rest of the file stays as
        // written.
        let on_file = existing.as_ref().and(saved.game.as_deref());
        let mut written = !game_from_command_line && on_file == Some(game.as_path());
        if writable
            && let Some(folder) =
                source::game_to_save(on_file, game_from_command_line, &chosen, &game)
        {
            let text = rea_launcher::cfg::spec::set_key(
                file_text,
                config::option("game"),
                &folder.display().to_string(),
            );
            match fsx::write_atomic(&settings_path, text) {
                Ok(()) => written = folder == game,
                Err(e) => log_line(&format!(
                    "warning: cannot write {}: {e}",
                    settings_path.display()
                )),
            }
        }
        if unpacked {
            let setting = if game_from_command_line {
                source::Setting::CommandLine
            } else if written {
                source::Setting::Saved
            } else {
                source::Setting::NotSaved
            };
            let message = source::unpacked_message(&game, setting);
            log_line(&message);
            // Said, not asked: a dialog here would hold the game back.
            if report::route_here(mode()).print {
                println!("{TITLE}: {message}");
            }
        }

        if !wine {
            match gpu::prefer_fast_gpu(&rt.join(EXE)) {
                Ok(true) => log_line("Windows: high-performance GPU for the game"),
                Ok(false) => {}
                Err(e) => log_line(&format!("warning: {e}")),
            }
        }
        // Wine's loader ignores WINEDLLOVERRIDES in the game's environment
        // block: plain Proton/Wine runs (no cmr3-play) get the bundled DLLs
        // through the per-program registry key, for the parts that did
        // install; a part not installed loses the override we wrote.
        if wine {
            let (on, off) = components::app_overrides(&natives);
            let modes = |dlls: &[&'static str]| {
                dlls.iter()
                    .map(|&d| (d, DllMode::NativeThenBuiltin))
                    .collect::<Vec<_>>()
            };
            match wine::app_dll_overrides(EXE, &modes(&on)) {
                Ok(applied) => {
                    if !applied.written.is_empty() {
                        log_line(&format!(
                            "Wine DLL overrides for {EXE}: {} (native, then builtin)",
                            applied.written.join(", ")
                        ));
                    }
                    for (dll, value) in &applied.player {
                        log_line(&format!(
                            "Wine DLL override for {dll} kept as the player set it: {value:?}"
                        ));
                    }
                }
                Err(e) => log_line(&format!("warning: Wine DLL overrides: {e}")),
            }
            match wine::remove_app_dll_overrides(EXE, &modes(&off)) {
                Ok(removed) if !removed.is_empty() => log_line(&format!(
                    "Wine DLL overrides removed for {EXE}: {}",
                    removed.join(", ")
                )),
                Ok(_) => {}
                Err(e) => log_line(&format!("warning: Wine DLL overrides: {e}")),
            }
        }
        // The environment value too: cmr3-play and the log read it.
        let overrides =
            wine.then(|| components::wine_overrides(env.overrides.as_deref(), &natives));
        if let Some(o) = &overrides {
            log_line(&format!("WINEDLLOVERRIDES={o}"));
        }
        log_line("starting the game");
        let mut launch = Launch::new(rt.join(EXE)).cwd(&rt);
        if layer_on {
            launch = launch.env(layer::MODE_ENV, layer::mode_value(wheel_shown));
        }
        if let Some(o) = overrides {
            launch = launch.env("WINEDLLOVERRIDES", o);
        }
        let code = launch.run()?;
        log_line(&format!("game exited with {code} (0x{code:08x})"));
        if code >= 0xC000_0000 {
            fatal(&format!(
                "{EXE} stopped unexpectedly (Windows status 0x{code:08X}).\n\n\
                 Launcher log: {}",
                home().join("launcher.log").display()
            ));
        }
        if rt_profile.is_file() {
            match controllers::keep_files(&rt_profile, &stash) {
                Ok(true) => log_line("profile: controller records kept"),
                Ok(false) => {}
                Err(e) => log_line(&format!("warning: controller records not kept: {e}")),
            }
            match mirror.after() {
                Ok(copied) if !copied.is_empty() => log_line("profile: saved"),
                Ok(_) => {}
                Err(e) => log_line(&format!(
                    "warning: profile not saved to {}: {e}",
                    save_profile.display()
                )),
            }
        }
        // One launcher at a time until here: the game has ended and its
        // profile is saved.
        drop(held);
        Ok(code)
    }

    /// Whether the game will see the virtual wheel: `auto` asks
    /// DirectInput which devices are attached now.
    fn virtual_wheel(setup: &Setup) -> bool {
        if setup.mode != Mode::Auto {
            return rig::shows_virtual_wheel(setup, &[]);
        }
        let devices = dinput::list();
        let names: Vec<&str> = devices.iter().map(|d| d.id.name.as_str()).collect();
        log_line(&format!("controllers: {}", names.join(", ")));
        rig::shows_virtual_wheel(setup, &devices)
    }

    /// The log line for a component left out because the runtime already
    /// holds a file of that name that is not the launcher's.
    fn not_ours(component: &str, name: &str, rt: &Path) -> String {
        format!(
            "warning: {component} left out: {} is not the launcher's (a file you put \
             there?); remove it to let the launcher install its own",
            rt.join(name).display()
        )
    }

    /// Runs the bundled DRM-free patch (CookiePLMonster's xdelta updates
    /// for the official 1.1 executables, owner decision of 2026-11-02) on
    /// `source`, writing `out`: the patch set whose result is one of the
    /// known DRM-free builds, or `None` when every set refused it (xdelta
    /// refuses a source it wasn't made for) or gave an unknown result.
    fn try_drm_free_patch(
        xdelta: &Path,
        source: &Path,
        out: &Path,
        cwd: &Path,
    ) -> Result<Option<&'static str>, String> {
        for variant in ["efigs", "polish"] {
            let args = components::xdelta_args(xdelta, source, variant, out);
            let code = Launch::new(xdelta)
                .args(&args)
                .cwd(cwd)
                .hidden(true)
                .run()?;
            if code != 0 {
                continue;
            }
            if fs::read(out).is_ok_and(|bytes| edition::is_drm_free(&bytes)) {
                return Ok(Some(variant));
            }
            log_line(&format!(
                "DRM-free patch ({variant}) gave an unknown result"
            ));
        }
        Ok(None)
    }

    /// Applies the bundled DRM-free patch to the runtime copy; anything
    /// it refuses is refused with the game's SHA-256 (`sha`).
    fn apply_drm_free(rt: &Path, bundle: &Path, sha: &str) -> Result<(), String> {
        let xdelta = bundle.join("patcher/xdelta.exe");
        if !xdelta.is_file() {
            return Err(format!(
                "this executable is not a known DRM-free build, and the DRM-free patcher \
                 that converts the official 1.1 update is not bundled (SHA-256 {sha}).\n\n\
                 Reinstall the launcher package, then try again."
            ));
        }
        let exe = rt.join(EXE);
        let backup = rt.join("Rally_3PC.exe.drm");
        let new = exe.with_extension("new");
        fsx::install_copy(&exe, &backup)
            .map_err(|e| format!("cannot back up the executable: {e}"))?;
        let cleanup = || {
            let _ = fs::remove_file(&backup);
            let _ = fs::remove_file(&new);
        };
        let patched = try_drm_free_patch(&xdelta, &backup, &new, rt);
        let result = match patched {
            Ok(Some(variant)) => fsx::install_copy(&new, &exe)
                .map(|()| log_line(&format!("DRM-free patch applied ({variant})")))
                .map_err(|e| format!("cannot replace the executable: {e}")),
            Ok(None) => Err(edition::refusal(sha)),
            Err(e) => Err(e),
        };
        cleanup();
        result
    }

    /// `--check`: the game folder, its executable and the bundled parts,
    /// without playing. An executable that is not DRM-free is tried with
    /// the bundled patcher on a scratch file in the launcher's folder (the
    /// game folder is only read). The report goes to launcher.log and the
    /// terminal (a message box only when nobody sees one); the exit code
    /// says whether everything needed is there.
    fn check(settings: &Settings) -> u32 {
        let bundle = components::bundle_dir();
        let rt = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("Rally3"));
        let mut game = find_game(settings);
        // The folder the executable is read from: the game folder, or the
        // launcher's unpacked copy of the installer found there. Nothing
        // is unpacked here.
        let mut unpack = None;
        let mut exe_dir = None;
        if let Ok(chosen) = &game {
            match source::find(chosen) {
                Ok(Found::Game(_)) => exe_dir = Some(chosen.clone()),
                Ok(Found::Installer(found)) => {
                    let dest = source::unpacked_dir(&rt);
                    let setup = found
                        .setup
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    if installer::is_unpacked(&dest, &found) {
                        unpack = Some(check::Unpack::Done {
                            setup,
                            name: found.build.name,
                            dest: dest.display().to_string(),
                        });
                        exe_dir = Some(dest);
                    } else {
                        let parent = dest.parent().unwrap_or(&dest);
                        unpack = Some(check::Unpack::Pending {
                            setup,
                            name: found.build.name,
                            sha256: source::sha256_hex(&found),
                            dest: dest.display().to_string(),
                            need: installer::space_needed(found.build),
                            free: disk::free_space(parent),
                        });
                    }
                }
                Ok(Found::Nothing) => game = Err(source::nothing_here(chosen)),
                Err(e) => game = Err(e),
            }
        }
        let exe = exe_dir.map(|dir| {
            let path = dir.join(EXE);
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let sha = edition::sha256_hex(&bytes);
            let exe = check::Exe::from_edition(edition::identify(&bytes), || {
                let xdelta = bundle.join("patcher/xdelta.exe");
                xdelta.is_file().then(|| {
                    let scratch = home();
                    fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
                    let out = scratch.join("check-Rally_3PC.exe.new");
                    let tried = try_drm_free_patch(&xdelta, &path, &out, &scratch);
                    let _ = fs::remove_file(&out);
                    tried
                })
            });
            Ok((exe, sha))
        });
        let game = game.map(|g| g.display().to_string());
        let pending = unpack.as_ref().is_some_and(check::Unpack::pending);
        let innoextract = components::components_dir().join(components::INNOEXTRACT);
        let mut parts = components::present(&bundle);
        parts.push(components::innoextract_part(innoextract.is_file(), pending));
        parts.push(components::layer_part(
            components::components_dir().join(layer::DLL).is_file(),
        ));
        let (lines, ok) = check::report(game, unpack.as_ref(), exe, &parts);
        for line in &lines {
            log_line(&format!("check: {line}"));
        }
        note(&lines.join("\n"), !ok);
        if ok { 0 } else { CHECK_FAILED }
    }
}
