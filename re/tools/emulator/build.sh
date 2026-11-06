#!/bin/sh
# Build Basilisk II in ouro-macemu next to this repository (OURO_MACEMU) with
# the shared tools' build-emulators.sh (OURO_TOOLS, default ../ouro-tools);
# ouro-macemu contains the S.C.Out frames-per-second counter, active only
# with SCOUT_FPS=1. Clones ouro-tools (and ouro-macemu) first if missing.
# Needs: brew install autoconf automake libtool mpfr gmp sdl2-compat
set -e
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
TOOLS="${OURO_TOOLS:-$(cd "$REPO/.." && pwd)/ouro-tools}"
[ -x "$TOOLS/build-emulators.sh" ] ||
    git clone https://github.com/metalsnake-classic-mac-ports/ouro-tools.git "$TOOLS"
exec "$TOOLS/build-emulators.sh" basilisk
