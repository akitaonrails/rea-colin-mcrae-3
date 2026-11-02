//! The settings window: one window for Windows and for Linux (shown
//! through Wine by `cmr3-play`). What it edits and saves is `form::Form`;
//! this module only draws it, with the SDK's widgets (`rea_launcher_ui`)
//! in CMR3's colours (`look`).
//!
//! Three tabs keep it short: Play (the game folder and one choice of look),
//! Picture and Controls.

use std::path::{Path, PathBuf};

use cmr3_launcher::config::Settings;
use cmr3_launcher::edition;
use cmr3_launcher::form::{self, Form, Look, Picture};
use cmr3_launcher::look::{self, ERROR, GOLD, INK, MUTED, PALETTE};
use cmr3_launcher::xidi;
use eframe::egui::{self, Align2, Key, Rect, TextureHandle, pos2, vec2};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::theme::{self, body, decal, semibold};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::{Layout, WindowFit};
use rea_launcher_win32::{dialog, display, wine, xinput};

const TITLE: &str = "Colin McRae Rally 3";

/// How the window was left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Play pressed: the settings are saved.
    Play,
    /// Closed without playing.
    Closed,
}

/// Shows the window over `settings_path` (its text, or the template for a
/// first run). `linux` adds the gamescope scaler and DXVK choices. Errors
/// when no window can be made (no OpenGL); the caller then goes on
/// without it.
pub fn show(settings_path: &Path, text: &str, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    // Under Wine the window is created at its final size, with Wine's own
    // DPI scale divided out (resizing after creation breaks the surface on
    // XWayland); on Windows it is fitted to the DPI on the first frame.
    // The scale is read first: it makes the process DPI aware under Wine.
    let wine = wine::is_wine();
    let wine_scale = display::wine_dpi_scale();
    let screen = display::screen_size();
    let fit = WindowFit::new(look::SIZE, wine, screen, wine_scale);
    let options = fit.native_options(TITLE);
    let path = settings_path.to_path_buf();
    let text = text.to_string();
    eframe::run_native(
        TITLE,
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx, &PALETTE);
            Ok(Box::new(Window::new(
                &cc.egui_ctx,
                path,
                text,
                linux,
                screen,
                fit,
                &mut outcome,
            )))
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(outcome)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];

    /// The tab `by` places along (wrapping), for LB and RB.
    fn step(self, by: isize) -> Tab {
        let at = Tab::ALL.iter().position(|&t| t == self).unwrap_or(0) as isize;
        Tab::ALL[(at + by).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

/// What the game folder holds, checked when its text changes.
enum Check {
    Empty,
    Found(&'static str),
    /// A folder with a Rally_3PC.exe the launcher doesn't know: the
    /// DRM-free patcher may still recognize it.
    Unknown,
    Problem(String),
}

fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem("No such folder.".into());
    }
    let Some(exe) = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("rally_3pc.exe"))
        })
    }) else {
        return Check::Problem("There is no Rally_3PC.exe in this folder.".into());
    };
    match std::fs::read(&exe) {
        Ok(bytes) => match edition::identify(&bytes) {
            edition::Edition::DrmFree(name) => Check::Found(name),
            edition::Edition::NeedsPatching => Check::Unknown,
        },
        Err(e) => Check::Problem(format!("Can't read Rally_3PC.exe: {e}")),
    }
}

struct Window<'a> {
    form: Form,
    tab: Tab,
    check: Check,
    checked: String,
    screen: Option<(u32, u32)>,
    resolutions: Vec<(Option<(u32, u32)>, String)>,
    pads: usize,
    linux: bool,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    text: String,
    error: Option<String>,
    fit: WindowFit,
    /// Gamepad edges, and the launcher's own steps (Play, tabs) waiting
    /// for this frame.
    pad: PadNav,
    pending: Vec<Nav>,
    outcome: &'a mut Outcome,
}

