#!/bin/sh
# Build Basilisk II (kanjitalk755/macemu, SDL2 via sdl2-compat) into the
# shared tools (OURO_TOOLS, default ../ouro-tools next to this repository):
# ouro-tools/basilisk.
# Needs: brew install autoconf automake libtool mpfr gmp sdl2-compat
set -e
REPO="$(cd "$(dirname "$0")/../.." && cd .. && pwd)"
TOOLS="${OURO_TOOLS:-$(cd "$REPO/.." && pwd)/ouro-tools}"
mkdir -p "$TOOLS"
SRC="$TOOLS/basilisk"
[ -d "$SRC" ] || git clone --depth 1 https://github.com/kanjitalk755/macemu.git "$SRC"
# S.C.Out speed measurement (SCOUT_FPS=1, see run.sh)
PATCH="$(cd "$(dirname "$0")" && pwd)/scout-fps.patch"
if git -C "$SRC" apply --check "$PATCH" 2>/dev/null; then
    git -C "$SRC" apply "$PATCH"
fi
cd "$SRC/BasiliskII/src/Unix"
NO_CONFIGURE=1 ./autogen.sh
# slirp (networking) is K&R C: compile C with gnu89.
CFLAGS="-O2 -std=gnu89 -Wno-implicit-function-declaration -Wno-int-conversion" \
CPPFLAGS="-I/opt/homebrew/include" LDFLAGS="-L/opt/homebrew/lib" \
./configure --enable-sdl-video --enable-sdl-audio --without-gtk --disable-jit-compiler \
    --without-mon --without-esd --without-x
make -j8
echo "built $SRC/BasiliskII/src/Unix/BasiliskII"
