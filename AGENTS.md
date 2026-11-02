# Project rules

**Read first:** the SDK's binding
[docs/STANDARDS.md](https://github.com/akitaonrails/rea-launcher-sdk/blob/master/docs/STANDARDS.md)
(vendored version: the tag in `sdk/VERSION`) and its playbooks
(`docs/playbooks/` in rea-launcher-sdk):

- `new-launcher.md` — research community fixes first (record them in
  `docs/RESEARCH.md`), then build; headless verification.
- `community-components.md` — pinning, bundling, download-on-request,
  licences, mirrors.
- `binary-patching.md` — exact identity (SHA-256) before patching,
  all-or-nothing sites, unknown builds refused with the digest.
- `contribute-back.md` — drafts in `docs/CONTRIBUTIONS.md`; the owner
  posts; credentials (e.g. `~/.config/rea/pcgamingwiki.txt`) are never
  printed, logged, committed, passed as arguments or handed to agents.
- `sdk-change.md` — SDK gaps go upstream, never patched locally; bumps
  and golden snapshots.

What those rules mean here:

- **Settings window:** any change to its look or layout goes through a
  design skill (`frontend-design`, then a `web-design-guidelines`
  review), within the SDK theme; accessibility first (palette and text
  size tests in `look.rs`).
- **GUI/CLI parity:** every option is a row of `config::OPTIONS`; the
  window test `every_option_has_a_control_in_the_window` fails when one
  lacks a control or a flag. Every option has end-to-end tests (flag →
  `launcher.cfg` → `SilentPatchCMR3.ini` / `Xidi.ini` / overrides, in
  `plan.rs`; window clicks → saved values, in `window.rs`).
- **Headless testing:** host tests and `sdk/scripts/wine-test` never
  open windows; the real game runs only in `gamescope --backend
  headless`, and only when a check needs it.
- **Golden snapshots** (`tests/snapshots/`): a diff must match an
  intended change or an SDK Migration item; never regenerate blindly.

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
  - The DRM-free conversion: **owner decision, 2026-11-02** — the
    SilentPatch project's community xdelta patcher, which turns an
    official 1.1 executable into the publisher's own DRM-free 1.1 build,
    is the approved way to remove the disc protection (also recorded in
    `THIRD_PARTY.md`). We never write a crack of our own; any build the
    patcher doesn't convert to a known DRM-free build is refused with its
    SHA-256 and the issue link.
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
  piece and propose the change upstream (issues and PRs about shared code
  go to rea-launcher-sdk, not here).
- Windows code must be run, not only linted: link the i686 build and run
  `sdk/scripts/wine-test --locked --workspace` (MinGW and Wine may live
  in the `gaming` distrobox; call it with absolute paths).
- Bumping: change the tag in the root `Cargo.toml` (and the `@vX.Y.Z` of
  the workflows in `.github/workflows/`), run
  `../rea-launcher-sdk/scripts/sdk-sync .` from an SDK checkout at that
  tag, commit `sdk/` with `sdk/VERSION`; CI's
  `sdk-sync --check` fails on drift.
- Tests and probes run headless by default: the Windows-target tests
  under Wine with the SDK's `scripts/wine-test`, real-game runs in
  `gamescope --backend headless`. Open a visible window (the settings
  window, the game) only deliberately.
