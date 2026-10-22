# How the original works (Mac version 1.4)

Findings from the 68k code. Offsets like `CODE 9 0xb70` are offsets in the
segment without its 4-byte header (`re/work/code/segNN.bin`). Globals are
offsets from A5 (application globals). Bugs and oddities of the original
are collected in BUGS.md.

## Versions

The rewrite reads version 1.4 US (`SCOUT_1_4.dsk`, application dated
1993-01-25). Version 1.3.1 (a StuffIt archive with only the application,
1992-12-17, comment "EkriirkE") differs in 6 of 118 resources:

- `vers` 1: "S.C.Out 1.3.1" / "1.4US", "S.C.OUT 1.4 US"
- `SHAP` 1, the saved settings as shipped (21 longs, see Settings below): 1.3.1
  has music off (`-0x6dc4`), get/drop Return (0x24) and fire Space (0x31),
  system display; 1.4 music on, Enter (0x4c), keypad 5 (0x57), custom
  display. The direction keys (keypad 6/4/8/2) are the same.
- `DITL` 257 (settings): "system displaying" / "quick displaying" became
  "system display" / "custom display"; `DITL` 128 (the warning about it)
  named the Macs it works on (LC, LC II, IIsi, IIci, Quadra with built-in
  video) and in 1.4 says it may not work on some models.
- `CODE` 7: 1.4 calls `SysBeep(10)` for every OS event (suspend / resume,
  CODE 7 0x1048); everything else is the same code moved by 8 bytes.
- `SHAP` 6642, the levels (file values): 6 levels changed, 68 (6 boxes),
  71 (1), 80 (5), 94 (3), 95 (17), 99 (21, the target moved from (16, 11)
  to (21, 12)).

Graphics, sounds, codes and the other CODE segments are identical.
`ouro-scout-extract` also reads the 1.3.1 application (its resource fork), e.g.
`ouro-scout-extract --only level-images --levels 68,71,80,94,95,99 "S.C.OUT 1.3.1" OUT`.

## Tools

- `re/tools/macres.py`, `lzrw1a.py`: read resources, unpack `SHAP`
- `re/tools/ghidra/patch_traps.py`: replace Toolbox A-line traps by TRAP #15
  (Ghidra stops decoding at A-line traps); writes `re/work/code_patched/`
- `re/tools/ghidra/decompile_all.sh`: Ghidra headless, 68020, function starts
  from the jump table; writes `re/work/decomp/segNN.c`
- REA (MCP) works on the patched segments: `open_binary` with
  `format: raw-binary`, `architecture: m68k`, `entry_point: 0x0`
- `re/tools/xref.py`: callers through the jump table
- `re/tools/disasm.py`: disassembles a range of a segment with the Toolbox
  traps by name and jump table calls as "CODE n 0xoffset" (the decompiler
  shows every trap as `trap(0xf)`)
- `re/tools/emulate_level_loader.py`: runs original code in Unicorn (68020)
- `re/work/handler_tables.json`: tile value → handler (segment, offset) for
  the four tables below

Jump table: `JSR d16(A5)` with `d16 = 32 + 8 × entry + 2`; CODE 0 maps the
entry to (segment, offset). Function pointers stored in globals are
`A5 + d16` of the same form.

## Main loop (CODE 9 0x0)

Single-threaded; one pass is one frame:

1. swap view buffers (`-0x57d6` / `-0x57d2`)
2. frame counter `-0x6d08` = (`-0x6d08` + 1) mod (960 / `-0x6d04`); every
   10th frame the pulse `-0x5390` (mod 30)
3. accelerator `-0x538e` counts down; at 0 the speed `-0x57ae` is 4 again
4. if the player stands (`-0x57a6` = 0): read the key (CODE 6 0x7a8), or
   command −1 while the level ends (`-0x3a46` ≠ 0); while moving the
   command `-0x5790` keeps its value
5. without magnifier (`-0x538c` = 0): player update (CODE 9 0xb70) or, while
   dying (`-0x5396` ≠ 0), count that down; world automaton (CODE 9 0x23fa),
   projectiles (CODE 4 0x2392), view buffer (CODE 9 0x25a), stop all
   projectiles if `-0x6d00` (tile 0x114), draw (`-0x1b00`). With magnifier:
   count it down; at 0 back to the player (CODE 4 0x2048), else CODE 4
   0x1fde (the move keys move the view one box)
