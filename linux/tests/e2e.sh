#!/bin/sh
# End-to-end check on the dev box: boots the game through cmr3-play and
# verifies the machine-checkable parts of docs/RELEASE-CHECKLIST.md.
#
#   sh linux/tests/e2e.sh
#
# Needs the gaming distrobox, the test kit (launcher + thirdparty/ +
# launcher.cfg) and a game — set:
#   CMR3_E2E_KIT      the kit folder          (default /mnt/data/tmp-cmr3-test)
#   CMR3_E2E_PREFIX   the wine prefix         (default $KIT/prefix)
#   CMR3_E2E_PLAY     this repo's cmr3-play   (default ../cmr3-play)
#   CMR3_E2E_BOX      gamescope+wine wrapper  (default: distrobox enter gaming --)
#   CMR3_E2E_ALIVE_AT seconds to wait first   (default 90)
# Nothing here presses pad buttons or listens to music: those lines of the
# checklist stay human.
set -u
HERE=$(dirname "$(readlink -f "$0")")
KIT=${CMR3_E2E_KIT:-/mnt/data/tmp-cmr3-test}
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

BOXLOG=$(mktemp)
$BOX sh -c "CMR3_PREFIX='$PREFIX' CMR3_LAUNCHER='$KIT/CMR3Launcher.exe' '$PLAY' --play" \
  > "$BOXLOG" 2>&1 &

# Wait for the game (a fresh prefix boots Wine first; that can take a
# while), then give it a moment to get past the launcher's own setup.
boot=${CMR3_E2E_BOOT:-180}
waited=0
while [ "$waited" -lt "$boot" ]; do
  pgrep -x Rally_3PC.exe >/dev/null && break
  sleep 5
  waited=$((waited + 5))
done
if pgrep -x Rally_3PC.exe >/dev/null; then
  ok "game running after ${waited}s"
  sleep "${CMR3_E2E_ALIVE_AT:-10}"
  pgrep -x Rally_3PC.exe >/dev/null && ok "still alive ${ALIVE_AT:=10}s later" \
    || bad "game died right after starting"
else
  bad "game never started within ${boot}s; cmr3-play said: $(tail -5 "$BOXLOG" | tr '\n' ' ')"
fi

# A quartz fault must not freeze the game: with winedbg disabled it ends
# the process instead of deadlocking it. Either way the game must stop.
faults=$(grep -c "page fault" "$BOXLOG" 2>/dev/null)
faults=${faults:-0}
pkill -x Rally_3PC.exe
sleep 8
if pgrep -x Rally_3PC.exe >/dev/null; then
  pkill -9 -x Rally_3PC.exe
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
              /mnt/data/distrobox/gaming/.config/cmr3-launcher/play.log; do
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
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
