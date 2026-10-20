#!/bin/sh
# Start the original S.C.OUT in Basilisk II.
# Expects in re/work/emu: rom/PERFORMA.ROM (Performa 630 ROM), system70.dsk
# (System 7.0 boot floppy) and scout.dsk (a writable copy of SCOUT_1_4.dsk).
# In the emulated Finder: type "s", then cmd-O to start the game.
# SCOUT_FPS=1 run.sh prints game frames and Mac ticks per second (needs the
# patch scout-fps.patch, which build.sh applies).
set -e
EMU="$(cd "$(dirname "$0")/../.." && pwd)/work/emu"
mkdir -p "$EMU/shared"
cat > "$EMU/basilisk.prefs" <<PREFS
rom $EMU/rom/PERFORMA.ROM
disk $EMU/system70.dsk
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
exec "$EMU/src/BasiliskII/src/Unix/BasiliskII" --config "$EMU/basilisk.prefs"