6. sounds; `-0x3a46` > 1 counts down, at 1 the level is won (`-0x6d0c` = 3)
   unless dying (then `-0x5396` = 1); `-0x5396` = 1 (and no level end):
   lives `-0x6dac` − 1 and `-0x6d0c` = 0xff (no lives left), 0xfe (self
   destruct) or 2; command 6 / 9 end with 1 / 0xfd
7. command 8 the first time (`-0x6cfc`), not in the magnifier: self
   destruct, `-0x5396` = 300, player hidden, tile 0x150 on the player

## Input (CODE 6 0x7a8)

Only the first pressed key counts: GetKeys, raw key codes translated to
virtual key codes with the `KMAP` resource (CODE 6 0x222, table at
`-0x1152`). Command `-0x5790`: 0–3 configured direction keys (left, up,
right, down), 7 and 4 actions (edge-triggered), 8 = Esc (virtual code
0x35: self destruct, 300 frames), 9 / 6 = Command + Q / P (CODE 6 0x222 sets `-0x1052` from the bit of key code
55, Command; it masks out key code 57, Caps Lock), −1 nothing.
The keys are set in Game → Setting (as virtual key codes): right 6, left
4, up 8, down 2 on the keypad, get/drop Enter, fire keypad 5. The rewrite
uses the arrow keys, Return, Space and Backspace instead.

Speed checked against a screen recording of the original in Basilisk II:
the start animation steps every 8 frames, the player moves 4 pixels per
frame, both at 60 frames per second.

## Level map and tile values

`-0x6546`: 40 × 24 words, values translated from the level file by the
loader (CODE 5 0x48c; tables in `crates/scout-core/src/translate.rs`).
Values < 40: object on floor, ≥ 40: full tile. The start box gets 0x108.
The loader fills the level copy at `-0x481a` (restart copy); starting the
level copies it to the map `-0x6546`, then plays the start animation
(CODE 5 ≈ 0x2ee0, player hidden via `-0x184` = 0, no world or player
update): 64 rounds of set_tile(start, 0xab + i/8, no skip) + redraw, then
0x10b (closed hatch) and `-0x184` = 1. So in play the start box is 0x10b.
The player code turns 0x10a (file values 288/297) into 0x10b when the
player stands on it.

Set a tile: CODE 9 0x2046 `(x, y, (tile << 16) | (skip << 8))` writes the
map and the visible view buffer, sets the skip flag `-0x5780[cell]` and
`-0x5791` if the player's box changed.

## Four handler tables (filled by CODE 5 0x1b54), 500 entries each

| Table | Called from | When |
|---|---|---|
| `-0x3a40` tick | CODE 9 0x23fa | every pass over the cell (see below) |
| `-0x3270` bullet enter | CODE 4 ≈ 0x2aaa | projectile enters the box |
| `-0x2aa0` bullet center | CODE 4 ≈ 0x2aaa | projectile at the box center |
| `-0x22d0` explosion | CODE 4 0x0 | explosion at the box |

World automaton CODE 9 0x23fa: per frame it processes the cells
`k·N … k·N + N − 1` (`N` = `-0x6d04` = **96**, so every cell every 10 frames;
`k` = frame counter); for each cell
with skip flag set it clears the flag, otherwise it calls `tick[tile]`
with `-0x53b2`/`-0x53ae` = x/y. Of the 227 tick handlers, about 100 just set
the next tile of an animation (e.g. 56 → 57 → 58 …). Others move things:
e.g. 400 (enemy): if the box to the right is free (CODE 9 0x1f94), the own
box becomes 0x191 and the right box 0x192 with skip flag; otherwise 0x19f
(turn). 243 (one-way door) closes again (0x6d) unless the player is next to it.

## Initialized globals

THINK C: CODE 1 copies `DATA` word by word below A5 and inserts `ZERO[i]`
zero bytes after every zero word; `DREL` relocates pointers.
`re/tools/thinkc_globals.py` rebuilds the 0x6e48 bytes of globals.

Explosion CODE 4 0x0 `(x, y)`: `explosion[tile]`, and if the player is at
(x, y), not protected (`-0x5392`) and the tile is not 0x10e: player death
animation (tile 0x150) and `-0x5396` = 300. Most explosion handlers set the
explosion animation 350 (things in motion also on their second box);
162–165 set the door switches `-0x53b8`…`-0x53b5`; 179 (target) sets
`-0x3a46` = 120 (level end); 33 sets `-0x5392` and explodes its neighbors.

