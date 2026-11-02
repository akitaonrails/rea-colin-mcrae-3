# Contributions back to the community

Drafts of findings from this launcher that players elsewhere could use
(SDK STANDARDS, "Contribute back"). **The owner reviews and posts; agents
only draft.** Nothing here has been posted. Master copies live in the
SDK's `docs/CONTRIBUTIONS.md` (C1, C2); keep both in step.

| # | Finding | Target | Status |
| --- | --- | --- | --- |
| C1 | A new profile has no controller selected, and the default joystick bindings put steering on the right stick | PCGamingWiki *Colin McRae Rally 3*, Issues fixed | **ready** (verified with an 8BitDo and an Xbox pad, `docs/RUNTIME_NOTES.md` v0.1.1); not posted |
| C2 | Under Wine/Proton: WineD3D gives about 10 FPS at 4K (use DXVK); exclusive fullscreen picks the wrong monitor and rewrites WIDTH/HEIGHT/ADAPTER (use borderless or gamescope) | PCGamingWiki (no Linux notes yet); AppDB 7563 only after a plain-Wine run | **ready** for PCGamingWiki; AppDB needs a plain-Wine run; not posted |

## C1 — Controller does nothing in a new profile

Page: <https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_3>, Issues
fixed. Text ready to paste:

```wikitext
===Controller does nothing in a new profile===
{{ii}} A newly created profile has no controller selected, and the game's default joystick bindings put steering on the right stick.<ref>{{Refcheck|user=OWNER|date=2026-11-XX|comment=Patch 1.1, DRM-free exe, SilentPatch 2.1, Xidi 4.3.1.}}</ref>
{{Fixbox|description=Select and rebind the controller|fix=
# With the keyboard, go to ''Options → Controls'' and choose the controller as Player 1's device.
# Rebind steering to the left stick and the pedals to the triggers (with [https://github.com/samuelgr/Xidi Xidi] both triggers get separate axes, so left-foot braking works).
}}
```

Evidence: `docs/RUNTIME_NOTES.md` (v0.1.1, issues #1 and #2) and
`crates/cmr3-launcher/src/profile.rs` (device index at 0x1DB98,
vibration at 0x1DB94, CRC-32/BZIP2 at 0x42A24 — repo detail only, not
wiki material).

## C2 — Low frame rate and wrong monitor under Wine/Proton

Page: the same, Issues fixed (or an "Other information → Linux" note).

```wikitext
===Low frame rate and wrong monitor under Wine/Proton===
{{Fixbox|description=Use DXVK and borderless mode|ref=<ref>{{Refcheck|user=OWNER|date=2026-11-XX|comment=Wine 11.20/11.21, RTX 5090, multi-monitor Hyprland (XWayland). Details: https://github.com/akitaonrails/rea-colin-mcrae-3/blob/master/docs/RUNTIME_NOTES.md}}</ref>|fix=
{{ii}} With Wine's built-in Direct3D (OpenGL) the game runs at about 10 FPS at 4K; through [https://github.com/doitsujin/dxvk DXVK] it runs at full speed.
{{ii}} In exclusive fullscreen Wine can choose another monitor (e.g. a portrait one) and crop the picture; the game also writes that monitor's WIDTH/HEIGHT/ADAPTER back to its settings.
# Put DXVK's 32-bit {{file|d3d9.dll}} next to {{file|Rally_3PC.exe}} and use the DLL override <code>d3d9=n,b</code> (keep <code>dsound=n,b</code> for [[#SilentPatch|SilentPatch]]).
# Use SilentPatch's borderless display mode, or run the game in fullscreen inside gamescope.
}}
```

ProtonDB: not applicable (not on Steam). WineHQ AppDB 7563 accepts
plain-Wine results only: the 10 FPS WineD3D observation is one, but the
playable setup used DXVK, Xidi and SilentPatch. File an AppDB report only
after a plain-Wine run. Evidence: `docs/RUNTIME_NOTES.md` ("display
modes under Wine, measured", M2).

## Not for posting

- The `CMR3` profile layout (repo detail).
- Anything about the DRM-free conversion beyond "use the SilentPatch
  project's DRM-free patcher" (wiki policy; already documented there).
