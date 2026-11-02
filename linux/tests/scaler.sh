#!/bin/sh
# Tests that cmr3-play hands launcher.cfg's scaler to gamescope (-F), for
# every scaler, the resolution (-W/-H/-w/-h), and that a windowed mode
# skips gamescope: sh linux/tests/scaler.sh
set -u
unset GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD WINEDLLOVERRIDES
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T CMR3_PREFIX=$T/prefix CMR3_WAIT=1 CMR3_LAUNCHER=$T/CMR3Launcher.exe
CFGDIR=$CMR3_PREFIX/drive_c/users/me/AppData/Local/CMR3Launcher
mkdir -p "$CFGDIR"; : > "$T/CMR3Launcher.exe"
# The wrapper (a program: the launch runs it through setsid): "gamescope
# is installed", and records the command it runs.
cat > "$T/fake-run" <<FAKE
#!/bin/sh
case "\$*" in *"command -v gamescope"*) exit 0 ;; *wineserver*|*"reg add"*) exit 0 ;; esac
echo "\$*" > "$T/ran"
FAKE
chmod +x "$T/fake-run"
REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$HERE/.. . "$HERE/../cmr3-play"
REA_RUN=$T/fake-run REA_GPU=
CFG=$CFGDIR/launcher.cfg
REA_CFG=$CFG
fails=0
for scaler in nearest pixel fsr linear; do
  printf 'resolution = 1920x1080\nscaler = %s\n' "$scaler" > "$CFG"
  rea_start > /dev/null
  case "$(cat "$T/ran")" in
    *"gamescope -W 1920 -H 1080 -w 1920 -h 1080 -f -S fit -F $scaler --"*) echo "ok   $scaler" ;;
    *) echo "FAIL $scaler: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
done
printf 'resolution = 1920x1080\n' > "$CFG"
rea_start > /dev/null
ran=$(cat "$T/ran")
case "$ran" in *"-F linear --"*) echo "ok   default is linear" ;; *) echo "FAIL default"; fails=$((fails + 1)) ;; esac
# The game's own settings reach its session.
for want in "WINEDLLOVERRIDES=dinput8,dsound,d3d9=n,b;winedbg=d" \
            "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1" "SDL_JOYSTICK_HIDAPI=0" \
            "REA_GAME_PROCS=Rally_3PC.exe" "WINEPREFIX=$CMR3_PREFIX" \
            "$CMR3_LAUNCHER --no-window"; do
  case "$ran" in *"$want"*) echo "ok   session has $want" ;; *) echo "FAIL no $want: $ran"; fails=$((fails + 1)) ;; esac
done
printf 'resolution = original\n' > "$CFG"
rea_start > /dev/null
case "$(cat "$T/ran")" in *"-w 640 -h 480 -f"*) echo "ok   original is 640x480" ;; *) echo "FAIL original: $(cat "$T/ran")"; fails=$((fails + 1)) ;; esac
# Other spellings the launcher accepts; anything else keeps 1920x1080.
for r in "2560X1440" " 2560 x 1440 "; do
  printf 'resolution = %s\r\n' "$r" > "$CFG"
  rea_start > /dev/null
  case "$(cat "$T/ran")" in *"-W 2560 -H 1440 "*) echo "ok   '$r' read as 2560x1440" ;; *) echo "FAIL '$r': $(cat "$T/ran")"; fails=$((fails + 1)) ;; esac
done
for r in "x1080" "1920x" "19a0x1080" '$(touch pwned)x1'; do
  printf 'resolution = %s\n' "$r" > "$CFG"
  rea_start > /dev/null
  case "$(cat "$T/ran")" in *"-W 1920 -H 1080 "*) echo "ok   '$r' falls back to 1920x1080" ;; *) echo "FAIL '$r': $(cat "$T/ran")"; fails=$((fails + 1)) ;; esac
done
for mode in borderless windowed; do
  printf 'resolution = 1920x1080\ndisplay_mode = %s\n' "$mode" > "$CFG"
  rea_start > /dev/null
  case "$(cat "$T/ran")" in
    *gamescope*) echo "FAIL $mode must skip gamescope"; fails=$((fails + 1)) ;;
    *inner.sh*) echo "ok   $mode runs directly" ;;
    *) echo "FAIL $mode: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
done
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
