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
