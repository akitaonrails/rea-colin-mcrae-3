//! The settings window's content, drawn with the SDK's widgets
//! (`rea_launcher_ui`) in CMR3's colours (`look`). It runs anywhere, so
//! the window is tested headless on the host (egui_kittest); the Windows
//! binary's `ui` module hosts it in a real window with the pad, the
//! folder picker and the portrait texture.
//!
//! Three tabs keep it short: Play (the game folder, one choice of look and
//! the profile folder), Picture and Controls. Every control is a row of
//! [`config::OPTIONS`]: its label, and its one-line description under it,
//! which is also its accessible description.

use std::path::{Path, PathBuf};

use rea_launcher::cfg::spec::OptionSpec;
use rea_launcher_ui::egui::{self, Align2, Rect, Response, TextureHandle, pos2, vec2};
use rea_launcher_ui::spec::{mark_rendered, spec_row, toggle_text};
use rea_launcher_ui::theme::{body, decal, semibold};
use rea_launcher_ui::widgets::{self, Chip, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::config::{self, DisplayMode, Gamepad, Postfx, Scaler, Settings};
use crate::edition;
use crate::form::{self, Form, Look, Picture};
use crate::look::{self, ERROR, GOLD, INK, MUTED};
use crate::xidi;

pub const TITLE: &str = "Colin McRae Rally 3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];

    /// The tab `by` places along (wrapping), for LB and RB.
    pub fn step(self, by: isize) -> Tab {
        let at = Tab::ALL.iter().position(|&t| t == self).unwrap_or(0) as isize;
        Tab::ALL[(at + by).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

/// What the game folder holds, checked when its text changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Empty,
    Found(&'static str),
    /// A Rally_3PC.exe the launcher doesn't know: the DRM-free patcher
    /// converts an official 1.1 one; anything else is refused at Play.
    Unknown,
    Problem(String),
}

pub fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem("No such folder. Check the path, or use Browse… to pick it.".into());
    }
    let Some(exe) = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("rally_3pc.exe"))
        })
    }) else {
        return Check::Problem(
            "There is no Rally_3PC.exe in this folder. Choose the folder the game is installed in."
                .into(),
        );
    };
    match std::fs::read(&exe) {
        Ok(bytes) => match edition::identify(&bytes) {
            edition::Edition::DrmFree(name) => Check::Found(name),
            edition::Edition::NeedsPatching => Check::Unknown,
        },
        Err(e) => Check::Problem(format!(
            "Can't read Rally_3PC.exe ({e}). Check that the file isn't open elsewhere and that you may read it."
        )),
    }
}

/// Everything the window shows and edits.
pub struct State {
    pub form: Form,
    pub tab: Tab,
    pub check: Check,
    checked: String,
    pub screen: Option<(u32, u32)>,
    resolutions: Vec<(Option<(u32, u32)>, String)>,
    /// Xbox-style pads connected when the window opened.
    pub pads: usize,
    /// Shows the Linux-only rows (DXVK, scaler, post-processing).
    pub linux: bool,
    pub error: Option<String>,
}

impl State {
    /// The window over a settings file's text (the template on a first
    /// run, which picks the recommended look).
    pub fn new(text: &str, linux: bool, screen: Option<(u32, u32)>, pads: usize) -> State {
        let settings = Settings::parse_file(text).0;
        let form = if Form::is_first_run(&settings) {
            Form::first_run(&settings, screen)
        } else {
            Form::new(&settings, screen)
        };
        State {
            check: check_game(&form.game),
            checked: form.game.clone(),
            form,
            tab: Tab::Play,
            screen,
            resolutions: form::resolutions(screen),
            pads,
            linux,
            error: None,
        }
    }

    /// Play is possible: a game folder with a Rally_3PC.exe.
    pub fn ready(&self) -> bool {
        matches!(self.check, Check::Found(_) | Check::Unknown)
    }

    /// Checks the game folder again after its text changed.
    pub fn refresh(&mut self) {
        if self.form.game != self.checked {
            self.checked = self.form.game.clone();
            self.check = check_game(&self.form.game);
        }
    }
}

/// The folder picker: a title in, a folder (or nothing) out.
pub type Pick<'a> = &'a mut dyn FnMut(&str) -> Option<PathBuf>;

