# Research: community work this launcher builds on

Per the SDK's STANDARDS ("Research before building"): each project, what
it fixes, its licence, whether it is maintained, and the decision. The
longer history (what was tried and why) is in `docs/PLAN.md` and
`docs/ENHANCEMENTS.md`. Checked 2026-11-01, reviewed 2026-11-02.

| Project | What it fixes | Licence | Maintained | Decision |
| --- | --- | --- | --- | --- |
| [SilentPatchCMR3](https://github.com/CookiePLMonster/SilentPatchCMR3) (CookiePLMonster), build 2.1 | resolutions and any aspect ratio with the UI repositioned, windowed/borderless, vsync, refresh rate, anisotropic filtering, FOV, portable ini, rendering and timing fixes, d-pad menus, Alt+F4 | MIT | yes (last build 2023; issues answered) | **bundle**; configure through `SilentPatchCMR3.ini` |
| Ultimate ASI Loader (ThirteenAG), shipped inside SilentPatch | loads SilentPatch's `.asi` | MIT | yes | **bundle** (as `dsound.dll`) |
| SilentPatch's DRM-free patcher (`CMR3Patcher.zip`: xdelta3 + efigs/polish patches) | turns the official 1.1 SecuROM/SafeDisc exe into the publisher's own DRM-free 1.1 build | patcher MIT, xdelta3 Apache-2.0 | yes (2023-01) | **bundle** — owner decision 2026-11-02 (AGENTS.md, THIRD_PARTY.md); output verified against the known DRM-free builds |
| Official 1.1 patch (Codemasters) | updates 1.0 to 1.1 (needed by SilentPatch and the patcher) | proprietary freeware; no stated redistribution terms | no (2003) | **not bundled, not downloaded**: the only source today is a PCGamingWiki community upload; README links it. One-button download pending an owner decision on the source/terms |
| [Bekoha's HD UI](https://bekoha.github.io/cmr3/) v1.0.1 | fonts and interface redrawn for 4K | no licence text | quiet since release | **bundle** — owner decision 2026-11-01 (THIRD_PARTY.md) |
| [Xidi](https://github.com/samuelgr/Xidi) v5.0.0 (samuelgr) | XInput pads as DirectInput controllers with separate trigger axes, keyboard simulation | BSD-3-Clause | yes | **bundle** the `dinput8.dll` wrapper and the `Xidi.32.dll` main binary v5 needs; `Xidi.ini` generated per preset (v5 dropped the `[Import]` section) |
| [DXVK](https://github.com/doitsujin/dxvk) v3.1.1 | Direct3D 9 through Vulkan under Wine (10 FPS → full speed at 4K) | Zlib | yes | **bundle** (32-bit `d3d9.dll`, Wine only) |
| [vkBasalt](https://github.com/DadSchoorse/vkBasalt) 0.3.2.10 | post-processing layer (LUT grade, CAS) for the washed-out picture | Zlib (ReShade FX compiler BSD-3, stb MIT) | slow but alive | **build from the pinned source**, Linux only |
| MxAO (ReShade shader by Marty McFly) | depth-based ambient occlusion for the Cinematic look | not established here (to check before any download) | yes | **not bundled**; players may add it (docs/ENHANCEMENTS.md). Download-on-request only if the owner clears its licence — pending |
| SilentPatch Language Pack / Nicky Grist Pack | other languages and co-drivers | original game content | — | **never bundled** (game content); players add it to their game folder |
| AuToMaNiAk005's widescreen hex patches | widescreen before SilentPatch | none stated | no | not used (superseded by SilentPatch) |
| dgVoodoo2 | D3D9 → D3D11 for broken drivers | freeware, no redistribution grant | yes | not used (native d3d9 on Windows, DXVK on Linux) |
| PCGamingWiki, ProtonDB, WineHQ AppDB 7563 | known issues, Linux reports | — | — | read; our own Linux findings drafted in `docs/CONTRIBUTIONS.md` |

## Owner decisions

Recorded in `AGENTS.md` ("Owner decisions") and `THIRD_PARTY.md`;
gathered here as `sdk/docs/STANDARDS.md` ("DRM exceptions") asks. The DRM
one is the `drm-exception` line in `.rea-check-waivers`.

| Date | Decision | Why |
|---|---|---|
| 2026-11-01 | Bekoha's HD UI ships with the package, fetched and checksummed at build time, never committed | it has no licence text; the owner chose to bundle it for 4K-sharp menus |
| 2026-11-02 | The SilentPatch project's community xdelta patcher is the approved way to remove the disc protection: it turns an official 1.1 executable into the publisher's own DRM-free 1.1 build. No crack of our own; anything it doesn't convert is refused | Windows 10/11 block SecuROM/SafeDisc and Wine can't run it, so the official 1.1 exe can't start at all |
| — | SilentPatch's Language Pack is never bundled | original game content from regional releases |
