#!/bin/sh
# Tests that linux/tests/e2e.sh, the real-game test, starts nothing unless
# CMR3_E2E=1 asks for it: run in a loop over linux/tests/*.sh it says
# "skipped" and exits 0, without touching the box, the kit or the game.
#   sh linux/tests/e2e-gate.sh
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
fails=0
# A box wrapper that records every call (the real one would start Wine).
cat > "$T/box" <<FAKE
#!/bin/sh
echo "\$*" >> "$T/box-calls"
FAKE
chmod +x "$T/box"
mkdir -p "$T/kit"; : > "$T/kit/CMR3Launcher.exe"
# A copy of the script beside a scratch folder, so the remembered-kit file
# it would write never lands in the repository.
mkdir -p "$T/tests"; cp "$HERE/e2e.sh" "$T/tests/e2e.sh"

run() { # value of CMR3_E2E ('' = unset)
  if [ -z "$1" ]; then
    env -i PATH=/usr/bin:/bin HOME="$T" CMR3_E2E_KIT="$T/kit" CMR3_E2E_BOX="$T/box" \
      sh "$T/tests/e2e.sh" > "$T/out" 2>&1
  else
    env -i PATH=/usr/bin:/bin HOME="$T" CMR3_E2E="$1" CMR3_E2E_KIT="$T/kit" \
      CMR3_E2E_BOX="$T/box" sh "$T/tests/e2e.sh" > "$T/out" 2>&1
  fi
}

for value in '' 0 yes; do
  rm -f "$T/box-calls" "$T/tests/.e2e-kit"
  run "$value"; code=$?
  name="CMR3_E2E='${value:-unset}'"
  [ "$code" = 0 ] && echo "ok   $name exits 0" || { echo "FAIL $name exit $code"; fails=$((fails + 1)); }
  grep -q "^skipped: real-game test; set CMR3_E2E=1 to run it (headless by default)$" "$T/out" \
    && echo "ok   $name says skipped" || { echo "FAIL $name: $(cat "$T/out")"; fails=$((fails + 1)); }
  [ ! -e "$T/box-calls" ] && echo "ok   $name starts nothing" \
    || { echo "FAIL $name called the box: $(cat "$T/box-calls")"; fails=$((fails + 1)); }
  [ ! -e "$T/tests/.e2e-kit" ] && echo "ok   $name remembers no kit" \
    || { echo "FAIL $name wrote .e2e-kit"; fails=$((fails + 1)); }
done

# Sourced (as a loop over test files might), it starts nothing either and
# leaves the caller running.
rm -f "$T/box-calls"
out=$(env -i PATH=/usr/bin:/bin HOME="$T" CMR3_E2E_KIT="$T/kit" CMR3_E2E_BOX="$T/box" \
  sh -c ". '$T/tests/e2e.sh'; echo caller-continues" 2>&1)
case "$out" in *caller-continues*) echo "ok   sourced: the caller goes on" ;; *) echo "FAIL sourced: $out"; fails=$((fails + 1)) ;; esac
[ ! -e "$T/box-calls" ] && echo "ok   sourced: starts nothing" || { echo "FAIL sourced called the box"; fails=$((fails + 1)); }

# Asked for (CMR3_E2E=1) but with no launcher.log in the kit's prefix, it
# stops before starting anything.
rm -f "$T/box-calls"
run 1; code=$?
[ "$code" = 2 ] && [ ! -e "$T/box-calls" ] && echo "ok   CMR3_E2E=1 without a prepared kit stops first" \
  || { echo "FAIL CMR3_E2E=1 without a kit: exit $code, $(cat "$T/out")"; fails=$((fails + 1)); }

# Headless by default: the play command puts a gamescope wrapper that adds
# --backend headless first in PATH, unless CMR3_E2E_VISIBLE=1. The
# wrapper, taken from e2e.sh, run against a fake gamescope later in PATH:
mkdir -p "$T/wrap" "$T/real"
sed -n "/<<'WRAPPER'\$/,/^WRAPPER\$/p" "$HERE/e2e.sh" | sed '1d;$d' > "$T/wrap/gamescope"
printf '#!/bin/sh\necho "real gamescope $*"\n' > "$T/real/gamescope"
chmod +x "$T/wrap/gamescope" "$T/real/gamescope"
got=$(env -i PATH="$T/wrap:$T/real:/usr/bin:/bin" gamescope -W 1280 -- sh x 2>&1)
[ "$got" = "real gamescope --backend headless -W 1280 -- sh x" ] \
  && echo "ok   the wrapper runs the real gamescope headless" \
  || { echo "FAIL wrapper: $got"; fails=$((fails + 1)); }
# Alone in PATH (only readlink beside it, so a real gamescope on this
# machine is never started):
mkdir -p "$T/tools"; ln -s "$(command -v readlink)" "$T/tools/readlink"
got=$(env -i PATH="$T/wrap:$T/tools" "$T/wrap/gamescope" 2>&1); code=$?
[ "$code" = 127 ] && echo "ok   the wrapper never runs itself" || { echo "FAIL wrapper alone: $code $got"; fails=$((fails + 1)); }
grep -q 'CMR3_E2E_VISIBLE' "$HERE/e2e.sh" && echo "ok   a visible window only with CMR3_E2E_VISIBLE" \
  || { echo "FAIL no CMR3_E2E_VISIBLE in e2e.sh"; fails=$((fails + 1)); }
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
