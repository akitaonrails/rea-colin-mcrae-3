# Colin McRae Rally 3 launcher — instructions for coding agents

This repository makes the player's own copy of **Colin McRae Rally 3**
(2003) run on modern Windows and on Linux (Proton/Wine, AppImage) with a
settings window, sensible defaults and no manual steps. It is not a
distribution of the game. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk), at
the tag in `sdk/VERSION`.

**Read `sdk/docs/STANDARDS.md` before any work; the playbooks are in
`sdk/docs/playbooks/`** (vendored from the SDK at the tag in `sdk/VERSION`).

## Always

- The game's files are read-only: never committed, copied into the repo or published.
- Everything outside this repository is the user's and read-only (game installs,
  other repos, Wine prefixes, saves, home configs, containers, the SDK checkout)
  unless the user asked for or approved that specific change: back up first,
  never delete by a broad glob, ask when unsure.
- Headless by default; open a visible window only when a check needs a person, and say so first.
- Never print, log or commit credentials.
- Push, tag, release or post only with the user's go-ahead.
- Research community fixes first (`docs/RESEARCH.md`); patch only what nobody solved.
- Never edit `sdk/`; SDK gaps and bugs go to the SDK repository as issues or PRs.
- Clean up after a release or a long session: `sdk/scripts/clean`, then `--yes` (log scratch in `.work/.task-created`).

## Layout

| Path | What |
|---|---|
| `crates/cmr3-launcher/src/edition.rs` | Known builds (SHA-256, the patcher's MD5 list), the unknown-edition refusal |
| `crates/cmr3-launcher/src/config.rs` | The option table `config::OPTIONS`: `launcher.cfg`, flags, `--help`, template |
| `crates/cmr3-launcher/src/plan.rs` | What a launch writes (`SilentPatchCMR3.ini`, `Xidi.ini`, overrides); option tests end to end |
| `crates/cmr3-launcher/src/form.rs`, `window.rs`, `look.rs`, `ui.rs` | The settings window, its looks, palette and text-size tests, headless window tests (`every_option_has_a_control_in_the_window`) |
| `crates/cmr3-launcher/src/runtime.rs`, `profile.rs` | The runtime copy's `Policy`, the `CMR3` profile mirror |
| `crates/cmr3-launcher/src/components.rs`, `sp.rs`, `xidi.rs` | SilentPatch, Xidi, DXVK, the HD UI and the DRM-free patcher |
| `crates/cmr3-launcher/src/main.rs` | Windows wiring: window, `launcher.log`, the game's process |
| `crates/cmr3-launcher/tests/snapshots/` | Golden snapshots; accept an intended change with `CMR3_UPDATE_SNAPSHOTS=1 cargo test --test snapshots`, then read the diff |
| `linux/cmr3-play`, `linux/tests/` | The Linux helper (sources `sdk/rea-play.sh`; gamescope scaler, vkBasalt post-processing) and its tests |
| `packaging/` | Pins (`third-party.txt`), allowlist, package script, vkBasalt build, AppImage entry and icon |
| `docs/RESEARCH.md`, `docs/RUNTIME_NOTES.md`, `docs/CONTRIBUTIONS.md` | Findings, verification, drafts to share |

The full check list is in `sdk/docs/STANDARDS.md` ("Code", "Testing");
here the Windows target is `i686-pc-windows-gnu`. MinGW, lld and Wine live
in the `gaming` distrobox: run the release build and `sdk/scripts/wine-test
--locked --workspace` as `distrobox enter gaming -- sh -c 'cd
/absolute/path && …'`. Examples take the game folder from `CMR3_GAME`,
never a developer's path.

## Colin McRae Rally 3: what is specific to this game

- **Editions:** the publisher's DRM-free 1.1 builds are known by SHA-256
  (`edition.rs`); an official 1.1 executable with SecuROM/SafeDisc is
  converted by the DRM-free patcher; anything the patcher refuses is
  refused with its SHA-256 and the issue link.
- **Community components used** (bundled and configured, never our own
  code for what they do; `docs/RESEARCH.md`, `THIRD_PARTY.md`):
  SilentPatch (widescreen, fixes, ASI loader), Xidi (the pad bridge, Rally
  preset), DXVK (Vulkan under Wine), the DRM-free xdelta patcher,
  Bekoha's HD UI, vkBasalt (Linux post-processing).
- **The modern look is the default:** the first run picks Enhanced — the
  screen's own resolution, widescreen, vsync, 16× anisotropic filtering
  and the HD UI, with the Rally gamepad preset and DXVK under Wine.
  Everything can still be turned off.
- **The game's own settings stay the game's:** the launcher writes only
  the keys it owns in `SilentPatchCMR3.ini` (recorded in
  `.cmr3-ini-owned`); the player's gamma, detail and control choices
  survive every launch.
- **Files the game writes:** its `CMR3` profile, mirrored to the saves
  folder (`Saved Games\Colin McRae Rally 3` by default) before and after
  each run.
- **Sibling launcher:** keep this repository recognisably alike to
  `rea-colin-mcrae` (the 1998 game): the same launcher shape, window look
  and Linux helper, so improvements cross over. That launcher patches the
  game in memory; this one assembles open-source components around it.
- **Owner decisions:**
  - **2026-11-01** — Bekoha's HD UI (`thirdparty/hd-ui`: redrawn fonts and
    interface art, no licence text) ships with the package, fetched and
    checksummed at build time from `packaging/third-party.txt`; never
    committed. Recorded in `THIRD_PARTY.md`.
  - **2026-11-02** — the SilentPatch project's community xdelta patcher,
    which turns an official 1.1 executable into the publisher's own
    DRM-free 1.1 build, is the approved way to remove the disc protection
    (also in `THIRD_PARTY.md`). We never write a crack of our own.
  - SilentPatch's Language Pack (original translations and co-driver
    audio from regional releases) is **not** bundled and never will be;
    players may add it to their game folder themselves.
