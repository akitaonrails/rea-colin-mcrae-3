#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn main() {
    use rea_launcher_win32::dialog::{self, Icon};
    let code = match app::run() {
        Ok(code) => code,
        Err(err) => {
            app::log_line(&format!("error: {err}"));
            dialog::message("Colin McRae Rally 3 launcher", &err, Icon::Error);
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

    use cmr3_launcher::components::{self, Installed};
    use cmr3_launcher::config::{self, Gamepad, Settings};
    use cmr3_launcher::edition;
    use cmr3_launcher::profile;
    use cmr3_launcher::runtime;
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, identify, paths};
    use rea_launcher_win32::dialog::{self, Icon};
    use rea_launcher_win32::process::Launch;
    use rea_launcher_win32::{display, gpu, wine};

    use crate::ui;

    const EXE: &str = "Rally_3PC.exe";
    const TITLE: &str = "Colin McRae Rally 3 launcher";

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR3Launcher")
    }

    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
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
            "Tell the launcher where the game is: run it once with\n\n  CMR3Launcher --game <folder with {EXE}>\n\nThe folder is remembered."
        ))
    }

    /// `--settings` exit codes for cmr3-play: closed without playing, or
    /// no window could be shown (play with the saved settings).
    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

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
        let (cli, switches) = Settings::parse_args(args)?;
        let switch = |name: &str| switches.iter().any(|s| s == name);
        // The settings window: shown when started without options (a
        // double click), or alone with --settings (Linux: cmr3-play shows
        // it, then starts the game in gamescope). Any other option skips
        // it.
        let settings_only = switch("--settings");
        let linux = switch("--linux");
        let no_window = switch("--no-window");
        let check_only = switch("--check");
        let other_options = given > switches.len() || check_only;
        if settings_only || (!other_options && !no_window) {
            // A window under Wine draws through OpenGL: GLX, not Wine 11's
            // default EGL (software on NVIDIA boxes without egl-x11).
            if wine::is_wine()
                && let Ok(true) = gpu::prefer_glx()
            {
                log_line("Wine: OpenGL through GLX (UseEGL=N)");
            }
            let text = read_settings(&settings_path)
                .ok()
                .flatten()
                .unwrap_or_else(|| config::TEMPLATE.into());
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
        let file_text = existing.as_deref().unwrap_or(config::TEMPLATE);
        let (saved, warnings) = Settings::parse_file(file_text);
        for w in &warnings {
            log_line(&format!("warning: {w}"));
        }
        let settings = saved.clone().merged(cli);

        let game = find_game(&settings)?;
        let exe_path = game.join(EXE);
        let exe_bytes =
            fs::read(&exe_path).map_err(|e| format!("cannot read {}: {e}", exe_path.display()))?;
        let exe_name = match edition::identify(&exe_bytes) {
            edition::Edition::DrmFree(name) => name.to_string(),
            edition::Edition::NeedsPatching => {
                "not one of the known DRM-free builds; the DRM-free patcher will try".into()
            }
        };
        log_line(&format!(
            "game: {} ({}) [sha256 {}]",
            game.display(),
            exe_name,
            identify::hex(&identify::sha256(&exe_bytes))
        ));
        let bundle = components::bundle_dir();
        if check_only {
            let parts: Vec<String> = components::present(&bundle)
                .iter()
                .map(|(name, there)| format!("{name}: {}", if *there { "yes" } else { "missing" }))
                .collect();
            dialog::message(
                TITLE,
                &format!(
                    "Game folder: {}\nVersion: {}\n\nBundled parts:\n{}",
                    game.display(),
                    exe_name,
                    parts.join("\n")
                ),
                Icon::Info,
            );
            return Ok(0);
        }

        let rt = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("Rally3"));
        paths::check_separate(&game, &rt)?;
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
            apply_drm_free(&rt, &bundle)?;
        } else {
            log_line("executable: already DRM-free");
        }

        // What we're running on, decided once: DXVK only helps under Wine,
        // and the display mode the game can honour depends on whether
        // gamescope already gives it a single screen.
        let wine = wine::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();

        // The bundled parts, and the DLL names Wine must then load from
        // the game folder.
        let mut natives = Vec::new();
        match components::install_silentpatch(&game, &rt, &bundle) {
            Ok(Installed::Done) => {
                natives.push("dsound");
                log_line("SilentPatch installed")
            }
            Ok(Installed::NotBundled) => log_line(
                "warning: SilentPatch not bundled; the widescreen and compatibility fixes are off",
            ),
            Ok(Installed::NotOurs(name)) => log_line(&not_ours("SilentPatch", name, &rt)),
            Err(e) => log_line(&format!("warning: SilentPatch: {e}")),
        }
        let gamepad = settings.gamepad.unwrap_or_default();
        let gamepad_log = settings.gamepad_log == Some(true);
        let xidi = match components::install_xidi(&game, &rt, &bundle, gamepad, gamepad_log) {
            Ok(Installed::Done) => {
                log_line(&format!("Xidi installed ({gamepad:?})"));
                if gamepad != Gamepad::Off {
                    natives.push("dinput8");
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
        // A new profile has no controller chosen: pick Xidi's, as the
        // player would in Options → Controls (see `profile`).
        if xidi && gamepad != Gamepad::Off {
            match profile::choose_first_controller_in(&rt_profile) {
                Ok(true) => log_line("profile: controller set to the gamepad"),
                Ok(false) => {}
                Err(e) => log_line(&format!("warning: profile controller: {e}")),
            }
        }
        let dxvk_on = settings.dxvk != Some(false) && wine;
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
        if settings.hd_ui == Some(true) {
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
        // Unset: SilentPatch's default is the desktop's size; on this
        // screen that is what the settings window would have picked.
        let resolution = settings.resolution.or_else(display::screen_size);
        // The display mode the game can actually honour here (see
        // `DisplayMode::effective`): exclusive fullscreen under raw Wine
        // chases the desktop's monitor list, so it plays borderless
        // instead — same size, no mode switch, no wrong display.
        let chosen = settings
            .display_mode
            .unwrap_or(config::DisplayMode::Fullscreen);
        let mode = chosen.effective(wine, gamescope);
        if mode != chosen {
            log_line(&format!(
                "display mode: {} plays borderless under Wine{} (exclusive fullscreen picks \
                 the wrong display when the desktop has several; install gamescope — cmr3-play \
                 uses it — for true fullscreen)",
                chosen.key(),
                if gamescope { " inside gamescope" } else { "" },
            ));
        }
        let mut ini_settings = settings.clone();
        ini_settings.display_mode = Some(mode);
        match components::apply_settings(&rt, &ini_settings, resolution) {
            Ok(log) => log_line(&format!("SilentPatchCMR3.ini: {}", log.join(", "))),
            Err(e) => log_line(&format!("warning: cannot write the settings: {e}")),
        }

        // Remember the game folder; the rest of the file stays as written.
        if writable && (existing.is_none() || saved.game.as_ref() != Some(&game)) {
            let text = config::set_key(file_text, "game", &game.display().to_string());
            if let Err(e) = fsx::write_atomic(&settings_path, text) {
                log_line(&format!(
                    "warning: cannot write {}: {e}",
                    settings_path.display()
                ));
            }
        }

        if !wine {
            match gpu::prefer_fast_gpu(&rt.join(EXE)) {
                Ok(true) => log_line("Windows: high-performance GPU for the game"),
                Ok(false) => {}
                Err(e) => log_line(&format!("warning: {e}")),
            }
        }
        // Plain Proton/Wine runs get the overrides cmr3-play would set.
        let overrides = wine.then(|| {
            let existing = std::env::var("WINEDLLOVERRIDES").ok();
            components::wine_overrides(existing.as_deref(), &natives)
        });
        if let Some(o) = &overrides {
            log_line(&format!("WINEDLLOVERRIDES={o}"));
        }
        log_line("starting the game");
        let mut launch = Launch::new(rt.join(EXE)).cwd(&rt);
        if let Some(o) = overrides {
            launch = launch.env("WINEDLLOVERRIDES", o);
        }
        let code = launch.run()?;
        log_line(&format!("game exited with {code}"));
        if rt_profile.is_file() {
            match mirror.after() {
                Ok(copied) if !copied.is_empty() => log_line("profile: saved"),
                Ok(_) => {}
                Err(e) => log_line(&format!(
                    "warning: profile not saved to {}: {e}",
                    save_profile.display()
                )),
            }
        }
        Ok(code)
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

    /// Applies the bundled DRM-free patch (CookiePLMonster's xdelta
    /// updates for the official 1.1 executables) to the runtime copy.
    fn apply_drm_free(rt: &Path, bundle: &Path) -> Result<(), String> {
        let xdelta = bundle.join("patcher/xdelta.exe");
        if !xdelta.is_file() {
            return Err(
                "this executable has its original disc copy protection, and the \
                 DRM-free patcher is not bundled.\n\nInstall the official 1.1 \
                 patch, then try again (see the project's README)."
                    .into(),
            );
        }
        let exe = rt.join(EXE);
        let backup = rt.join("Rally_3PC.exe.drm");
        fsx::install_copy(&exe, &backup)
            .map_err(|e| format!("cannot back up the executable: {e}"))?;
        for variant in ["efigs", "polish"] {
            let args =
                components::xdelta_args(&xdelta, &backup, variant, &exe.with_extension("new"));
            let run = Launch::new(&xdelta).args(&args).cwd(rt).hidden(true).run();
            match run.map(|code| code == 0) {
                Ok(true) => {
                    fsx::install_copy(&exe.with_extension("new"), &exe)
                        .map_err(|e| format!("cannot replace the executable: {e}"))?;
                    let _ = fs::remove_file(&backup);
                    let _ = fs::remove_file(exe.with_extension("new"));
                    log_line(&format!("DRM-free patch applied ({variant})"));
                    return Ok(());
                }
                Ok(false) => continue,
                Err(e) => return Err(e),
            }
        }
        let _ = fs::remove_file(&backup);
        let _ = fs::remove_file(exe.with_extension("new"));
        Err(
            "the DRM-free patch did not apply: this is not an official 1.1 \
             executable.\n\nInstall the official 1.1 patch (linked in the \
             README) and point the launcher at the game again."
                .into(),
        )
    }
}
