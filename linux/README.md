# Colin McRae Rally 3 on Linux

The launcher itself is a Windows program (the same one as on Windows);
on Linux it runs through Wine or Proton. There are three ways to play:
the AppImage, Steam with Proton, or your own Wine with `cmr3-play`.

## The AppImage

```sh
chmod +x CMR3Launcher-<version>-x86_64.AppImage
./CMR3Launcher-<version>-x86_64.AppImage
```

It contains the launcher, the bundled components (SilentPatch, Xidi,
DXVK, the HD UI) and a portable Wine, so nothing else has to be
installed. The settings window opens: press **Browse…** and pick your
game folder (your Linux files are under drive `Z:`), choose a look and
press **Play**. If [gamescope](https://github.com/ValveSoftware/gamescope)
is installed, the game runs in it at the right size (recommended);
otherwise in a plain window. Its Wine prefix is
`~/.local/share/cmr3-launcher/prefix` (change it with `CMR3_PREFIX`).

## Steam (Proton)

1. Unpack the zip, for example into `~/Games/cmr3-launcher`.
2. In Steam: **Games → Add a Non-Steam Game to My Library… → Browse** and
   pick `CMR3Launcher.exe`.
3. Right-click it in your library → **Properties → Compatibility** →
   tick **Force the use of a specific Steam Play compatibility tool** and
   choose a Proton (GE-Proton tested on the 1998 game's launcher;
   Proton Experimental should work too).
4. Press **Play**. The settings window opens: press **Browse…** and pick
   your game folder (under drive `Z:`), choose a look and press **Play**.
   Everything is remembered for next time.

DXVK ships with Proton, so the game draws through Vulkan there already;
the launcher's `thirdparty` folder must sit next to `CMR3Launcher.exe`
(it does in the zip).

### Steam Deck

Follow **Steam (Proton)** above in Desktop Mode, then play from Game Mode:

- Pick the **Steam Deck** look: 1280×800 at the panel's 60 Hz (the
  window recommends it on a Deck).
- The buttons follow the **Rally** gamepad preset: the triggers are the
  pedals (both at once), and the d-pad and A work the menus.
- The touchscreen and the right trackpad drive the settings window.

None of this has been tested on a Deck yet.

## Your own Wine

If you have Wine and
[gamescope](https://github.com/ValveSoftware/gamescope) installed,
`linux/cmr3-play` runs the game in its own correctly sized screen:

```sh
~/Games/cmr3-launcher/linux/cmr3-play
```

It shows the settings window first, with three more rows (Vulkan (DXVK),
the gamescope upscaling and post-processing), then starts the game in
gamescope. When Wine and gamescope live in a
[distrobox](https://github.com/containers/distrobox) container, run it
through the container:

```sh
CMR3_RUN="distrobox enter gaming --" ~/Games/cmr3-launcher/linux/cmr3-play
```

`cmr3-play` also understands `--play` (no settings window), `stop` and
`status`. Launcher flags may follow `--play` for one run and reach
gamescope too, e.g. `cmr3-play --play --scaler fsr --resolution
1280x720`, or `cmr3-play --play --look steam_deck` (gamescope opens at
the look's size: 640×480, the screen's, or 1280×800). A start while the
game runs exits 5 and starts nothing. These commands only look at the session `cmr3-play` itself
started (its own process group, recorded by pid and start time): `stop`
ends that session and the prefix's Wine processes, and it neither reports
nor stops a `Rally_3PC.exe` started some other way.

## Environment

`cmr3-play` reads these, all optional:

| Variable | What it does |
| --- | --- |
| `CMR3_PREFIX` | The Wine prefix (default `~/.local/share/cmr3-launcher/prefix`) |
| `CMR3_LAUNCHER` | Where `CMR3Launcher.exe` is (default: next to `cmr3-play`, or one folder up) |
| `CMR3_RUN` | A wrapper that runs Wine and gamescope, e.g. `distrobox enter gaming --` |
| `CMR3_WAIT` | Set: wait for the game to end (the AppImage sets it) |
| `CMR3_VKBASALT` | A folder with `libvkbasalt.so`, for post-processing |

## Where things are

| What | Where |
| --- | --- |
| Settings (`launcher.cfg`) | inside the Wine prefix, `…/drive_c/users/<you>/AppData/Local/CMR3Launcher/` |
| Log (`launcher.log`) | the same folder |
| The launcher's copy of the game | `…/AppData/Local/CMR3Launcher/Rally3` (about 3 GB; hard-linked when possible) |
| Your profile | `…/drive_c/users/<you>/Saved Games/Colin McRae Rally 3` |
| The Wine prefix | `~/.local/share/cmr3-launcher/prefix` (`CMR3_PREFIX`) |
| `cmr3-play`'s session lock and logs (`play.log`, `settings.log`) | `~/.config/cmr3-launcher` |

`cmr3-play` needs `rea-play.sh` (the shared play library) beside it; the
zip and the AppImage ship it there.

## Troubleshooting

- **The game crawls at 4K**: DXVK is off or missing. In the settings
  window turn **Vulkan (DXVK)** on; with your own Wine, check that
  `thirdparty/dxvk/d3d9.dll` exists next to the launcher. Without it
  Wine draws through its OpenGL translator, which manages about 10 FPS
  at 4K on a fast card.
- **Post-processing (Colour, Cinematic) changes nothing**: it is a
  Vulkan layer on the game's DXVK, so it needs **Vulkan (DXVK)** on (and
  `cmr3-play`); with DXVK off the game draws through OpenGL and the
  layer has nothing to work on.
- **The game window is the wrong shape**: keep the game's own display
  options as they are and set the resolution in the launcher; SilentPatch
  matches the aspect ratio to the resolution.
- **The pad steers but has no throttle**: use the **Rally** gamepad
  preset (the default), and in the game's control setup leave the
  accelerator and brake on the left stick's Y axis.
- **Anything else:** run `CMR3Launcher.exe` with `--check`, look at
  `launcher.log` (see the table above), and open a
  [bug report](https://github.com/akitaonrails/rea-colin-mcrae-3/issues/new?template=bug-report.yml)
  with it attached (remove personal paths).