Bug of the original: explosions of moving enemies write their second box
without a bounds check; on the bottom edge the skip flag lands in the
automaton's own position variable. Real levels have walls on almost all
edge boxes, so it hardly matters; the Rust version ignores such writes.

Testing: `re/tools/emu_world.py` (real levels) and `re/tools/emu_fuzz.py`
(random levels with walls around) run the original world automaton in
Unicorn; the test `crates/scout-original/tests/original_world.rs` compares the
Rust version frame by frame (tiles, floor, skip flags, global flags).

Projectiles: 40 entries of 0x30 bytes at `-0x6cf6` (active, …, x `-0x6cf2`,
y `-0x6cee`, vx `-0x6cea`, vy `-0x6ce6`, kind `-0x6ce2`), pixel positions.

## Player (CODE 9 0xb70)

- box `-0x579a`/`-0x5796`, relative to view `-0x57a2`/`-0x579e`, view
  `-0x57c6`/`-0x57c2`, pixel position in the view `-0x57be`/`-0x57ba`
- direction `-0x57aa` (0 left, 1 up, 2 right, 3 down), turret `-0x5786`
  (= direction unless carrying a missile: item 0x10–0x13 → 0–3)
- sprite = objects base + turret × 0x2000 + direction × 0x800, i.e. object
  `4 × turret + direction`
- a move: `-0x57a6` = 32, minus speed `-0x57ae` (4; 8 with the accelerator
  tile 0x10d for 600 frames) per frame; box updated when it reaches 0
- passable (CODE 9 0x1f12, view-relative): < 40, 184–188, ≥ 219
- one-way: target 246 not from the right … (left 246, up 243, right 245,
  down 244)

Ported completely to `scout-core/src/player.rs` and checked against the
original main loop running in Unicorn (`re/tools/emu_play.py`: the real
loop with keys fed into CODE 6 0x7a8, sound and blitting replaced by RTS,
stopping at CODE 9 0x28 after each pass), 101 levels × 3000 frames of
random keys plus 1000 random levels full of player tiles × 600 frames.
What the code does when the player stands on a box (`-0x57a6` = 0):

- hit by a projectile (`-0x6d1a`): tile 0x150, dying 300, done
- mines around (CODE 4 0x1c50): 0xb6/0xb7 explode with the player's box,
  0xb8 → 0xb9
- the tile (read from the view buffer, which mirrors the map in view):
  ≥ 0x124 explodes; leaving rails/conveyors (0x8c–0xaa, 0xf7–0xfe) clears
  the rail flag `-0x539e`; on 0x8c–0xaa the move keys do nothing
- tile switch (table at CODE 9 0x1dae): 0xe8 explodes when carrying,
  0x63–0x67 explode, conveyors 0xef–0xf2 repeat the direction (not when
  firing), cannons 0xdf–0xe2 fire once per visit (`-0x5398`), 0x10a →
  0x10b + checkpoint (CODE 5 0x301e), 0x10d accelerator (600 frames speed
  8), 0x10c magnifier (300 frames; projectiles move once more, the frame
  ends), 0x114 stops all projectiles, rails steer (`-0x53a2`) while on a
  rail
- forced command `-0x3a42` replaces the command
- commands (table CODE 9 0x1d9e): 0–3 move (passable, one-way doors,
  enemies in the way explode with the player), 7 use, 4 fire; with no move
  command a rail direction starts a move
- use (7): with empty hands: teleporter 0x101–0x104 → last box with 0x105–
  0x108 (CODE 4 0x2054, the view scrolls box by box to center the target),
  chips 0x110–0x112 (socket 0xff stays), objects < 40 (missiles 16–19 load
  the turret), on a rail a piece of rail (31). Carrying: swap with objects
  < 40, teleport, or drop: 30 (key) on 0x100 clears switch 4, 31 fills rail
  gaps (tile + 1), chips into sockets 0xff, otherwise only on empty boxes
- fire (4): the loaded missile from the turret (spawn CODE 9 0x2132 takes
  view-relative pixels)

Moving: 4 (8) px per frame; on arrival at view column 1/12 or row 1/8 the
view scrolls by up to 6/4 boxes within the same frame (CODE 9 0x3c6 …), each
step redraws (one VBL) and updates the drawing state of the projectiles
(CODE 9 0x232c). Teleporting likewise. The rewrite does the logic at once
and lets the frontend show the extra frames (`Game::tick`, `shown_view`).

See BUGS.md for the checkpoint and its projectile count, the magnifier
and the player's position, and the scroll condition that is always true.

