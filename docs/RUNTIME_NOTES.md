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
README as `docs/images/gameplay-4k.png`.

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
