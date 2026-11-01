#!/bin/sh
# Tests that cmr3-play hands launcher.cfg's scaler to gamescope (-F), for
# every scaler, the resolution (-W/-H/-w/-h), and that a windowed mode
# skips gamescope: sh linux/tests/scaler.sh
set -u
unset GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T CMR3_PREFIX=$T/prefix CMR3_WAIT=1 CMR3_SYSFS=$T/nosys
CFGDIR=$CMR3_PREFIX/drive_c/users/me/AppData/Local/CMR3Launcher
mkdir -p "$CFGDIR"; : > "$T/CMR3Launcher.exe"
# The wrapper: "gamescope is installed", and records the command it runs.
fake_run() {
  case "$*" in *"command -v gamescope"*) return 0 ;; esac
  echo "$*" > "$T/ran"
}
CMR3_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr3-play"
RUN=fake_run LAUNCHER=$T/CMR3Launcher.exe GPU=
fails=0
for scaler in nearest pixel fsr linear; do
  printf 'resolution = 1920x1080\nscaler = %s\n' "$scaler" > "$CFGDIR/launcher.cfg"
  CFG=$CFGDIR/launcher.cfg
  start > /dev/null
  case "$(cat "$T/ran")" in
    *"gamescope -W 1920 -H 1080 -w 1920 -h 1080 -f -S fit -F $scaler --"*) echo "ok   $scaler" ;;
    *) echo "FAIL $scaler: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
done
printf 'resolution = 1920x1080\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
case "$(cat "$T/ran")" in *"-F linear --"*) echo "ok   default is linear" ;; *) echo "FAIL default"; fails=$((fails + 1)) ;; esac
printf 'resolution = original\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
case "$(cat "$T/ran")" in *"-w 640 -h 480 -f"*) echo "ok   original is 640x480" ;; *) echo "FAIL original: $(cat "$T/ran")"; fails=$((fails + 1)) ;; esac
printf 'resolution = 1920x1080\ndisplay_mode = borderless\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
case "$(cat "$T/ran")" in
  *gamescope*) echo "FAIL borderless must skip gamescope"; fails=$((fails + 1)) ;;
  *inner.sh*) echo "ok   borderless runs directly" ;;
  *) echo "FAIL borderless: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
esac
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
