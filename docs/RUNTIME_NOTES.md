# Runtime notes

What was actually run, where, and what happened. Newest last.

## 2026-11-01 — baseline (before this launcher)

The stack this launcher reproduces was validated by hand on this
machine's `gaming` distrobox (Arch, RTX 5090, 4K screen, Wine with
DXVK in the prefix, Xidi with the rally mapper, gamescope at 3840×2160):
menus, stages and driving at full frame rate, 8BitDo (X-input) and Xbox
pads playable. Recorded in `~/Projects/distrobox-gaming` and its memory
pages; condensed in `docs/PLAN.md`.

## 2026-11-01 — M2: first full run through CMR3Launcher (gaming box)

Built `CMR3Launcher.exe` inside the `gaming` distrobox (wine 11.21,
mingw) and ran it under the box's Wine against the repack install, with a
**brand-new prefix** on the same volume as the game and
`--display-mode windowed --resolution 1920x1080 --no-window`:

- Edition detected: DRM-free v1.1 variant B (sha256 `c56344b4…`).
- Runtime copy: **1576 files, 1569 hard-linked** (near-instant first
  run), `CMR3` profile created in `Saved Games\Colin McRae Rally 3`.
- SilentPatch + ASI loader (dsound), Xidi (Rally), DXVK d3d9, HD UI
  (38 files) installed; `SilentPatchCMR3.ini` written
  (WIDTH=1920, HEIGHT=1080, DISPLAY_MODE=1).
- **The game started** from the runtime copy and ran; DXVK's log shows a
  Vulkan swapchain (B8G8R8A8, FIFO) — the game drew through Vulkan at
  1080p, not Wine's GL translator. Killed after ~50 s (exit 1 = our
  kill); the user's game folder was verified byte-identical before and
  after the whole smoke.

Not yet run: a real pad through Xidi in-game, gamescope via `cmr3-play`
(the unit-tested path), the DRM→DRM-free xdelta conversion on a
protected 1.1 executable (the refusal path on a mismatched file passed
in tests; no wine-free host check of a real patch yet), and the settings
window itself (skipped with `--no-window`).

## 2026-11-01 — the settings window under Wine (XWayland, scale 1.5)

First builds resized the window after creation (egui viewport commands),
which on XWayland at fractional scaling leaves the surface bigger than
winit thinks — the picture sat squashed in the top left, white where the
theme should be, and clicks landed on the wrong widgets (a pressed Play
did nothing). Fix: create the window at its final size (`SIZE × zoom`,
zoom worked out once up front) and only set egui's zoom factor after —
no post-creation resize commands. Verified on this box: hyprctl lists
`cmr3launcher.exe` at 880×640 logical, and a DP-1 capture shows the full
crimson window with the gold Play plate, photo panel and text fields in
place. (CMR1's launcher still does the resize dance; worth porting this
fix there.)

## 2026-11-01 — display modes under Wine, measured

Controlled runs (this box: XWayland on Hyprland, scale 1.5, portrait +
landscape monitors; DXVK swapchain and on-screen content width measured
per run):

| path | swapchain | content |
| --- | --- | --- |
| exclusive fullscreen, raw Wine | 2160×3840 (the portrait monitor!) | 56% — cropped |
| exclusive fullscreen, ADAPTER overridden | unchanged | 56% |
| borderless, raw Wine | 3840×2160 | 100% |
| wine virtual desktop `explorer /desktop=W,H` | 3840×2160 | 100% |
| fullscreen inside gamescope | correct | full (user-confirmed) |

Wine's exclusive-fullscreen chases the desktop's monitor list and lands
on the wrong display (portrait neighbours included); the game also
rewrites WIDTH/HEIGHT/ADAPTER in the ini from that bad enumeration.
Conclusions, now policy (`DisplayMode::effective`): Windows keeps the
choice; Wine inside gamescope keeps true fullscreen; Wine anywhere else
plays fullscreen as borderless at the same size — the game sizes that
window itself, so display enumeration never runs. The launcher rewrites
its keys before every launch, which also heals whatever the game saved.
cmr3-play additionally found its settings next to the launcher exe
(portable installs) before this round and defaulted gamescope to
1920×1080; fixed and covered by tests.

