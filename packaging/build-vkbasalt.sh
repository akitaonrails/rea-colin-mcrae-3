#!/bin/sh
# Builds both Vulkan layer bitnesses from an unpacked vkBasalt source tree.
# The caller installs Meson, Ninja, a multilib C++ toolchain and 32/64-bit X11
# development files: packaging/build-vkbasalt.sh <source> <output>
set -eu
SRC=$(readlink -f "$1")
OUT=$(readlink -f "$2")
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

meson setup "$WORK/build64" "$SRC" --buildtype=release --libdir=lib
ninja -C "$WORK/build64"

cat > "$WORK/i386.ini" <<'EOF'
[binaries]
c = ['gcc', '-m32']
cpp = ['g++', '-m32']
ar = 'ar'
strip = 'strip'
pkg-config = 'pkg-config'

[host_machine]
system = 'linux'
cpu_family = 'x86'
cpu = 'i686'
endian = 'little'

[properties]
pkg_config_libdir = ['/usr/lib/i386-linux-gnu/pkgconfig', '/usr/lib32/pkgconfig', '/usr/share/pkgconfig']
EOF
meson setup "$WORK/build32" "$SRC" --cross-file "$WORK/i386.ini" --buildtype=release --libdir=lib
ninja -C "$WORK/build32"

mkdir -p "$OUT/lib32"
cp "$WORK/build64/src/libvkbasalt.so" "$OUT/libvkbasalt.so"
cp "$WORK/build32/src/libvkbasalt.so" "$OUT/lib32/libvkbasalt.so"
cp "$SRC/LICENSE" "$OUT/LICENSE"
