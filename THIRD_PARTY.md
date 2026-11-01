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
| The ASI loader shipped with SilentPatch (Ultimate ASI Loader by ThirteenAG), installed as `dsound.dll` | MIT | https://github.com/ThirteenAG/Ultimate-ASI-Loader |
| Xidi v4.3.1 Win32 (`dinput8.dll`) and its `Xidi.ini`: DirectInput for XInput controllers, by samuelgr | BSD-3-Clause (bundles Hookshot, BSD-3) | https://github.com/samuelgr/Xidi |
| The DRM-free patcher from the SilentPatch project: `xdelta.exe` (an xdelta3 release binary) and its `efigs`/`polish` patches that turn the official 1.1 executables into the DRM-free ones the late re-releases shipped | xdelta3: Apache-2.0; the patcher's scripts: MIT | https://github.com/jmacd/xdelta-gpl , https://github.com/CookiePLMonster/SilentPatchCMR3 |
| DXVK v3.1.1, `x32/d3d9.dll` only: draws the game's Direct3D 9 through Vulkan under Wine (never installed on Windows, where the system's own Direct3D is used) | Zlib | https://github.com/doitsujin/dxvk |
| HD UI v1.0.1 by Bekoha: font atlases and interface art redrawn for 4K (owner decision, 2026-11-01 — the same call as the 1998 game's HD art pack; redistributed unmodified from the author's release) | the author's release carries no licence text; credited here and in the README | https://bekoha.github.io/cmr3/ |

License texts for the bundled components are in `packaging/licenses/` and
travel with the package inside `thirdparty/`.

**Not bundled, by policy:** SilentPatch's Language Pack and Nicky Grist
Pack (original game text, fonts and co-driver audio from the regional
releases; see `AGENTS.md`), and the official 1.1 patch itself (a
Codemasters download — the launcher links to it when a game needs it).

## Bundled in the Linux AppImage (`packaging/appimage`)

| Component | Licence | Source |
| --- | --- | --- |
| Wine 11.20 (amd64, WoW64), Kron4ek's build of the unmodified WineHQ sources; licence and source notes inside the AppImage under `usr/share/doc/wine` | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz , build scripts https://github.com/Kron4ek/Wine-Builds |
| AppImage runtime (added by `appimagetool` 1.9.1) | MIT | https://github.com/AppImage/type2-runtime |
| The AppImage icon `packaging/appimage/cmr3-launcher.png` | ours (MIT OR Apache-2.0), set in Barlow (OFL) | this repository |

## Settings window artwork and fonts (compiled into CMR3Launcher.exe)

| File | Licence | Source |
| --- | --- | --- |
| `crates/cmr3-launcher/assets/portrait.jpg` — Colin McRae, photo by Carolyn Williams (resized) | CC BY 2.0 (credited in the window) | https://commons.wikimedia.org/wiki/File:Colin_Mcrae_crop.jpg |
| `crates/cmr3-launcher/assets/fonts/Barlow-*.ttf`, `BarlowCondensed-*.ttf` | SIL OFL 1.1 (`assets/fonts/OFL.txt`) | https://github.com/jpt/barlow |