## Drawing (CODE 8)

The main loop draws through the pointer `-0x1b00`: CODE 8 0x0 ("custom
display", `-0x6dbc` ≠ 0: own copy routines) or 0x440 ("system display":
QuickDraw), chosen in Game → Setting. One call per frame, after waiting for
the vertical blank (`-0xf72`, set by the VBL task of CODE 6):

1. erase projectiles in reverse slot order (draw state 1 or 2): restore
   the 4 × 4 background saved at slot +0x20, at the old screen address
   (slot +0x18)
2. boxes of the draw list `-0xb6e` (entries of 0x12 bytes, built by CODE 9
   0x25a from the differences between the two view buffers, and by the
   scroll functions): type 1 copy a tile, type 2 floor plus masked object
3. the player (`-0x184` visible) with its background handling
   (`-0x5792`, `-0x5791`: set_tile hit the player's box or its target)
4. projectiles with draw state 2 or 3 (slot +0x1c = new screen address,
   computed by CODE 9 0x232c): save the background, then a 4 × 4 pattern
   with the top left corner at the projectile position: rows `. a a .`,
   `64 65 65 65`, `64 65 65 65`, `. b b .` (palette 0x64 orange, 0x65
   yellow); a and b come from a bug, see BUGS.md
5. the carried object in both side panels when `-0x16e` is set (CODE 9
   0x5416 prepares it): icons of 28 × 19 pixels from SHAP 5105 (24 icons,
   icon = object − 16, 23 = empty hands), copied to view-relative (−28,
   260) and (450, 260), i.e. screen (68, 360) and (546, 360). Objects
   below 16 would read before the table; no level has any.

## Game flow (CODE 7, CODE 5)

The outer loop (CODE 7 ≈ 0x780) runs the main loop (CODE 9 0x0, jump
table A5+0x882) depending on the state `-0x6db2` and evaluates its result
(the low word of `-0x6d0c`); the level number (from 0) is `-0x6dae`:

- state 1, new level: level screen (CODE 8 0x176c), load the level (CODE 5
  0x1a0a), start it (CODE 5 0x2b7e), main loop
- state 3, after losing a life: the same without loading
- state 2, resume after a pause (Game → Resume)
- result 3 (target destroyed): screen CODE 8 0x1c26 and a sound, next
  level; after the last level (101) the picture 0x3ea, then level 1 again
- result 2 (died) / 0xfe (self destruct): screen CODE 8 0x1d2e / 0x1bb0, a
  sound, state 3
- result 0xff (no lives left): game over screen CODE 8 0x1b3a, picture
  0x3e9, back to the title (state 0)
- result 1 (Command + P) back to the title, 0xfd (Command + Q) quit;
  4 / 5 would go one level back / forward, but nothing returns them

Loading (CODE 5 0x1a0a) fills the state to start from (`-0x3e5e` …:
view, position, speed 4, direction down, turret down, nothing carried,
rail direction −1; switches `-0x3a54` … 0, 0, 0, 0, 1 (switch 4 = door 61
starts set); no protection; saved map `-0x481a` with 0x108 on the start
box, floor `-0x4bda`, skip flags `-0x3e1c` all clear, view buffer
`-0x409a`) and clears in the live state: protection, cannon flag, death
and level end countdowns, magnifier, forced command, player hit.

Starting (CODE 5 0x2b7e) clears death, level end, forced command, player
hit and all projectiles (count 0), copies the saved state into the live
one (view, position, speed, direction, remaining move, carried object,
turret and its lock, rail direction, switches, protection, map, floor,
skip flags, view buffers), draws the screen and plays the start animation
on the player's box. Entering the main loop clears the self destruct flag
`-0x6cfc`.

A checkpoint (tile 0x10a, CODE 5 0x301e) saves the live state as the one
to start from: the view is computed from the player's box like for the
start box; the carried object is saved as 0; the 41 projectile slots are
copied to `-0x538a` but never restored.

Never reset by loading or starting (they carry over to the next life and
the next level): lives, the world's frame counters `-0x6d08`/`-0x5390`,
hits on reinforced walls `-0x1ae8`, the accelerator countdown `-0x538e`,
the rail flag `-0x539e`, the delay timer `-0x6cfe`, the random seed.

New game (Game → Play, CODE 2 0x7e6): level 0, 5 lives. A level code
(Game → Enter Password, CODE 2 ≈ 0x4b4) starts that level with 5 lives;
the code `NOWALL` instead sets `-0x6d19`, which makes every box passable
(CODE 9 0x1f12).

The rewrite: `Game::new` / `load_level` / `restart` / `next_level`, checked
with `re/tools/emu_play.py`, which runs the original start routine and,
after a lost life, starts again from the saved state like CODE 7.

The red arrows, small displays and lights in the side panels are part of
the frame graphics (`SHAP 5104`); the game never changes them.

## Settings, lives, level screen

- Settings (Game → Setting, copied into globals by CODE 5 0x24c): sound
  effects `-0x6dc0` (guards all sound calls), music `-0x6dc4` (starts the
  music player in CODE 7),
  system/custom display `-0x6dbc`, keys `-0x6d5a` … `-0x6d46` (virtual key
  codes)
- Where they are kept: in the application itself. At the start CODE 5
  0x24c reads `SHAP` 1 (21 longs: sound, music, two arrays of six longs
  the game does not use otherwise, the six keys, display) and, if there is
  none, `SHAP` 0, the factory settings (music and sound off, system
  display). When the program quits, CODE 5 0x16ce removes `SHAP` 1 from
  the current resource file (the application), adds a new one with the
  current values and writes it (`AddResource`, `WriteResource`; a
  `SysBeep` on an error, e.g. a locked disk). So every copy of the program
  carries its own settings, and the shipped 1.3.1 and 1.4 differ in them.
- Lives `-0x6dac`: 5 at the start of a game (CODE 2), −1 per death in the
  main loop; 0 ends the game (`-0x6d0c` = 0xff)
- The level code dialog is Game → Enter Password (⌘E)

## Sound

25 sampled sounds, 'snd ' 100–124 (format 1, one bufferCmd, standard
header, 8-bit unsigned): 11127 Hz, the four after a level 5564 Hz. CODE 5
0x1218 loads 'snd ' n into `-0x1284 + 4·(n − 100)`. One channel: CODE 8
0x1ff8 (jump table A5+0x12ea) starts a sound and stops the one playing,
CODE 8 0x20e8 (A5+0x12f2) tells whether the channel still plays.

The game sets a request `-0x539c`; at the end of each pass (CODE 9 ≈
0x140, only with sound effects on, `-0x6dc0`) the main loop forgets the
playing sound `-0x1afc` once the channel is quiet, adds the sounds of what
is in view (CODE 9 0x546a, not during the magnifier, the last box with a
sound wins, each only if not already playing) and starts the request.

| Sound | When |
|---|---|
| 100 | accelerator (tile 0x10d) |
| 101 / 108 | a move starts (on conveyor 0xef–0xf2: 108), only if the channel is quiet and nothing else is requested |
| 102 | the player fires a missile |
| 103 | the key (object 30) into the lock 0x100 |
| 104 | a cannon fires: under the player, or hit by a projectile within the view |
| 105 | in view: explosion 350 |
| 106 | in view: tiles 96, 99 |
| 107 | self destruct (started directly); in view: big explosion 0x150 |
| 109 | teleporter, started directly; 111 after arriving |
| 110 | in view: doors 62, 67, 72, 77, 82, 87 |
| 112 | taking, swapping, dropping |
| 113 | in view: tile 213 |
| 114 | magnifier on and off |
| 116 | in view: tiles 105, 114, 123, 132 |
| 117 | a projectile hits a computer (166) |
| 118 | standing on tile 0x10e |
| 120 | a rail pulse picks the player up |
| 121 / 122 / 123 / 124 | PERFECT / GAME OVER / ABORTED / TRY AGAIN (CODE 7) |
| 115, 119 | loaded, never played |

## Music (CODE 10)

`SHAP 200` is the music: an uncompressed ProTracker module ("M.K.", 4
channels, 31 sample slots of which 11 are used, 24 positions, 17
patterns; sample names "composed by frederic motte"; title "s.c.out").
`SHAP 0` holds the factory settings (see Settings), not music.

`SHAP 5200`, packed like the graphics (LZRW1-A), is an earlier version of
the music the game never loads (in 1.3.1 and 1.4): a complete M.K. module
titled "2ul", 76072 bytes, with the same order list (24 positions, 17
patterns), but most rows of every pattern are different and samples 3 and 5
are other sounds ("of acoustic dreams", "music (only!) at :"); the other
nine samples are the same. Its sample names mention "ingame jingle : pos24"
and "lost jingle : pos25", but the song ends at position 24 and those
positions point at pattern 0: the jingles are not in the module. Played by
the game's player it takes 3:13 (OpenMPT: 3:16 at 50 ticks per second
instead of 50.76).


