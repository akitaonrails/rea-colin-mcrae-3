# Enhancing the look: research and recommendations

Why the game looks flat, what can realistically be done about it, and
what to build in the launcher. Everything here is opt-in; the launcher's
default stays "the faithful game with working pads and widescreen".

## Why it looks flat

Colin McRae Rally 3 is a **fixed-function Direct3D 9** renderer: lighting
is computed per **vertex** with one sun and a flat ambient term, there is
no per-pixel normal map, no ambient occlusion, no bounce light, and cars
cast blob shadows (a top-down render, blurred by 8 downsample/upsample
passes — SilentPatch already reduces that to 2005's 2 and can sharpen
further). Combined, everything reads even and weightless — exactly the
"flat" impression. Nothing in the pipeline produces a G-buffer, so
**true GI cannot be injected without rewriting the renderer** (the parked
recomp route, a different project).

What we *can* have, cheaply: the game's **depth buffer**. Post-processors
can read it and compute screen-space ambient occlusion, screen-space GI
and depth-aware effects — which address precisely the missing contact
shading — plus grading, bloom and sharpening. That is the same leap
ReShade presets give other fixed-function-era games.

## Recommendation ladder

### 1. Post-processing presets (do this) — the ~90% win

**Linux (primary path): vkBasalt** — a host-side Vulkan post-processing
layer that already runs **ReShade FX shaders** (open-source), sits on
DXVK without touching a single DLL slot, and is configured by one conf
file + env. CMR3 through our launcher is DXVK+gamescope already, so it
composes with everything we ship. Candidate stack, tuned subtle:

- MXAO (ambient occlusion; small radius, half-res if needed)
- a light screen-space GI pass (Pascal Glitcher's RTGI-style FX; the
  classic companion of MXAO) — *the* flatness fix
- MultiLUT/colour grade (lift the mids, slight warm tilt), mild bloom,
  CAS sharpening

Caveats found in research: depth effects must not run on the frontend
videos/menus (no depth there) — bind a toggle key and default effects to
race-only if vkBasalt's depth capture makes that automatic; verify depth
capture actually grabs CMR3's buffer through DXVK (single test run; if
not, fall back to colour-only effects: grade + bloom + CAS still help).

**Windows: ReShade** — official, BSD-3-Clause, generic-depth add-on
finds D3D9 depth buffers (verified pattern in old D3D9 games). Our
`d3d9.dll` slot is free on Windows (DXVK is Wine-only), so ReShade
installs as the classic local `d3d9.dll`. Same preset, ReShade-native.
Under Wine ReShade is possible too (ReShade-as-d3d9 chaining into DXVK
installed in the prefix), but vkBasalt is cleaner — no DLL juggling.

**Launcher design**: `postfx = off | cinematic` (default off), plus the
effect preset as a checksummed bundle exactly like the other third-party
parts (`packaging/third-party.txt` gets vkBasalt + the FX sources;
ReShade for the Windows zip). Off removes the layer entirely. The
settings window gains a Look card ("Cinematic": Enhanced + postfx) with
a "post-processing: off/colour-only/full" chip so weak GPUs and the
Steam Deck can dial it down.

### 2. In-engine tuning (cheap, already partly there)

- `SHARPER_SHADOWS=0` in `SilentPatchCMR3.ini` keeps the softer stock
  shadows — consider exposing shadow softness as a setting (it exists in
  SP's source as one flag).
- A wider FOV and the right grade do a lot for the sense of speed.
- Real per-pixel shadow or light changes mean patching the fixed-function
  pipeline — REA + Ghidra on `Rally_3PC.exe`, following SilentPatch's
  map of the shadow passes (its blog documents the exact blur chain and
  the state bugs). Diminishing returns next to option 1; keep as a
  study, not a promise.

### 3. RTX Remix (stretch, Windows only, manual)

Remix is built for exactly this class of game (fixed-function DX8/9):
it captures meshes/textures and re-renders with path tracing. A
path-traced CMR3 is plausible as a community-scale project on a 5000-series
card, but it is a large manual modding effort (asset replacement, light
placement) and Windows+NVIDIA only. Document it; do not integrate.

### 4. DLSS 5: recommend against (and it would not help)

DLSS needs the game's own SDK integration (motion vectors, jittered
render) — there is no injectable path into a fixed-function D3D9 title
(OptiScaler-class wrappers target DX11/12 swapchains). More to the
point, the 5090 renders CMR3 at native 4K/60+ without effort (measured
through DXVK), so upscaling solves a problem this game does not have.
What *is* worth having for weak GPUs already ships: gamescope's FSR
scaler in `cmr3-play` and, on Windows, the driver's own NIS. Frame
generation is also a poor fit: the physics runs a fixed timestep and
frame gen would only add latency around it.

## Provenance

- vkBasalt (ReShade FX support, depth capture, conf):
  https://github.com/DadSchoorse/vkBasalt , Phoronix 0.3 announcement.
- ReShade: license (BSD-3-Clause) https://reshade.me , generic depth in
  D3D9 (community threads), Marty's Mods depth guide (cutscene caveat).
- ReShade-under-Wine pattern: codeberg.org/kevincs/reshade-steam-proton.
- Shadow pipeline internals: CookiePLMonster's "Remastering CMR3" write-up
  (the 8-pass blob shadow chain, SHARPER_SHADOWS).
- DLSS/OptiScaler applicability: OptiScaler targets DX11/12
  (github.com/optiscaler/OptiScaler).
- This box: CMR3 at 4K through DXVK measured at native framerate (see
  `docs/RUNTIME_NOTES.md`).
