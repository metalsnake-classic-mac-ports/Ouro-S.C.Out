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

The game waits for the vertical blank after every frame (VBL task in
CODE 6 sets `-0xf72`, the drawing code in CODE 8 waits for it), so it runs
at 60 frames per second when the Mac is fast enough. Basilisk II does:
`SCOUT_FPS=1 re/tools/emulator/run.sh` prints game frames and Mac ticks
per second to the terminal (60/60 in level 1). `run.sh` sets `frameskip 1`;
Basilisk's default 6 shows only every 6th frame (10 images per second).

Sound: Basilisk II only plays sound through a Sound Manager 3.x component
(see `TECH` 6.8 in its source). System 7.0 has an older Sound Manager, so
with the plain boot floppy the emulator is silent, and the game hangs after
PERFECT / TRY AGAIN / GAME OVER, where it waits for the end of a sound.

    re/tools/emulator/make-sound-disk.sh MacOS75.ZIP Sound_Manager_3.1.sea_.hqx

builds `re/work/emu/system75-sound.dsk` (needs `brew install unar
hfsutils`): the System Folder of the System 7.5.0 "Disk Tools" floppy on a
20 MB volume plus the Sound Manager 3.1 extension and Sound control panel
(both free downloads from Apple), with the boot blocks of the floppy.
`run.sh` boots from it when it exists. (Sound Manager 3.1 on System 7.0
does not work: its control panel refuses that Mac.) With `SCOUT_FPS=1`
Basilisk also prints its first 20 audio component calls: 1 and 257 mean
the Sound Manager really plays through it.

For automated tests: `osascript` (System Events) sends key presses to the
process `BasiliskII`, `cliclick` sends mouse clicks (`brew install cliclick`),
`screencapture -R x,y,w,h` takes screenshots of the window.
