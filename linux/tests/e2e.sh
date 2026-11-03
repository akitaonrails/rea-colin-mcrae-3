#!/bin/sh
# End-to-end check on the dev box: boots the game through cmr3-play and
# verifies the machine-checkable parts of docs/RELEASE-CHECKLIST.md.
#
#   CMR3_E2E=1 sh linux/tests/e2e.sh
#
# It runs the real game, so it does nothing unless CMR3_E2E=1 asks for it
# (a loop over linux/tests/*.sh skips it). gamescope runs headless
# (--backend headless, no window on the desktop) unless CMR3_E2E_VISIBLE=1.
#
# Needs the gaming distrobox, the test kit (launcher + thirdparty/ +
# launcher.cfg) and a game — set:
#   CMR3_E2E_KIT      the kit folder          (required on the first run;
#                                             remembered in .e2e-kit)
#   CMR3_E2E_PREFIX   the wine prefix         (default $KIT/prefix)
#   CMR3_E2E_PLAY     this repo's cmr3-play   (default ../cmr3-play)
#   CMR3_E2E_PLAYLOG  the helper's play.log   (default: ~/.config/cmr3-launcher)
#   CMR3_E2E_BOX      gamescope+wine wrapper  (default: distrobox enter gaming --)
#   CMR3_E2E_ALIVE_AT seconds to wait first   (default 90)
#   CMR3_E2E_VISIBLE  1: gamescope opens a window on the desktop
# Nothing here presses pad buttons or listens to music: those lines of the
# checklist stay human.
set -u
if [ "${CMR3_E2E:-}" != 1 ]; then
  echo "skipped: real-game test; set CMR3_E2E=1 to run it (headless by default)"
  return 0 2>/dev/null || exit 0
fi
HERE=$(dirname "$(readlink -f "$0")")
KIT=${CMR3_E2E_KIT:-}
if [ -z "$KIT" ] && [ -f "$HERE/.e2e-kit" ]; then KIT=$(cat "$HERE/.e2e-kit"); fi
if [ -z "$KIT" ]; then
  echo "set CMR3_E2E_KIT to the test kit folder (launcher + thirdparty/ + launcher.cfg)"
  exit 2
fi
mkdir -p "$HERE" && printf '%s\n' "$KIT" > "$HERE/.e2e-kit"
PREFIX=${CMR3_E2E_PREFIX:-$KIT/prefix}
PLAY=${CMR3_E2E_PLAY:-$HERE/../cmr3-play}
BOX=${CMR3_E2E_BOX:-distrobox enter gaming --}
ALIVE_AT=${CMR3_E2E_ALIVE_AT:-90}

fails=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fails=$((fails + 1)); }

[ -f "$KIT/CMR3Launcher.exe" ] || { echo "no kit at $KIT (CMR3_E2E_KIT)"; exit 2; }
LOGFILE=$(ls "$PREFIX"/drive_c/users/*/AppData/Local/CMR3Launcher/launcher.log 2>/dev/null | head -1)
INI=$(ls "$PREFIX"/drive_c/users/*/AppData/Local/CMR3Launcher/Rally3/SilentPatchCMR3.ini 2>/dev/null | head -1)
[ -n "$LOGFILE" ] || { echo "no launcher.log in $PREFIX; run the launcher once first"; exit 2; }

# A clean slate: end anything a previous run left, so this measures this
# run and not its leftovers.
$BOX sh -c "CMR3_PREFIX='$PREFIX' '$PLAY' stop" >/dev/null 2>&1
sleep 2

