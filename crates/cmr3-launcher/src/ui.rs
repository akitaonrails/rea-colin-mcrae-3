//! The settings window on screen: one window for Windows and for Linux
//! (shown through Wine by `cmr3-play`). What it shows and edits is the
//! library's `window` (tested headless); this module hosts it in eframe
//! with the gamepad, the folder picker and the portrait.

use std::path::{Path, PathBuf};

use cmr3_launcher::instance;
use cmr3_launcher::look::{self, PALETTE};
use cmr3_launcher::window::{self, State};
use rea_launcher_ui::eframe;
use rea_launcher_ui::egui::{self, Key, TextureHandle};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::theme;
use rea_launcher_ui::widgets;
use rea_launcher_ui::window::WindowFit;
use rea_launcher_win32::{dialog, display, wine, xinput};

use rea_launcher::installer::{Installer, UnpackJob, UnpackOutcome};
use rea_launcher_ui::unpack;

/// How the window was left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Play pressed: the settings are saved.
    Play,
    /// Closed without playing.
    Closed,
}

/// Shows the window over `settings_path` (its text, or the template for a
/// first run). `linux` adds the Linux-only rows. Errors when no window
/// can be made (no OpenGL); the caller then goes on without it.
pub fn show(settings_path: &Path, text: &str, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    // Under Wine the window is created at its final size, with Wine's own
    // DPI scale divided out (resizing after creation breaks the surface on
    // XWayland); on Windows it is fitted to the DPI on the first frame.
    // The scale is read first: it makes the process DPI aware under Wine.
    let wine = wine::is_wine();
    let wine_scale = display::wine_dpi_scale();
    let screen = display::screen_size();
    let fit = WindowFit::new(look::SIZE, wine, screen, wine_scale).with_native_fit(true);
    let options = fit.native_options(instance::WINDOW_TITLE);
    let path = settings_path.to_path_buf();
    let text = text.to_string();
    eframe::run_native(
        instance::WINDOW_TITLE,
        options,
        Box::new(|cc| {
            theme::install_with(&cc.egui_ctx, &PALETTE, &look::METRICS);
            Ok(Box::new(Window {
                state: State::new(&text, linux, screen, xinput::connected()),
                portrait: widgets::load_jpeg(
                    &cc.egui_ctx,
                    "portrait",
                    include_bytes!("../assets/portrait.jpg"),
                ),
                path,
                text,
                fit,
                pad: PadNav::default(),
                pending: Vec::new(),
                outcome: &mut outcome,
            }))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(outcome)
}

/// The unpack window over `job` (the SDK's, in the settings window's
/// looks and size): the game's title, which installer, the progress, and
/// "Stop unpacking". Errors when no window can be made; the caller then
/// waits without one.
pub fn show_unpack(found: &Installer, job: &mut dyn UnpackJob) -> Result<UnpackOutcome, String> {
    let wine = wine::is_wine();
    let wine_scale = display::wine_dpi_scale();
    let fit =
        WindowFit::new(look::SIZE, wine, display::screen_size(), wine_scale).with_native_fit(true);
    unpack::show(
        &PALETTE,
        &look::METRICS,
        unpack::Window {
            title: instance::WINDOW_TITLE,
            fit,
            hero_jpeg: Some(include_bytes!("../assets/portrait.jpg")),
            photo_width: look::PHOTO_W,
        },
        unpack::Unpacking::new(window::TITLE, found.build.name, found.build.unpacked),
        job,
        xinput::buttons,
    )
}

struct Window<'a> {
    state: State,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    text: String,
    fit: WindowFit,
    /// Gamepad edges, and the launcher's own steps (Play, tabs) waiting
    /// for this frame.
    pad: PadNav,
    pending: Vec<Nav>,
    outcome: &'a mut Outcome,
}

impl Window<'_> {
    fn play(&mut self, ctx: &egui::Context) {
        if !self.state.ready() {
            self.state.tab = window::Tab::Play;
            return;
        }
        let saved = self.state.form.save(&self.text);
        match rea_launcher::fsx::write_atomic(&self.path, saved) {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => self.state.error = Some(format!("Can't save the settings: {e}")),
        }
    }
}

impl eframe::App for Window<'_> {
    /// The gamepad, as keys: the d-pad moves the focus, A presses the
    /// control in focus; Start and LB/RB are handled in `ui`.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update(xinput::buttons());
        pad::inject(ctx, raw, &navs);
        self.pending.extend(navs);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.fit.first_frame(&ctx);
        let mut pick = |title: &str| dialog::pick_folder(title);
        if window::draw(ui, &mut self.state, self.portrait.as_ref(), &mut pick) {
            self.play(&ctx);
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        // A pad sends no events: look again soon.
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        // Taken, so a second egui pass in one frame can't play twice.
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(&ctx),
                Nav::NextTab => self.state.tab = self.state.tab.step(1),
                Nav::PreviousTab => self.state.tab = self.state.tab.step(-1),
                _ => {}
            }
        }
    }
}
