# S.C.Out
Old arcade/strategy game http://en.wikipedia.org/wiki/S.C.OUT

# About this project
A new implementation of S.C.Out in Rust, for macOS, Windows and Linux.
It reads the game data directly from the original Mac version (1993), so you
need the original disk image `SCOUT_1_4.dsk`. The gameplay should stay as close
to the original as possible, with optional modern extensions.

Work in progress, see [TODO.md](TODO.md).

## Building
You need Rust (via [rustup](https://rustup.rs)) and SDL3
(macOS: `brew install sdl3`). Then:

    cargo test
    cargo run -p scout -- path/to/SCOUT_1_4.dsk

`scout` plays the game: logo, title and menu (Play, Password, Settings,
About, Quit), levels, music and sound. Default keys: arrows move, Return
takes / drops, Space fires, Esc blows yourself up (as in the original);
Command (or Ctrl) + P pauses, + Q quits, F11 or Command (or Ctrl) + F
switches to full screen. The game pauses when its window loses the focus.
The keys can be changed in the settings, which are saved in SDL's
preference folder (macOS: `~/Library/Application Support/S.C.Out`).
Without a disk image on the command line (or in `SCOUT_DISK`) the game asks
for one on the first start and remembers it. `--dev` adds keys for
development: N/P next/previous level, R restart, Tab level map, F1 palette.
`scout DISK --export LEVEL out.bmp [screen|map|level] [PALETTE]` writes a
level as an image, `scout DISK --export-menu PAGE out.bmp` a menu page.

The tests that need the original disk look for it in `SCOUT_DISK` or at
`re/work/SCOUT_1_4.dsk` and are skipped without it.

## Folders
- `crates/scout-core/`: game model and simulation, no I/O
- `crates/scout-data/`: reads the original disk image (HFS, resource fork,
  LZRW1-A, PICT, palettes, levels, graphics)
- `crates/scout/`: the SDL3 frontend
- `legacy-port/`: the C++/SDL 1.2 port by Paul Mercat (2008), translated from
  French to English. It is kept as a reference. Its source was published at
  http://mercat.jean.free.fr/S.C.Out/
- `re/NOTES.md`: how the original works, from the analysis of its 68k code
- `re/BUGS.md`: bugs and oddities of the original and how the rewrite
  treats them
- `scout-extract [OPTIONS] [DISK] OUT_DIR` (crate `scout-data`): exports graphics
  (PNG, single and/or sheets), sounds (WAV), the music and its unused earlier version
  (ProTracker `.mod` and WAV), levels (text and/or pictures of the whole
  level), level codes and the palette as the game reads them, and the data pack
  `scout-data.zip` (`--only pack`: everything the game needs in ordinary formats, which
  the game reads instead of the disk image); `--help` lists the options. Install it with `cargo install --path crates/scout-data`; with `SCOUT_DISK`
  set to the disk image it works from any folder, e.g.
  `scout-extract --only level-images ~/scout-levels`
- `re/tools/`: tools to read and unpack the resources of the original game
