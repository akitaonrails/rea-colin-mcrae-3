#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
fn main() {
    let code = match app::run() {
        Ok(code) => code,
        Err(err) => {
            app::log_line(&format!("error: {err}"));
            win32::message("Colin McRae Rally 3 launcher", &err, true);
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
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    use cmr3_launcher::components;
    use cmr3_launcher::config::{self, Gamepad, Settings};
    use cmr3_launcher::edition;
    use cmr3_launcher::profile;
    use cmr3_launcher::runtime;

    use crate::{ui, win32};

    const EXE: &str = "Rally_3PC.exe";
    const TITLE: &str = "Colin McRae Rally 3 launcher";

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("CMR3Launcher")
    }

    pub fn log_line(line: &str) {
        let dir = home();
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("launcher.log"))
        {
            let _ = writeln!(f, "{line}");
        }
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

    fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
        let found = args.iter().any(|a| a == flag);
        args.retain(|a| a != flag);
        found
    }

    pub fn run() -> Result<u32, String> {
        log_line(concat!("CMR3Launcher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        let mut cli: Vec<String> = std::env::args().skip(1).collect();
        // The settings window: shown when started without options (a
        // double click), or alone with --settings (Linux: cmr3-play shows
        // it, then starts the game in gamescope). Any other option skips
        // it.
        let settings_only = take_flag(&mut cli, "--settings");
        let linux = take_flag(&mut cli, "--linux");
        let no_window = take_flag(&mut cli, "--no-window");
        if settings_only || (cli.is_empty() && !no_window) {
            // A window under Wine draws through OpenGL: GLX, not Wine 11's
            // default EGL (software on NVIDIA boxes without egl-x11).
            if win32::is_wine()
                && let Ok(true) = win32::prefer_glx()
            {
                log_line("Wine: OpenGL through GLX (UseEGL=N)");
            }
            let text =
                fs::read_to_string(&settings_path).unwrap_or_else(|_| config::TEMPLATE.into());
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
        let existing = fs::read_to_string(&settings_path).ok();
        let file_text = existing.as_deref().unwrap_or(config::TEMPLATE);
        let (saved, warnings) = Settings::parse_file(file_text);
        for w in &warnings {
            log_line(&format!("warning: {w}"));
        }
        let check_only = cli.iter().any(|a| a == "--check");
        cli.retain(|a| a != "--check");
        let settings = saved.clone().merged(Settings::parse_args(cli)?);

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
            edition::to_hex(&edition::sha256(&exe_bytes))
        ));
        let bundle = components::bundle_dir();
        if check_only {
            let parts: Vec<String> = components::present(&bundle)
                .iter()
                .map(|(name, there)| format!("{name}: {}", if *there { "yes" } else { "missing" }))
                .collect();
            win32::message(
                TITLE,
                &format!(
                    "Game folder: {}\nVersion: {}\n\nBundled parts:\n{}",
                    game.display(),
                    exe_name,
                    parts.join("\n")
                ),
                false,
            );
            return Ok(0);
        }

        let rt = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("Rally3"));
        runtime::check_separate(&game, &rt)?;
        let (brought, linked) = runtime::sync(&game, &rt)
            .map_err(|e| format!("cannot prepare {}: {e}", rt.display()))?;
        log_line(&format!(
            "runtime: {} ({} files, {} linked)",
            rt.display(),
            brought,
            linked
        ));

        // The player's profile lives outside the runtime copy; the game
        // reads and writes its own `CMR3`, kept in step before and after.
        let saves = settings.saves.clone().unwrap_or_else(default_saves);
        let rt_profile = rt.join(runtime::PROFILE);
        let save_profile = saves.join(runtime::PROFILE);
        // The newer copy wins. One here is newer when the last session
        // ended (a crash) before the launcher could save it.
        if rt_profile.is_file() {
            fs::create_dir_all(&saves)
                .map_err(|e| format!("cannot create {}: {e}", saves.display()))?;
            if runtime::mirror_file(&rt_profile, &save_profile)
                .map_err(|e| format!("cannot save {}: {e}", save_profile.display()))?
            {
                log_line(&format!("profile: saved to {}", save_profile.display()));
            }
        }
        if runtime::mirror_file(&save_profile, &rt_profile)
            .map_err(|e| format!("cannot restore the profile: {e}"))?
        {
            log_line("profile: restored");
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
        let wine = win32::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();

        // The bundled parts.
        match components::install_silentpatch(&rt, &bundle) {
            Ok(true) => log_line("SilentPatch installed"),
            Ok(false) => log_line(
                "warning: SilentPatch not bundled; the widescreen and compatibility fixes are off",
            ),
            Err(e) => log_line(&format!("warning: SilentPatch: {e}")),
        }
        let gamepad = settings.gamepad.unwrap_or_default();
        let gamepad_log = settings.gamepad_log == Some(true);
        let xidi = match components::install_xidi(&rt, &bundle, gamepad, gamepad_log) {
            Ok(true) => {
                log_line(&format!("Xidi installed ({gamepad:?})"));
                true
            }
            Ok(false) => {
                log_line(
                    "warning: Xidi not bundled; the gamepad runs through the game's own DirectInput",
                );
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
        match components::install_dxvk(&rt, &bundle, dxvk_on) {
            Ok(true) => log_line(if dxvk_on {
                "DXVK installed (d3d9 through Vulkan)"
            } else {
                "DXVK not active"
            }),
            Ok(false) => log_line(
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
        let resolution = settings.display_resolution().or_else(|| {
            // Unset: SilentPatch's default is the desktop's size; on this
            // screen that is what the settings window would have picked.
            win32::screen_size().filter(|_| settings.resolution.is_none())
        });
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
        if existing.is_none() || saved.game.as_ref() != Some(&game) {
            let text = config::set_key(file_text, "game", &game.display().to_string());
            if let Err(e) = fs::write(&settings_path, text) {
                log_line(&format!(
                    "warning: cannot write {}: {e}",
                    settings_path.display()
                ));
            }
        }

        if !wine {
            match win32::prefer_fast_gpu(&rt.join(EXE)) {
                Ok(true) => log_line("Windows: high-performance GPU for the game"),
                Ok(false) => {}
                Err(e) => log_line(&format!("warning: {e}")),
            }
        }
        log_line("starting the game");
        let code = win32::run(&rt.join(EXE), &rt)?;
        log_line(&format!("game exited with {code}"));
        if rt_profile.is_file() {
            match runtime::mirror_file(&rt_profile, &save_profile) {
                Ok(true) => log_line("profile: saved"),
                Ok(false) => {}
                Err(e) => log_line(&format!(
                    "warning: profile not saved to {}: {e}",
                    save_profile.display()
                )),
            }
        }
        Ok(code)
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
        runtime::install_copy(&exe, &backup)
            .map_err(|e| format!("cannot back up the executable: {e}"))?;
        for variant in ["efigs", "polish"] {
            let args =
                components::xdelta_args(&xdelta, &backup, variant, &exe.with_extension("new"));
            match win32::run_hidden(&xdelta, &args, rt) {
                Ok(true) => {
                    runtime::install_copy(&exe.with_extension("new"), &exe)
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