/// Draws the whole window; true when Play was pressed (the caller saves).
pub fn draw(
    ui: &mut egui::Ui,
    state: &mut State,
    portrait: Option<&TextureHandle>,
    pick: Pick<'_>,
) -> bool {
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, look::BASE);
    let layout = Layout::new(full, look::PHOTO_W);
    draw_photo(ui, layout.photo, portrait);
    widgets::stripes(ui.painter(), full, &look::stripes());

    ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
        ui.label(egui::RichText::new(TITLE).font(decal(46.0)).color(INK));
        ui.add_space(-6.0);
        status(ui, state);
        ui.add_space(6.0);
        widgets::tabs_bar(
            ui,
            &[
                (Tab::Play, "Play"),
                (Tab::Picture, "Picture"),
                (Tab::Controls, "Controls"),
            ],
            &mut state.tab,
        );
        ui.add_space(14.0);
        widgets::tab_body(ui, state.tab, |ui| match state.tab {
            Tab::Play => play_tab(ui, state, pick),
            Tab::Picture => picture_tab(ui, state),
            Tab::Controls => controls_tab(ui, state),
        });
    });
    let hint = if state.ready() {
        "Your game folder is only read; everything the launcher adds goes to its own copy."
    } else {
        "Play needs the game folder: choose it in the Play tab."
    };
    let play = widgets::footer(ui, layout.footer, hint, state.ready());
    let play = state.ready() && play.clicked();
    state.refresh();
    play
}

fn draw_photo(ui: &egui::Ui, rect: Rect, portrait: Option<&TextureHandle>) {
    let p = ui.painter();
    // Fill the height; keep the left of the picture (Colin's face).
    widgets::hero_image(p, rect, portrait, Crop::Start, look::BASE);
    // The caption sits on a solid band, never on the photo itself.
    let band = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 82.0), rect.max);
    p.rect_filled(band, 0.0, look::BASE);
    let x = rect.left() + 20.0;
    let y = rect.bottom() - 16.0;
    p.text(
        pos2(x, y),
        Align2::LEFT_BOTTOM,
        "Photo: Carolyn (Flickr), CC BY 2.0",
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

fn status(ui: &mut egui::Ui, state: &State) {
    let (text, color) = match (&state.error, &state.check) {
        (Some(e), _) => (e.clone(), ERROR),
        (None, Check::Found(_)) => ("Ready. Press Play or Enter.".to_string(), MUTED),
        (None, Check::Unknown) => (
            "Ready. This executable is checked when you press Play.".to_string(),
            MUTED,
        ),
        (None, _) => (
            "First, show the launcher where the game is.".to_string(),
            MUTED,
        ),
    };
    ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
}

/// Names a control for screen readers by its option row.
fn describe(ui: &egui::Ui, response: &Response, row: &OptionSpec) {
    let (label, description) = (row.label, row.description);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label(label.to_owned());
        node.set_description(description.to_owned());
    });
}

/// A row of chips for an option whose unset value ("As the game") is a
/// choice of its own: label, chips, then the row's description.
fn option_chips<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    key: &'static str,
    chips: &[Chip<'_, T>],
    value: &mut T,
) -> Response {
    let row = config::option(key);
    mark_rendered(ui.ctx(), row.key);
    let mut response = None;
    ui.vertical(|ui| {
        ui.horizontal_top(|ui| {
            widgets::field(ui, row.label);
            let r = widgets::chips_with(ui, chips, value);
            describe(ui, &r, row);
            response = Some(r);
        });
        note_under(ui, row.description);
    });
    response.expect("drawn")
}

/// A description under a row, indented to the controls.
fn note_under(ui: &mut egui::Ui, text: &str) {
    ui.horizontal_top(|ui| {
        ui.add_space(rea_launcher_ui::theme::metrics(ui.ctx()).field_width + 8.0);
        ui.vertical(|ui| widgets::note(ui, text));
    });
    ui.add_space(6.0);
}

