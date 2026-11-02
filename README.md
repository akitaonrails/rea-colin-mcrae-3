# Colin McRae Rally 3 — play it today

> **Work in progress.** The pieces it is built from (SilentPatch, Xidi,
> DXVK) are proven on this machine's gaming setup; the launcher itself is
> new and needs testing on real machines. Help is welcome, especially from
> **Windows** players: try it and
> [report what happens](https://github.com/akitaonrails/rea-colin-mcrae-3/issues).

Play your own copy of the 2003 PC game **Colin McRae Rally 3** on a modern
computer: Windows 10/11, or Linux through Steam (Proton), Wine or the
AppImage.

You point the launcher at your installed game and press **Play**. The rest
— disc-protection removal, widescreen, the 4K interface, the gamepad, the
Vulkan renderer — is bundled and configured for you. Your game folder is
never changed: the launcher works on its own private copy.

[![Video: the game's intro, then a gravel stage in Japan at 4K with the Cinematic look and a gamepad (1:56, click to play)](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/rea-colin-mcrae-3-demo-v012-poster.jpg)](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/rea-colin-mcrae-3-demo-v012.mp4)

*Click to watch (1:56): a stage on Linux through the AppImage, with the
Cinematic look.*

## The launcher

| Play | Picture | Controls |
| --- | --- | --- |
| ![The Play tab with game folder and look presets](docs/images/launcher-play.jpg) | ![The Picture tab with display and rendering options](docs/images/launcher-picture.jpg) | ![The Controls tab with the Rally gamepad preset](docs/images/launcher-controls.jpg) |

## Problems it solves

What goes wrong when you install the 2003 game on a computer today, and
what the launcher does about it:

| Problem | Fix |
| --- | --- |
| The game won't start on Windows 10/11: its SecuROM/SafeDisc disc protection is blocked since 2015, and Wine can't run it either. | The official 1.1 executable is converted to the DRM-free one of the later re-releases (SilentPatch project's patches). |
| A stretched 4:3 picture, a short list of resolutions, an interface placed for 640×480. | Any resolution and aspect ratio, with the interface repositioned for wide screens (SilentPatch). |
| A tiny, blurry interface at 4K. | Bekoha's HD UI, redrawn for 4K. |
| Rendering bugs on modern GPUs and CPUs: a flickering sun that hitches with anti-aliasing, broken water reflections, split-screen and timing faults on multi-core CPUs. | Fixed by SilentPatch. |
| Modern pads barely work: through DirectInput both triggers share one axis (no left-foot braking), a new profile has no controller chosen, the game's default bindings point steering at the right stick, and the menus ignore the pad. | Xidi turns the pad into a proper game controller; new profiles start with the pad chosen, steering on the left stick and the pedals on the triggers; the d-pad and A work the menus. |
| Under Wine or Proton the game crawls at 4K (about 10 FPS) through Wine's OpenGL translator. | DXVK draws it through Vulkan, at full speed. |
| Under Wine, fullscreen opens on the wrong monitor or comes out cropped. | gamescope gives the game a screen of its own; without it, borderless. |
| Under Wine, a fault in Wine's video playback froze the game behind a hidden debugger, so it couldn't even quit. | The debugger and its crash dialog are off: the game ends cleanly instead. |
| A flat, washed-out picture with grey blacks. | On Linux, the Colour and Cinematic looks restore real blacks and contrast. |
| Patching the game folder risks the copy you own. | Your folder is only read; the launcher works on its own private copy. |

## What it adds to the original game

**Runs on today's computers**

- Converts an official 1.1 executable's disc protection (SecuROM/SafeDisc,
  disabled by Windows since 2015 and unsupported under Wine) to the
  DRM-free code the late re-releases shipped, using the SilentPatch
  project's xdelta updates.
- Every display resolution and **any aspect ratio**: 16:9 shows more to
  the sides, unstretched, with the whole interface repositioned for wide
  screens (all of this is SilentPatch, bundled).
- Fullscreen, windowed or borderless; vsync; your refresh rate.
- Under Wine or Proton, the game draws through **DXVK** (Vulkan): the
  difference between native speed and about 10 FPS at 4K. On Windows the
  system's own Direct3D is used.

**The 4K picture**

- The **HD UI** (by Bekoha): font atlases and race graphics redrawn for
  4K, for every language the game has.
- 16× anisotropic filtering by default, sharper car shadows, fixed water
  reflections, sun rendering that no longer flickers or hitches with
  anti-aliasing.
- Field-of-view control for the chase and interior cameras.
- On Linux, the **Colour** and **Cinematic** looks fix the game's
  washed-out picture: real blacks and fuller contrast, then sharpening.

![The same spot of a stage before and after: the washed-out original on the left, the Cinematic look's deeper blacks and richer colour on the right](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/rea-colin-mcrae-3-cinematic-before-after.jpg)

![Colin McRae Rally 3 running at 4K with the widescreen HD interface](docs/images/gameplay.jpg)

**Gamepads**

- **Xidi** (bundled) makes modern XInput pads a game controller in the
  game's DirectInput, with a mapping validated on real hardware: the
  triggers are the pedals (both at once — left-foot braking), resting
  thumbs can't open the throttle, and pads can be swapped while the game
  runs. The game's menus answer only the keyboard, so the d-pad and A
  also press the arrow keys and Enter.
- A new profile starts with the pad chosen in Options → Controls and
  vibration at Medium; a profile you set up yourself is left as it is.

**Comfort**

- Your profile (the game's `CMR3` file with your progress) lives in a
  folder you choose, outside the game.
- Your game folder is never modified; a manifest keeps the launcher's
  copy up to date with what changed in yours.
- A settings window, the same on Windows and Linux (larger on big
  screens), with one-click looks (Original 2003, Enhanced, Steam Deck)
  and a commented settings file.

This project contains **no game files**. You need your own copy of the
game.

## 1. The game you need

**Colin McRae Rally 3 for Windows PC, installed and updated to the
official 1.1 patch.** The folder contains `Rally_3PC.exe` next to `Data`,
`fonts` and `shaders`.

- Original CDs, a backup of an installed game, or a digital re-release all
  work. **If your copy is still 1.0**, install the official 1.1 patch
  first ([download on PCGamingWiki](https://community.pcgamingwiki.com/files/file/2506-colin-mcrae-rally-3-patch/)).
- The DRM-free re-releases (the Polish eXtra Klasyka 2-CD, the German
  DVD) are already 1.1 and DRM-free.
- The launcher removes the disc protection from an official 1.1
  executable automatically. It does **not** crack anything else, and it
  cannot run v1.0.

## 2. Download

Download `CMR3Launcher-….zip` from the
[latest release](https://github.com/akitaonrails/rea-colin-mcrae-3/releases/latest)
and unpack it anywhere, for example into your game folder. It contains:

- `CMR3Launcher.exe` — the launcher
- `thirdparty/` — SilentPatch, Xidi, the DRM-free patcher, DXVK and the
  HD UI; keep it next to the launcher
- `linux/` — `cmr3-play`, the Linux helper for Wine and gamescope

On Linux without Steam you can instead download
`CMR3Launcher-…-x86_64.AppImage`, which brings its own Wine (see
[linux/README.md](linux/README.md)).

## 3. Play

1. Unpack the zip anywhere, for example into your game folder.
2. Double-click `CMR3Launcher.exe`. The settings window opens.
3. If the game folder is empty, press **Browse…** and pick the folder with
   your `Rally_3PC.exe`; the window shows what it found.
4. Choose a look: **Enhanced** (the first time's choice: your screen's
   size, widescreen, HD UI), **Steam Deck**, **Original 2003** or
   **Custom**, then press **Play** (or Enter, or Start on a gamepad).

The first start takes a moment while the launcher makes its private copy
of the game (about 3 GB — near-instant when your game folder and the
launcher's folder sit on one disk, because the big data files are
hard-linked). Your choices are remembered for next time.

On Linux, follow [linux/README.md](linux/README.md): the AppImage (no
Wine needed), Steam with Proton, or your own Wine with gamescope through
`linux/cmr3-play`.

## Settings

Everything is optional; the first run's Enhanced look gives you the
modern experience out of the box. The settings window covers all of it:
**Play** (the game folder, the look, your profile folder), **Picture**
and **Controls**, each control with a line saying what it changes. They
are kept in `launcher.cfg`, which you can also edit by hand. Starting
`CMR3Launcher.exe` with any option (for example `--resolution
1920x1080`) skips the window and plays straight away; `--no-window`
plays with the saved settings.

Every setting has the same name in the window, in `launcher.cfg` and as
a flag (`hd_ui` is `--hd-ui`). Unset means the game keeps its own value;
if you set one and later unset it, the game's earlier value comes back
unless you changed it in the game's own menu since.

| `launcher.cfg` / flag | In the window | What it does | Unset |
| --- | --- | --- | --- |
| `game` / `--game` | Play: Game folder | The folder with your `Rally_3PC.exe` (only read). Remembered. | found next to the launcher |
| `saves` / `--saves` | Play: Profile folder | Where your profile (`CMR3`) is kept. | `Saved Games\Colin McRae Rally 3` |
| `runtime` / `--runtime` | — (file and flag only) | Where the launcher's private copy goes (about 3 GB). | `%LOCALAPPDATA%\CMR3Launcher\Rally3` |
| `resolution` / `--resolution` | Picture: Resolution | Any size and aspect ratio; `original` is the 640×480 of 2003. | the desktop's size |
| `display_mode` / `--display-mode` | Picture: Display mode | `fullscreen`, `windowed` or `borderless` (fullscreen plays borderless under Wine without gamescope). | fullscreen |
| `vsync` / `--vsync` | Picture: VSync | `on`/`off`: wait for the screen's refresh (no tearing). | the game's (Enhanced: on) |
| `anisotropic` / `--anisotropic` | Picture: Texture sharpness | `off`, `2`, `4`, `8`, `16`: the road stays sharp into the distance. | the game's (Enhanced: 16) |
| `fsaa` / `--fsaa` | Picture: Smooth edges | `on`/`off` anti-aliasing (its rendering bugs are fixed by SilentPatch). | the game's (Enhanced: off) |
| `fov` / `--fov` | Picture: Field of view | 30–150 degrees in steps of 5, both cameras. | the game's (75) |
| `shadows` / `--shadows` | Picture: Car shadows | `sharp` (the 2005 look) or `soft` (2003's blur). | the game's |
| `hd_ui` / `--hd-ui` | Picture: HD menus | Bekoha's 4K interface. | off (Enhanced: on) |
| `dxvk` / `--dxvk` | Picture: Vulkan (DXVK), Linux | Draw through Vulkan instead of Wine's OpenGL translator. Ignored on Windows. | on |
| `scaler` / `--scaler` | Picture: Upscaling, Linux | gamescope's filter: `nearest`, `pixel`, `fsr` or `linear`. Read by `cmr3-play`. | `linear` |
| `postfx` / `--postfx` | Picture: Post-processing, Linux | `off`, `color` (real blacks, fuller contrast, sharpening) or `cinematic` (a stronger grade; ambient occlusion only if you add the MxAO shader, see `docs/ENHANCEMENTS.md`). Needs DXVK. Read by `cmr3-play`. | off |
| `gamepad` / `--gamepad` | Controls: Gamepad | `rally` (the triggers are the pedals; the d-pad and A work the menus), `standard`, or `off` (no Xidi). | `rally` |
| `gamepad_log` / `--gamepad-log` | Controls: Gamepad log | `on`: Xidi logs what the game asks and what each control sent, for bug reports. | off |

Only `--game` is remembered; the others override `launcher.cfg` for one
run (with `cmr3-play --play …` too, the gamescope ones included).
`CMR3Launcher.exe --help` prints this list (generated from the same
table as the window). `CMR3Launcher.exe --check --game <folder>` checks
a game folder, its executable (with its SHA-256) and the bundled parts
without playing: the report goes to the terminal and `launcher.log`, and
it exits 0 when ready, 1 when something is missing or unknown. Exit
codes for scripts: 2 for an unknown flag or a bad value; with
`--settings` (what `cmr3-play` uses), 0 Play pressed, 3 closed without
Play, 4 no window possible.

### Gamepad

Pick the **Rally** preset (the default) and play: left stick steers, the
triggers are the pedals, the d-pad and A work the menus. A new profile
starts with the pad chosen as its controller and vibration at Medium. A
profile from an earlier install that points at the keyboard keeps it:
choose the Xidi controller once in the game's Options → Controls. Under
Steam the Deck's controls arrive as an Xbox-style pad; on 8BitDo pads use
X-input mode. If you prefer the game's own handling, set `gamepad` to
`standard` (bind the pedals to the left stick's Y axis in the game's
control setup) or `off`.

## Where things are

| What | Where |
| --- | --- |
| Settings | `%LOCALAPPDATA%\CMR3Launcher\launcher.cfg` (a `launcher.cfg` next to `CMR3Launcher.exe` is used instead, for a portable setup) |
| Log | `%LOCALAPPDATA%\CMR3Launcher\launcher.log` |
| The launcher's copy of the game | `%LOCALAPPDATA%\CMR3Launcher\Rally3` |
| Your profile | `%USERPROFILE%\Saved Games\Colin McRae Rally 3` (or your `saves` setting) |

Under Wine and Proton these are inside the Wine prefix (for Steam:
`~/.steam/steam/steamapps/compatdata/<id>/pfx/drive_c/users/steamuser/…`).

## Troubleshooting

- **"This Rally_3PC.exe is not a version the launcher knows"** — your
  executable is neither a DRM-free 1.1 build nor an official 1.1 one the
  bundled patcher converts (an unofficial no-CD, or still v1.0).
  Re-install the official 1.1 patch and try again. If it is a 1.1 build
  anyway, [report it](https://github.com/akitaonrails/rea-colin-mcrae-3/issues/new?template=bug.yml)
  with the SHA-256 the message shows and `launcher.log`.
- **The game crawls at 4K under Wine** — DXVK is off; turn it on in the
  settings window (Linux) or keep `thirdparty/dxvk/d3d9.dll` next to the
  launcher.
- **The pad has no throttle** — use the Rally preset, and leave the
  game's accelerator/brake bound to the left stick's Y axis.
- **Anything else:** run `CMR3Launcher.exe --check`, and attach
  `launcher.log` (see the table above) to an issue.

## Known limitations

- The interface languages travel with your copy of the game. To use every
  language and co-driver together, add SilentPatch's Language Pack to
  your **game** folder yourself (we don't redistribute game content); the
  launcher picks the rest up from there.
- Only the official 1.1 executables (and the DRM-free ones) are
  supported; v1.0 needs the official patch first.
- Not yet tested on native Windows, a Steam Deck, or Proton without
  `cmr3-play`; the full list of what was and wasn't checked is in
  [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md#platform-matrix).

## Uninstall

1. Delete the folder you unpacked the launcher into (or the AppImage).
2. **Back up your profile first:** `%USERPROFILE%\Saved Games\Colin McRae
   Rally 3` (or your `saves` folder) holds your progress. Then delete
   `%LOCALAPPDATA%\CMR3Launcher` (settings, log and the launcher's ~3 GB
   copy of the game).
3. Linux: the same folders live in the Wine prefix
   `~/.local/share/cmr3-launcher/prefix` (the profile under
   `drive_c/users/<you>/Saved Games`); back up the profile, then delete
   the prefix and `~/.config/cmr3-launcher` (`cmr3-play`'s logs).

Your game folder was never changed, so there is nothing to undo there.

## For developers

Built on [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk)
(pinned by tag in `Cargo.toml`; its shell and packaging files are
vendored in `sdk/` by `scripts/sdk-sync`). The SDK's
[standards](https://github.com/akitaonrails/rea-launcher-sdk/blob/master/docs/STANDARDS.md)
apply here.

```text
cargo test --workspace                       # host tests, the settings window headless
for t in linux/tests/*.sh; do sh "$t"; done  # cmr3-play (e2e.sh needs a game; skipped by default)
cargo clippy --release --target i686-pc-windows-gnu --workspace -- -D warnings
cargo build --release --target i686-pc-windows-gnu   # CMR3Launcher.exe (MinGW + lld)
sdk/scripts/wine-test --locked --workspace   # the Windows-target tests under Wine, headless
cargo deny check
sh <sdk>/scripts/sdk-sync --check .          # sdk/ matches the pinned SDK
```

MinGW, lld and Wine can live in a container (the `gaming` distrobox from
github.com/akitaonrails/distrobox-gaming): run the commands there with an
absolute path. GitHub Actions (the SDK's reusable workflows) runs the
same checks, the Windows-target tests under Wine, builds the Windows
package (with the checksummed components from
`packaging/third-party.txt`) and the Linux AppImage on every push, and
publishes a release for tags named `v…`. Design notes: [docs/PLAN.md](docs/PLAN.md);
community fixes and decisions: [docs/RESEARCH.md](docs/RESEARCH.md);
rules for contributors and agents: [AGENTS.md](AGENTS.md).

## Contributing

Issues and pull requests are welcome. Where they go depends on what they
touch:

- **Here** (this repository): anything about Colin McRae Rally 3 — the
  game's versions, SilentPatch/Xidi/DXVK settings, the profile, the HD
  UI, the looks, `cmr3-play`'s settings and post-processing, packaging
  pins, this README.
- **[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk/issues)**:
  the shared parts every rea launcher uses — the runtime folder and its
  syncing, the settings window's widgets and theme, `rea-play.sh`
  (`sdk/rea-play.sh`), the packaging scripts and AppImage builder
  (`sdk/packaging`), the CI workflows, anything under `sdk/`.

Not sure? If the same problem would happen with another rea launcher,
it's the SDK's; otherwise report it here and we'll move it. Never patch
`sdk/` or copy SDK code into this repository: change the SDK and bump the
pinned version instead.

## Credits

The launcher is a small integration layer over other people's work:
[SilentPatchCMR3](https://github.com/CookiePLMonster/SilentPatchCMR3) by
CookiePLMonster (widescreen, display and compatibility fixes),
[Bekoha's HD UI](https://bekoha.github.io/cmr3/),
[Xidi](https://github.com/samuelgr/Xidi) by samuelgr,
[DXVK](https://github.com/doitsujin/dxvk), and
[vkBasalt](https://github.com/DadSchoorse/vkBasalt), on the shared
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk). The settings window's
Colin McRae photo is by "Carolyn" on Flickr (CC BY 2.0, via Wikimedia Commons); its Barlow fonts are
by Jeremy Tribby (SIL OFL). See [THIRD_PARTY.md](THIRD_PARTY.md) for the
full component and licence list.

## Legal

Colin McRae Rally 3 is © Codemasters. This project is not affiliated with
or endorsed by Codemasters or Electronic Arts, contains no game code or
assets, and only runs a copy you own. Our code is MIT or Apache-2.0
([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)); bundled
third-party parts (SilentPatch, Xidi, DXVK, the DRM-free patcher, the HD
UI, Wine) and their licences are listed in
[THIRD_PARTY.md](THIRD_PARTY.md).