CODE 10 is a NoiseTracker-style replay with its own software mixer, no
Toolbox calls inside:

- 0x5e86 (jump table A5+0x1322) opens a second sound channel
  (`sampledSynth`, mono, no interpolation) and plays a double buffer of
  837 frames, 8-bit mono at 14161.98 Hz (`0x3751fbba`, 7/11 of 22254.54);
  each buffer is filled by three calls of CODE 10 0x0, one tick of 279
  samples each. 0x5fdc (A5+0x132a) stops it. So a tick is 1/50.76 s and
  speed 6 is about 8.5 rows per second.
- 0x5740, start: picks M.K. (31 samples) or the old 15-sample format, finds
  the highest pattern, reverses every sample in place and XORs it with
  0x80 (signed to unsigned; the name gets the mark "ARE" so it happens
  once), divides the volume table (0x1740, 64 rows of 256) by 4 once (mark
  "CNX"), speed 6, position = the start parameter.
- 0x587a, tick: a new row every `speed` ticks (0x58c8), the effects in
  between (0x5a8e): 1 / 2 portamento (limits 0x71 / 0x358), 3 tone
  portamento, 4 vibrato, 5 / 6 with volume slide, A volume slide, 0
  arpeggio; per row B position jump, C volume, D pattern break, F speed
  (1–31 only). At the end of the order list the song goes back to the start
  parameter; this module ends with B00, so it always continues at 0.
