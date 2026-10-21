# TODO

## Decision (2026-10-19)

S.C.Out gets a **new implementation in Rust**, based on the original Mac game (1993):

- `scout-core`: deterministic simulation with a fixed tick, no I/O, tested with unit and replay tests
- frontend with SDL3 for macOS, Windows and Linux
- the game data is read directly from the original disk image `SCOUT_1_4.dsk` (like a Doom WAD)
- gameplay as close to the original as possible; modern extensions as options,
  later also high-resolution graphics and sound packs
- the original 68k code (Ghidra) is the reference for the game logic,
  Paul Mercat's C++ port shows where to look

### Milestone 1: show the levels ✓

- [x] Cargo workspace: `scout-core`, `scout-data`, `scout` (SDL3)
- [x] Read the original disk image directly: HFS, resource fork, LZRW1-A, PICT, pltt
- [x] Levels, tiles, objects, level codes, pictures; tests against the real disk
      (skipped without it) and against the 92 levels the port did not change
- [x] Level values are translated to Mac graphics like the original loader does
      (tables taken by running CODE 5 / 0x48c in a 68020 emulator, see
      `re/tools/emulate_level_loader.py` and `crates/scout-data/src/translate.rs`)
- [x] Level viewer: ←/→ level, ↑/↓ palette; `--export` writes a BMP
- [x] Correct in-game palette: `pltt 128` in Palette Manager order, screen index i =
      entry (256 − swap_nibbles(i)) mod 256. Derived from a screenshot of the running original;
      level 1 now matches that screenshot pixel for pixel except the player sprite

### Next

- [x] Screen frame; level 1 start screen matches the original screenshot (99.15 %)
- [x] Player sprite and the start animation
- [x] Graphics: frame `SHAP 5102`–`5104`, side panel icons `5105`, block letters `5106`, box
      tiles `5108`; `5107` is still unknown (see "To check")
- [x] Game logic in `scout-core`, using the original code as reference
  - [x] Main loop (CODE 9, 0x0): single-threaded, one pass = one frame
  - [x] Input (CODE 6, 0x7a8): only the first pressed key counts; commands 0–3 move,
        7 and 4 are actions (once per key press), Esc starts a 300-frame self destruct,
        Command + Q / P quit / pause (CODE 6 0x222: bit of key code 55; Caps Lock is masked out)
  - [x] Movement (CODE 9, 0xb70 and 0x1f12): a move is one box, 4 px per frame (8 frames);
        passable: values < 40, 184–188, ≥ 219; one-way tiles 243–246 block one direction;
        the view scrolls up to 6 sideways / 4 up or down, within one frame of the logic
  - [x] Player sprite: object 4 × turret + hull direction (objects 0–15), starts facing down
  - [x] Frame rate of the original: one frame per vertical blank, 60 fps (VBL task in
        CODE 6, wait in CODE 8; measured in Basilisk II with `SCOUT_FPS=1`). Always 60 fps
        (decided 2026-10-21; slower Macs of the time may have dropped to 30 fps). Confirmed by a screen
        recording: start animation 8 frames per step, player 4 px per frame at 60 fps
  - [x] Start pad animation: 0x108 after loading, 64 frames 0xab…0xb2, then the closed
        hatch 0x10b (267), as the screenshot of the original shows
  - [x] Player update completely (CODE 9 0xb70, `player.rs`): tiles acting on the player,
        mines, rails and conveyors, cannons, hatch/checkpoint, accelerator, magnifier,
        teleporters, take / swap / drop, chips, firing missiles, scrolling. Main loop with
        death, level end, lives and self destruct. Matches the original in 101 levels ×
        3000 frames of random keys and 1000 random levels × 600 frames
        (`re/tools/emu_play.py`, tests `playing_matches_the_original*`)
  - [x] Projectiles drawn like the original (4 × 4, CODE 8 0x364)
  - [x] Restart from the last checkpoint after losing a life, lives across levels, game over,
        next level (CODE 7, CODE 5 0x1a0a / 0x2b7e / 0x301e); checked with restarts in the
        recordings of `emu_play.py`
  - [x] Screens between the levels (level / password / lives, try again, aborted, game over,
        perfect) like the original (CODE 8 0x176c …); waiting for a key on the level screen
  - [x] Music: `SHAP 200` with a port of the original player (CODE 10), byte for byte like
        the original (`re/tools/emu_music.py`, test `music_matches_the_original`); from
        position 1 at the start, stops in the background, from 0 when back
  - [ ] Title screen and menu (Play, Resume, Setting, Enter Password), level codes dialog,
        pictures 0x3e9 (game over) and 0x3ea (end)
  - [ ] Pause and quit like the original: Command + P back to the menu (Resume goes on),
        Command + Q quit
  - [ ] Dissolve between screens (CODE 8 0x1354); the random numbers it uses only with
        the lowest priority (see below)
  - [x] Sound effects ('snd ' 100–124) requested like the original, one channel; the messages
        after a level wait for a key and the end of their sound
  - [x] World automaton (CODE 9 0x23fa): 96 boxes per frame, skip flags, 254 tick handlers
        (198 animation steps generated, the rest ported by hand). Matches the original
        exactly in all 101 levels over 3000 frames and in 500 random scenarios
        (`re/tools/emu_world.py`, `re/tools/emu_fuzz.py`, test `original_world`)
  - [x] Explosions (handler table `-0x22d0`, 150 handlers): death of the player, colored
        circles set the door switches, the target starts the 120-frame level end,
        chain reactions. All 150 handlers covered by the random scenarios and matching
  - [x] Projectiles (tables `-0x3270` / `-0x2aa0`), cannons, mirrors, homing projectiles
        with the Toolbox `Random`: all 40 slots match the original frame by frame in 500
        random scenarios (projectiles of all types, aimed shots, computers, wall hits)
  - [x] Rails/conveyors carrying the player (`-0x539e` → forced direction)
  - [x] Switch flags `-0x53b8`…`-0x53b5`: set by exploding colored circles 162–165
  - [x] Who sets `-0x53b4` (switch of tile 61) and the carried object `-0x578c`: the player;
        dropping the key (object 30) on tile 0x100 clears `-0x53b4`

