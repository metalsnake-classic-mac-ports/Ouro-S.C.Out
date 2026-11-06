#!/bin/sh
# Build Basilisk II into the shared tools (OURO_TOOLS, default ../ouro-tools
# next to this repository): ouro-tools/basilisk, kanjitalk755/macemu on the
# branch `ouro` (the skill's patch series; it contains the S.C.Out
# frames-per-second counter, active only with SCOUT_FPS=1).
# Needs: brew install autoconf automake libtool mpfr gmp sdl2-compat
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
TOOLS="${OURO_TOOLS:-$(cd "$REPO/.." && pwd)/ouro-tools}"
mkdir -p "$TOOLS"
exec "$REPO/../skills/classic-mac-port/scripts/emulator/build-ouro.sh" "$TOOLS/basilisk" basilisk
