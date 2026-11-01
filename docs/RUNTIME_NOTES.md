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