## 2026-11-01 — Xidi's launch-time configuration popup

A Windows popup before the game said that configuration-file reading had
failed. The screenshot alone did not identify the component; Xidi's own
`Xidi_DInput8_Rally_3PC.exe_*.log` on the prefix desktop did: version
4.3.1 rejected `[Log]`'s `Append` as an unrecognized setting at
`Xidi.ini:30`. Removed that unsupported key from both generated presets
and the byte-for-byte Rally fixture. An earlier, unverified attempt to
suppress Wine's crash dialog was unrelated and was withdrawn.

Rebuilt the release launcher in the gaming container and launched the
real game through `cmr3-play` with the existing 3840×2160 settings. The
game process started, DXVK created its 3840×2160 swapchain, and the
runtime `Xidi.ini` contained only the supported `Enabled` and `Level`
log keys. No new Xidi error log appeared (the three earlier failing runs
each created one); the test prefix was then closed. Host tests: 45
passed; GPU/scaler shell suites passed.

## 2026-11-01 — soft shadows and Colour post-processing

Enabled `shadows = soft` and `postfx = color` in the local test
`launcher.cfg`. The real launch wrote `SHARPER_SHADOWS=0` to
`SilentPatchCMR3.ini`. vkBasalt initially remained inactive: a legacy
`layer.json` shadowed the new `layer64.json` by layer name and referred
to a bare `libvkbasalt.so`, which the loader could not open. `cmr3-play`
now removes its current and legacy generated manifests before writing
new ones; `linux/tests/postfx.sh` covers this upgrade case and absolute
library paths.

Repeated the real 3840×2160 launch with Vulkan loader diagnostics. The
64-bit vkBasalt library was inserted at instance and device creation and
read the generated config (`effects = cas`, `casSharpness = 0.65`,
`executeAt = end`). The game ran and no new NVIDIA Xid appeared during
the check. Rust tests (45), GPU/scaler suites, and the new postfx suite
passed. Direct `wine CMR3Launcher.exe` launches do not run this Linux
post-processing setup; use `cmr3-play`.

## 2026-11-01 — launcher layout and README captures

A 4K gameplay capture showed the car's dynamic shadow but no shadows
from trackside billboards. That is the original renderer's scenery
treatment, not a failure of `SHARPER_SHADOWS`: that setting changes car
shadows only and does not add shadows to billboards.

The Enhanced card's “Your screen's 3840×2160” note wrapped to four lines
and crossed its border; shortened it to the resolution, widescreen and
HD UI. Adding Shadows and Post-processing also made Picture overrun the
fixed window, leaving DXVK under the footer. Increased the design height
from 640 to 704 points, added vertical scrolling for smaller screens,
and wrapped toggle labels to their available width. Shortened the ready
line so the edition text no longer crosses the gold decal (the edition
remains below the game path).

Verified the release build through Wine with Play, Picture and Controls
captures at 1320×1056. Every option is visible, the Enhanced note stays
inside its card, and long labels wrap without entering the footer. Used
those three captures, with the generic `C:\Games\Colin McRae Rally 3`
path, in the README. Rust tests (45), postfx, GPU and scaler suites all
passed.

## 2026-11-01 — gameplay heap fault and frozen window

A live gamescope session froze at 18:47. `play.log` ended with `wine:
Unhandled page fault on read access to 0E99896F at address 7BCEA8D7
(thread 0158), starting debugger...`; the game and gamescope processes
stayed alive. No NVIDIA Xid, OOM event or Linux core dump occurred. The
faulting address maps to Wine's 32-bit `ntdll.dll`, where the instruction
reads heap-block metadata at `edi-1`. This is a Windows heap fault, not
a GPU hang. GDB was attached and detached without changing the process,
but its WoW64 stack could not be symbolized, so the module that caused
the heap corruption remains unknown; do not blame DXVK or vkBasalt from
this evidence alone. The user's new 4K gameplay capture was added to the
README as `docs/images/gameplay.jpg` (resized to 1920×1080 JPEG).

