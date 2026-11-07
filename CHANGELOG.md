# Changelog

## Unreleased

## [0.3.4] - 2026-11-07

### Changed

- Updated rea-launcher-sdk to v0.4.4: the AppImage's portable Wine is
  now 11.21, which fixes the startup crash on machines that cannot
  reserve ~64 GiB of address space (16 GB handhelds).

## [0.3.3] - 2026-11-06

### Fixed

- Windows: bundled the 32-bit Microsoft Visual C++ 2015–2022 runtime DLLs
  (`msvcp140.dll`, `msvcp140_atomic_wait.dll`, `concrt140.dll` and
  `vcruntime140.dll`) alongside Xidi, so the game starts out of the box on
  a clean Windows 10/11 install where the 32-bit Visual C++ Redistributable
  is not installed.
- If `Rally_3PC.exe` exits with a fatal Windows status code (`0xC0000000`
  and above), the launcher now shows a clear error dialog with the status
  code and log path instead of closing silently.

## [0.3.2] - 2026-11-05

### Changed

- `CMR3Launcher.exe` now says what it is: Windows shows its name, version,
  publisher and icon (Properties > Details), so antivirus programs are
  less likely to flag it as an unknown program by mistake.
- Updated rea-launcher-sdk to v0.4.3.

## [0.3.1] - 2026-11-04

### Changed

