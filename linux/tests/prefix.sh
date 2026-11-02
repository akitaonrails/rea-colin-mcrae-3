#!/bin/sh
# Tests the Wine settings cmr3-play gives the game: sh linux/tests/prefix.sh
set -u
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T CMR3_PREFIX=$T/prefix CMR3_SYSFS=$T/nosys
# A stand-in wine that records its calls and writes what reg add would.
mkdir -p "$T/bin" "$T/prefix"
cat > "$T/bin/wine" <<'WINE'
#!/bin/sh
echo "wine $*" >> "$HOME/calls"
echo '"ShowCrashDialog"=dword:00000000' >> "$WINEPREFIX/user.reg"
WINE
printf '#!/bin/sh\necho "wineserver $*" >> "$HOME/calls"\n' > "$T/bin/wineserver"
chmod +x "$T/bin/wine" "$T/bin/wineserver"
PATH=$T/bin:$PATH
CMR3_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../cmr3-play"
fails=0
check() {
  if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: $2"; fails=$((fails + 1)); fi
}

unset WINEDLLOVERRIDES
check "overrides on their own" "$(dll_overrides)" "dinput8,dsound,d3d9=n,b;winedbg=d"
# The AppImage sets its own first; ours must still apply.
WINEDLLOVERRIDES="mscoree,mshtml="
check "overrides kept beside the AppImage's" "$(dll_overrides)" \
  "dinput8,dsound,d3d9=n,b;winedbg=d;mscoree,mshtml="

prepare_prefix
prepare_prefix
check "crash dialog turned off once" "$(grep -c 'reg add' "$T/calls")" 1
check "its wineserver awaited" "$(grep -c 'wineserver -w' "$T/calls")" 1

[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
