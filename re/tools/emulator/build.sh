#!/bin/sh
# Build Basilisk II (kanjitalk755/macemu, SDL2 via sdl2-compat) into re/work/emu.
# Needs: brew install autoconf automake libtool mpfr gmp sdl2-compat
set -e
EMU="$(cd "$(dirname "$0")/../.." && pwd)/work/emu"
mkdir -p "$EMU"
[ -d "$EMU/src" ] || git clone --depth 1 https://github.com/kanjitalk755/macemu.git "$EMU/src"
# S.C.Out speed measurement (SCOUT_FPS=1, see run.sh)
PATCH="$(cd "$(dirname "$0")" && pwd)/scout-fps.patch"
if git -C "$EMU/src" apply --check "$PATCH" 2>/dev/null; then
    git -C "$EMU/src" apply "$PATCH"
fi
cd "$EMU/src/BasiliskII/src/Unix"
NO_CONFIGURE=1 ./autogen.sh
# slirp (networking) is K&R C: compile C with gnu89.
CFLAGS="-O2 -std=gnu89 -Wno-implicit-function-declaration -Wno-int-conversion" \
CPPFLAGS="-I/opt/homebrew/include" LDFLAGS="-L/opt/homebrew/lib" \
./configure --enable-sdl-video --enable-sdl-audio --without-gtk --disable-jit-compiler \
    --without-mon --without-esd --without-x
make -j8
echo "built $EMU/src/BasiliskII/src/Unix/BasiliskII"
