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
- [ ] Identify the graphics in `SHAP 5100`–`5200` and compare them with `images.bmp`
- [ ] Run the original in an emulator (Basilisk II or Mini vMac, needs a Mac II/Quadra ROM)
      to compare behavior
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