## 2026-11-01 — v0.1.0 release-candidate gate

Reviewed all 12 initial direct commits and the release changes. Pinned
GitHub Actions to immutable SHAs, added a host-tested Windows command-line
quoter after finding that xdelta arguments with spaces were unquoted, and
made the AppImage build script executable. Built both vkBasalt bitnesses
from the pinned source and packaged their licence.

Final checks: 46 Rust tests, postfx/GPU/scaler suites, Windows-target
clippy and release build all passed. The three ignored real-game smoke
tests passed, including byte-preservation of the source folder. The
90-second `linux/tests/e2e.sh` lifecycle run passed with no page faults.
Built the 180 MB AppImage and launched the real game through its bundled
Wine. The first smoke attempts were environmental (CMR3 runtime copies
filled `/tmp`, then stale gamescope X sockets); removed only those
project/test leftovers and reran green.

An isolated AppImage check with no portable config selected Wine's
portrait monitor (2160×3840) in the shared test prefix. Restored the
player's saved settings through the normal `cmr3-play` path and verified
`WIDTH=3840`, `HEIGHT=2160`, `DISPLAY_MODE=0` before stopping further use
of that prefix. This was test setup, not a code-path change.

## 2026-11-01 — postfx first light, and the crash dialog

- The vkBasalt layer chain, measured: `VK_INSTANCE_LAYERS` accepts a
  colon-separated list; wine passes `VK_*` through to the game but drops
  `LD_LIBRARY_PATH`, so the manifests use absolute paths; a WoW64 prefix
  loads the 64-bit Vulkan loader even for the 32-bit game, so both
  bitnesses ship under their own layer names. With that, "vkBasalt info:
  config file:" appears in play.log — the layer runs (CAS at 0.65,
  `postfx = color`), verified by e2e.
- The Windows-style error box on every launch was wine's crash dialog for
  the quartz fault (`winedbg` disabled turned the old hang into a visible
  dialog while the game carried on). The launcher now sets
  `ShowCrashDialog=0` (the `winetricks nocrashdialog` verb) in its prefix;
  the faulting thread still ends, quietly.

## 2026-11-01 — v0.1.1: the pad on a new profile, issues #1 and #2

All on the gaming box through the v0.1.0 AppImage's own Wine (11.20),
with a fresh prefix under `.work/`.

- **Issue #1's `Map Controllers` diagnosis does not hold.** A 32-bit
  probe calling `XInputGetState` on brand-new prefixes saw the 8BitDo
  with no WineBus settings at all, and with `Map Controllers=1` or
  `DisableHidraw=1` (HKLM, where winebus reads them) or the issue's HKCU
  keys alike. Moving the stick changed its state in every case.
  `SDL_JOYSTICK_DEVICE=/dev/input/js0` made the pad appear twice (two
  `IG_00` devices, slots 0 and 1); `cmr3-play` no longer sets it.
- **What left the pad dead in the game:** a new profile has no
  controller chosen (Options → Controls). Xidi's log showed the game
  binding virtual controller 1 normally. One false lead along the way:
  another session's `py-evdev-uinput` script was creating a virtual
  "Microsoft X-Box 360 pad" that took XInput slot 1 and then vanished
  ("Physical controller 1: Hardware disconnected").
- **The profile (`CMR3`), from Rally_3PC.exe 1.1:** the save routine at
  0x4142B0 copies 0x42A20 bytes of settings, writes 1 at 0x42A20 and a
  CRC at 0x42A24: the routine at 0x412380, MSB-first CRC-32
  (0x04C11DB7, init and final inverted; CRC-32/BZIP2) over the whole file
  with the CRC field zeroed. It matched every profile the game wrote.
  Diffs of profiles saved after single changes: player 1's joystick block
  at 0x1DB98 (device index, -1 none, then bindings), keyboard block at
  0x191D4 (0x300 once Keyboard is chosen), vibration at 0x1DB94 (80 =
  Medium). A fresh profile the game wrote with the launcher's change
  applied equals, byte for byte, the one the game wrote when the player
  chose the controller, apart from vibration and a two-byte counter at
  0x429E0 (the ignored test `matches_the_games_own_choice`). Confirmed in
  the game: Options → Controls showed the Xidi controller and vibration
  Medium on a new profile.
