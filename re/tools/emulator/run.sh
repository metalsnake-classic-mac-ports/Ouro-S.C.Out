#!/bin/sh
# Start the original S.C.OUT in Basilisk II.
# Expects the shared tools (OURO_TOOLS, default ../ouro-tools next to this
# repository): basilisk/ (build.sh), roms/PERFORMA.ROM (Performa 630 ROM),
# systems/system70.dsk (System 7.0 boot floppy) or system75-sound.dsk;
# and in re/work/emu: scout.dsk (a writable copy of SCOUT_1_4.dsk).
# In the emulated Finder: type "s", then cmd-O to start the game.
# SCOUT_FPS=1 run.sh prints game frames and Mac ticks per second (needs the
# frames-per-second counter, part of the `ouro` build that build.sh makes).
set -e
REPO="$(cd "$(dirname "$0")/../.." && cd .. && pwd)"
EMU="$REPO/re/work/emu"
TOOLS="${OURO_TOOLS:-$(cd "$REPO/.." && pwd)/ouro-tools}"
mkdir -p "$EMU/shared"
# boot disk: the one with Sound Manager 3.1 if present (make-sound-disk.sh)
BOOT="$TOOLS/systems/system70.dsk"
[ -f "$TOOLS/systems/system75-sound.dsk" ] && BOOT="$TOOLS/systems/system75-sound.dsk"
cat > "$EMU/basilisk.prefs" <<PREFS
rom $TOOLS/roms/PERFORMA.ROM
disk $BOOT
disk $EMU/scout.dsk
extfs $EMU/shared
xpram $EMU/xpram
ramsize 16777216
modelid 5
cpu 4
fpu true
screen win/640/480
displaycolordepth 8
frameskip 1
nocdrom true
nogui true
jit false
idlewait true
title S.C.Out (original)
PREFS
exec "$TOOLS/basilisk/BasiliskII/src/Unix/BasiliskII" --config "$EMU/basilisk.prefs"
