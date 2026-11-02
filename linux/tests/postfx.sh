#!/bin/sh
# Tests vkBasalt manifest generation, including upgrades from the legacy
# bare-soname manifest: sh linux/tests/postfx.sh
set -u
unset VK_INSTANCE_LAYERS VK_LAYER_PATH
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T CMR3_PREFIX=$T/prefix CMR3_SYSFS=$T/nosys
export CMR3_LAUNCHER=$T/CMR3Launcher.exe
mkdir -p "$T/thirdparty/vkbasalt/lib32"
: > "$T/CMR3Launcher.exe"
: > "$T/thirdparty/vkbasalt/libvkbasalt.so"
: > "$T/thirdparty/vkbasalt/lib32/libvkbasalt.so"
printf 'postfx = color\n' > "$T/launcher.cfg"
CMR3_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr3-play"
LAUNCHER=$T/CMR3Launcher.exe
fails=0
mkdir -p "$STATE/vkbasalt"
printf '{"legacy":true}\n' > "$STATE/vkbasalt/layer.json"
postfx_env > /dev/null
if [ ! -e "$STATE/vkbasalt/layer.json" ]; then
  echo "ok   legacy manifest cannot shadow the current one"
else
  echo "FAIL legacy manifest survived"
  fails=$((fails + 1))
fi
for bits in 64 32; do
  manifest=$STATE/vkbasalt/layer$bits.json
  if [ -f "$manifest" ] && grep -q '"library_path": "/' "$manifest"; then
    echo "ok   $bits-bit manifest uses an absolute library path"
  else
    echo "FAIL $bits-bit manifest"
    fails=$((fails + 1))
  fi
done
case "$CMR3_VK_INSTANCE_LAYERS" in
  *post_processing:*post_processing32) echo "ok   both bitnesses requested" ;;
  *) echo "FAIL layer names: $CMR3_VK_INSTANCE_LAYERS"; fails=$((fails + 1)) ;;
esac
# The layers reach the game only, never gamescope: nothing Vulkan reads is
# exported here; the game's script sets them (see supervision.sh).
if [ -z "${VK_INSTANCE_LAYERS:-}${VK_LAYER_PATH:-}" ]; then
  echo "ok   no Vulkan layer variables for gamescope"
else
  echo "FAIL VK_INSTANCE_LAYERS/VK_LAYER_PATH exported"
  fails=$((fails + 1))
fi
conf=$STATE/vkbasalt/vkBasalt.conf
if grep -qx "effects = lut:cas" "$conf" && grep -qx "lutFile = $STATE/vkbasalt/grade.cube" "$conf"; then
  echo "ok   colour grades through the LUT, then sharpens"
else
  echo "FAIL vkBasalt.conf: $(tr '\n' '|' < "$conf")"
  fails=$((fails + 1))
fi
if [ "$(grep -c '^[0-9]' "$STATE/vkbasalt/grade.cube")" = 35937 ] \
   && head -1 "$STATE/vkbasalt/grade.cube" | grep -qx "LUT_3D_SIZE 33"; then
  echo "ok   the LUT has every entry"
else
  echo "FAIL grade.cube"
  fails=$((fails + 1))
fi
# Cinematic with the player's MxAO: the effect is declared by its file.
printf 'postfx = cinematic\n' > "$T/launcher.cfg"
mkdir -p "$STATE/vkbasalt/shaders"; : > "$STATE/vkbasalt/shaders/MxAO.fx"
postfx_env > /dev/null
if grep -qx "effects = mxao:lut:cas" "$conf" && grep -qx "mxao = $STATE/vkbasalt/shaders/MxAO.fx" "$conf"; then
  echo "ok   cinematic declares MxAO and chains it first"
else
  echo "FAIL cinematic: $(tr '\n' '|' < "$conf")"
  fails=$((fails + 1))
fi
# Without DXVK the layer has no Vulkan to work on: nothing is set up.
unset CMR3_VK_INSTANCE_LAYERS
printf 'postfx = color\ndxvk = off\n' > "$T/launcher.cfg"
case "$(postfx_env)" in *"needs DXVK"*) echo "ok   postfx says it needs DXVK" ;; *) echo "FAIL no DXVK note"; fails=$((fails + 1)) ;; esac
postfx_env > /dev/null
if [ -z "${CMR3_VK_INSTANCE_LAYERS:-}" ]; then echo "ok   no layer without DXVK"; else echo "FAIL layer set without DXVK"; fails=$((fails + 1)); fi
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
