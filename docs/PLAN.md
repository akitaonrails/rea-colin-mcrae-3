# Compatibility package: plan, inventory and status

This is the living status page. The **Status** section says where the work
is; the **Research** section records what was studied (and where it came
from) so decisions don't get re-litigated; the **Design** section explains
how the pieces fit.

## Status (2026-11-01)

**Current milestone: M1 (assembled, host-tested).** Every component this
launcher wraps is proven on real hardware — the distrobox-gaming setup
(see Research) plays CMR3 at 4K/60 with pads using exactly the stack
bundled here. The launcher code is new: 42 host unit tests and the Linux
helper tests pass; the Windows build and a real run through the launcher
are the next step (CI builds the package on push).

| Milestone | What it proves | State |
| --- | --- | --- |
| M0 Research + design | every requirement has a covered, licence-checked path | **done** (below) |
| M1 Launcher assembled | user's folder → bundled components configured → game starts | **done** 2026-11-01 |
| M2 Linux run | fresh prefix → the game plays through the launcher | **done (first pass)**: windowed 1080p, Vulkan swapchain confirmed, profile saved, game folder byte-identical (`docs/RUNTIME_NOTES.md`) |
| M3 Windows run | the same package natively | needs a Windows machine |
| M4 Field test | a real pad in a stage, gamescope via cmr3-play, DRM→DRM-free on a protected 1.1 copy, the settings window | open |

**Next:**
1. Run the built launcher under the gaming distrobox's Wine against the
   user's game copy; record it in `docs/RUNTIME_NOTES.md`.
2. Push and let CI build the zip and AppImage; test the AppImage on the
   desktop.
3. Verify the DRM→DRM-free path on an original 1.1 executable (needs a
   real protected copy).
4. Steam Deck pass: the window at 800p, the Deck look, Game Mode.

## Research (2026-11-01)

### The game on this machine (distrobox-gaming)

The `gaming` distrobox has CMR3 installed from the Magipack repack
(`Colin McRae Rally 3_Repack_v4_Setup.exe`) and tuned by hand; the notes
below are condensed from `ansible/group_vars/all/pc_racing.yml` and the
memory pages (`gotchas/pc-racing-black-window-stale-prefix`,
`concepts/wine-distrobox-gaming`):