fn play_tab(ui: &mut egui::Ui, state: &mut State, pick: Pick<'_>) {
    let game = config::option("game");
    mark_rendered(ui.ctx(), game.key);
    widgets::heading(ui, game.label);
    let game_hint = if state.linux {
        "The folder with your Rally_3PC.exe, e.g. Z:\\home\\you\\Games\\Colin McRae Rally 3"
    } else {
        "The folder with your installed Rally_3PC.exe"
    };
    if widgets::folder_field(ui, &mut state.form.game, game_hint, || {
        pick("Choose the folder with your Rally_3PC.exe")
    }) {
        state.error = None;
        state.refresh();
    }
    let (note, color) = match &state.check {
        Check::Empty => (
            "The game folder is only read, never changed.".to_string(),
            MUTED,
        ),
        Check::Found(name) => (format!("Found: {name}"), GOLD),
        Check::Unknown => (edition::UNKNOWN_NOTE.to_string(), GOLD),
        Check::Problem(p) => (p.clone(), ERROR),
    };
    ui.add(egui::Label::new(egui::RichText::new(note).font(body(13.0)).color(color)).wrap());
    ui.add_space(10.0);

    widgets::heading(ui, "Look");
    ui.add_space(6.0);
    let enhanced_note = match Picture::enhanced(state.screen).resolution {
        Some((w, h)) => format!("{w}×{h}, widescreen, HD menus."),
        None => "Widescreen at the desktop size, HD menus.".to_string(),
    };
    let best = form::recommended(state.screen);
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
            "1280×800, the Deck's own screen.".to_string(),
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
                selected: state.form.look == look,
                badge: (look == best).then_some("Recommended"),
                title_size: 0.0,
            };
            if widgets::look_card(ui, vec2(w, 104.0), &card).clicked() {
                state.form.set_look(look, state.screen);
                if look == Look::Custom {
                    state.tab = Tab::Picture;
                }
            }
        }
    });
    ui.add_space(14.0);

    let saves = config::option("saves");
    mark_rendered(ui.ctx(), saves.key);
    widgets::heading(ui, saves.label);
    widgets::folder_field(
        ui,
        &mut state.form.saves,
        "Saved Games\\Colin McRae Rally 3 (the default)",
        || pick("Choose where your profile is kept"),
    );
    widgets::note(
        ui,
        "Leave empty for the default. The game's CMR3 profile file is copied in before \
         the game starts and back when it ends.",
    );
}

fn chips_of<T: Clone>(choices: &[(T, String)]) -> Vec<Chip<'_, T>> {
    choices
        .iter()
        .map(|(v, label)| Chip::new(v.clone(), label))
        .collect()
}

fn owned<T: Copy>(choices: &[(T, &str)]) -> Vec<(T, String)> {
    choices.iter().map(|(v, l)| (*v, l.to_string())).collect()
}

/// A spec row edited through its `launcher.cfg` text.
fn spec_choice(ui: &mut egui::Ui, key: &str, current: &str) -> Option<String> {
    let mut text = current.to_string();
    spec_row(ui, config::option(key), &mut text)
        .changed()
        .then_some(text)
}

fn picture_tab(ui: &mut egui::Ui, state: &mut State) {
    let before = state.form.picture;
    let p = &mut state.form.picture;
    let resolutions = form::with_saved(&state.resolutions, p.resolution, |(w, h)| {
        format!("{w}×{h}")
    });
    if option_chips(ui, "resolution", &chips_of(&resolutions), &mut p.resolution).changed() {
        p.original = p.resolution == Some((640, 480));
    }
    if let Some(mode) = spec_choice(ui, "display_mode", p.display_mode.key()) {
        p.display_mode = DisplayMode::ALL
            .into_iter()
            .find(|m| m.key() == mode)
            .unwrap_or(DisplayMode::Fullscreen);
    }
    option_chips(ui, "vsync", &chips_of(&owned(&form::VSYNCS)), &mut p.vsync);
    option_chips(
        ui,
        "anisotropic",
        &chips_of(&owned(&form::ANISOTROPICS)),
        &mut p.anisotropic,
    );
    option_chips(ui, "fsaa", &chips_of(&owned(&form::FSAAS)), &mut p.fsaa);
    let fovs = form::with_saved(&owned(&form::FOVS), p.fov, |f| format!("{f}°"));
    option_chips(ui, "fov", &chips_of(&fovs), &mut p.fov);
    option_chips(
        ui,
        "shadows",
        &chips_of(&owned(&form::SHADOWS)),
        &mut p.shadows,
    );
    if let Some(on) = spec_choice(ui, "hd_ui", toggle_text(p.hd_ui)) {
        p.hd_ui = on == "on";
    }
    if state.linux {
        if let Some(on) = spec_choice(ui, "dxvk", toggle_text(state.form.dxvk)) {
            state.form.dxvk = on == "on";
        }
        if let Some(scaler) = spec_choice(ui, "scaler", state.form.scaler.key()) {
            state.form.scaler = Scaler::ALL
                .into_iter()
                .find(|s| s.key() == scaler)
                .unwrap_or(Scaler::Linear);
        }
        let needs = (!state.form.dxvk).then_some("Needs Vulkan (DXVK) on");
        let postfx: Vec<_> = form::POSTFX
            .iter()
            .map(|(v, l)| Chip::new(*v, l).disabled(needs.filter(|_| *v != Postfx::Off)))
            .collect();
        option_chips(ui, "postfx", &postfx, &mut state.form.picture.postfx);
    }
    if state.form.picture != before {
        state.form.look = Form::look_of(&state.form.picture, state.screen);
    }
}

