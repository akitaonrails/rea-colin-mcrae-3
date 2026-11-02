# Project rules

This repository builds a self-contained compatibility package that runs the
user's own copy of the 2003 PC game **Colin McRae Rally 3** on modern
Windows and on Linux (Proton/Wine). It is not a distribution of the game.

## Non-negotiable boundaries

- Never commit, vendor, embed, or publish original executables, DLLs,
  media, archives, fonts, tracks, car data, or other game assets, or
  anything copied out of them.
  Exceptions (owner decisions, recorded in `THIRD_PARTY.md`):
  - The HD UI overlay (`thirdparty/hd-ui`, Bekoha's redrawn fonts and
    interface art) ships with the package, fetched and checksummed at
    build time from `packaging/third-party.txt`; it is never committed.
  - SilentPatch's Language Pack (original translations and co-driver
    audio) is **not** bundled and never will be: it is original game
    content from regional releases. Players who want it can add it to
    their game folder themselves.
- The user's game folder is read only. The launcher builds a separate
  runtime folder from it and swaps whole files in (`rea_launcher::fsx`),
  never writing through a hard link back to the original.
- Never commit a developer's absolute game path. Commands and code take an
  explicit path (or `CMR3_GAME` in examples).
- Put local experiments, traces and investigation output under `.work/`.

## Product rules

- **Self-contained.** The user installs this package and points it at the
  game. Whatever the game needs — disc-protection removal from the
  official 1.1 update, widescreen, the pad bridge, the Vulkan translator —
  is bundled and configured by us.
- **Windows first, easy on Linux.** Ship Windows binaries that also run
  under Proton or Wine with no extra setup.
- **Reuse open source.** Where a maintained open-source component already
  does the job (SilentPatch, Xidi, DXVK, xdelta), bundle and configure it
  instead of writing our own. Check licences before bundling and record
  each component with its licence and source in `THIRD_PARTY.md`.
- **The modern experience is the default.** The launcher's first run
  picks the Enhanced look: the screen's own resolution, widescreen,
  vsync, 16× anisotropic filtering and the HD UI, with the Rally gamepad
  preset and DXVK under Wine. Everything can still be turned off.
- **The game's own settings stay the game's.** The launcher writes only
  the keys it owns in `SilentPatchCMR3.ini`; the player's gamma, detail
  and control choices survive every launch.

## Code and commits

- General solutions first. Before adding a case, find the single place a
  whole class of cases goes through and fix it there.
- Rust for our own code. Keep `unsafe` and Win32 FFI in narrow modules.
- Unit-test the logic that can run on the host (edition detection, runtime
  folder layout, configuration, ini editing, component install) before
  each milestone commit; record what was checked by running the game in
  `docs/RUNTIME_NOTES.md`.
- Use Conventional Commit subjects. Commit at working milestones.
- This launcher is a sibling of `rea-colin-mcrae` (the 1998 game): keep
  the two repositories recognisably alike — the same launcher shape, the
  same settings-window look, the same Linux helper — so improvements
  cross over easily. That launcher patches the game in memory; this one
  assembles open-source components around it.

## Shared SDK

- The generic launcher code lives in
  [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk),
  pinned by tag: the Rust crates `rea-launcher` (runtime sync, whole-file
  swaps, save mirroring, the component ledger, ini and `launcher.cfg`
  editing, Wine overrides, display and pad logic, the log),
  `rea-launcher-win32` (all Win32 calls) and `rea-launcher-ui` (the
  settings window's theme, widgets, sizing and pad navigation); the
  Linux play library and packaging scripts, vendored under `sdk/`; and
  the reusable CI, release and upstream-watch workflows.
- This repository keeps the game's knowledge: the runtime `Policy`, the
  profile, SilentPatch keys, Xidi presets, the HD UI record, the DRM-free
  patcher, the look's colours and art, `cmr3-play`'s settings and
  post-processing, the pins and the package contents.
- Never fork SDK code locally, and never edit `sdk/` by hand. When the
  SDK lacks something or has a bug, keep the game's own code for that
  piece and propose the change upstream.
- Bumping: change the tag in the root `Cargo.toml` (and the `@vX.Y.Z` of
  the workflows in `.github/workflows/`), run
  `../rea-launcher-sdk/scripts/sdk-sync .` from an SDK checkout at that
  tag, commit `sdk/` with `sdk/VERSION`; CI's
  `sdk-sync --check` fails on drift.
- Tests and probes run headless by default: the Windows-target tests
  under Wine with the SDK's `scripts/wine-test`, real-game runs in
  `gamescope --backend headless`. Open a visible window (the settings
  window, the game) only deliberately.
