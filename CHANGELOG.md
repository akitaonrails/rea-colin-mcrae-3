# Changelog

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

[0.1.0]: https://github.com/akitaonrails/rea-colin-mcrae-3/releases/tag/v0.1.0