fn controls_tab(ui: &mut egui::Ui, state: &mut State) {
    if let Some(preset) = spec_choice(ui, "gamepad", state.form.gamepad.key()) {
        state.form.gamepad = Gamepad::ALL
            .into_iter()
            .find(|g| g.key() == preset)
            .unwrap_or_default();
    }
    widgets::note(ui, xidi::note(state.form.gamepad));
    ui.add_space(12.0);
    let pads = match state.pads {
        0 => "No Xbox-style (XInput) pad is connected now. Under Steam the Deck's controls \
              count as one; on 8BitDo pads use X-input mode."
            .to_string(),
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
    ui.add_space(12.0);
    if let Some(on) = spec_choice(ui, "gamepad_log", toggle_text(state.form.gamepad_log)) {
        state.form.gamepad_log = on == "on";
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::cfg::spec;
    use rea_launcher_ui::egui::accesskit::Role;
    use rea_launcher_ui::testing::{self, Harness, Queryable};

    const SCREEN: Option<(u32, u32)> = Some((2560, 1440));

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cmr3-window-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    struct App {
        state: State,
        played: u32,
    }

    fn open(text: &str, linux: bool) -> Harness<'static, App> {
        let app = App {
            state: State::new(text, linux, SCREEN, 0),
            played: 0,
        };
        testing::harness(&look::PALETTE, &look::METRICS, app, |ui, app| {
            if draw(ui, &mut app.state, None, &mut |_| None) {
                app.played += 1;
            }
        })
    }

    /// Scrolls a control into view, then clicks it (a long tab scrolls).
    fn press(ui: &mut Harness<'static, App>, role: Role, label: &str) {
        ui.get_by_role_and_label(role, label).scroll_to_me();
        ui.run();
        ui.get_by_role_and_label(role, label).click();
        ui.run();
    }

    fn tab(ui: &mut Harness<'static, App>, name: &str) {
        ui.get_by_role_and_label(Role::Tab, name).click();
        ui.run();
    }

    fn saved(ui: &Harness<'static, App>) -> Settings {
        let (s, warnings) = Settings::parse_file(&ui.state().state.form.save(""));
        assert!(warnings.is_empty(), "{warnings:?}");
        s
    }

    #[test]
    fn every_option_has_a_control_in_the_window() {
        let mut ui = open("", true);
        for name in ["Picture", "Controls", "Play"] {
            tab(&mut ui, name);
        }
        spec::assert_parity(
            config::OPTIONS,
            &rea_launcher_ui::spec::rendered_keys(&ui.ctx),
        );
    }

    #[test]
    fn each_tab_renders_its_labelled_controls() {
        let mut ui = open("", true);
        ui.get_by_label(
            "The folder with your Rally_3PC.exe, e.g. Z:\\home\\you\\Games\\Colin McRae Rally 3",
        );
        ui.get_by_role_and_label(Role::RadioButton, "Enhanced");
        tab(&mut ui, "Picture");
        for key in [
            "resolution",
            "display_mode",
            "vsync",
            "anisotropic",
            "fsaa",
            "fov",
            "shadows",
            "dxvk",
            "scaler",
            "postfx",
        ] {
            let row = config::option(key);
            // vsync and anti-aliasing are tri-state chips (As the game).
            let role = if key == "dxvk" {
                Role::Switch
            } else {
                Role::RadioGroup
            };
            let node = ui.get_by_role_and_label(role, row.label);
            assert_eq!(
                testing::description(&node).as_deref(),
                Some(row.description),
                "{key}"
            );
        }
        ui.get_by_role_and_label(Role::Switch, "HD menus");
        tab(&mut ui, "Controls");
        ui.get_by_role_and_label(Role::RadioGroup, "Gamepad");
        ui.get_by_role_and_label(Role::Switch, "Gamepad log");
    }

    #[test]
    fn the_linux_rows_are_only_for_cmr3_play() {
        let mut ui = open("", false);
        tab(&mut ui, "Picture");
        assert!(ui.query_by_label("Upscaling").is_none());
        assert!(ui.query_by_label("Post-processing").is_none());
    }

    #[test]
    fn clicking_changes_the_saved_values() {
        let mut ui = open("vsync = on\n", true);
        tab(&mut ui, "Picture");
        let click = |ui: &mut Harness<'static, App>, label: &str| {
            press(ui, Role::RadioButton, label);
        };
        click(&mut ui, "Borderless");
        click(&mut ui, "4×");
        click(&mut ui, "110°");
        click(&mut ui, "Soft");
        click(&mut ui, "1080p");
        click(&mut ui, "FSR");
        click(&mut ui, "Cinematic");
        press(&mut ui, Role::Switch, "HD menus");
        let s = saved(&ui);
        assert_eq!(s.display_mode, Some(DisplayMode::Borderless));
        assert_eq!(s.anisotropic, Some(4));
        assert_eq!(s.fov, Some(110));
        assert_eq!(s.shadows, Some(crate::config::ShadowStyle::Soft));
        assert_eq!(s.resolution, Some((1920, 1080)));
        assert_eq!(s.scaler, Some(Scaler::Fsr));
        assert_eq!(s.postfx, Some(Postfx::Cinematic));
        assert_eq!(s.hd_ui, Some(true));
        assert_eq!(ui.state().state.form.look, Look::Custom);
        tab(&mut ui, "Controls");
        click(&mut ui, "Standard");
        press(&mut ui, Role::Switch, "Gamepad log");
        let s = saved(&ui);
        assert_eq!(s.gamepad, Some(Gamepad::Standard));
        assert_eq!(s.gamepad_log, Some(true));
    }

    #[test]
    fn as_the_game_chips_unset_and_opening_picture_changes_nothing() {
        // The Original look leaves anti-aliasing and vsync to the game;
        // opening the Picture tab must not turn that into Custom.
        let mut ui = open("resolution = original\n", false);
        assert_eq!(ui.state().state.form.look, Look::Original);
        tab(&mut ui, "Picture");
        assert_eq!(ui.state().state.form.look, Look::Original);
        assert_eq!(saved(&ui).fsaa, None);
        // On, then As the game again: unset in the file.
        let on: Vec<_> = ui
            .get_all_by_role_and_label(Role::RadioButton, "On")
            .collect();
        on[1].click(); // vsync's On is first, anti-aliasing's second
        ui.run();
        assert_eq!(saved(&ui).fsaa, Some(true));
        let unset: Vec<_> = ui
            .get_all_by_role_and_label(Role::RadioButton, "As the game")
            .collect();
        unset[2].click(); // vsync, anisotropic, anti-aliasing
        ui.run();
        assert_eq!(saved(&ui).fsaa, None);
        assert_eq!(ui.state().state.form.look, Look::Original);
    }

    #[test]
    fn a_saved_value_outside_the_chips_is_shown_and_kept() {
        let mut ui = open("fov = 45\nresolution = 1600x900\n", false);
        tab(&mut ui, "Picture");
        assert!(testing::checked(
            &ui.get_by_role_and_label(Role::RadioButton, "45°")
        ));
        assert!(testing::checked(
            &ui.get_by_role_and_label(Role::RadioButton, "1600×900")
        ));
        let s = saved(&ui);
        assert_eq!((s.fov, s.resolution), (Some(45), Some((1600, 900))));
    }

    #[test]
    fn post_processing_says_why_it_is_off_without_dxvk() {
        let mut ui = open("dxvk = off\n", true);
        tab(&mut ui, "Picture");
        let chip = ui.get_by_role_and_label(Role::RadioButton, "Colour");
        assert!(testing::disabled(&chip));
        assert_eq!(
            testing::description(&chip).as_deref(),
            Some("Needs Vulkan (DXVK) on")
        );
    }

    #[test]
    fn looks_set_the_picture_and_play_needs_a_game() {
        let mut ui = open("", false);
        assert_eq!(ui.state().state.form.look, Look::Enhanced, "first run");
        ui.get_by_role_and_label(Role::RadioButton, "Original 2003")
            .click();
        ui.run();
        assert_eq!(saved(&ui).resolution, Some((640, 480)));
        // Play is disabled until the folder holds the game, and says why.
        let play = ui.get_by_role_and_label(Role::Button, "Play");
        assert!(testing::disabled(&play));
        play.click();
        ui.run();
        assert_eq!(ui.state().played, 0);
        let game = temp("play-game");
        std::fs::write(game.join("Rally_3PC.exe"), b"not a known build").unwrap();
        ui.state_mut().state.form.game = game.display().to_string();
        ui.run();
        assert_eq!(ui.state().state.check, Check::Unknown);
        ui.get_by_role_and_label(Role::Button, "Play").click();
        ui.run();
        assert_eq!(ui.state().played, 1);
        std::fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn a_game_folder_with_a_hostile_name_is_found() {
        let game = temp("José/Jogos d'Ação/Colin McRae Rally 3");
        std::fs::write(game.join("RALLY_3PC.EXE"), b"x").unwrap();
        assert_eq!(check_game(&game.display().to_string()), Check::Unknown);
        assert!(matches!(
            check_game(&game.join("missing").display().to_string()),
            Check::Problem(_)
        ));
        std::fs::remove_dir_all(temp("José")).unwrap();
    }
}
