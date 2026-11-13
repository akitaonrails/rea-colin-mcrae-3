//! The settings window's content, drawn with the SDK's widgets
//! (`rea_launcher_ui`) in CMR3's colours (`look`). It runs anywhere, so
//! the window is tested headless on the host (egui_kittest); the Windows
//! binary's `ui` module hosts it in a real window with the pad, the
//! folder picker and the portrait texture.
//!
//! Four tabs keep it short: Play (the game folder, one choice of look and
//! the profile folder), Picture, Controls and Wheel. Every control is a row
//! of [`config::OPTIONS`]: its label, and its one-line description under
//! it, which is also its accessible description. The Wheel tab is the
//! SDK's (`rea_launcher_ui::wheel`): its controls are the `wheel_*`
//! options' (saved in `launcher.cfg`); the pedal calibration it makes is
//! saved in `rea_wheel.ini` (`crate::wheel`).

use std::path::{Path, PathBuf};

use rea_launcher::wheel::rig::WheelDevices;
use rea_launcher_ui::egui::{self, Rect, Response, TextureHandle, pos2, vec2};
use rea_launcher_ui::spec::{mark_rendered, spec_row, toggle_text};
use rea_launcher_ui::theme::{body, decal};
use rea_launcher_ui::wheel::WheelTab;
use rea_launcher_ui::widgets::{self, CaptionStyle, Chip, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::config::{self, DisplayMode, Gamepad, Postfx, Scaler, Settings, ShadowStyle};
use crate::edition;
use crate::form::{self, Form, Look, Picture};
use crate::look::{self, ERROR, GOLD, INK, MUTED};
use crate::source;
use crate::xidi;

pub const TITLE: &str = "Colin McRae Rally 3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Play,
    Picture,
    Controls,
    Wheel,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Play, Tab::Picture, Tab::Controls, Tab::Wheel];

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
    /// An installer the launcher knows, unpacked at the first Play: what
    /// the window says about it (`source::installer_note`).
    Installer(String),
    /// A game the launcher unpacked from an installer: the note.
    Unpacked(String),
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
    let exe = match source::find(dir) {
        Ok(source::Found::Game(exe)) => exe,
        Ok(source::Found::Installer(found)) => {
            return Check::Installer(source::installer_note(&found));
        }
        Ok(source::Found::Nothing) => {
            return Check::Problem(
                "There is no Rally_3PC.exe in this folder, and no installer the launcher knows. \
                 Choose the folder the game is installed in, or the folder with its installer."
                    .into(),
            );
        }
        // An installer it doesn't know, or one with a part missing.
        Err(e) => return Check::Problem(e),
    };
    match std::fs::read(&exe) {
        Ok(bytes) => match edition::identify(&bytes) {
            edition::Edition::DrmFree(name) if source::is_unpacked_by_launcher(dir) => {
                Check::Unpacked(source::unpacked_note(name))
            }
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
    /// The Wheel tab.
    pub wheel: WheelTab,
}

impl State {
    /// The window over a settings file's text (the template on a first
    /// run, which picks the recommended look) and the pedal calibration's
    /// (`rea_wheel.ini`; `None`: none yet). A `look =` line shows as that
    /// look, and the window saves its values.
    pub fn new(
        text: &str,
        calibration: Option<&str>,
        linux: bool,
        screen: Option<(u32, u32)>,
        pads: usize,
    ) -> State {
        let settings = form::with_look(Settings::parse_file(text).0, Settings::default(), screen);
        let wheel = WheelTab::new(settings.wheel.setup.clone(), calibration);
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
            wheel,
        }
    }

    /// Play can start the game: [`State::ready`], and no pedal
    /// calibration running.
    pub fn can_play(&self) -> bool {
        self.ready() && !self.wheel.calibrating()
    }

    /// What Play saves: `launcher.cfg`'s new text over `text`, and the
    /// calibration's over `calibration` (`None`: nothing to write). Both
    /// keep every other line.
    pub fn save(&self, text: &str, calibration: &str) -> (String, Option<String>) {
        (
            self.wheel.setup.to_launcher_cfg(&self.form.save(text)),
            self.wheel.calibration_text(calibration),
        )
    }

    /// Play is possible: a game folder with a Rally_3PC.exe, or with an
    /// installer the launcher unpacks.
    pub fn ready(&self) -> bool {
        matches!(
            self.check,
            Check::Found(_) | Check::Unknown | Check::Installer(_) | Check::Unpacked(_)
        )
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
/// `devices` are the controllers the Wheel tab shows.
pub fn draw(
    ui: &mut egui::Ui,
    state: &mut State,
    portrait: Option<&TextureHandle>,
    devices: &mut dyn WheelDevices,
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
                (Tab::Wheel, "Wheel"),
            ],
            &mut state.tab,
        );
        ui.add_space(14.0);
        widgets::tab_body(ui, state.tab, |ui| match state.tab {
            Tab::Play => play_tab(ui, state, pick),
            Tab::Picture => picture_tab(ui, state),
            Tab::Controls => controls_tab(ui, state),
            // The SDK's tab; it marks the `wheel_*` options it shows.
            Tab::Wheel => state.wheel.draw(ui, devices),
        });
    });
    let hint = if !state.ready() {
        "Play needs the game folder: choose it in the Play tab."
    } else if state.wheel.calibrating() {
        "Finish calibrating the pedals in the Wheel tab, or stop it, to play."
    } else {
        "Your game folder is only read; everything the launcher adds goes to its own copy."
    };
    let play = widgets::footer(ui, layout.footer, hint, state.can_play());
    let play = state.can_play() && play.clicked();
    state.refresh();
    play
}

