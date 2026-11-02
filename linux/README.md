# Playing on Linux

The launcher itself is a Windows program (the same one as on Windows);
on Linux it runs through Wine or Proton. Three ways to play:

## The AppImage (nothing else to install)

```sh
chmod +x CMR3Launcher-…-x86_64.AppImage
./CMR3Launcher-…-x86_64.AppImage
```

It contains the launcher, the bundled components (SilentPatch, Xidi,
DXVK, the HD UI) and a portable Wine, so nothing else has to be
installed. The settings window opens: press **Browse…** and pick your
game folder (your Linux files are under drive `Z:`), choose a look and
press **Play**. If [gamescope](https://github.com/ValveSoftware/gamescope)
is installed, the game runs in it at the right size (recommended);
otherwise in a plain window. Its Wine prefix is
`~/.local/share/cmr3-launcher/prefix` (change it with `CMR3_PREFIX`).

## With Steam (Proton)

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

Follow **With Steam** above in Desktop Mode, then play from Game Mode:

- Pick the **Steam Deck** look: 1280×800 at the panel's 60 Hz.
- The buttons follow the **Rally** gamepad preset: the triggers are the
  pedals (both at once), and the d-pad and A work the menus.
- The touchscreen and the right trackpad drive the settings window.

## With your own Wine and gamescope

If you have Wine and
[gamescope](https://github.com/ValveSoftware/gamescope) installed,
`linux/cmr3-play` runs the game in its own correctly sized screen:

```sh
~/Games/cmr3-launcher/linux/cmr3-play
```

It shows the settings window first (with two more choices: the gamescope
scaler, and DXVK), then starts the game in gamescope. When Wine and
gamescope live in a [distrobox](https://github.com/containers/distrobox)
container, run it through the container:

```sh
CMR3_RUN="distrobox enter gaming --" ~/Games/cmr3-launcher/linux/cmr3-play
```

`cmr3-play` also understands `--play` (no settings window), `stop` and
`status`. They look only at the session `cmr3-play` itself started (its
own process group, recorded by pid and start time): `stop` ends that
session and the prefix's Wine processes, and a `Rally_3PC.exe` started
some other way is neither reported nor stopped.

## Where things are

| What | Where |
| --- | --- |
| Settings (`launcher.cfg`) | inside the Wine prefix, `…/drive_c/users/<you>/AppData/Local/CMR3Launcher/` |
| Log (`launcher.log`) | the same folder |
| The launcher's copy of the game | `…/AppData/Local/CMR3Launcher/Rally3` (about 3 GB; hard-linked when possible) |
| Your profile | `…/drive_c/users/<you>/Saved Games/Colin McRae Rally 3` |

## Troubleshooting

- **The game crawls at 4K** — DXVK is off or missing. In the settings
  window (Linux) turn **DXVK** on; with your own Wine, check that
  `thirdparty/dxvk/d3d9.dll` exists next to the launcher. Without it
  Wine draws through its OpenGL translator, which manages about 10 FPS
  at 4K on a fast card.
- **The game window is the wrong shape** — keep the game's own display
  options as they are and set the resolution in the launcher; SilentPatch
  matches the aspect ratio to the resolution.
- **The pad steers but has no throttle** — use the **Rally** gamepad
  preset (the default), and in the game's control setup leave the
  accelerator and brake on the left stick's Y axis.
- **Anything else:** run `CMR3Launcher.exe` with `--check`, and look at
  `launcher.log` (see the table above).