- **Rally preset:** with the d-pad as keys only, Xidi's controller lost
  its hat (17 → 16 elements) and the game's stored bindings no longer
  matched: no steering or pedals. `Compound(Pov(...), Keyboard(...))` and
  `Compound(Button(4), Keyboard(Enter))` keep StandardGamepad's layout;
  verified: d-pad and A work the main menu, stick and triggers drive.
  The Controls menu reads the hat too, so there the d-pad moves two rows.
- **The gameplay crash** ("Unhandled page fault on read access to
  61726170", the address being ASCII text) happened twice in races, at
  0x7A068212 and 0x7A05A516: both inside DXVK's `d3d9.dll`, mapped at
  0x79E60000 (from `/proc/<pid>/maps`), not Wine's `ntdll` as the entry
  above guessed. With `winedbg=d` now really in effect under the
  AppImage, the game ended instead of freezing. Cause not found yet.
- After a crash the launcher never copies the profile back; the next
  start used to restore the older saved one over it. `mirror_file` now
  only copies onto an older file, and the launcher saves a newer runtime
  profile first.

## 2026-11-02 — v0.1.2: the washed-out picture, pad bindings by layout

- **Picture.** A 4K frame from a gravel stage (Cinematic of 0.1.1, CAS
  only) measured 30/255 at the darkest 2 % of the scene, 59 at the median
  and 143 at the 99th percentile: a picture squeezed into 25–160. The
  looks now grade through a vkBasalt `.cube` LUT that `cmr3-play` writes
  (levels from a black and white point, a gamma that keeps the midtones,
  a saturation lift): Colour 0.08/0.90/γ1.15/×1.08 brings the darkest 2 %
  to 18, Cinematic 0.10/0.86/γ1.12/×1.14 to 10, midtones within a few
  levels. Judged in the game: both good, Cinematic the stronger.
- vkBasalt's effect list is colon-separated, and a ReShade effect needs
  `name = file.fx`: Cinematic's `effects = mxao cas` never loaded MxAO.
- **Pad bindings depend on the controller's layout.** The game stores
  axis codes; with a hat on the d-pad (0.1.1's `Compound(Pov(...))`) the
  left stick's X and the triggers' Y are 14/15, without one (0.1.0 and
  now) 12/13. The game's own defaults point at the right stick in both.
  Bindings measured by remapping on the keys-only layout: steering in
  words 9–10, pedals in 23–24 of player 1's joystick block. Verified on a
  new profile with no remap: left stick steers, triggers are the pedals.
  The 0.1.1 note that "the keys-only d-pad broke the stored bindings" was
  this: the defaults then pointed at the right stick.
- **Local Windows builds from the gaming distrobox** (MinGW GCC 16.2,
  binutils 2.47, Rust 1.99.1) crashed the launcher in `win32::is_wine`,
  jumping into unrelated code (`f32::from_str`); the same source built by
  CI (Ubuntu's MinGW, same Rust) runs. Test with CI builds (`gh workflow
  run build.yml --ref <branch>`); the cause in the local toolchain is not
  isolated.

## Unreleased — launcher robustness fixes (not yet verified in the game)

Host-tested only; check each on Windows and under Proton/Wine before the
next release.

- **DRM-free patcher.** The xdelta arguments now decode a real patch on
  the host (xdelta3 on Linux). Not yet verified: an official 1.1
  `Rally_3PC.exe` (efigs and Polish) patched by the bundled `xdelta.exe`
  on Windows and under Wine, then the game starting from it.

- **Hard-linked originals are left alone.** Replacing or removing a
  runtime file no longer clears its read-only attribute first, and sync no
  longer sets the time on linked files (it shares the original's inode).
  Removal of a read-only file relies on Rust's `remove_file`, which on
  Windows retries with `FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE`.
  Read-only originals are copied rather than linked, so a new runtime
  never has to replace a read-only file; the fallback remains for older
  runtimes. Not yet verified: a game folder whose files are read-only (a
  copied CD install) on Windows and under Wine, with the HD UI on (it replaces
  linked `Data\` files) and the DRM-free patch (it replaces the linked
  exe).
- **HD UI off restores the game's files** (recorded in `.cmr3-hd-ui` in
  the runtime folder). Not yet verified: the menus and fonts look like the
  original game after switching it off.
- **Component ownership ledger** (`.cmr3-launcher-components`). Not yet
  verified: Gamepad off with a game folder that ships its own
  `dinput8.dll` (the game should load it again), and DXVK off under Wine.
- **WINEDLLOVERRIDES from the launcher** (`dsound,dinput8,d3d9=n,b` for
  what it installed, merged with the caller's rules). Not yet verified:
  CMR3Launcher.exe added to Steam as a non-Steam game and run with Proton
  (no `cmr3-play`): SilentPatch, Xidi and DXVK should all load (the
  launcher log shows the overrides; DXVK's HUD or log shows it drawing).
- **cmr3-play supervision and postfx scoping.** Not yet verified in the
  game: `cmr3-play stop` while racing (gamescope, Wine and the game all
  end); `postfx = color` still grades the game with the layer variables
  now set inside gamescope (in the game's script) instead of around it,
  and gamescope itself no longer loads vkBasalt.
- **Paths on volumes without canonicalization** (RAM disk, `subst`
  drive): compared lexically. Not yet verified: a game on such a drive on
  Windows launches.
- **Profile sync by recorded state.** Not yet verified in the game:
  restoring an older `CMR3` backup into the saves folder loads it on the
  next launch; ending the game with `cmr3-play stop` (no write-back) and
  launching again keeps the progress.
- **Settings window and Wine's DPI.** Under Wine the creation zoom now
  divides out `LOGPIXELSY / 96` (the process is made DPI aware first). At
  the default 96 DPI the zoom is exactly what the 2026-11-01 XWayland
  notes above measured (1.5x on 4K). Not yet verified: with `winecfg` DPI
  at 144 and 192 on a 4K screen the window fits the screen (about 1.5x
  overall) and is fully drawn under XWayland with fractional scaling.

## Unreleased — rea-launcher-sdk v0.2.0 pass (host- and Wine-tested only)

Host tests and `sdk/scripts/wine-test` (the settings window headless
under Wine) pass; none of this was run in the real game yet.

- **Unset gives back the game's value** (`.cmr3-ini-owned`). Not yet
  verified: set vsync On, play, set it in the game's menu to Off, unset
  it in the launcher: the game's Off stays; unset an untouched value: the
  game's own value comes back.
- **Unknown builds refused.** Not yet verified in the game: a v1.0 or
  no-CD `Rally_3PC.exe` is refused with its SHA-256 at Play and by
  `--check` (exit 1); the official 1.1 exe is still converted (the
  result must now be one of the known DRM-free builds).
- **`--check` and `--help` in the terminal** (`console::attach_parent`).
  Not yet verified: from `cmd.exe` on Windows (text in the console) and
  by double-click (a message box).
- **One-run flags in `cmr3-play`** (`--play --scaler fsr --resolution
  1280x720`): tested with fakes; not yet in a real gamescope session.
- **Window**: anti-aliasing tri-state, 4×, 75°, the gamepad log switch,
  descriptions under every row (the Picture tab now scrolls on a
  1080p screen). Not yet looked at on screen.

## Unreleased — rea-launcher-sdk v0.3.0 (host- and Wine-tested only)

Host tests, the shell tests and `sdk/scripts/wine-test` pass; nothing
below was run in the real game yet.

- **LLD-linked build.** `check-imports.sh` finds no orphaned imports
  (GNU ld had left 13). Pending: start the game once from the new
  `CMR3Launcher.exe` on Windows and under Wine.
- **One-run flags through the SDK** (`REA_HELPER_KEYS`). Pending:
  `cmr3-play --play --scaler fsr` (play.log shows `-F fsr`), then a plain
  `cmr3-play --play` (back to launcher.cfg's scaler).
- **Window**: the SDK's "As the game" rows (VSync, Texture sharpness,
  Smooth edges, Car shadows) and the four-line portrait caption on a
  taller band. Not yet looked at on screen.
- **Clean exit, single instance** (STANDARDS). Done: the named mutex
  (`instance`), tested under Wine; `cmr3-play --play` while a session
  runs starts nothing (shell test with fakes). The launcher never stops
  the game itself (it waits for it), so `close_gracefully` has no caller.
  Pending, in the real game, headless (`gamescope --backend headless`),
  with the process table checked afterwards (no `Rally_3PC.exe`,
  launcher, `wineserver` or gamescope left):
  - the game's own Quit;
  - Alt+F4 / the window's close button (`WM_CLOSE`), and whether the game
    honours it at all;
  - Steam's "Stop" (non-Steam game);
  - `cmr3-play stop`, Ctrl+C and SIGTERM to `cmr3-play`;
  - gamescope being closed;
  - a second `CMR3Launcher.exe` on Windows while the window is open (the
    window comes forward) and while the game runs (the message).
  (Under Wine all but the game's own Quit and Steam's Stop were verified
  on 2026-11-02, below; native Windows is still pending.)

## 2026-11-02 — real-game verification, headless (gaming box)

The owner's install (DRM-free v1.1 variant B, read-only, on NFS) through
a fresh release build (`cargo build --release --locked --target
i686-pc-windows-gnu`, LLD, `check-imports.sh` clean) and this repo's
`linux/cmr3-play --play`, with `CMR3_PREFIX`/`CMR3_LAUNCHER`, `HOME` and
the XDG dirs all under `.work/realgame/` (a throwaway prefix). gamescope
ran headless (`--backend headless`, added by a `gamescope` wrapper first
in `PATH`: the SDK has no setting for it); audio went to a private
PipeWire + WirePlumber + pipewire-pulse with a null sink in a private
`XDG_RUNTIME_DIR` (Wine's winepulse played into it). A supervisor inside
the box (`.work/realgame/run.sh NAME LOOK QUIT`) waited for
`Rally_3PC.exe`, captured with `gamescopectl screenshot`, sent the quit,
polled the session's processes, and killed anything left with our
environment tag (`procs.sh kill`; gamescope by its command line, since
its file capability hides its environment). Logs per run in
`.work/realgame/logs/<run>/` (run.log, launcher.log, play.log, DXVK's
`Rally_3PC_d3d9.log`, the written `SilentPatchCMR3.ini`/`Xidi.ini`).
Every run ended with the process table clean; no leftover.

| Check | Result | Evidence |
| --- | --- | --- |
| Enhanced (1920×1080 here) reaches the game | VERIFIED | `enhanced-stop`: runtime built (1565 files copied, 0 linked: NFS → local disk, 99 s before the game started; later starts 4–7 s), SilentPatch/Xidi/DXVK/HD UI installed, ini `WIDTH=1920 HEIGHT=1080 DISPLAY_MODE=0 VSYNC=1 ANISOTROPIC=4 FSAA=0 EXTERIOR_FOV=75 INTERIOR_FOV=75` (as `look-enhanced.txt`); DXVK swapchain 1920×1080 FIFO; title screen captured |
| Original 2003 | VERIFIED | `original-taskkill`: ini `WIDTH=640 HEIGHT=480 DISPLAY_MODE=0`, VSync/anisotropy/FSAA/FOV keys the launcher had set removed ("back to the game's"), swapchain 640×480 |
| Steam Deck look | VERIFIED (values) | `deck-gamescope`: ini 1280×800 VSYNC=1 ANISOTROPIC=4 FSAA=0 FOV 75; swapchain 1280×800. Not a Deck: the matrix row stays pending |
| HD UI off restores the game's files | NOT CHECKABLE on this install | launcher: "HD UI off: 38 files back to the game's", but this repack already ships the HD UI in its own folder (34 of the 37 overlay files are byte-identical in it), so the game still shows it. Needs a retail/clean install |
| `cmr3-play stop` | VERIFIED | everything gone 1.9 s after the command |
| SIGTERM to a waiting `cmr3-play` (`CMR3_WAIT=1`) | VERIFIED | gone in 1.8 s, helper exit 143 |
| Ctrl+C (SIGINT) to a waiting `cmr3-play` | VERIFIED | gone in 2.7 s, exit 143. Note for tests: a background job of a non-interactive shell starts with SIGINT ignored and a trap can't undo it, so the first attempt (`sh … &`) did nothing; the supervisor now restores SIGINT's default like a terminal's foreground job has |
| gamescope closed (SIGTERM) | VERIFIED | everything gone in 1.8 s |
| WM_CLOSE (`wine taskkill /im Rally_3PC.exe`, no `/f`) | VERIFIED | the game honours it: "game exited with 0", everything gone within 6 s |
| WM_SYSCOMMAND SC_CLOSE (the close button's path) | VERIFIED | test helper `.work/realgame/tools/sendkeys.exe syscmd`: exit 0, gone in 4.1 s |
| Alt+F4 as key presses (SendInput) | VERIFIED | `keep-altf4`: exit 0, gone in 4.1 s. No close hook needed for CMR3 |
| The game's own Exit (main menu) | NOT CHECKABLE HEADLESS | injected Enter reaches the game (title → main menu → Championship) but neither SendInput nor XTest arrow keys move the menu, so EXIT can't be selected. A person: Main menu → Exit, then `cmr3-play status` says idle and no `Rally_3PC.exe`/`wineserver` is left |
| Steam's Stop | NOT CHECKABLE here | no Steam in the box; needs the Deck/Steam run |
| Second `cmr3-play --play` while playing | VERIFIED | "already running", exit 0, `status` running, no second game |
| Second `CMR3Launcher.exe` while the game runs | FAILED, fixed | it raised the game's window and said nothing (`another launcher is running: WindowRaised`): the settings window had the game's own title. Fixed (window title "Colin McRae Rally 3 launcher"); rerun: "Colin McRae Rally 3 is already running…", `GameRunning`, exit 0 |
| Second `CMR3Launcher.exe` while the window is open | VERIFIED | private Xvfb: exit 0, `WindowRaised`, still one launcher window |
| One-run flags `--play --scaler fsr --resolution 1280x720` | VERIFIED | helper "starting at 1280x720, scaler fsr", ini 1280×720, swapchain 1280×720, launcher.cfg unchanged; the next plain `--play` went back to "1920x1080, scaler linear" |
| `postfx = color` with the layer set inside gamescope | VERIFIED | play.log: "postfx: color via …/thirdparty/vkbasalt", vkBasalt read `effects = lut:cas`, its first load after gamescope was up (none during gamescope's own Vulkan start); picture: approved by the owner on 2026-11-02 after viewing `shots/postfx-color.jpg` — the Colour grade stays as it is |
| LLD-linked build starts the game under Wine | VERIFIED | every run above |
| VSync on/off visible in frame timing | NOT MEASURED | gamescope's `-T` stats file stayed empty headless; DXVK presents FIFO in both cases (its interval follows the game). Needs a frame-time overlay on a real display |

Screenshots for the owner (`.work/realgame/shots/`): `enhanced-1080p.jpg`
(title, 1920×1080), `original-640x480.jpg`, `deck-1280x800.jpg`,
`main-menu.jpg`, `postfx-color.jpg`.

## 2026-11-02 — bundled DLLs under plain Wine (SDK v0.3.3)

The launcher started by plain `wine` inside a headless gamescope, as
Steam/Proton would start it (no `cmr3-play`, a scrubbed environment from
`env -i`, no `WINEDLLOVERRIDES`), against the owner's game, with
`WINEDEBUG=+loaddll`; then the same through `cmr3-play --play`. Same
throwaway prefix under `.work/realgame/`, the runtime already built.
Supervisor: `.work/realgame/plainwine.sh NAME KIT [helper]`; the
`Loaded …` lines for the bundled DLLs are in
`.work/realgame/logs/{before,after}-{plain,helper}/loaddll.txt`.

| DLL (runtime folder) | before, plain wine | before, cmr3-play | after, plain wine | after, cmr3-play |
| --- | --- | --- | --- | --- |
| `DSOUND.dll` (ASI loader) → `SilentPatchCMR3.asi` | native, ASI loaded | native | native | native |
| `DINPUT8.dll` (Xidi) | **builtin** (Wine's own: no Xidi, no Rally preset) | native | **native** | native |
| `d3d9.dll` (DXVK) | native | native | native | native |

Before, the launcher's `WINEDLLOVERRIDES` (logged) reached the game only
in its environment block, which Wine's loader ignores: dsound and d3d9
happened to load from the game folder anyway, dinput8 did not. After, the
launcher logs `Wine DLL overrides for Rally_3PC.exe: dsound, dinput8, d3d9
(native, then builtin)` and `wine reg query` shows the three values in
`HKCU\Software\Wine\AppDefaults\Rally_3PC.exe\DllOverrides`; Xidi then
loads Wine's `system32\dinput8.dll` itself, as its `[Import]` says. Each
run ended with the game, launcher, wineserver and gamescope gone.

Still pending: the same on real Proton (Steam, non-Steam game) and the
Steam Deck.

## Unreleased — rea-launcher-sdk v0.4.1 (host- and Wine-tested; real game pending)

Host tests, the shell tests (`linux/tests/*.sh` except `e2e.sh`) and
`sdk/scripts/wine-test --locked --workspace` in the gaming box pass;
`sdk/scripts/launcher-check`: 0 errors, 0 warnings, the DRM waiver. What
changed for players, to run headless in the real game (as above):

- `--look original|enhanced|steam_deck` (and `look =`): each look reaches
  the game with the values of its window card; `cmr3-play --play --look
  steam_deck` opens gamescope at 1280×800 (REA_LOOK_SIZES). **Pending.**
- A second `CMR3Launcher.exe` while the game runs: the SDK's message
  ("… is already running from this launcher …") and exit **5** (was 0);
  while the window is open it still comes forward (exit 0). **Pending.**
- Play while `Rally_3PC.exe` still runs from the runtime copy (its
  launcher killed): refused with the same message, exit 5. **Pending.**
- A Play-time error (a wrong game folder) under `cmr3-play` shows a
  dialog (`report::audience`). **Pending.**
- The first start on a fresh prefix opens the settings window
  (`gpu::prefer_glx_or_restart`). **Pending.**
- Quitting and leftovers: game menu, window close / Alt+F4, gamescope
  closed, SIGINT/SIGTERM, `cmr3-play stop` leave no process behind.
  **Pending** (to rerun on this build).

## Platform matrix

What has been checked in the real game, per STANDARDS "Platforms to
verify". Update a row when it is run.

| Platform | Status |
| --- | --- |
| Linux, Wine 11.x + DXVK through `cmr3-play` and gamescope, 4K, NVIDIA | verified 2026-11-01/02 (sections above) |
| Linux AppImage (portable Wine) | verified 2026-11-02 (the README video) |
| Native Windows 10/11 | **pending** — no run on record |
| Steam Deck, Game Mode, as a non-Steam game (controller, 1280×800, suspend/resume) | **pending**; the Steam Deck look is made of individually verified values (1280×800, vsync, 16×, HD UI) |
| Proton without `cmr3-play` (the launcher's own WINEDLLOVERRIDES) | **pending** |
| Multi-monitor under Wine | verified: exclusive fullscreen picks the wrong (portrait) monitor, so fullscreen plays borderless outside gamescope (2026-11-01) |
| HiDPI / 4K settings window | verified at Wine 96 DPI on 4K (1.5×); 144/192 DPI **pending** |
| Ultrawide (21:9) | **pending** (SilentPatch supports any aspect ratio; not seen here) |
| Old-game trouble spots | timers/multi-core: SilentPatch's fixes; audio (DirectSound, no EAX needed): plays under Wine; CD audio: none; intro videos: Wine's quartz fault noted above (crash dialog off); focus loss: **pending** |
