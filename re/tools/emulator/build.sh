#!/bin/sh
# Build Basilisk II into the shared tools (OURO_TOOLS, default ../ouro-tools
# next to this repository) with ouro-tools' build-emulators.sh: ouro-macemu,
# branch ouro (it contains the S.C.Out frames-per-second counter, active
# only with SCOUT_FPS=1). Clones ouro-tools first if it is missing.
# Needs: brew install autoconf automake libtool mpfr gmp sdl2-compat
set -e
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
TOOLS="${OURO_TOOLS:-$(cd "$REPO/.." && pwd)/ouro-tools}"
[ -x "$TOOLS/build-emulators.sh" ] ||
    git clone https://github.com/metalsnake-classic-mac-ports/ouro-tools.git "$TOOLS"
exec "$TOOLS/build-emulators.sh" basilisk
