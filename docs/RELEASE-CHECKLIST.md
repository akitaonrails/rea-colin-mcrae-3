# Release checklist

Run before publishing a release (and after any change to the input path,
the display path, or a pinned third-party version). CI covers building;
these need a person at the machine, because they are exactly the
regressions that pixel-and-log tests cannot see — the gamepad problem of
2026-11-01 shipped green through every automated test.

Setup: the gaming distrobox, a test kit folder (the launcher exe +
`thirdparty/` + a `launcher.cfg` naming the game), a pad connected, and
`linux/tests/e2e.sh` for the machine-checked parts.

## Automated, on the dev box

```sh
CMR3_E2E_KIT=/path/to/kit sh linux/tests/e2e.sh
# Boots the game through cmr3-play and checks: alive, no fault deadlock,
# clean exit, components installed, settings written, postfx engaged.
# When the helper runs inside a distrobox, CMR3_E2E_PLAYLOG points at
# its play.log (the box's ~/.config/cmr3-launcher/play.log).
```

## By hand — Linux (gamescope path), then the AppImage, then Windows

**Gamepad** (Xidi, the `rally` preset)

- [ ] Menus: d-pad moves, Start selects, Back/Esc goes back.
- [ ] **A, B, X, Y work somewhere**: A selects in menus; in a race they
      are handbrake / view / gears as bound in Options → Controls.
- [ ] Steering on the left stick; **both** triggers at once (left-foot
      braking); resting the left stick does not creep.
- [ ] If anything fails: rerun with `--gamepad-log on`, press every
      button once at the menu, quit, and read the log Xidi drops on the
      prefix's desktop — it shows what the game asked for and what each
      control sent. Attach it to the issue.
- [ ] In-game once: Options → Controls → Control Method is JOYSTICK 1
      (not keyboard) — a profile carried from an old install can point
      elsewhere; re-selecting it fixes the buttons for good.

**Picture**

- [ ] With `postfx = color`: the frame is visibly sharpened and the game
      still runs at full speed; with `cinematic` (and the AO shader
      dropped in): shading gains contact depth. Off restores the
      untouched picture.
- [ ] Menu and race fill the screen, correct aspect (no left-side crop:
      that was Wine's exclusive fullscreen picking a portrait monitor —
      keep `DisplayMode::effective` and its test).
- [ ] Resolution, vsync and anisotropic take effect; HD UI fonts look
      sharp at 4K.

**Sound** — menu music, engine, co-driver.

**Video** — intro plays; quitting during and after it exits within a few
seconds (a Wine quartz fault used to freeze the game in an invisible
debugger — `winedbg` stays disabled in cmr3-play).

**Lifecycle**

- [ ] Quit from the in-game menu exits to the desktop cleanly.
- [ ] The profile (stage progress) survives: play a stage, quit, relaunch.
- [ ] The game folder is byte-identical before and after a session
      (`e2e.sh` checks this against the manifest; see also the smoke
      tests).

## Before a third-party version bump

- [ ] Play a full stage with the new SilentPatch/Xidi/DXVK build.
- [ ] Update `packaging/third-party.txt` (exact url + new sha256) in the
      same PR, run the mirror workflow, and re-run this checklist.
