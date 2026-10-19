# S.C.Out
Old arcade/strategy game http://en.wikipedia.org/wiki/S.C.OUT

# About this project
A new implementation of S.C.Out in Rust, for macOS, Windows and Linux.
It reads the game data directly from the original Mac version (1993), so you
need the original disk image `SCOUT_1_4.dsk`. The gameplay should stay as close
to the original as possible, with optional modern extensions.

Work in progress, see [TODO.md](TODO.md).

## Folders
- `legacy-port/`: the C++/SDL 1.2 port by Paul Mercat (2008), translated from
  French to English. It is kept as a reference. Its source was published at
  http://mercat.jean.free.fr/S.C.Out/
- `re/tools/`: tools to read and unpack the resources of the original game
