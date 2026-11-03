#!/bin/sh
# Tests that cmr3-play hands launcher.cfg's scaler to gamescope (-F), for
# every scaler, the resolution (-W/-H/-w/-h), the size of each look, and
# that a windowed mode skips gamescope: sh linux/tests/scaler.sh
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
# Flags for one run beat launcher.cfg, reach the launcher too, and leave
# the file as it was.
printf 'resolution = 1920x1080\nscaler = linear\ndisplay_mode = windowed\n' > "$CFG"
before=$(cat "$CFG")
rea_main --play --no-window --scaler fsr --resolution 1280x720 --display-mode fullscreen > /dev/null
ran=$(cat "$T/ran")
case "$ran" in
  *"gamescope -W 1280 -H 720 -w 1280 -h 720 -f -S fit -F fsr --"*"--no-window --no-window --scaler fsr --resolution 1280x720 --display-mode fullscreen"*)
    echo "ok   one-run flags override launcher.cfg" ;;
  *) echo "FAIL one-run flags: $ran"; fails=$((fails + 1)) ;;
esac
[ "$(cat "$CFG")" = "$before" ] && echo "ok   launcher.cfg untouched" || { echo "FAIL cfg changed"; fails=$((fails + 1)); }
# The next plain run is back to launcher.cfg (windowed: no gamescope).
rea_main --play > /dev/null
case "$(cat "$T/ran")" in
  *gamescope*) echo "FAIL the one-run values outlived their run: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  *inner.sh*) echo "ok   a plain --play reads launcher.cfg again" ;;
  *) echo "FAIL plain --play: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
esac
# The --flag=value spelling works as well.
printf 'resolution = 1920x1080
scaler = linear
' > "$CFG"
rea_main --play --scaler=nearest > /dev/null
case "$(cat "$T/ran")" in *"-F nearest --"*) echo "ok   --scaler=nearest for one run" ;; *) echo "FAIL --scaler=: $(cat "$T/ran")"; fails=$((fails + 1)) ;; esac
# Window modes in any case, as the launcher reads them.
printf 'resolution = 1920x1080\n' > "$CFG"
rea_main --play --display-mode Windowed > /dev/null
case "$(cat "$T/ran")" in
  *gamescope*) echo "FAIL --display-mode Windowed must skip gamescope"; fails=$((fails + 1)) ;;
  *inner.sh*) echo "ok   --display-mode Windowed runs directly" ;;
  *) echo "FAIL --display-mode Windowed: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
esac
# Looks (--look / look =, REA_LOOK_SIZES): gamescope opens at the size
# the launcher gives the game, as it resolves it: this run's --resolution,
# else its --look, else the file's resolution, else the file's look.
look_case() { # name, launcher.cfg text, wanted size, cmr3-play args...
  name=$1 text=$2 want=$3; shift 3
  printf '%b' "$text" > "$CFG"
  REA_SCREEN=2560x1440 rea_main --play "$@" > /dev/null
  case "$(cat "$T/ran")" in
    *"-w ${want%x*} -h ${want#*x} -f"*) echo "ok   $name" ;;
    *) echo "FAIL $name: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
}
look_case "--look steam_deck is 1280x800" 'resolution = 1920x1080\n' 1280x800 --look steam_deck
look_case "--look original is 640x480" 'resolution = 1920x1080\n' 640x480 --look original
look_case "--look enhanced is the screen" 'resolution = original\n' 2560x1440 --look enhanced
look_case "--resolution beats --look" '' 1600x900 --look steam_deck --resolution 1600x900
look_case "look = steam_deck in the file" 'look = steam_deck\n' 1280x800
look_case "the file's resolution beats its look" 'look = steam_deck\nresolution = 1920x1080\n' 1920x1080
printf 'resolution = 1920x1080\n' > "$CFG"
rea_main --play --look steam_deck > /dev/null
case "$(cat "$T/ran")" in
  *"--no-window --look steam_deck"*) echo "ok   --look reaches the launcher" ;;
  *) echo "FAIL --look not passed on: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
esac
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