### Options (modern extensions, off by default unless noted)

- [ ] Turn off the sound that plays when the player starts moving (101 / 108)
- [ ] Choice of the projectile colors: intended (default) or as on a real Mac / in
      Basilisk II (see re/BUGS.md)

### Data packs (later)

- [ ] Choose a folder (or archive) with extracted data instead of the disk image: the layout
      `scout-extract` writes (PNG, WAV, level text) as the open format
- [ ] Packs that replace parts: sound packs, level packs, graphics packs, … on top of the
      original data
- [ ] High-resolution graphics and sounds (larger tiles / higher sample rates than the original)
- [ ] Levels larger than the original's 40 × 24 boxes (needs a level format with its size)

### Requirements

- Sound starts without delay: the rewrite must not add latency. Measured 2026-10-21 in
  the VM: from starting a sound to the device taking it ≈ 8 ms (max. 12; SDL's rate
  conversion ≈ 1 ms of it), the VM's sound device reports ≈ 13 ms; the way from the VM
  to the host's speakers is not measurable from inside. To compare: `afplay` a WAV from
  `scout-extract` in the VM and on the host.

### To check

- [ ] Objects 26–31 have pink frames in the extracted graphics (mask?): compare with the
      original in the emulator (26/27 are mirrors)
- [ ] `SHAP 5107`: what is it (second font?)

### Lowest priority

- [ ] Random numbers number for number like a real Mac (Basilisk II); not required, the
      game does not need to match the original's random numbers

The sections below were written for the old port and stay as a reference.
The old port now lives in `legacy-port/`, so `build/` below means `legacy-port/build/`.

## 0. Run the current code as a baseline

- [x] Install SDL 1.2 (`brew install sdl12-compat freetype pkg-config`)
- [x] Install SDL_ttf 2.0 (SDL 1.2 version). Homebrew no longer has it, so it is
      built from the `SDL-1.2` branch of SDL_ttf as a static library in `~/.local`