# This run's game only: a Rally_3PC.exe whose working folder is inside
# this prefix (the runtime copy). Another prefix's game, or anything else
# on the machine, is never signalled; nothing about other processes is
# printed.
PREFIX_REAL=$(readlink -f "$PREFIX")
our_games() {
  for pid in $(pgrep -x Rally_3PC.exe); do
    case "$(readlink "/proc/$pid/cwd" 2>/dev/null)" in "$PREFIX_REAL"/*) echo "$pid" ;; esac
  done
}
game_running() { [ -n "$(our_games)" ]; }
end_games() { # signal
  for pid in $(our_games); do kill "-$1" "$pid" 2>/dev/null; done
}

BOXLOG=$(mktemp)
# Headless: a `gamescope` first in PATH that adds --backend headless to the
# real one (the play library's own seam is for its shell tests only).
WRAP=""
if [ "${CMR3_E2E_VISIBLE:-}" != 1 ]; then
  WRAP=$(mktemp -d)
  cat > "$WRAP/gamescope" <<'WRAPPER'
#!/bin/sh
# The first gamescope in PATH that isn't this wrapper, headless.
self=$(readlink -f "$0")
IFS=:
for dir in $PATH; do
  [ -x "$dir/gamescope" ] && [ "$(readlink -f "$dir/gamescope")" != "$self" ] &&
    exec "$dir/gamescope" --backend headless "$@"
done
echo "gamescope not found" >&2
exit 127
WRAPPER
  chmod +x "$WRAP/gamescope"
  echo "gamescope: headless (CMR3_E2E_VISIBLE=1 shows a window)"
fi
$BOX sh -c "${WRAP:+PATH='$WRAP':\$PATH }CMR3_PREFIX='$PREFIX' CMR3_LAUNCHER='$KIT/CMR3Launcher.exe' '$PLAY' --play" \
  > "$BOXLOG" 2>&1 &

# Wait for the game (a fresh prefix boots Wine first; that can take a
# while), then give it a moment to get past the launcher's own setup.
boot=${CMR3_E2E_BOOT:-180}
waited=0
while [ "$waited" -lt "$boot" ]; do
  game_running && break
  sleep 5
  waited=$((waited + 5))
done
if game_running; then
  ok "game running after ${waited}s"
  sleep "${CMR3_E2E_ALIVE_AT:-10}"
  game_running && ok "still alive ${ALIVE_AT:=10}s later" \
    || bad "game died right after starting"
else
  bad "game never started within ${boot}s; cmr3-play said: $(tail -5 "$BOXLOG" | tr '\n' ' ')"
fi

# A quartz fault must not freeze the game: with winedbg disabled it ends
# the process instead of deadlocking it. Either way the game must stop.
faults=$(grep -c "page fault" "$BOXLOG" 2>/dev/null)
faults=${faults:-0}
end_games TERM
sleep 8
if game_running; then
  end_games KILL
  bad "game would not exit (faults seen: $faults) — the frozen-quit bug"
else
  ok "game exits when ended (faults during run: $faults)"
fi
wait 2>/dev/null

# What the launcher did this run: the lines from its last banner on.
last=$(grep -n "^CMR3Launcher " "$LOGFILE" | tail -1 | cut -d: -f1)
[ -n "$last" ] || { bad "launcher.log has no run"; exit 1; }
section() { tail -n +"$last" "$LOGFILE"; }
section | grep -q "runtime: " && ok "runtime synced" || bad "no runtime line"
section | grep -q "SilentPatch installed" && ok "SilentPatch installed" || bad "no SilentPatch"
section | grep -q "Xidi installed" && ok "Xidi installed" || bad "no Xidi"
section | grep -q "DXVK installed" && ok "DXVK installed" || bad "no DXVK"
# After the kill below, the launcher (which waits on the game) records the
# exit — checked at the end, once the log has settled.
section | grep -q "game exited with" && ok "launcher saw the game exit" \
  || bad "launcher never saw the game exit"

# The settings it promised, in the file the game reads.
[ -n "$INI" ] || { bad "no SilentPatchCMR3.ini in the runtime"; exit 1; }
width=$(sed -n 's/^WIDTH=//p' "$INI" | tail -1)
height=$(sed -n 's/^HEIGHT=//p' "$INI" | tail -1)
[ -n "$width" ] && [ -n "$height" ] \
  && ok "resolution recorded (${width}x${height})" \
  || bad "no WIDTH/HEIGHT in the ini"
grep -q "DISPLAY_MODE=" "$INI" && ok "display mode recorded" || bad "no DISPLAY_MODE"

# Post-processing, when the settings ask for it: the layer must engage
# (vkBasalt announces itself in the game's play.log, wherever the box
# keeps the helper's state).
postfx=$(sed -n 's|^postfx *= *||p' "$KIT/launcher.cfg" 2>/dev/null | tail -1)
case "$postfx" in
  color|cinematic)
    playlog=""
    for pl in "$HOME/.config/cmr3-launcher/play.log" \
              "${CMR3_E2E_PLAYLOG:-/nonexistent}"; do
      [ -f "$pl" ] && playlog=$pl && break
    done
    if [ -n "$playlog" ] && grep -q "vkBasalt info" "$playlog"; then
      ok "postfx layer engaged"
    else
      bad "postfx configured but the layer never engaged"
    fi
    ;;
  *) ok "postfx off (or unset)" ;;
esac

# The user's game folder is read-only for us: our marker must not be
# there (their own Xidi.ini may be; it predates this launcher).
game=$(sed -n 's|^game = \(.*\)$|\1|p' "$KIT/launcher.cfg" | tail -1 | sed 's|^Z:||; s|\\|/|g')
if [ -n "$game" ] && [ ! -f "$game/.cmr3-manifest" ]; then
  ok "game folder clean (no manifest of ours)"
else
  bad "game folder missing or written to: $game"
fi

rm -f "$BOXLOG"
[ -n "$WRAP" ] && rm -rf "$WRAP"
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