/// What the portrait shows, for screen readers.
pub const PHOTO_ALT: &str = "Colin McRae, smiling, in a cap and racing suit";

/// The portrait's credit (THIRD_PARTY.md), two lines to fit the strip.
pub const PHOTO_CREDIT: [&str; 2] = [
    "Photo: Carolyn Williams, CC BY 2.0",
    "(via Wikimedia Commons)",
];

fn draw_photo(ui: &mut egui::Ui, rect: Rect, portrait: Option<&TextureHandle>) {
    // Fill the height; keep the left of the picture (Colin's face).
    widgets::hero_image(ui.painter(), rect, portrait, Crop::Start, look::BASE);
    // The caption sits on a solid band, never on the photo itself.
    let band = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 104.0), rect.max);
    ui.painter().rect_filled(band, 0.0, look::BASE);
    widgets::photo_caption(
        ui,
        rect,
        PHOTO_ALT,
        &[
            ("Colin McRae", CaptionStyle::Title),
            ("1968–2007, World Rally Champion 1995", CaptionStyle::Text),
            (PHOTO_CREDIT[0], CaptionStyle::Credit),
            (PHOTO_CREDIT[1], CaptionStyle::Credit),
        ],
    );
}

fn status(ui: &mut egui::Ui, state: &State) {
    let (text, color) = match (&state.error, &state.check) {
        (Some(e), _) => (e.clone(), ERROR),
        (None, Check::Found(_) | Check::Unpacked(_)) => {
            ("Ready. Press Play or Enter.".to_string(), MUTED)
        }
        (None, Check::Installer(_)) => (
            "Ready. Play unpacks the game first, once; that takes a few minutes.".to_string(),
            MUTED,
        ),
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

/// A row of chips the SDK's `spec_row` can't draw: a choice list built at
/// run time (the screen's size, a saved value outside the steps) or chips
/// disabled one by one. Label, chips, then the row's description.
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
            widgets::name_control(&r, Some(row.label), Some(row.description));
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
        "The folder with your Rally_3PC.exe or the game's installer, e.g. \
         Z:\\home\\you\\Games\\Colin McRae Rally 3"
    } else {
        "The folder with your installed Rally_3PC.exe, or with the game's installer"
    };
    if widgets::folder_field(ui, &mut state.form.game, game.label, game_hint, || {
        pick("Choose the folder with your Rally_3PC.exe or the game's installer")
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
        Check::Installer(note) | Check::Unpacked(note) => (note.clone(), GOLD),
        Check::Problem(p) => (p.clone(), ERROR),
    };
    ui.add(egui::Label::new(egui::RichText::new(note).font(body(13.0)).color(color)).wrap());
    ui.add_space(10.0);

    // The look cards are the `look` option's control (`--look`).
    let looks = config::option("look");
    mark_rendered(ui.ctx(), looks.key);
    widgets::heading(ui, looks.label);
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
                disabled: None,
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
        saves.label,
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

/// The `launcher.cfg` text of an optional value: empty when unset.
fn unset_or(text: Option<&str>) -> &str {
    text.unwrap_or_default()
}

/// A tri-state toggle's value from its text (empty: unset).
fn toggle_of(text: &str) -> Option<bool> {
    match text {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
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
    // Toggles and choices with an "As the game" chip (the row's unset
    // label): the empty text leaves the key unset.
    if let Some(text) = spec_choice(ui, "vsync", unset_or(p.vsync.map(toggle_text))) {
        p.vsync = toggle_of(&text);
    }
    let level = p.anisotropic.map(|a| match a {
        0 => "off".to_string(),
        n => n.to_string(),
    });
    if let Some(text) = spec_choice(ui, "anisotropic", unset_or(level.as_deref())) {
        p.anisotropic = match text.as_str() {
            "" => None,
            "off" => Some(0),
            n => n.parse().ok(),
        };
    }
    if let Some(text) = spec_choice(ui, "fsaa", unset_or(p.fsaa.map(toggle_text))) {
        p.fsaa = toggle_of(&text);
    }
    let fovs = form::with_saved(&owned(&form::FOVS), p.fov, |f| format!("{f}°"));
    option_chips(ui, "fov", &chips_of(&fovs), &mut p.fov);
    if let Some(text) = spec_choice(ui, "shadows", unset_or(p.shadows.map(|s| s.key()))) {
        p.shadows = ShadowStyle::ALL.into_iter().find(|s| s.key() == text);
    }
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
    ui.add_space(8.0);
    widgets::note(
        ui,
        "Racing wheel and pedals: set them up in the Wheel tab. The pad keeps working \
         beside the wheel.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::cfg::spec;
    use rea_launcher::wheel::dinput::{Mode, Profile, Setup};
    use rea_launcher::wheel::rig::{Device, Fixed};
    use rea_launcher::wheel::{DeviceId, DeviceState, Kind, SteeringLock};
    use rea_launcher_ui::egui::accesskit::Role;
    use rea_launcher_ui::testing::{self, Harness, NodeT, Queryable};

    const SCREEN: Option<(u32, u32)> = Some((2560, 1440));

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cmr3-window-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    struct App {
        state: State,
        devices: Fixed,
        played: u32,
    }

    fn open(text: &str, linux: bool) -> Harness<'static, App> {
        open_with(text, None, linux, Vec::new())
    }

    /// The window over `launcher.cfg`'s and the calibration's texts, with
    /// these controllers attached.
    fn open_with(
        text: &str,
        calibration: Option<&str>,
        linux: bool,
        devices: Vec<Device>,
    ) -> Harness<'static, App> {
        let app = App {
            state: State::new(text, calibration, linux, SCREEN, 0),
            devices: Fixed {
                devices,
                ..Fixed::default()
            },
            played: 0,
        };
        testing::harness(&look::PALETTE, &look::METRICS, app, |ui, app| {
            if draw(ui, &mut app.state, None, &mut app.devices, &mut |_| None) {
                app.played += 1;
            }
        })
    }

    /// A force-feedback wheel resting centred, pedals up.
    fn csl() -> Device {
        let state = DeviceState {
            axes: vec![0, 32_767, 32_767, 0, 0, 0, 0, 0],
            buttons: vec![false; 12],
        };
        Device {
            id: DeviceId {
                name: "FANATEC CSL DD".into(),
                vid: 0x0eb7,
                pid: 0x0020,
                guid: "{csl}".into(),
            },
            kind: Kind::Wheel,
            axes: 8,
            buttons: 12,
            ffb: true,
            initial: state.clone(),
            state,
        }
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
        for name in ["Picture", "Controls", "Wheel", "Play"] {
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
        // The folder fields are named by their headings; the hint is the
        // description (SDK v0.3.0 folder_field).
        let game = ui.get_by_role_and_label(Role::TextInput, "Game folder");
        assert_eq!(
            testing::description(&game).as_deref(),
            Some(
                "The folder with your Rally_3PC.exe or the game's installer, e.g. \
                 Z:\\home\\you\\Games\\Colin McRae Rally 3"
            )
        );
        ui.get_by_role_and_label(Role::TextInput, "Profile folder");
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
    fn the_portrait_and_its_full_credit_reach_screen_readers() {
        let ui = open("", false);
        ui.get_by_role_and_label(Role::Image, PHOTO_ALT);
        // The caption is one label, its lines joined.
        ui.get_by_role_and_label(
            Role::Label,
            &format!(
                "Colin McRae. 1968–2007, World Rally Champion 1995. {}. {}",
                PHOTO_CREDIT[0], PHOTO_CREDIT[1]
            ),
        );
        assert_eq!(
            PHOTO_CREDIT.join(" "),
            "Photo: Carolyn Williams, CC BY 2.0 (via Wikimedia Commons)"
        );
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
        // The choice rows' unset chips (SDK spec_row): anisotropic and
        // shadows set, then unset again.
        press(&mut ui, Role::RadioButton, "8×");
        press(&mut ui, Role::RadioButton, "Sharp");
        let s = saved(&ui);
        assert_eq!(
            (s.anisotropic, s.shadows),
            (Some(8), Some(ShadowStyle::Sharp))
        );
        for i in [1, 4] {
            let unset: Vec<_> = ui
                .get_all_by_role_and_label(Role::RadioButton, "As the game")
                .collect();
            unset[i].click(); // vsync, anisotropic, anti-aliasing, fov, shadows
            ui.run();
        }
        let s = saved(&ui);
        assert_eq!((s.anisotropic, s.shadows), (None, None));
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

    /// `look = steam_deck` in launcher.cfg (or a script's `--look`
    /// saved there): the window opens on that card, and saving writes
    /// the look's values in place of the line.
    #[test]
    fn a_look_named_in_the_file_opens_on_its_card() {
        let ui = open("look = steam_deck\n", false);
        assert_eq!(ui.state().state.form.look, Look::SteamDeck);
        assert!(testing::checked(
            &ui.get_by_role_and_label(Role::RadioButton, "Steam Deck")
        ));
        let text = ui.state().state.form.save("look = steam_deck\n");
        assert!(!text.lines().any(|l| l.starts_with("look")), "{text}");
        let s = saved(&ui);
        assert_eq!((s.resolution, s.hd_ui), (Some((1280, 800)), Some(true)));
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

    #[test]
    fn a_folder_with_an_installer_can_be_played_and_says_so() {
        let mut ui = open("", false);
        let note = "Found: the installer of the Magipack repack (v4). Play unpacks it once \
                    into the launcher's own folder.";
        // The real repack's digest can't be made here: the note is the
        // one `source::installer_note` gives (tested there).
        ui.state_mut().state.check = Check::Installer(note.into());
        ui.state_mut().state.checked = ui.state().state.form.game.clone();
        ui.run();
        ui.get_by_label(note);
        ui.get_by_role_and_label(Role::Button, "Play").click();
        ui.run();
        assert_eq!(ui.state().played, 1);
    }

    #[test]
    fn an_unknown_installer_is_refused_with_its_digest() {
        let dir = temp("unknown-installer");
        std::fs::write(dir.join("setup.exe"), b"MZ Inno Setup Setup Data (5.5.0)").unwrap();
        let Check::Problem(text) = check_game(&dir.display().to_string()) else {
            panic!("an unknown installer is a problem");
        };
        assert!(
            text.contains("left alone") && text.contains("SHA-256"),
            "{text}"
        );
        let mut ui = open("", false);
        ui.state_mut().state.form.game = dir.display().to_string();
        ui.run();
        assert!(!ui.state().state.ready());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_game_the_launcher_unpacked_says_the_installer_is_no_longer_needed() {
        let dir = temp("unpacked");
        // The DRM-free executable is the game's own; a known name stands
        // in for it through the marker check only when it is DRM-free, so
        // an unknown build still reads as Unknown.
        std::fs::write(dir.join("Rally_3PC.exe"), b"x").unwrap();
        std::fs::write(dir.join(".rea-unpacked"), "ab").unwrap();
        assert_eq!(check_game(&dir.display().to_string()), Check::Unknown);
        let mut ui = open("", false);
        let note = source::unpacked_note("DRM-free v1.1");
        ui.state_mut().state.check = Check::Unpacked(note.clone());
        ui.state_mut().state.checked = ui.state().state.form.game.clone();
        ui.run();
        ui.get_by_label(&note);
        assert!(ui.state().state.ready());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_empty_folder_points_to_the_game_or_its_installer() {
        let dir = temp("nothing");
        let Check::Problem(text) = check_game(&dir.display().to_string()) else {
            panic!("an empty folder is a problem");
        };
        assert!(text.contains("installer"), "{text}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_unpack_window_names_the_repack_and_can_be_stopped() {
        use rea_launcher_ui::unpack::{self, Unpacking};
        let build = source::INSTALLERS[0].value;
        let state = Unpacking::new(TITLE, build.name, build.unpacked);
        let mut ui = testing::harness(&look::PALETTE, &look::METRICS, state, |ui, u| {
            unpack::draw_unpack(ui, u)
        });
        ui.get_by_label("Colin McRae Rally 3");
        ui.get_by_label(
            "Unpacking the game from the Magipack repack (v4). This happens once, on the first \
             start.",
        );
        ui.get_by_label(unpack::NOTE);
        let stop = ui.get_by_role_and_label(Role::Button, unpack::STOP);
        assert_eq!(
            testing::description(&stop).as_deref(),
            Some(unpack::STOP_DESCRIPTION)
        );
        stop.click();
        ui.run();
        assert!(ui.state().cancel);
    }

    /// The wheel setup the window saves in `launcher.cfg`.
    fn saved_wheel(ui: &Harness<'static, App>) -> Setup {
        let (cfg, _) = ui.state().state.save("", "");
        let (s, warnings) = Settings::parse_file(&cfg);
        assert!(warnings.is_empty(), "{warnings:?}");
        s.wheel.setup
    }

    #[test]
    fn the_wheel_tab_saves_every_control_in_launcher_cfg() {
        let mut ui = open_with("", None, false, vec![csl()]);
        tab(&mut ui, "Wheel");
        // Wheel support's On is the first "On", at the top of the tab.
        ui.get_all_by_role_and_label(Role::RadioButton, "On")
            .next()
            .unwrap()
            .click();
        ui.run();
        assert_eq!(saved_wheel(&ui).mode, Mode::On);
        // The tab's own rows (SDK labels).
        let clicks: [(Role, &str); 6] = [
            (Role::RadioButton, "900°"),
            (Role::RadioButton, "4 %"),
            (Role::RadioButton, "Mild"),
            (Role::Switch, "Invert brake"),
            (Role::RadioButton, "75 %"),
            (Role::Switch, "Reverse force feedback"),
        ];
        for (role, label) in clicks {
            press(&mut ui, role, label);
        }
        let w = saved_wheel(&ui);
        assert_eq!(w.wheel.lock, SteeringLock::Degrees(900));
        assert_eq!((w.deadzone, w.linearity, w.ffb.strength), (4, 40, 75));
        assert!(w.wheel.invert_brake && w.ffb.invert);
        ui.get_by_role_and_label(Role::RadioGroup, "Wheel support")
            .scroll_to_me();
        ui.run();
        ui.get_all_by_role_and_label(Role::RadioButton, "FANATEC CSL DD")
            .next()
            .unwrap()
            .click();
        ui.run();
        assert_eq!(saved_wheel(&ui).wheel.device, "{csl}");
        // Its Off is the first (the strength row has one too).
        ui.get_all_by_role_and_label(Role::RadioButton, "Off")
            .next()
            .unwrap()
            .click();
        ui.run();
        assert_eq!(saved_wheel(&ui).mode, Mode::Off);
        assert_eq!(ui.state().state.save("", "").1, None, "no calibration yet");
    }

    #[test]
    fn the_tab_opens_on_launcher_cfg_and_keeps_the_calibration_file() {
        let csl = csl();
        let calibration = Profile::guess(csl.id, &csl.initial, None).to_cfg("# mine\n", "");
        let calibration = calibration.as_str();
        let ui = open_with(
            "wheel = off\nwheel_lock = 270\n",
            Some(calibration),
            false,
            Vec::new(),
        );
        let setup = &ui.state().state.wheel.setup;
        assert_eq!(setup.mode, Mode::Off);
        assert_eq!(setup.wheel.lock, SteeringLock::Degrees(270));
        assert!(ui.state().state.wheel.profile.is_some());
        let (cfg, text) = ui.state().state.save("vsync = on\n", calibration);
        assert!(
            cfg.contains("vsync = on") && cfg.contains("wheel = off"),
            "{cfg}"
        );
        let text = text.expect("the calibration is saved again");
        assert!(text.starts_with("# mine\n"), "{text}");
        assert_eq!(Profile::from_cfg(&text), Profile::from_cfg(calibration));
        let ui = open("", false);
        assert_eq!(
            ui.state().state.wheel.setup.wheel.lock,
            SteeringLock::Degrees(540),
            "the rally default"
        );
    }

    /// LB/RB (and Ctrl+Tab) walk the four tabs, wrapping; the Wheel tab
    /// is last.
    #[test]
    fn tab_cycling_reaches_the_wheel_tab() {
        assert_eq!(Tab::Controls.step(1), Tab::Wheel);
        assert_eq!(Tab::Wheel.step(1), Tab::Play);
        assert_eq!(Tab::Play.step(-1), Tab::Wheel);
        let mut ui = open("", false);
        tab(&mut ui, "Controls");
        ui.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Tab);
        ui.run();
        assert_eq!(ui.state().state.tab, Tab::Wheel);
        ui.get_by_role_and_label(Role::RadioGroup, "Wheel support");
        let names: Vec<String> = ui
            .get_all_by_role(Role::Tab)
            .map(|t| t.accesskit_node().label().unwrap_or_default())
            .collect();
        assert_eq!(names, ["Play", "Picture", "Controls", "Wheel"]);
    }

    /// Keyboard and pad focus walk the Wheel tab's controls in order.
    #[test]
    fn the_keyboard_reaches_the_wheel_tabs_controls_in_order() {
        let mut ui = open_with("", None, false, vec![csl()]);
        tab(&mut ui, "Wheel");
        let mut seen: Vec<String> = Vec::new();
        for _ in 0..24 {
            ui.key_press(egui::Key::Tab);
            ui.run();
            if let Some(node) = ui.query_by(|n| n.is_focused()) {
                let label = node.accesskit_node().label().unwrap_or_default();
                if !seen.contains(&label) {
                    seen.push(label);
                }
            }
        }
        let at = |l: &str| {
            seen.iter()
                .position(|s| s == l)
                .unwrap_or_else(|| panic!("{l} in {seen:?}"))
        };
        assert!(at("Wheel support") < at("Steering lock"));
        assert!(at("Steering lock") < at("Calibrate pedals"));
        assert!(at("Calibrate pedals") < at("Test force feedback"));
    }

    #[test]
    fn play_waits_for_a_pedal_calibration() {
        let game = temp("calibrating");
        std::fs::write(game.join("Rally_3PC.exe"), b"x").unwrap();
        let text = format!("game = {}\n", game.display());
        let mut ui = open_with(&text, None, false, vec![csl()]);
        tab(&mut ui, "Wheel");
        press(&mut ui, Role::Button, "Calibrate pedals");
        assert!(ui.state().state.wheel.calibrating());
        let play = ui.get_by_role_and_label(Role::Button, "Play");
        assert!(testing::disabled(&play));
        assert_eq!(
            testing::description(&play).as_deref(),
            Some("Finish calibrating the pedals in the Wheel tab, or stop it, to play.")
        );
        press(&mut ui, Role::Button, "Stop calibrating");
        assert!(ui.state().state.can_play());
        std::fs::remove_dir_all(game).unwrap();
    }
}