impl<'a> Window<'a> {
    fn new(
        ctx: &egui::Context,
        path: PathBuf,
        text: String,
        linux: bool,
        screen: Option<(u32, u32)>,
        fit: WindowFit,
        outcome: &'a mut Outcome,
    ) -> Window<'a> {
        let settings = Settings::parse_file(&text).0;
        let form = if Form::is_first_run(&settings) {
            Form::first_run(&settings, screen)
        } else {
            Form::new(&settings, screen)
        };
        let check = check_game(&form.game);
        Window {
            checked: form.game.clone(),
            check,
            form,
            tab: Tab::Play,
            screen,
            resolutions: form::resolutions(screen),
            pads: xinput::connected(),
            linux,
            portrait: widgets::load_jpeg(ctx, "portrait", include_bytes!("../assets/portrait.jpg")),
            path,
            text,
            error: None,
            fit,
            pad: PadNav::default(),
            pending: Vec::new(),
            outcome,
        }
    }

    fn ready(&self) -> bool {
        matches!(self.check, Check::Found(_) | Check::Unknown)
    }

    fn play(&mut self, ctx: &egui::Context) {
        if !self.ready() {
            self.tab = Tab::Play;
            return;
        }
        let saved = self.form.save(&self.text);
        match rea_launcher::fsx::write_atomic(&self.path, saved) {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => self.error = Some(format!("Can't save the settings: {e}")),
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
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, look::BASE);
        let layout = Layout::new(full, look::PHOTO_W);
        self.draw_photo(ui, layout.photo);
        widgets::stripes(ui.painter(), full, &look::stripes());

        ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
            ui.label(egui::RichText::new(TITLE).font(decal(46.0)).color(INK));
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            widgets::tabs_bar(
                ui,
                &[
                    (Tab::Play, "Play"),
                    (Tab::Picture, "Picture"),
                    (Tab::Controls, "Controls"),
                ],
                &mut self.tab,
            );
            ui.add_space(14.0);
            match self.tab {
                Tab::Play => self.play_tab(ui),
                Tab::Picture => self.picture_tab(ui),
                Tab::Controls => self.controls_tab(ui),
            }
        });
        let hint =
            "Your game folder is only read; everything the launcher adds goes to its own copy.";
        if widgets::footer(ui, layout.footer, hint, self.ready()).clicked() {
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
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PreviousTab => self.tab = self.tab.step(-1),
                _ => {}
            }
        }
        if self.form.game != self.checked {
            self.checked = self.form.game.clone();
            self.check = check_game(&self.form.game);
        }
    }
}

