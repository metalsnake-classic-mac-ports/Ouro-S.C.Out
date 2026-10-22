# TODO

Open work for Ouro S.C.Out. What is done is in [CHANGELOG.md](CHANGELOG.md) and
the git history; how the original works is in [NOTES.md](NOTES.md), its bugs
and oddities in [BUGS.md](BUGS.md).

Principles: gameplay as close to the original as possible (its 68k code is
the reference, checked against it in the emulator); modern extensions as
options, off by default unless noted.

## Game

- [ ] Mouse in the menus (now only keys; a click opens the menu and closes screens)
- [ ] The block dissolve between screens (CODE 8 0x1354 / 0x1416)
- [ ] Check in Basilisk II whether the original game pauses in the background (we pause;
      the original only stops its music)

## Options

- [ ] Turn off the sound that plays when the player starts moving (101 / 108)
- [ ] Choice of the projectile colors: intended (default) or as on a real Mac / in
      Basilisk II (see BUGS.md)

## Level selection

- [ ] A level selection in the menu: every level with its name and a thumbnail (the part of
      the level that is visible when it starts, i.e. the start view)
- [ ] Levels get names; the original levels are named after their password, level 1 (which
      has none; the original has no name for it either, only "LEVEL 000") is START
- [ ] Save per level whether it was completed and the times of three successful runs: the
      first, the fastest and the latest; mark completed levels in the selection
- [ ] Decide: what counts as the time of a run (from the start animation to the target,
      over lost lives / restarts at checkpoints?), and how a level started from the
      selection relates to lives and the normal game

## Level editor

- [ ] Edit levels and make new ones: tiles, floor, objects, start box, level name; try a
      level right away from the editor
- [ ] Save levels into a level pack (see data packs)
- [ ] Decide: inside the game (a menu entry) or a separate program

## Data packs

- [ ] The pack layout as a folder (not zipped), to edit files in place
- [ ] Packs that replace parts on top of the original data: level packs, graphics packs,
      sound packs, music
- [ ] Music in other formats in a pack (WAV, Ogg), e.g. for own music
- [ ] High-resolution graphics and sounds (larger tiles, higher sample rates; needs an RGB
      renderer instead of palette indices)
- [ ] Levels larger than the original's 40 × 24 boxes (needs a level format with its size)

## Pack library

A place to share packs, free of charge for everyone. (Not to be called a
"marketplace": nothing is sold.)

- [ ] Browse and download packs from the pack library inside the game
- [ ] Players can submit their packs (levels, graphics, sounds, music)
- [ ] Every submission is reviewed and approved by MetalSnake before it appears; the game
      only loads approved packs
- [ ] Decide how: e.g. a GitHub repository where submissions are pull requests and approval
      is the merge, with an index the game reads from GitHub Pages; pack metadata (name,
      author, license, version, game version); a signature or checksum so the game knows a
      pack is approved and unchanged

## Release

- [ ] Build and test on Windows and Linux (so far built only on macOS; the Homebrew bottle
      also builds on Linux) - e.g. GitHub Actions for all three systems
- [ ] Downloads on GitHub Releases: a macOS app (`.app`, signed and notarized), a Windows
      program, a Linux package
- [ ] Homebrew bottles for Intel Macs (a `macos-15-intel` runner); macOS 14 has none because
      Homebrew's sdl3 has no bottle there
- [ ] A release checklist: CHANGELOG.md, version, tag, formula pull request in the tap

## Requirements

- Sound starts without delay: the rewrite must not add latency. Measured 2026-10-21 in
  the VM: from starting a sound to the device taking it ≈ 8 ms (max. 12; SDL's rate
  conversion ≈ 1 ms of it), the VM's sound device reports ≈ 13 ms; the way from the VM
  to the host's speakers is not measurable from inside. To compare: `afplay` a WAV from
  `ouro-scout-extract` in the VM and on the host.
- 60 frames per second always (like the original on a fast enough Mac).

## Lowest priority

- [ ] Random numbers number for number like a real Mac (Basilisk II); not required, the
      game does not need to match the original's random numbers