- Tables (CODE 10): the vibrato sine (0x3e) and the periods (0x5e, 36
  and a 0) are ProTracker's; the steps (0x9d8, periods 0–856) are
  `16800004 / period` in 16.16; the volume table (0x1740) is row v:
  `128 + floor((s − 128) · g_v)` with gains g_v that rise unevenly from
  0.049 (row 1) to 1 (row 63), no formula found. The rewrite builds all
  tables from these rules and the 64 gains (`scout_original::music::tables`,
  test `tables_match_the_original_code`); the data pack holds only the
  gains.
- 0x5d0a, mixer: per channel a step from the table at 0x9d8 (period × 4:
  integer and 16-bit fraction), samples read backwards through the volume
  table (row volume − 1), each channel into its own buffer, then the four
  added with 8-bit wrap-around (the quarter volume table keeps the sum in
  range).

CODE 7 starts the music at position 1 when the program starts (0x72a, music
on in the settings `-0x6dc4`, `-0x6db8` = playing) and at position 0 when
it is switched on again or the program comes back to the front (0x7d8). It
plays all the time, in the menu, the screens and the game; it stops when
it is switched off or the program goes to the background (suspend event,
CODE 7 0x105a, `-0x6db4`).

The rewrite: `scout_original::music::MusicPlayer`, a port of CODE 10; it
matches the original byte for byte over 7 minutes from positions 0 and 1
(`re/tools/emu_music.py`, test `music_matches_the_original`).
`ouro-scout-extract` renders the song once through from position 0 (as the
original or with its bugs fixed) into the data pack as FLAC; the game
plays that in a loop on a second stream, from the start (the original's
start at position 1 is not to be heard). Unlike the original it does not
start again when the game comes back to the front: it pauses in the
background and goes on where it stopped. `ouro-scout-extract --only music`
writes both modules as `.mod` and as WAV played by the game's player.

## Title, menus and dialogs (CODE 7, CODE 2)

Start (CODE 7 0x622): Toolbox init, menus (CODE 2 0x8b4), palette and
picture 2000 (the Kalisto logo), wait for a key (CODE 7 0x10be), black,
picture 1000 (the title), load the data, start the music (position 1),
settings and sounds, then the outer loop in state 0: nothing but the
picture until a menu command.

Pictures (CODE 8 0x12d8) are drawn with their own palette at the top left
corner of the 512 × 364 area of the frame (screen 64, 68), clipped: the
pictures are 512 × 380 (1000), 519 × 380 (1001, 1002, with a reddish
column right of x 512) and 512 × 384 (2000). CODE 8 0x1416 shows the
offscreen picture in 832 blocks of 16 × 14 in random order (Toolbox
`Random`); 0x1354 clears the screen the same way.

Menus (`MENU` 256–259, the menu bar of CODE 2 0x8b4, commands CODE 2
0x84c):

- Apple: About … (CODE 2 0x174: dialog 256 with picture 256, until a click)
  and the desk accessories