impl Window<'_> {
    fn draw_photo(&self, ui: &egui::Ui, rect: Rect) {
        let p = ui.painter();
        // Fill the height; keep the left of the picture (Colin's face).
        widgets::hero_image(p, rect, self.portrait.as_ref(), Crop::Start, look::BASE);
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 20.0;
        p.text(
            pos2(x, y),
            Align2::LEFT_BOTTOM,
            "Photo: Carolyn Williams, CC BY 2.0",
            body(11.0),
            MUTED,
        );
        p.text(
            pos2(x, y - 16.0),
            Align2::LEFT_BOTTOM,
            "1968–2007, World Rally Champion 1995",
            body(13.0),
            MUTED,
        );
        p.text(
            pos2(x, y - 34.0),
            Align2::LEFT_BOTTOM,
            "Colin McRae",
            semibold(19.0),
            INK,
        );
    }

    fn status(&self, ui: &mut egui::Ui) {
        let (text, color) = match (&self.error, &self.check) {
            (Some(e), _) => (e.clone(), ERROR),
            (None, Check::Found(_)) => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, Check::Unknown) => (
                "Executable not one of the known DRM-free builds; the bundled patcher will try."
                    .to_string(),
                MUTED,
            ),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Game folder");
        let game_hint = if self.linux {
            "The folder with your Rally_3PC.exe, e.g. Z:\\home\\you\\Games\\Colin McRae Rally 3"
        } else {
            "The folder with your installed Rally_3PC.exe"
        };
        if widgets::folder_field(ui, &mut self.form.game, game_hint, || {
            dialog::pick_folder("Choose the folder with your Rally_3PC.exe")
        }) {
            self.error = None;
        }
        let (note, color) = match &self.check {
            Check::Empty => (
                "The game folder is only read, never changed.".to_string(),
                MUTED,
            ),
            Check::Found(name) => (format!("Found: {name}"), GOLD),
            Check::Unknown => (
                "Executable not known; the bundled DRM-free patcher will try it.".into(),
                GOLD,
            ),
            Check::Problem(p) => (p.clone(), ERROR),
        };
        ui.label(egui::RichText::new(note).font(body(13.0)).color(color));
        ui.add_space(10.0);

        widgets::heading(ui, "Look");
        ui.add_space(6.0);
        let enhanced_note = match Picture::enhanced(self.screen).resolution {
            Some((w, h)) => format!("{w}×{h}, widescreen, HD UI."),
            None => "Widescreen at the desktop size, HD UI.".to_string(),
        };
        let best = form::recommended(self.screen);
        let cards = [
            (
                Look::Original,
                "Original 2003",
                "640×480, as the game shipped.".to_string(),
            ),
            (Look::Enhanced, "Enhanced", enhanced_note),
            (
                Look::SteamDeck,
                "Steam Deck",
                "1280×800, easy on the battery.".to_string(),
            ),
            (
                Look::Custom,
                "Custom",
                "Pick each setting in Picture.".to_string(),
            ),
        ];
        let gap = 10.0;
        let w = (ui.available_width() - 3.0 * gap) / 4.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (look, title, note) in cards {
                let card = LookCard {
                    title,
                    note: &note,
                    detail: None,
                    selected: self.form.look == look,
                    badge: (look == best).then_some("Recommended"),
                    title_size: 0.0,
                };
                if widgets::look_card(ui, vec2(w, 104.0), &card).clicked() {
                    self.form.set_look(look, self.screen);
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
        ui.add_space(14.0);

        widgets::heading(ui, "Profile");
        widgets::folder_field(
            ui,
            &mut self.form.saves,
            "Saved Games\\Colin McRae Rally 3 (the default)",
            || dialog::pick_folder("Choose where your profile is kept"),
        );
        widgets::note(
            ui,
            "Leave empty for the default. The game's CMR3 profile file is copied in before \
             the game starts and back when it ends.",
        );
    }

    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let before = self.form.picture;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let p = &mut self.form.picture;
            let resolutions = &self.resolutions;
            let row = |ui: &mut egui::Ui, name: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                ui.horizontal_top(|ui| {
                    widgets::field(ui, name);
                    ui.vertical(|ui| add(ui));
                });
            };
            row(ui, "Resolution", &mut |ui| {
                widgets::chips(ui, resolutions, &mut p.resolution, None);
                if p.resolution == Some((640, 480)) {
                    p.original = true;
                } else if p.resolution.is_some() {
                    p.original = false;
                }
            });
            row(ui, "Display mode", &mut |ui| {
                widgets::chips(ui, &form::DISPLAY_MODES, &mut p.display_mode, None);
            });
            row(ui, "Vsync", &mut |ui| {
                widgets::chips(ui, &form::VSYNCS, &mut p.vsync, None);
            });
            row(ui, "Anisotropic", &mut |ui| {
                widgets::chips(ui, &form::ANISOTROPICS, &mut p.anisotropic, None);
            });
            row(ui, "Field of view", &mut |ui| {
                widgets::chips(ui, &form::FOVS, &mut p.fov, None);
            });
            row(ui, "Shadows", &mut |ui| {
                widgets::chips(ui, &form::SHADOWS, &mut p.shadows, None);
            });
            row(ui, "", &mut |ui| {
                widgets::toggle(
                    ui,
                    &mut p.hd_ui,
                    "HD UI: fonts and race graphics redrawn for 4K (Bekoha)",
                    None,
                );
                let mut fsaa = p.fsaa.unwrap_or(false);
                widgets::toggle(
                    ui,
                    &mut fsaa,
                    "Anti-aliasing (its rendering bugs are fixed)",
                    None,
                );
                p.fsaa = Some(fsaa);
            });
            if self.linux {
                let scaler = &mut self.form.scaler;
                row(ui, "Scaler", &mut |ui| {
                    let options: Vec<_> = form::SCALERS.iter().map(|s| (s.0, s.1)).collect();
                    widgets::chips(ui, &options, scaler, None);
                    let note = form::SCALERS
                        .iter()
                        .find(|s| s.0 == *scaler)
                        .map_or("", |s| s.2);
                    widgets::note(ui, note);
                });
                let dxvk = self.form.dxvk;
                row(ui, "Post-processing", &mut |ui| {
                    widgets::chips(ui, &form::POSTFX, &mut self.form.picture.postfx, None);
                    if !dxvk {
                        widgets::note(ui, "Needs DXVK (it works on the game's Vulkan)");
                    }
                });
                row(ui, "", &mut |ui| {
                    widgets::toggle(
                        ui,
                        &mut self.form.dxvk,
                        "DXVK: draw through Vulkan instead of Wine's OpenGL translator",
                        None,
                    );
                });
            }
        });
        if self.form.picture != before {
            self.form.look = Form::look_of(&self.form.picture, self.screen);
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        let gamepad = &mut self.form.gamepad;
        ui.horizontal_top(|ui| {
            widgets::field(ui, "Gamepad");
            widgets::chips(ui, &form::GAMEPADS, gamepad, None);
        });
        widgets::note(ui, xidi::note(self.form.gamepad));
        ui.add_space(12.0);
        let pads = match self.pads {
            0 => "No Xbox-style (XInput) pad is connected now. Under Steam the Deck's controls count as one; on 8BitDo pads use X-input mode.".to_string(),
            1 => "One Xbox-style pad connected.".to_string(),
            n => format!("{n} Xbox-style pads connected."),
        };
        widgets::note(ui, &pads);
        ui.add_space(8.0);
        widgets::note(
            ui,
            "A new profile starts with the pad as its controller. An older one set to the \
             keyboard keeps that: pick the Xidi controller once in the game's Options > \
             Controls. In a race F5 hides the HUD.",
        );
    }
}
