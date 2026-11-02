#!/bin/sh
# Tests cmr3-play's own-session lock, stop, the game's script and the
# settings exit codes; only harmless sleeps and fake programs run:
# sh linux/tests/supervision.sh
set -u
unset VK_INSTANCE_LAYERS VK_LAYER_PATH CMR3_WAIT GAMESCOPE_WAYLAND_DISPLAY
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
export T HOME=$T CMR3_PREFIX=$T/prefix CMR3_SYSFS=$T/nosys
STALE_PID= STALE_START= OTHER_PID= OTHER_START=
stop_helper() {
  [ -n "$1" ] || return 0
  stats=$(proc_stat "$1") || return 0
  if [ "${stats#* }" = "$2" ]; then kill "$1" 2>/dev/null || true; fi
  wait "$1" 2>/dev/null || true
}
cleanup() {
  stop_helper "$STALE_PID" "$STALE_START"
  stop_helper "$OTHER_PID" "$OTHER_START"
  rm -rf "$T"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$CMR3_PREFIX/drive_c/users/me/AppData/Local/CMR3Launcher"
CMR3_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr3-play"
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }

# The wrapper records the commands it would run (wineserver -k included).
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"command -v gamescope"*) exit 0 ;; esac
case "$*" in *"wineserver -k"*) echo "$*" >> "$T/wineserver" ; exit 0 ;; esac
echo "$*" > "$T/ran"
printf '<%s>\n' "$@" > "$T/args"
FAKE
chmod +x "$T/fake-run"
RUN=$T/fake-run GPU=

# A live pid with the wrong start time stands for a stale, reused lock.
sleep 30 & STALE_PID=$!
stats=$(proc_stat "$STALE_PID"); STALE_START=${stats#* }
printf '%s %s\n' "$STALE_PID" "$((STALE_START + 1))" > "$LOCK"
if running; then fail "stale lock reported running"; else ok "stale/reused pid refused"; fi
stop_session
if kill -0 "$STALE_PID" 2>/dev/null; then ok "stop did not signal a reused pid"; else fail "stale helper was signalled"; fi
if grep -q "WINEPREFIX=$CMR3_PREFIX wineserver -k" "$T/wineserver"; then ok "stop ends the prefix's wineserver"; else fail "stop: no wineserver -k"; fi
stop_helper "$STALE_PID" "$STALE_START"; STALE_PID= STALE_START=

# A Rally_3PC.exe of someone else's is not our launch.
ln -s "$(command -v sleep)" "$T/Rally_3PC.exe"
"$T/Rally_3PC.exe" 30 & OTHER_PID=$!
stats=$(proc_stat "$OTHER_PID"); OTHER_START=${stats#* }
rm -f "$LOCK"
if running; then fail "unrelated Rally_3PC.exe blocked the launch"; else ok "unrelated Rally_3PC.exe does not block"; fi
stop_session
if kill -0 "$OTHER_PID" 2>/dev/null; then ok "unrelated Rally_3PC.exe not signalled"; else fail "unrelated helper was signalled"; fi
stop_helper "$OTHER_PID" "$OTHER_START"; OTHER_PID= OTHER_START=

CFG=$CMR3_PREFIX/drive_c/users/me/AppData/Local/CMR3Launcher/launcher.cfg
printf 'resolution = 1920x1080\n' > "$CFG"
LAUNCHER=$T/CMR3Launcher.exe; : > "$LAUNCHER"

# Background launch: the lock names our own session until it ends.
cat > "$T/slow-run" <<'FAKE'
#!/bin/sh
case "$*" in *inner.sh*) exec sleep 30 ;; esac
exit 0
FAKE
chmod +x "$T/slow-run"
RUN=$T/slow-run
start > /dev/null
i=0; while ! running && [ "$i" -lt 20 ]; do sleep 0.05; i=$((i + 1)); done
if running; then ok "a background launch is running"; else fail "background launch not running"; fi
read_lock; SESSION=$LOCK_PID
stop_session
sleep 0.2
if kill -0 "$SESSION" 2>/dev/null; then fail "stop left the session"; else ok "stop ends our session"; fi
RUN=$T/fake-run

CMR3_WAIT=1 start > /dev/null
if [ -f "$LOCK" ]; then fail "a finished launch left its lock"; else ok "a finished launch removes its lock"; fi

# Spaces and an apostrophe stay one argument, never script text.
mkdir -p "$T/launcher it's here"
LAUNCHER=$T/"launcher it's here"/CMR3Launcher.exe
: > "$LAUNCHER"
CMR3_WAIT=1 start > /dev/null
if [ "$(tail -1 "$T/args")" = "<$LAUNCHER>" ]; then ok "apostrophe+space launcher is one argument"; else fail "launcher argument: $(tail -1 "$T/args")"; fi
case "$(cat "$STATE/inner.sh")" in
  *"$LAUNCHER"*) fail "launcher path in the generated script" ;;
  *'wine "$1" --no-window'*) ok "the game's script takes the launcher as \$1" ;;
  *) fail "inner.sh does not run wine \"\$1\"" ;;
esac

# The game's script gives the postfx layers to wine (the game), and only
# when postfx set them.
mkdir -p "$T/bin"
printf '#!/bin/sh\necho "layers=${VK_INSTANCE_LAYERS:-} path=${VK_LAYER_PATH:-} arg=$1" > "$T/wine-env"\n' > "$T/bin/wine"
printf '#!/bin/sh\nexit 1\n' > "$T/bin/pgrep"
printf '#!/bin/sh\nexit 0\n' > "$T/bin/wineserver"
printf '#!/bin/sh\nexit 0\n' > "$T/bin/sleep"
chmod +x "$T/bin/"*
PATH=$T/bin:$PATH CMR3_VK_INSTANCE_LAYERS=L CMR3_VK_LAYER_PATH=P CMR3_VKBASALT_CONFIG_FILE=C \
  sh "$STATE/inner.sh" "$LAUNCHER"
if [ "$(cat "$T/wine-env")" = "layers=L path=P arg=$LAUNCHER" ]; then ok "postfx layers reach the game"; else fail "game env: $(cat "$T/wine-env")"; fi
PATH=$T/bin:$PATH sh "$STATE/inner.sh" "$LAUNCHER"
if [ "$(cat "$T/wine-env")" = "layers= path= arg=$LAUNCHER" ]; then ok "no layers without postfx"; else fail "game env: $(cat "$T/wine-env")"; fi

# The settings window: Play (0) and no window (4) go on to the game;
# closing it (3) or a failure does not.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in *"--settings --linux"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc")" ;; esac
exit 0
FAKE
for case in 0:0 4:0 3:1 1:1; do
  echo "${case%:*}" > "$T/rc"
  if settings; then got=0; else got=1; fi
  if [ "$got" = "${case#*:}" ]; then ok "settings exit ${case%:*} -> $got"; else fail "settings exit ${case%:*} gave $got"; fi
done

# launch() reset the EXIT trap of the shell it ran in.
trap cleanup EXIT HUP INT TERM
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
