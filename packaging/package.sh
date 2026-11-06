#!/bin/sh
# Builds the release packages: the Windows zip (which also plays on Linux
# through linux/cmr3-play) and the Linux AppImage with a portable Wine.
#
#   packaging/package.sh [--no-appimage] <version-name> <release-dir> <out-dir> [cache-dir]
#
# <release-dir> holds the built CMR3Launcher.exe. The bundled components
# are fetched from packaging/third-party.txt and checked by SHA-256 (the
# SDK's sdk/packaging/pins.sh: the first url that matches wins, later urls
# are our mirror), then unpacked into thirdparty/. vkBasalt is built from
# its pinned source (packaging/build-vkbasalt.sh). The result is
# <out-dir>/CMR3Launcher-<version-name>/ and .zip, and
# <out-dir>/CMR3Launcher-<version-name>-x86_64.AppImage.
set -eu
APPIMAGE=1
if [ "${1:-}" = --no-appimage ]; then APPIMAGE=; shift; fi
[ $# -ge 3 ] || { sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }
NAME=CMR3Launcher-$1
RELEASE=$(cd "$2" && pwd -P)
mkdir -p "$3"
OUT=$(cd "$3" && pwd -P)
CACHE=${4:-${XDG_CACHE_HOME:-$HOME/.cache}/cmr3-launcher}
ROOT=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$ROOT"

. sdk/packaging/pins.sh
PIN_FILE=packaging/third-party.txt
PIN_DIR=
PIN_CACHE=$CACHE/pins
export PIN_FILE PIN_DIR PIN_CACHE

WORK=$(mktemp -d "${TMPDIR:-/tmp}/cmr3-package.XXXXXX")
trap 'rm -rf "$WORK"' EXIT
fetch() { pin_fetch "$1" "$WORK/downloads" || { echo "no source for $1 matched its pin" >&2; exit 1; }; }

TP=$WORK/thirdparty
mkdir -p "$TP/silentpatch" "$TP/xidi" "$TP/patcher" "$TP/dxvk" "$TP/hd-ui"
unzip -qo "$(fetch silentpatch)" -d "$WORK/sp"
unzip -qo "$(fetch patcher)" -d "$WORK/patcher"
unzip -qo "$(fetch xidi)" -d "$WORK/xidi"
unzip -qo "$(fetch hd-ui)" -d "$TP/hd-ui"
tar -xzf "$(fetch dxvk)" -C "$WORK"
tar -xzf "$(fetch vkbasalt)" -C "$WORK"
cp "$WORK/sp/SilentPatchCMR3.asi" "$WORK/sp/dinput8.dll" "$WORK/sp/ReadMe.txt" "$TP/silentpatch/"
cp "$WORK/patcher/patcher/xdelta.exe" "$WORK/patcher/patcher/efigs.xdelta" \
   "$WORK/patcher/patcher/polish.xdelta" "$WORK/patcher/ReadMe-Patcher.txt" "$TP/patcher/"
cp "$WORK/xidi/Xidi-v5.0.0/Win32/dinput8.dll" "$WORK/xidi/Xidi-v5.0.0/Win32/Xidi.32.dll" "$TP/xidi/"
cp "$WORK/xidi/Xidi-v5.0.0/LICENSE" "$TP/xidi/LICENSE"
cabextract -q -d "$WORK/vc" "$(fetch vcredist)"
for cab in "$WORK"/vc/a*; do
  if cabextract -l "$cab" 2>/dev/null | grep -q 'vcruntime140.dll_x86'; then
    cabextract -q -d "$WORK/vcdll" -F 'msvcp140.dll_x86' -F 'msvcp140_atomic_wait.dll_x86' \
      -F 'concrt140.dll_x86' -F 'vcruntime140.dll_x86' "$cab"
    break
  fi
done
for dll in msvcp140 msvcp140_atomic_wait concrt140 vcruntime140; do
  [ -f "$WORK/vcdll/$dll.dll_x86" ] || { echo "VC++ runtime: $dll.dll not found" >&2; exit 1; }
  mv "$WORK/vcdll/$dll.dll_x86" "$TP/xidi/$dll.dll"
done
cp "$WORK/dxvk-3.1.1/x32/d3d9.dll" "$TP/dxvk/"
pin_verify "$TP/dxvk/d3d9.dll" "$(pin_sha dxvk-d3d9-sha256)"
cp packaging/licenses/DXVK-LICENSE "$TP/dxvk/LICENSE"
cp packaging/licenses/SilentPatchCMR3-LICENSE "$TP/silentpatch/LICENSE"
cp packaging/licenses/Ultimate-ASI-Loader-LICENSE "$TP/silentpatch/Ultimate-ASI-Loader-LICENSE"
cp packaging/licenses/xdelta3-LICENSE "$TP/patcher/LICENSE"
cp packaging/licenses/Xidi-LICENSE packaging/licenses/Xidi-Hookshot-LICENSE "$TP/xidi/"
# The postfx layer, from the pinned source: both bitnesses ship — a
# WoW64 prefix loads the 64-bit Vulkan loader even for its 32-bit game,
# so each process finds its own library.
packaging/build-vkbasalt.sh "$WORK"/vkBasalt-*/ "$TP/vkbasalt"
# vkBasalt compiles in ReShade's FX compiler (BSD-3) and stb (MIT).
cp "$WORK"/vkBasalt-*/src/reshade/LICENSE.md "$TP/vkbasalt/ReShade-LICENSE.md"
cp packaging/licenses/stb-LICENSE "$TP/vkbasalt/stb-LICENSE"

PKG=$OUT/$NAME
rm -rf "$PKG" "$OUT/$NAME.zip"
mkdir -p "$PKG/docs"
cp "$RELEASE/CMR3Launcher.exe" README.md CHANGELOG.md THIRD_PARTY.md LICENSE-MIT LICENSE-APACHE "$PKG/"
# The settings window's fonts (Barlow, SIL OFL), compiled into the exe.
mkdir -p "$PKG/licenses"
cp packaging/licenses/Barlow-OFL.txt "$PKG/licenses/Barlow-OFL.txt"
cp -r docs/images "$PKG/docs/images"
cp docs/ENHANCEMENTS.md "$PKG/docs/"
cp -r "$TP" "$PKG/thirdparty"
mkdir -p "$PKG/linux/tests"
cp linux/cmr3-play linux/README.md "$PKG/linux/"
cp linux/tests/*.sh "$PKG/linux/tests/"
# The shared play library cmr3-play sources, beside it.
cp sdk/rea-play.sh "$PKG/linux/rea-play.sh"
(cd "$OUT" && zip -qr "$NAME.zip" "$NAME")
echo "built $OUT/$NAME.zip"

[ -n "$APPIMAGE" ] || exit 0
sdk/packaging/appimage-wine.sh --app cmr3 --exe CMR3Launcher.exe \
  --icon packaging/appimage/cmr3-launcher.png \
  --desktop packaging/appimage/cmr3-launcher.desktop \
  --payload thirdparty --env-path CMR3_THIRDPARTY=thirdparty \
  --cache "$CACHE/appimage" --name "$NAME" "$PKG" "$OUT"