- File: only Quit (⌘Q) does something (state 0xff)
- Edit: only for desk accessories
- Game: Play (⌘G: level 0, 5 lives, state 1), Resume (⌘R: state 2, only
  enabled after a pause), Setting (⌘I, CODE 2 0x1108), Enter Password (⌘E,
  CODE 2 0x33a)

The resources 1256–1259 (menus, dialogs, alerts) are French; the ID base
`-0x6dc6` (low word of `-0x6dc8`) would select them, but it is always 0.

In the game: Command + P ends the main loop with result 1: the PAUSE box,
the screen is kept, Resume is enabled; Resume (state 2) puts the screen
back (CODE 8 0x1048) and the main loop goes on. Command + Q: result 0xfd,
quit. After GAME OVER the original disables Resume and shows picture 1001;
that picture stays as the title until the next game. After the last level
(101) picture 1002, then level 1.

Settings (dialog 257, CODE 2 0x1108): check boxes music on, sound effects
on, system / custom display (custom shows the warning `ALRT` 128 once per
dialog), and six edit fields for the keys right, left, up, down, get /
drop, fire. A key pressed in a field (its virtual key code, Tab and Esc
excepted) is stored, its name shown (CODE 6 0x38a) and the next field
selected. Music and sound switch at once; Cancel restores all values, OK
checks that no two keys are the same and else shows "You have selected
identical game keys" (`ALRT` 666 with `ParamText` strings from the
globals). Return and Enter are the OK button (filter CODE 2 0x16a8).

Password (dialog 259, CODE 2 0x33a): OK takes the first six characters,
makes them upper case and compares them with the 100 codes (CODE 2 0x63e;
code i starts level i + 1, with 5 lives); the code NOWALL (`-0xe72`) sets
`-0x6d19` (every box passable, CODE 9 0x1f38). A code that is not a level
code, NOWALL included, shows "Wrong password !" (`ALRT` 256). OK and
Cancel both close the dialog.

Other alerts (`ALRT` 666 via CODE 2 0x1780): problem during resource
loading, memory exhausted, system error, 256 colors only.