- [x] Build the current code unchanged (15 warnings, no errors)
- [x] Start the game from `build/`: Kalisto logo and title screen work
  - macOS asks for microphone access when the sound starts. The game waits
    until the dialog is answered. "Don't allow" is fine, the game needs no microphone.
- [x] Play a few levels: **crashes after about a minute of playing**
  - Crash in the `DoBullets` thread in `SDL_BlitSurface(board → screen)`, reading a NULL
    pixel format (`dst_fmt->bytes_per_pixel`). Most likely cause: several threads blit
    from the same surface (`board`) to different targets at the same time (e.g. `MoveUs`
    blits `board → boximg`), and SDL's per-surface blit cache is not thread-safe.
    See step 4.

## Original game (Mac version 1.4, 1993)

Disk image `SCOUT_1_4.dsk` (HFS floppy). Read it with `hfsutils` (`hmount`, `hcopy -m`).
Everything is in the resource fork of the application:

- `CODE` 0–10: 68k code (about 110 KB)
- `snd ` 100–124: the 25 sounds. `son01.wav` … `son25.wav` are `snd ` 100 … 124
  in the same order (e.g. 100 ACCELERATION, 101 DEPLACEMENT, 103 SERRURE, 121 gagne, 122 perdu)
- `PICT` 1000/1001/1002/2000/256: intro, title, Kalisto logo, about box
- `SHAP` ("secret stuff"): compressed with **LZRW1-A** (Ross N. Williams, public domain;
  the `DATA` resource contains "LZRW1-A 1.0 22-Jun-1991 Public Domain").
  Header: 4 bytes unpacked size, 4 bytes LZRW1-A flag. Decompressor: `re/tools/lzrw1a.py`.
  - `SHAP 6642` → 290880 bytes: the 101 levels (2880 bytes each)
  - `SHAP 5300` → the 100 level codes
  - `SHAP 5100`–`5108`, `5200`: probably graphics (to be checked)
  - `SHAP 0`, `1`, `200`: not compressed, probably settings/saved data
- `pltt`: 256-color palettes

- [x] Write a decompressor for `SHAP` (`re/tools/macres.py`, `re/tools/lzrw1a.py`)
- [x] Compare levels and codes with the port
  - Codes: 99 of 100 identical. Level 69: original `MUFUPO`, port and Codebook `MUFUMO`
    (check in the emulator which one the game accepts)
  - `LEVEL.DAT` of the port differs from the original in 9 levels (8, 10, 68, 71, 80, 86,
    94, 95, 99). Partly leftovers from saving with the port's editor (start tile 11 → 283,
    empty ground 0 → 41), partly real edits (e.g. level 8: 48 tiles, level 71: a grenade
    replaced by an arrow). The new version uses the original data.
- [x] Identify the graphics (`re/tools/tiles.py`, `re/tools/pict.py`)
  - The port's `images.bmp` (32×25 tiles, 320×200 view) comes from the **MS-DOS version**.
    The Mac version has its own graphics: 32×32 tiles, a view of 14×10 tiles (448×320).
  - `SHAP 5101`: 480 opaque tiles, 32×32, 8 bit, stored one after another (1024 bytes each).
    Floor values and tile values ≥ 40 index into it directly.
  - `SHAP 5100`: 39 objects (tile values 1–39), 2048 bytes each = 32×32 pixels + 32×32 mask.
    Drawn on top of the floor tile.
  - `SHAP 5102`: top band (448×32), `5103`: bottom bar (448×12), `5104`: left and right
    side panel (2 × 32×364). Together with the view a 512×364 frame at (64, 68) on the
    640×480 screen; the view is at (96, 100).
  - `SHAP 5106`: large font, 45 glyphs 32×32 masked (A–Z, 0–9, punctuation)
  - `SHAP 5107`: small font, the same 45 glyphs 16×16 masked
  - `SHAP 5108`: nine 32×32 pink panel pieces (side panel indicators?)
  - `SHAP 5105`: 19 small green displays of 28×24 (status monitor in the side panels?)
  - `SHAP 200`: the music (ProTracker module, "composed by frederic motte"); `SHAP 5200`
    a packed variant the game does not load.
  - `PICT 1000`, `1001`, `1002`, `2000`, `256`: intro, illustrations, Kalisto logo, about box.
    They decode correctly with their own color tables.
