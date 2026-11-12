# TODO

Open work for Ouro S.C.Out. What is done is in [CHANGELOG.md](CHANGELOG.md) and
the git history; how the original works is in [NOTES.md](NOTES.md), its bugs
and oddities in [BUGS.md](BUGS.md).

Principles: gameplay as close to the original as possible (its 68k code is
the reference, checked against it in the emulator); modern extensions as
options, off by default unless noted.

## Game

- [ ] The block dissolve between screens (CODE 8 0x1354 / 0x1416)
- [ ] No lives (decided 2026-10-22): a lost life only restarts from the last checkpoint, no
      game over. Goes with it: LIVES on the level screen, the GAME OVER screen, the lives
      in the core (`Player::lives`, `Exit::GameOver`); BUGS.md / NOTES.md describe the
      original's lives. Instead a death counter where the lives were shown ("DEATHS : n"
      on the level screen; the rules for all ports, `../CLAUDE.md`, 2026-11-12)
- [ ] No passwords (`../CLAUDE.md` rule 2): Game → Enter Password (⌘E) and "PASSWORD:" on
      the level screen go; the level selection replaces them
- [ ] Continue: the exact state saved when quitting the program or the game; "Continue" as
      the main menu's first entry
- [ ] Rewind: a snapshot every few seconds; playing on from an earlier one makes a fork; a
      timeline of the forks with screenshots (or a live render) and when each was
- [ ] Game speed apart from the frame rate: the picture at the display's rate (60 / 120 Hz),
      the game at the original's speed; the ball drawn between game frames
- [ ] Keep the game over picture (PICT 1001): show it when the game quits (until a key or a
      few seconds)

## Options

- [ ] Gamepad (`../CLAUDE.md` rule 10, decided 2026-11-12): d-pad / stick for the four
      directions, buttons for get-drop, fire and self destruct, set on the keys page; the
      menus by gamepad too. Maybe later shared by all ports in a common Ouro framework
- [ ] Turn off the sound that plays when the player starts moving (101 / 108)

## Level selection

- [ ] A level selection in the menu: every level with its name and a thumbnail (the part of
      the level that is visible when it starts, i.e. the start view)
- [ ] Levels get names; the original levels are named after their password, level 1 (which
      has none; the original has no name for it either, only "LEVEL 000") is START
- [ ] Save per level whether it was completed and the times of three successful runs: the
      first, the fastest and the latest; mark completed levels in the selection; the deaths
      per level too
- [ ] The selection looks the same in every Ouro port (layout, behaviour), with the game's
      decoration (`../CLAUDE.md` rule 3)
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
- [ ] Fan datasets (`../CLAUDE.md` rule 8): graphics with more animation steps, a config file
      per dataset in the format shared by all Ouro ports (name, author, licence, version,
      game version, what it replaces, its limits)

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