The rewrite does not copy the Mac interface: the title shows the pictures
like the original, the menu and the dialogs are marble boxes with block
letters (`crates/scout/src/menu.rs`), with hints and key names in the
system font. The shortcuts work with Command or Ctrl. Settings are saved
in a text file (SDL's preference folder) instead of the application.
The game pauses (the PAUSE box and the menu) when its window loses the
focus. The keys are a page of their own under Settings. Assigning a key that is in
use swaps the two keys. Self destruct, fixed to
Esc in the original, is a seventh key in the settings (default Esc).

## Pink frames

Objects 24–29 (mirrors and the like) have pink frames in the graphics.
The original shows them that way too (checked in Basilisk II, levels 7
and 68).

## Screens between the levels (CODE 8)

Boxes of marble tiles (SHAP 5108: 9 tiles of 32 × 32, corners, edges and
inside row by row; CODE 8 0x15ce) with block letters (SHAP 5106: 45
masked glyphs of 32 × 32, 32 pixels apart: A–Z, 1–9, 0, `.` = 38, `:` =
40, `-` = 44, CODE 8 0x19c2), on black. Positions are
relative to the view's top left corner (screen 96, 100), boxes given as
x, y, columns, rows:

- level (CODE 8 0x176c, before every life): box 0x20, −10, 12 × 10;
  "LEVEL nnn" at 0x50, 0x26 (number from 0: level 17 shows 016),
  "PASSWORD:" at 0x50, 0x66, the code at 0x70, 0x96, "LIVES : n" at 0x50,
  0xe6 ("LIFE  : 1" with one life). The first level has no code: box 0x20,
  0x36, 12 × 6, the level line 0x40 lower and the lives 0x40 higher.
- died (0x1d2e) "TRY AGAIN", self destruct (0x1bb0) " ABORTED ", game over
  (0x1b3a) "GAME OVER": box 0x30, 0x66, 11 × 3, text at 0x50, 0x86
- target destroyed (0x1c26) "PERFECT": box 0x50, 0x66, 9 × 3, text at
  0x70, 0x86
- pause (0x1cf8, after Command + P) "PAUSE": box 0x70, 0x66, 7 × 3, text
  at 0x90, 0x86, on the game screen
- LEVEL UP (0x1cc2) and LEVEL DOWN (0x1c8c) belong to the results 4 and 5,
  which nothing returns
- French texts exist for all of them (`-0x6dc8` = 1000: ETAPE, VIES, CODE,
  JEU TERMINE, INTERROMPU, PARFAIT, RECOMMENCE), but `-0x6dc8` is never
  written, so it is always 0 (English)

The font has 45 glyphs: A–Z, 1–9, 0, then `! ? . , : ; = + -` (36–44);
the game only uses letters, digits, `.`, `:` and `-`. `SHAP 5107` has the
same glyphs at 16 × 16; the game never uses it.

Every screen waits until a key or the mouse button is pressed and released
(CODE 7 0x10be); after the messages the original also waits for the end of
their sound before the level screen.
Between screens the original dissolves the picture (CODE 8 0x1354), which
uses the Toolbox `Random` and so changes the random numbers of the game.

## Player (continued)

- view scrolls on arrival one box from the edge (up to 6 sideways, 4
  up/down), within the same frame of the logic
- switch on the tile under the player: rails 0x8c–0xa9 and conveyors
  0xf7–0xfd force the next direction (`-0x53a2`) when `-0x539e` is set by
  the rail/conveyor tick handlers; steering is ignored on 0x8c–0xaa;
  cannons 0xdf–0xe2 fire once; 0x10c magnifier (300 frames); 0x10d speed
- the forced move does not check passability itself

## Projectiles (CODE 4 0x2392, called from the main loop via A5+0x3a2)

40 slots of 0x30 bytes at `-0x6cf6`: +0 active, +1 draw state (0 off screen,
3 just drawn, 2 on screen, 1 erase pending), +2 lifetime (word, type 3),
+4 x, +8 y (pixels in the level), +0xc vx, +0x10 vy, +0x14 type
(1 normal, 2 ricochet, 3 homing). `-0x57ca` counts active projectiles.

Spawn (CODE 9 0x2132, jump table A5+0x8ca) `(x, y, vx, vy)` with x/y
relative to the view: the first slot that is inactive **and** has draw
state 0 (so the draw state matters for slot order), type 1; returns 0 when
all 40 are taken.

Per frame, per slot in order (normal and ricochet): move by (vx, vy), box =
pixel / 32 (also into `-0x53b2`/`-0x53ae`); outside the level → off.
- just entered the box (speed 2: moving right x % 32 == 0, left
  (x + 2) % 32 == 0, same for y): tiles 40–94 (walls) stop it, type 2
  steps back and reflects using the wall test CODE 9 0x1fe4 on the
  neighbors; tiles 192–214 become explosion 350 and stop it; other tiles:
  `bullet_enter[tile]` (table `-0x3270`) if there is one
- otherwise at the box center (x + 16 or y + 16 multiple of 32, depending
  on the direction) on an occupied box: `bullet_center[tile]` (`-0x2aa0`)
- after a handler call the rest of the frame is skipped for this slot
- common part: draw state, reflect (type 2) or stop at the level edge
  (x 0/0x4fc, y 0/0x31c), lifetime of type 3, hitting the player
  (sets `-0x6d1a` unless protected or on tile 0x10e)

Type 3 (homing): velocity ±2 towards the player's box center; on a wall
tile it calls the Toolbox `Random` (trap A861) to decide which axis to undo.

Handlers: mirrors 26/27/220/221 turn the direction; 24/25/28/29/219/275
reflect; rails 144–159 switch; 162–165 colored circles set the door
switches; 20–23 set a forced player command (`-0x3a42`); 34 splits into four;
222–226 are cannons (spawn); 166 counts 0x110 neighbors; 218, 227, 228 call
further functions (A5+0x8e2, 0x8d2, 0x8da). Enemies and their second box
explode (350).

Reinforced wall 218 (enter handler CODE 4 0x42e8) pushes the low words of
the box x/y (`-0x53b0`, `-0x53ac`) and calls CODE 9 0x5586: hit counter
word at `-0x1ae8 + 2·(40·y + x)`, at 20 the box explodes. That function
also starts `-0x6cfe` at 600 (cleared when the wall explodes); the main loop
counts it down and at 0 runs an empty 1000-iteration loop, i.e. a tiny
delay with no effect on the game (not ported).

Checked against the original: the harness replaces the two `Random` traps
(CODE 4 0x2524, CODE 9 0x221e) by NOPs and puts Park-Miller numbers on the
stack, records all 40 slots per frame plus count, forced command, player
hit and seed. The random scenarios add projectiles of all types (some aimed
at a box), computers with chips, earlier wall hits and a random view.
