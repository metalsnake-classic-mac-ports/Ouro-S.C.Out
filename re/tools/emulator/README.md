# Running the original in an emulator

Basilisk II emulates a 68040 Mac. It needs files that are not in this repo:

- a Mac ROM: `re/work/emu/rom/PERFORMA.ROM` (Performa 630, 1 MB)
- a System 7.0 boot disk: `re/work/emu/system70.dsk`
- the game disk: `re/work/emu/scout.dsk` (writable copy of `SCOUT_1_4.dsk`)

Then:

    re/tools/emulator/build.sh   # once
    re/tools/emulator/run.sh

In the emulated Finder type `s` and press ⌘O to start S.C.OUT; click
to get past the Kalisto logo. The folder `re/work/emu/shared` appears as
a disk inside the emulator.

For automated tests: `osascript` (System Events) sends key presses to the
process `BasiliskII`, `cliclick` sends mouse clicks (`brew install cliclick`),
`screencapture -R x,y,w,h` takes screenshots of the window.
