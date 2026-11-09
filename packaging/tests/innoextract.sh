#!/bin/sh
# The package carries innoextract (sdk/packaging/innoextract.sh), which
# unpacks the game's installer, in both the Windows zip and the AppImage:
# the pins are the SDK's, package.sh bundles it and hands components/ and
# licenses/ to the AppImage, the allowlist lets both paths in, and
# THIRD_PARTY.md credits it. Run from anywhere: sh packaging/tests/innoextract.sh
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd -P)
cd "$ROOT"
fail=0
no() { echo "FAIL: $*" >&2; fail=1; }

. sdk/packaging/innoextract.sh
# The SDK's two pin lines, the first with our mirror after the word
# `mirror` (packaging/third-party.txt's convention).
innoextract_pins | grep -v '^#' | while read -r name rest; do
  line=$(grep "^$name " packaging/third-party.txt) || { echo "FAIL: no $name pin" >&2; exit 1; }
  case $line in
    "$name $rest"*) ;;
    *) echo "FAIL: the $name pin differs from the SDK's: $line" >&2; exit 1 ;;
  esac
done || fail=1

grep -q '^\. sdk/packaging/innoextract\.sh$' packaging/package.sh || no "package.sh doesn't source innoextract.sh"
grep -q '^innoextract_bundle "\$PKG" "\$WORK"' packaging/package.sh || no "package.sh doesn't bundle innoextract into the zip"
grep -q -- '--payload components' packaging/package.sh || no "the AppImage leaves components/ out"
grep -q -- '--payload licenses' packaging/package.sh || no "the AppImage leaves licenses/ out"
grep -qx 'components/innoextract.exe' packaging/package.allow || no "package.allow lacks components/innoextract.exe"
grep -qx 'licenses/innoextract/\*.txt' packaging/package.allow || no "package.allow lacks licenses/innoextract/*.txt"
grep -q 'innoextract 1.9' THIRD_PARTY.md || no "THIRD_PARTY.md doesn't list innoextract 1.9"
grep -q '^innoextract dscharrer/innoextract ' packaging/upstream-watch.txt || no "nobody watches innoextract upstream"

[ "$fail" = 0 ] && echo "ok: innoextract is pinned, bundled, allowed and credited"
exit "$fail"
