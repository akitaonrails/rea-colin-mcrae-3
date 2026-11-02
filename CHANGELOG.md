# Changelog

## [Unreleased]

### Fixed

- The DRM-free patcher never ran: the launcher passed xdelta its own path
  as a first argument, and xdelta stopped with "too many filenames".
- The runtime copy no longer touches the game folder's files through hard
  links: it used to clear their read-only attribute before replacing a
  linked file and to reset their modification time on every sync.
- A runtime folder inside the game folder is refused even when it does
  not exist yet and is named through `..`, a relative path or a symlink.
- Files the launcher puts in the runtime copy are written beside it and
  renamed into place: an interrupted or failed write no longer leaves a
  missing or half-written file (the settings ini, a DLL, the patched exe).
- Switching the HD UI off now takes it out: the game's own fonts and
  pictures come back. Upgrading: an overlay put in by an earlier launcher
  is not recorded, so switch the HD UI on for one launch, then off.
- Gamepad "off" and DXVK off remove only the `dinput8.dll`, `Xidi.ini` and
  `d3d9.dll` the launcher installed (recorded by checksum in
  `.cmr3-launcher-components`), and bring back the game folder's own
  `dinput8.dll`/`d3d9.dll` when it has one; a player's own files stay.
- Started from plain Proton or Wine (without `cmr3-play`), the game now
  loads the installed ASI loader, Xidi and DXVK: the launcher gives it
  `WINEDLLOVERRIDES` for them, keeping the player's other rules. Wine is
  also recognised by `WINEPREFIX`, `WINELOADER` or
  `STEAM_COMPAT_DATA_PATH` when `wine_get_version` is hidden.
- A `SilentPatchCMR3.ini` whose last line had no newline got the next new
  section header glued onto that line.

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
