# Third-party components

Everything bundled with or compiled into the package, with its licence and
source. Add a row before bundling anything new. Downloads are pinned by
SHA-256 in `packaging/third-party.txt` and fetched when the package is
built.

## Compiled into CMR3Launcher.exe

| Component | Licence | Source |
| --- | --- | --- |
| `md-5`, `sha2` and their RustCrypto dependencies | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `windows-sys` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `eframe`, `egui` and their dependencies (`winit`, `glutin`, `glow`, …): the settings window | MIT OR Apache-2.0 (dependencies: MIT, Apache-2.0, Zlib, BSD; see `cargo tree`) | https://github.com/emilk/egui |
| `jpeg-decoder` | MIT OR Apache-2.0 | https://github.com/image-rs/jpeg-decoder |
| Rust standard library | MIT OR Apache-2.0 | https://github.com/rust-lang/rust |
| MinGW-w64 runtime (static parts linked by the GNU target) | ZPL-2.1 / public domain | https://www.mingw-w64.org |

## Bundled in `thirdparty/` (installed into the launcher's private copy of the game)

| Component | Licence | Source |
| --- | --- | --- |
| SilentPatch for Colin McRae Rally 3, build 2.1, by CookiePLMonster: widescreen (any aspect ratio), all display resolutions, windowed/borderless, vsync, refresh rate, anisotropic filtering, FOV options, portable settings (`SilentPatchCMR3.ini`), sun/water/shadow fixes, split-screen on modern CPUs, d-pad menu navigation, Alt+F4, timer precision | MIT | https://github.com/CookiePLMonster/SilentPatchCMR3 |
| The ASI loader shipped with SilentPatch (Ultimate ASI Loader by ThirteenAG), installed as `dsound.dll` | MIT (`thirdparty/silentpatch/Ultimate-ASI-Loader-LICENSE`) | https://github.com/ThirteenAG/Ultimate-ASI-Loader |
| Xidi v5.0.0 Win32 (the `dinput8.dll` wrapper and the `Xidi.32.dll` main binary it loads) and its `Xidi.ini`: DirectInput for XInput controllers, by samuelgr | BSD-3-Clause (bundles Hookshot, BSD-3) | https://github.com/samuelgr/Xidi |
| The DRM-free patcher from the SilentPatch project: `xdelta.exe` (an xdelta3 release binary) and its `efigs`/`polish` patches that turn the official 1.1 executables into the DRM-free ones the publisher's late re-releases shipped. **Owner decision, 2026-11-02:** this community patcher, which yields the publisher's own DRM-free 1.1 build, is the approved way to remove the disc protection (also in `AGENTS.md`); the launcher writes no crack of its own and refuses any build the patcher doesn't convert | xdelta3: Apache-2.0; the patcher's scripts: MIT | https://github.com/jmacd/xdelta-gpl , https://github.com/CookiePLMonster/SilentPatchCMR3 |
| DXVK v3.1.1, `x32/d3d9.dll` only: draws the game's Direct3D 9 through Vulkan under Wine (never installed on Windows, where the system's own Direct3D is used) | Zlib | https://github.com/doitsujin/dxvk |
| vkBasalt 0.3.2.10, built as 32/64-bit Vulkan layers: optional CAS sharpening and ReShade FX post-processing under Linux (`cmr3-play`) | Zlib | https://github.com/DadSchoorse/vkBasalt |
| Compiled into vkBasalt: ReShade's FX compiler by Patrick Mours (`src/reshade`), and stb_image / stb_image_resize by Sean Barrett | ReShade: BSD-3-Clause (`thirdparty/vkbasalt/ReShade-LICENSE.md`); stb: MIT or public domain (`thirdparty/vkbasalt/stb-LICENSE`) | https://github.com/crosire/reshade , https://github.com/nothings/stb |
| HD UI v1.0.1 by Bekoha: font atlases and interface art redrawn for 4K (owner decision, 2026-11-01 — the same call as the 1998 game's HD art pack; redistributed unmodified from the author's release) | the author's release carries no licence text; credited here and in the README | https://bekoha.github.io/cmr3/ |

License texts for the bundled components are in `packaging/licenses/` and
travel with the package inside `thirdparty/`.

**Not bundled, by policy:** SilentPatch's Language Pack and Nicky Grist
Pack (original game text, fonts and co-driver audio from the regional
releases; see `AGENTS.md`), and the official 1.1 patch itself (a
Codemasters update; its only download today is a community mirror on
PCGamingWiki with no stated redistribution terms, so the README points
players to it and the launcher does not fetch it — pending an owner
decision).

## Bundled in the Linux AppImage (`packaging/appimage`)

| Component | Licence | Source |
| --- | --- | --- |
| Wine 11.20 (amd64, WoW64), Kron4ek's build of the unmodified WineHQ sources, pinned by the shared rea-launcher-sdk packaging (`sdk/packaging/appimage/tools.pins`); licence and source notes inside the AppImage under `usr/share/doc/wine` | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz , build scripts https://github.com/Kron4ek/Wine-Builds |
| AppImage type2 runtime 20251108 (pinned by the shared rea-launcher-sdk packaging, packed by `appimagetool` 1.9.1) | MIT | https://github.com/AppImage/type2-runtime/releases/tag/20251108 |
| The AppImage icon `packaging/appimage/cmr3-launcher.png` | ours (MIT OR Apache-2.0), set in Barlow (OFL) | this repository |

## Settings window artwork and fonts (compiled into CMR3Launcher.exe)

| File | Licence | Source |
| --- | --- | --- |
| `crates/cmr3-launcher/assets/portrait.jpg` — Colin McRae, photo by Carolyn Williams (Flickr user carolyn.will, 8365241@N02; Wikimedia Commons lists the author as "Carolyn"), cropped on Wikimedia Commons and resized here | CC BY 2.0 (credited in the window: "Photo: Carolyn Williams, CC BY 2.0 (via Wikimedia Commons)") | https://commons.wikimedia.org/wiki/File:Colin_Mcrae_crop.jpg (from https://www.flickr.com/photos/8365241@N02/1429137255/) |
| Barlow and Barlow Condensed (`Barlow-*.ttf`, `BarlowCondensed-*.ttf`), bundled by the shared settings-window kit `rea-launcher-ui` (rea-launcher-sdk, `crates/rea-launcher-ui/assets/fonts`) | SIL OFL 1.1 (shipped as `licenses/Barlow-OFL.txt` in the package) | https://github.com/jpt/barlow |

## Images in the README and the package (`docs/images`)

| File | Author and licence | Source |
| --- | --- | --- |
| `launcher-play.jpg`, `launcher-picture.jpg`, `launcher-controls.jpg` — screenshots of this launcher's settings window (the portrait in them is credited above) | this project, MIT OR Apache-2.0 | taken by the project's owner |
| `gameplay.jpg` — the game running at 4K through this launcher, resized to 1920×1080 | screenshot by the project's owner; it shows Colin McRae Rally 3 (© Codemasters) to illustrate the launcher at work, no game files | the owner's own copy, 2026-11-01 (`docs/RUNTIME_NOTES.md`) |

## Provenance and updates

Downloads are pinned by exact release url and SHA-256 in
`packaging/third-party.txt`; a weekly watcher (`.github/workflows/watch-upstream.yml`)
opens an issue when an upstream publishes something new, and adopting it
is a deliberate pin change after a test run. The exact validated
downloads are mirrored on this repository's `thirdparty` release
(`.github/workflows/mirror.yml`) and listed there as build fallbacks, so
an upstream move or deletion cannot break the build. The reasoning,
including why this was chosen over git submodules, is in
`docs/PLAN.md`.