- [x] **Palette of the game screen** (solved, see Milestone 1). The tiles are drawn without color conversion
      (offscreen port with the screen's color table). The screen's color table is set by the
      Mac Palette Manager from `pltt 128` (`CODE 6` at 0x85e: `GetNewPalette`, color table
      blacked out, `SetPalette`). Which palette entry ends up at which index is decided by
      Mac OS at runtime; obvious mappings don't match the box screenshots (green floor = index 169).
      → read it from the running original in an emulator.
- Code: compiled for the **68020** (uses `muls.l`, memory-indirect addressing).
  Ghidra headless: `re/tools/ghidra/` (use processor `68000:BE:32:MC68020`).
  Level loader: `CODE 5` at 0x48c. Draw list for the view: `CODE 9` around 0x2c0.
  Toolbox traps stop Ghidra's decoding; `re/tools/ghidra/patch_traps.py` replaces them
  with TRAP #15 first, `re/tools/ghidra/decompile_all.sh` runs everything,
  `re/tools/xref.py` finds callers through the jump table.
  Important globals (offsets from A5): -0x579a/-0x5796 player box, -0x57aa direction,
  -0x57a6 pixels left of the move, -0x57ae speed, -0x57c6/-0x57c2 view, -0x5790 command,
  -0x6546 level map (40 × 24, translated values).
- [x] Run the original in an emulator: Basilisk II with a Performa 630 ROM and System 7.0,
      see `re/tools/emulator/` (ROM and system disk are not in the repo)
- [x] Level 69 code: the original accepts `MUFUPO` (as in its data); `MUFUMO` from the port
      and the Codebook gives "Wrong Code" (tested in the emulator, 2026-10-19).
      The original shows level numbers from 0 (so level 69 is shown as "68").
- [ ] Rename the sound files after the original resource names
- [ ] Write down what works and what doesn't (crashes, flickering, sound, timing)

## 1. Build setup

- [ ] Replace the Makefile with CMake
- [ ] Copy the resources into `build/` during the build, so they exist only once in the repo
  - Keep both `rules.txt` files: the root copy is for development and the editor,
    `build/rules.txt` is for the shipped game
- [ ] Make debug/release builds switch `FINAL` (and the matching `rules.txt`)

## 2. Fix obvious bugs

- [ ] Turn on compiler warnings and fix them
- [ ] Functions without a return value on some paths: `GetDir`, `dirx`, `diry`, `DoView`, `EnterCode` (without TTF)
- [ ] `na`/`na2` can be used uninitialized in `DoAnim`
- [ ] `GetLevel(int, char *)` returns without `fclose`
- [ ] `LoadSound`: the `malloc` result is used even when it is `NULL`
- [ ] Check prefs file handling (`prefs.dat` stores raw `SDLKey` values)

## 3. Port to SDL2 or SDL3

- [ ] Decide between SDL2 and SDL3
- [ ] Graphics: surfaces/`SDL_Flip` → renderer/textures
- [ ] Input: `SDL_GetKeyState`, `SDL_EnableUNICODE` → keyboard state and text input events
- [ ] Sound: `SDL_OpenAudio` callback → audio stream (or SDL_mixer)
- [ ] Text: SDL_ttf for the new SDL version

## 4. Replace threads with one game loop

The game draws to the screen and changes the level from several threads
(`DoBullets`, `DoAnim`, `DoView`, plus the main thread) without locks.
That is not allowed on modern macOS and can crash.

- [ ] Update animations, bullets and the view once per frame in the main thread
- [ ] Keep the original timing (`TANIM`, `TBALLS`, `TVIEW`, `TMUS`)

## 5. Optional cleanup

- [ ] Split `main.cpp` (about 4000 lines) into modules
- [ ] Use modern C++ where it makes the code clearer
- [ ] Decide whether to rename the sound files (`son01.wav` → ...)