- Updated Xidi, the gamepad bridge, from 4.3.1 to 5.0.0 (#3). Xidi 5
  needs its main file, `Xidi.32.dll`, next to the game as well, so the
  launcher now installs both and takes both out again when the gamepad
  is set to off. `Xidi.ini` no longer has an `[Import]` section, which
  Xidi 5 would report as a configuration error. Game copies made by any
  earlier release are upgraded in place.
- `CMR3Launcher.exe --check` while the launcher's window is open no
  longer runs the check next to it: it says to use the window, or close
  it and try again, and exits 5, like the other launchers of the family
  (from the Windows 10 test report, rea-launcher-sdk#1).
- Updated rea-launcher-sdk to v0.4.2.

### Fixed

- The weekly upstream check no longer reports vkBasalt 0.3.2.10 as an
  update to itself (#4). The package now fetches vkBasalt from its
  release's own source archive instead of the matching commit archive.
  Both hold the same files, so the built layers don't change.

### Documentation

- The README says what to do when Windows SmartScreen stops the
  unsigned launcher ("Windows protected your PC": More info, then Run
  anyway), and lists every registry entry the launcher writes, in
  "Where things live" and "Uninstall" (rea-launcher-sdk#1).

## [0.3.0] - 2026-11-03

### Added

- A `--look` flag and a `look` setting in `launcher.cfg`: `original`,
  `enhanced` or `steam_deck` sets every picture option the way the
  matching card in the window does. Picture options you also give win
  over it. `cmr3-play --play --look steam_deck` sizes gamescope for that
  look too.

### Changed

- Starting the launcher again while the game runs now exits with code 5
  instead of 0, so scripts can tell nothing was started. The message is
  the same as before. With the window open, a second start still brings
  the window to the front.
- Updated rea-launcher-sdk to v0.4.1.

### Fixed

- If the launcher was closed or killed while the game kept running,
  pressing Play again no longer starts a second copy over the first one's
  files. It says the game is running instead.
- On a fresh Wine or Proton prefix the settings window now opens on the
  very first start. Before, it only appeared from the second start on.
- `cmr3-play`'s settings window and `--check --linux` no longer pop up a
  message box of their own; the window or the helper shows the message.
  Errors while starting the game still show one.

## [0.2.1] - 2026-11-02

### Fixed

- The --help golden snapshot no longer contains the version, which made
  the v0.2.0 release build fail; v0.2.0 was tagged but never published,
  so this release carries everything listed under 0.2.0.

## [0.2.0] - 2026-11-02

### Added

- One launcher at a time. Starting it again while it is open brings the
  open window to the front. While the game is running, a second start
  only tells you so, and Play can't start a second copy of the game from
  the same runtime folder. `--check` still runs alongside. On Linux,
  `cmr3-play --play` already refused a second session; a test now covers
  it.
- `.github/workflows/sdk-update.yml`: every Monday the SDK's
  `sdk-update` workflow opens or updates a pull request that moves the
  launcher to the newest rea-launcher-sdk tag and lists its Migration
  items. It never merges.
- README in the rea launcher layout (`sdk/docs/playbooks/readme.md`):
  install steps for Windows, the Steam Deck, the AppImage, Proton and your
  own Wine; First run; the settings window with a looks table; Gamepad;
  the Options table in the shared column order; Where things live for
  Windows and Linux (saves and the GPU preference included); an Uninstall
  that backs up the profile first; How it works; Built on
  rea-launcher-sdk; Contributing; credits for every bundled project, Wine
  and the AppImage tools. `linux/README.md` gets an Environment table.
- Golden snapshots (`crates/cmr3-launcher/tests/snapshots/`) of `--help`,
  the `launcher.cfg` template, and what each look and two option
  combinations write (components, `SilentPatchCMR3.ini`, `Xidi.ini`,
  `WINEDLLOVERRIDES`, patch sets, `cmr3-play` values), so an SDK upgrade
  can't change them unnoticed.
- README: the full settings table (window, file and flag names), exit
  codes, an Uninstall section, "built on rea-launcher-sdk", a developer
  section (shell tests, `wine-test`, `sdk-sync`) and where issues go
  (here or the SDK). `docs/RESEARCH.md` (community components, licences,
  decisions), `docs/CONTRIBUTIONS.md` (drafts C1–C2, not posted) and a
  platform matrix in `docs/RUNTIME_NOTES.md`. AGENTS.md points to the
  SDK's standards and playbooks and records the owner's DRM decision of
  2026-11-02.
- The package carries the licence texts it lacked: Ultimate ASI Loader
  (MIT), ReShade's FX compiler and stb inside vkBasalt, and the Barlow
  fonts (OFL).
- One option table (`config::OPTIONS`) declares every setting once:
  `--help`, the `launcher.cfg` template written on a first run, the file
  and flag parsing and the settings window's rows all come from it, and a
  headless window test fails when a setting lacks a window control or a
  flag. Every setting now has a one-line description under its control.
- The settings window shows the gamepad log switch (Controls), 4×
  texture filtering, a 75° field of view, and any resolution or field of
  view saved in `launcher.cfg` or given as a flag as a chip of its own
  (it was kept but not shown).
- `cmr3-play --play --scaler fsr --resolution 1280x720 …`: flags for one
  run now also reach gamescope and post-processing, without changing
  `launcher.cfg`.
- Issue templates are the SDK's: `bug-report.yml` (where you play, the
  launcher version, `Rally_3PC.exe`'s SHA-256, `launcher.log`) and
  `unknown-edition.yml` (the SHA-256 and where your copy comes from), plus
  a link to rea-launcher-sdk for the shared parts. The unknown-build
  message, `--check` and the README link to `unknown-edition.yml` (it was
  one `bug.yml`).
- Releases carry a `SHA256SUMS` file for the package and the AppImage,
  provenance attestations, and the version's own changelog section as
  notes.

### Changed

- Updated rea-launcher-sdk to v0.3.3, for Wine's per-program DLL
  overrides (below).
- Updated rea-launcher-sdk to v0.3.2. Nothing changes for players on its own;
  the fixes below use what it adds.
- Updated rea-launcher-sdk to v0.3.0. Screen readers now name the two
  folder fields by their headings ("Game folder", "Profile folder") and
  read the example path as their description.
- `cmr3-play --play --scaler fsr …`: the one-run flags are read by the
  SDK's play library (`REA_HELPER_KEYS`) instead of a local copy of
  `launcher.cfg`; `--scaler=fsr` now works too.
- The VSync, Texture sharpness, Smooth edges and Car shadows rows are the
  SDK's option rows with an "As the game" chip (it was a local copy).
  Their descriptions now sit under the label, like Display mode's.
- The portrait credit names the photographer in full: "Photo: Carolyn
  Williams, CC BY 2.0 (via Wikimedia Commons)", in the window, README and
  `THIRD_PARTY.md`. The caption is the SDK's `photo_caption`, so screen
  readers hear the photo described and the caption read out.
- The portrait credit follows Wikimedia Commons ("Carolyn" on Flickr,
  CC BY 2.0); `THIRD_PARTY.md` lists every image in the window, README
  and package. README images are JPEG (the 13 MB 4K capture is now a
  400 KB 1080p one); a stray screenshot (`gs7.png`) is gone.
- The game-folder messages say how to fix the problem they name.
- Clearer names in the window: Texture sharpness (anisotropic
  filtering), Smooth edges (anti-aliasing), Car shadows, HD menus,
  Vulkan (DXVK), Upscaling (the gamescope scaler).
- Anti-aliasing has As the game / On / Off, like vsync.
- `--check` prints its report to the terminal (a message box only when
  started without one), writes it to `launcher.log` and exits 1 when a
  part is missing or the executable is not a known build. `--help` goes
  to the terminal too and is generated from the options; bad flags exit
  with 2.
- An executable that is neither a known DRM-free build nor converted by
  the DRM-free patcher is refused with its SHA-256 and a link to report
  it; the patcher's output must be a known DRM-free build. Builds are
  identified by SHA-256 where a copy was measured (variant B), else by
  the patcher's MD5 list.
- On a Steam Deck the window shows the Steam Deck look as chosen (it has
  the same values as Enhanced there, which was shown instead); its card
  no longer claims to save battery.
- The caption under Colin McRae's photo sits on a solid band.
- `vsync` and `fsaa` are written as on/off (true/false still read).
- Built on rea-launcher-sdk v0.2.0 (Cargo tags, vendored `sdk/`, reusable
  workflows). The settings window's control borders are lighter
  (`#af6d7d`) so they reach 3:1 contrast; text fields show the selection
  in gold. CI also runs the Windows-target tests under Wine.
- The mirror workflow is the SDK's: it never deletes the `thirdparty`
  release and uploads each pinned file under the name its pin expects
  (the vkBasalt source as its commit archive's name).
- `linux/tests` use the SDK's `REA_FUNCTIONS_ONLY` / `REA_PLAY_HERE`
  convention (`CMR3_PLAY_FUNCTIONS_ONLY` and `CMR3_PLAY_HERE` are gone).
- The launcher is built on the shared
  [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk)
  v0.1.0, with the sibling launchers: the runtime copy, profile
  mirroring, component ledger, settings file, settings window, Linux
  helper (`cmr3-play` now sources `rea-play.sh`, shipped beside it) and
  the AppImage come from there. Files, settings, commands and paths stay
  as they were; existing runtime folders, ledgers and profile records
  keep working (temp files of the older launcher are cleaned up once).
- A file in the runtime folder that is neither the launcher's nor the
  game folder's own (one you put there) is no longer overwritten by
  SilentPatch, Xidi or DXVK: that component is left out, with a line in
  `launcher.log` naming the file.
- `launcher.cfg`: unknown settings are named in `launcher.log` (still
  kept in the file); setting a key fills its `#key =` line in place; a
  line starting with `;` is a comment; a file saved in a Windows code
  page reads correctly, and one that cannot be read is never written
  over. `SilentPatchCMR3.ini` keeps its own line endings and encoding.
- The settings window shows where the keyboard or pad focus is; the
  d-pad's left and right step through a row of choices. On Windows the
  window is sized to the screen's DPI.
- `launcher.log` starts afresh after 1 MiB.
- `cmr3-play`: the settings window's errors go to
  `~/.config/cmr3-launcher/settings.log`; `help`, `play` and `settings`
  work as commands too; an interrupted AppImage run also ends the
  prefix's Wine processes.

### Fixed

- SilentPatch's ASI loader, Xidi and DXVK now load when Steam or Proton
  starts `CMR3Launcher.exe` directly, without `cmr3-play`. Wine ignores
  `WINEDLLOVERRIDES` in the game's environment, so under plain Wine the
  game loaded Wine's own `dinput8` and the gamepad preset never applied.
  The launcher now writes Wine's per-program overrides for
  `Rally_3PC.exe` (`HKCU\Software\Wine\AppDefaults\Rally_3PC.exe\DllOverrides`,
  native then builtin) for the parts it installed, keeps a value the
  player set, and removes its own when a part is turned off. The golden
  snapshots gain an "app DLL overrides" line.

- Starting the launcher a second time while the game runs now says the
  game is running. The settings window had the game's own window title
  ("Colin McRae Rally 3"), so the second start found the game's window,
  raised it and said nothing (seen in the real game under Wine). The
  window is now titled "Colin McRae Rally 3 launcher".

- `--check` no longer calls an official 1.1 executable with disc
  protection "not ready" when the bundled patcher converts it. It tries
  the patcher on a scratch copy (the game folder is only read) and says
  the executable is converted at Play. A missing HD UI is listed as
  optional and no longer fails the check.
- Under `cmr3-play`, the AppImage and Proton, an error that stops the
  launcher (game folder not found, an unknown game build, "already
  running") shows a message box again. Since SDK v0.3.0 it only went to
  the log, so the game simply never started. `--help` and `--check` still
  print to the terminal; they open a box only when nobody can see one, and
  never for a Windows script started with `--no-window`.
- `cmr3-play` reads `postfx` and `dxvk` in any case, as the launcher
  does: `--postfx Color` or `postfx = Cinematic` turns the post-processing
  on, and `--dxvk False` gives the "needs DXVK" note. It only matched the
  lowercase spellings before. `--display-mode Windowed` skipping gamescope
  is now tested too.
- The launcher reads `--scaler=fsr` and underscore spellings such as
  `--display_mode windowed` again, as release 0.1 did (scripts used
  them). A test runs the launcher's own parser on both.
- The Windows builds link with LLD (`.cargo/config.toml`). GNU ld 2.47
  left 13 of the launcher's kernel32 imports (`CreateProcessW`,
  `GetModuleHandleW`, `CloseHandle`, …) outside every import table, so
  Windows never filled them in and the launcher could crash on its first
  such call. The SDK's `check-imports.sh` now finds none.
- Unsetting a picture option no longer wipes the choice you made in the
  game's own menu: the launcher records what it wrote to
  `SilentPatchCMR3.ini` (`.cmr3-ini-owned` in the runtime folder) and
  gives a key back only while it still holds that value. Values written
  by earlier releases (no record) are left as they are.
- Opening the Picture tab no longer turned the Original look into Custom
  and wrote `fsaa = off`.
- The Windows-target test of the DRM-free patcher's command compared
  paths as text and failed under Wine; it compares paths now.
- A profile the player copied into the saves folder (a backup, say) is
  used even when it is older than the game's copy; the newer copy used to
  win and could later overwrite it. The launcher records each profile's
  state at the last mirror (`.cmr3-profile-state` in the runtime folder):
  a saves copy changed since then wins, else a runtime copy the last
  session did not get to save.
- The DRM-free patcher never ran: the launcher passed xdelta its own path
  as a first argument, and xdelta stopped with "too many filenames".
- The runtime copy no longer touches the game folder's files through hard
  links: it used to clear their read-only attribute before replacing a
  linked file and to reset their modification time on every sync.
  Read-only originals (a CD or ISO copy) are now copied, never linked,
  and on Linux their copies become writable by the owner only (they used
  to become writable by everyone).
- Symlinked files in the game folder were skipped by the runtime copy;
  they are now brought over (the real file is linked or copied), and a
  dangling symlink is ignored.
- A runtime folder inside the game folder is refused even when it does
  not exist yet and is named through `..`, a relative path or a symlink.
  On volumes that cannot resolve paths (Windows RAM disks, `subst`
  drives, some FUSE mounts) the folders are compared as written instead
  of the launch being refused.
- Files the launcher puts in the runtime copy are written beside it and
  renamed into place: an interrupted or failed write no longer leaves a
  missing or half-written file (the settings ini, a DLL, the patched exe).
  On Windows a refused rename is retried for half a second first (an
  antivirus or OneDrive briefly holding the new file).
- Switching the HD UI off now takes it out: the game's own fonts and
  pictures come back. Upgrading: an overlay put in by an earlier launcher
  is not recorded, so switch the HD UI on for one launch, then off.
- Gamepad "off" and DXVK off remove only the `dinput8.dll`, `Xidi.ini` and
  `d3d9.dll` the launcher installed (recorded by checksum in
  `.cmr3-launcher-components`), and bring back the game folder's own
  `dinput8.dll`/`d3d9.dll` when it has one; a player's own files stay.
  Upgrading: files from before the record are recognised only when they
  are byte-for-byte this release's bundled DLL or one of its `Xidi.ini`
  presets; anything older stays, so remove it by hand (or delete the
  runtime folder) if Gamepad off or DXVK off seems to have no effect.
- Started from plain Proton or Wine (without `cmr3-play`), the game now
  loads the installed ASI loader, Xidi and DXVK: the launcher gives it
  `WINEDLLOVERRIDES` for them, keeping the player's other rules (a rule
  for the same DLL, spelled `dsound.dll` or `DSOUND`, is replaced). Wine is
  also recognised by `WINEPREFIX`, `WINELOADER` or
  `STEAM_COMPAT_DATA_PATH` when `wine_get_version` is hidden.
- A `SilentPatchCMR3.ini` whose last line had no newline got the next new
  section header glued onto that line.
- `cmr3-play`: the launcher's path is handed to the game's script as an
  argument instead of being pasted into its text (a folder name with a
  quote broke it). The run is its own session, recorded by pid and start
  time: `status` and `stop` act only on that session (a stale lock or a
  `Rally_3PC.exe` started elsewhere no longer counts), and `stop` ends its
  whole process group and the prefix's Wine processes. A game still
  running from an older `cmr3-play` (its pid-only lock) counts as running,
  and closing the AppImage while the game runs waits for it to end. The resolution is
  validated before it reaches gamescope, and the postfx Vulkan layer is
  given to the game only, no longer to gamescope as well. Post-processing
  therefore needs DXVK (it works on the game's Vulkan): with `dxvk = off`
  `cmr3-play` says so and skips it, and the settings window notes it.
- `--resolution 1920x1080` on the command line now wins over a file's
  `resolution = original` completely, and `--help` lists `--scaler`.
- Settings window under Wine with its DPI raised (`winecfg` → Graphics,
  e.g. 144 or 192): Wine's own scaling is taken into account, so the
  window is no longer enlarged twice (2.25x or 3x, taller than a 4K
  screen). At Wine's default 96 DPI nothing changes.
- The settings window recommends the Enhanced look until it has saved
  once. It used to do so only for an untouched template, so a first
  `--game` or `--no-window` run (which store the game folder) meant the
  player never got the recommended look.

## [0.1.2] - 2026-11-02

### Changed

- The Colour and Cinematic looks (Linux) grade the game's washed-out
  picture: a colour lookup table gives real blacks and fuller contrast
  (Cinematic more strongly) before the CAS sharpening.
- New profiles steer with the left stick and brake/accelerate with the
  triggers, with no remapping: the game's own defaults pointed both at the
  right stick. The bindings are the ones saved by the game after a remap
  on real hardware.
- The Rally preset's d-pad is the arrow keys only again, so the Controls
  menu moves one row per press; A still presses Enter as well as its
  button.

### Fixed

- Cinematic's depth pass (a player-supplied MxAO) could never load: the
  vkBasalt effect list used a space instead of colons and never declared
  the shader.

### Upgrading

- A profile whose pad was bound under 0.1.1 (where the d-pad was a hat)
  steers from the wrong stick now: remap steering and the pedals once in
  Options > Controls. Profiles from 0.1.0 are unaffected.

## [0.1.1] - 2026-11-01

### Changed

- New profiles start on the gamepad: with a Xidi preset on, a profile
  whose controls were never set gets Xidi's controller and Medium
  vibration chosen in Options > Controls, as the game itself would write
  them. Profiles set up by hand, keyboard included, are left alone.
- The Rally preset keeps every control a game-controller control; the
  d-pad and A also press the arrow keys and Enter, because the game's
  menus answer only the keyboard. (Start and Back no longer act as keys.)
- The settings window and `launcher.cfg` describe what each preset does,
  from one shared text.

### Fixed

- Under the AppImage, Wine's debugger stayed on and Xidi/DXVK overrides
  were skipped: the AppImage's own `WINEDLLOVERRIDES` replaced the
  launcher's instead of joining them. A Wine fault could freeze the game
  instead of ending it.
- Wine's crash dialog is turned off in the launcher's prefix, as the
  0.1.0 notes said but the code never did.
- Every pad appeared twice to the game under Linux: `cmr3-play` named
  `/dev/input/js0` to SDL, which found the same pad on its own.
- After a game crash, the next launch could put the older saved profile
  over the newer one the game had written; the newer copy now wins.
- `postfx = off`, `--postfx`, `--shadows` and `--gamepad-log` were
  documented but rejected; the command line and `launcher.cfg` now share
  one parser.

### Known issues

- Native Windows and Steam Deck still need hands-on verification.
- Some Linux races end on a page fault inside DXVK's `d3d9.dll` (a
  corrupted pointer, read as text); 0.1.0's notes blamed Wine's `ntdll`.
  The game now ends instead of freezing; the cause is not fixed yet.
- A brand-new player's very first session runs before the game has
  created its profile, so the pad is chosen from the second start on (or
  once by hand in Options > Controls).
- In the game's Controls menu the d-pad moves two rows per press: that
  menu reads the controller's hat as well as the arrow key.

## [0.1.0] - 2026-11-01

First packaged release of the launcher and its Linux helper.

### Added

- Windows launcher package and Linux AppImage, built and published by
  GitHub Actions for version tags.
- Automatic game-edition detection, a separate runtime copy, and the
  official 1.1-to-DRM-free xdelta conversion from the SilentPatch project.
- Bundled SilentPatch, Xidi, DXVK, Bekoha's HD UI, and 32/64-bit vkBasalt;
  every download is pinned by SHA-256 and has a repository mirror fallback.
- Widescreen at any aspect ratio, every display resolution, fullscreen,
  borderless and windowed modes, vsync, anisotropic filtering, anti-aliasing,
  FOV control, and the HD interface.
- Original 2003, Enhanced, Steam Deck and Custom looks, with a settings
  window usable by keyboard, mouse or gamepad.
- Rally and standard Xidi mappings, hot-swappable controllers, optional Xidi
  diagnostics, and profile preservation across launches.
- Optional soft/sharp car shadows and Linux Colour (CAS) or Cinematic
  post-processing. Cinematic's AO shaders remain user-supplied.
- Launcher and 4K gameplay screenshots, installation/setup instructions,
  component credits, licence documentation, and runtime verification notes.

### Fixed

- Reliable Wine window creation, GPU-accelerated GLX, monitor-independent
  fullscreen behavior, and correct resolution restoration before each run.
- Clean game exit and working controller environment under Wine.
- DRM-free patching when the game or patcher path contains spaces, using
  correct Windows command-line quoting for every argument.
- Xidi's startup configuration popup by removing an unsupported log key.
- vkBasalt loading when a legacy generated layer manifest is present.
- The 4K Enhanced label overflowing its card and Picture options colliding
  with the footer; long labels now wrap and Picture scrolls on small screens.

### Known issues

- Native Windows and Steam Deck still need hands-on verification.
- One Linux gameplay session stopped on a Wine `ntdll` heap fault. The
  originating module was not isolated, so this release does not claim a fix.

[0.1.2]: https://github.com/akitaonrails/rea-colin-mcrae-3/releases/tag/v0.1.2
[0.1.1]: https://github.com/akitaonrails/rea-colin-mcrae-3/releases/tag/v0.1.1
[0.1.0]: https://github.com/akitaonrails/rea-colin-mcrae-3/releases/tag/v0.1.0