- The repack ships a **DRM-free 1.1** executable (MD5
  `2744404b…`, one of the three known DRM-free builds in
  CookiePLMonster's patcher), SilentPatch 2.1 and an ASI loader — but
  **no native d3d9.dll**, despite the repack's claims of dgVoodoo.
- Without intervention the game falls through to Wine's builtin d3d9 →
  wined3d → OpenGL: **~10 FPS at 4K on an RTX 5090**. With DXVK's d3d9
  in the prefix (`WINEDLLOVERRIDES d3d9=n,b`) it runs at native speed.
  The launcher instead drops DXVK's d3d9.dll **next to the game in its
  runtime copy** (app-dir wins the DLL search), so nothing touches the
  prefix.
- **Xidi** as `dinput8.dll` with a custom mapper makes pads work: CMR3
  binds accelerator/brake to the left stick's Y axis, so RT/LT map onto
  it (both at once — left-foot braking), the physical Y is nulled (no
  accidental throttle), and the d-pad/Start are keyboard arrows/Enter/Esc
  for the menus. That mapping is preserved verbatim as the `rally`
  preset. The repack's own ASI loader was renamed to `dsound.dll` to free
  the `dinput8` slot — this launcher does the same from the start.
- The game is a 32-bit Direct3D 9 title (PS2.0 shaders in `shaders/`);
  `Rally_3PC.exe` imports d3d9, dinput8, dsound, winmm.
- The game's settings live in the registry (stock) — redirected by
  SilentPatch to a portable `SilentPatchCMR3.ini` in the game folder,
  whose `[Registry]`/`[Graphics]` keys are what this launcher writes.
- The player profile is one file, `CMR3`, in the game's root.
- Operational gotchas on that box (lib32 rebinds after container
  restarts, stale Wayland socket binds after a Hyprland restart) are
  handled by the distrobox tooling, not the launcher; `cmr3-play`
  inherits their fixes through `CMR3_RUN`.

### The wider scene (what exists, what we chose)

- **SilentPatch for CMR3** (CookiePLMonster, MIT,
  github.com/CookiePLMonster/SilentPatchCMR3): the centerpiece. All
  resolutions and **arbitrary aspect ratios** with the whole UI
  repositioned, desktop-resolution default, windowed/borderless, vsync,
  refresh rate, anisotropic filtering, FOV options (30–150, chase and
  interior), portable ini settings, d-pad menu navigation, F5 HUD toggle,
  plus dozens of rendering fixes (sun occlusion queries, water-reflection
  state cache, shadows with AA, split-screen on modern CPUs, timer
  precision, Alt+F4). Requires the official 1.1 executable. Bundled.
- **DRM-free patcher** (same project): xdelta updates (efigs + polish)
  that turn the official 1.1 SecuROM/SafeDisc executables into the
  DRM-free code the late re-releases shipped; the patch itself only
  applies to matching files, which is how it tells versions apart. The
  launcher runs it (hidden) on its runtime copy when the executable
  isn't already DRM-free, and verifies by MD5 against the three known
  DRM-free builds.
- **HD UI** (Bekoha, v1.0.1): font atlases and most of the race and menu
  interface redrawn for 4K, EFIGS+PL+CS. Redistributed unmodified with
  the package (owner decision, as with the 1998 game's HD pack); overlay
  into the runtime copy.
- **Xidi** (samuelgr, BSD-3-Clause): DirectInput/WinMM for XInput pads,
  virtual controllers, live pad swapping, keyboard simulation. Bundled as
  `dinput8.dll` (Win32 build, v4.3.1 — the version proven on this box).
- **DXVK** (Zlib): d3d9 through Vulkan; only needed under Wine/Proton
  (Proton ships its own). v3.1.1, x32 `d3d9.dll` only, installed next to
  the game in the runtime copy, never on Windows.
- **AuToMaNiAk005's widescreen patches**: the pre-SilentPatch hex
  patches for CMR3/04 (credited by SilentPatch). Superseded — nothing to
  incorporate.
- **dgVoodoo2**: a D3D9→D3D11 wrapper useful on Windows for broken
  drivers; unnecessary here (native d3d9 on Windows, DXVK on Linux) and
  not licence-cleared for bundling. Not included.
- **SilentPatch Language Pack / Nicky Grist Pack**: original game
  content (regional strings, fonts, co-driver audio) — never bundled
  (see AGENTS.md); players can add it to their game folder.
- The **official 1.1 patch** is a Codemasters download (linked in the
  README); the launcher cannot ship it, and says so when a v1.0 game
  shows up.
- Rejected alternatives: writing our own in-memory widescreen/gamepad
  hooks (the 1998 game's approach) — SilentPatch's two years of fixes
  are far ahead, and MIT-licensed; wrapping the game in CemUI/general
  "old game" wrappers — none handle CMR3's pad axis quirks.

### Sources

- CookiePLMonster, "Remastering Colin McRae Rally 3 with SilentPatch"
  (silentsblog.com / cookieplmonster.github.io, 2023-01-15) and the
  SilentPatchCMR3 source (INI schema, defaults, menu value ranges).
- PCGamingWiki: Colin McRae Rally 3 (patch links, the SilentPatch
  recommendation).
- This machine: `~/Projects/distrobox-gaming` (ansible role, Xidi
  config, launch script, the 3.2 GB install itself) and its ai-memory
  pages.

## Design

### Shape

A sibling of `rea-colin-mcrae` (the 1998 game's launcher): the same
Rust/i686 Windows binary with an egui settings window, the same
`launcher.cfg` conventions, the same Linux helper and AppImage packaging.
Where the 1998 launcher applies its own fixes in memory to a suspended
process, this one **assembles open-source components around the game** in
a private runtime folder — the product rules' "reuse open source" path.

```text
CMR3Launcher.exe          settings window, edition check, runtime sync,
                          component install, SilentPatchCMR3.ini, launch
thirdparty/silentpatch/   SilentPatchCMR3.asi + ASI loader (as dsound.dll)
thirdparty/xidi/          dinput8.dll + generated Xidi.ini
thirdparty/patcher/       xdelta.exe + efigs/polish patches (DRM-free)
thirdparty/dxvk/          d3d9.dll (Wine only)
thirdparty/hd-ui/         the 4K interface overlay
linux/cmr3-play           Wine + gamescope wrapper (AppImage: + portable Wine)
```

### Launch sequence

1. Read `launcher.cfg` (portable one wins), merge the command line.
2. Find the game (`--game`, `CMR3_GAME`, exe-adjacent), hash
   `Rally_3PC.exe` (MD5 against the DRM-free set; SHA-256 logged).
3. `runtime::sync` — bring the runtime copy up to date: static trees
   (`Data`, `fonts`, `shaders`, root DLLs/exe) **hard-linked** when the
   volumes allow, the rest copied; a manifest records source stamps; the
   `CMR3` profile is copied once and then owned by the game.
4. Mirror the profile in from the saves folder (and out again at exit).
5. If the runtime exe isn't DRM-free: run the bundled xdelta (efigs, then
   polish) against a backup, swap the patched file in, verify.
6. Install the components (whole-file swaps, so hard links never write
   through): SilentPatch + loader, Xidi + ini, DXVK under Wine, HD UI
   overlay.
7. Write the owned keys of `SilentPatchCMR3.ini` (resolution, display
   mode, vsync, anisotropic, FSAA, FOV), leaving every other line as the
   game wrote it.
8. Windows: prefer the fast GPU; start `Rally_3PC.exe` and wait.
   Linux: same binary under Wine/Proton; `cmr3-play` sizes gamescope
   from `launcher.cfg` and sets the DLL overrides.

### What is intentionally not here

- No in-memory binary patching of our own (nothing needs it yet); if a
  fix is ever needed that no component covers, REA + Ghidra are the
  tooling for finding it, and `cmrhooks`-style injection is the pattern.
- No language-pack or official-patch redistribution (original game
  content).
- No rumble plumbing (the game has no force feedback on PC).
