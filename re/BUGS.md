# Bugs and oddities of the original

Errors and strange corners found while reading the original code (S.C.Out
1.4 for the Mac, 1993). Addresses are CODE segment and offset, globals are
offsets from A5 (see NOTES.md).

For each entry: what the code does, what it was probably meant to do, the
effect in the game, and what the rewrite does. "Kept" means the rewrite
does the same as the original (the comparison tests depend on it),
"fixed" means it does what was intended.

## Projectile colors come from an interrupt vector (fixed)

CODE 8 0x3aa … 0x3d4, drawing projectiles. The top and bottom row of the
4 × 4 projectile are written with `move.b $64.w, 1(a0)` and
`move.b $65.w, …`: the bytes at the *addresses* 0x64 and 0x65, not the
values 0x64 and 0x65 (`#$64`). The middle rows use the literal
`#$64656565`. Address 0x64 holds the level-1 interrupt autovector, a
pointer, so the colors depend on the machine: Basilisk II (ROM behind the
RAM) has 0x01009b60 there, giving a grey top and white bottom row; on a
real Mac the vector points into ROM (0x408…), giving grey for both.

Intended: orange (0x64) top row, yellow (0x65) bottom row, a symmetric
shape. The rewrite draws that (decided 2026-10-20).

## The checkpoint resets the projectile count (kept)

CODE 5 0x301e, saving the state to restart from when the player closes a
hatch (tile 0x10a). Everything is copied from the live state to the saved
one, except the projectile count: `-0x57ca = -0x3e62` copies the saved
count (always 0, set at level start) into the live counter instead of the
other way round.

What the counter is for: the number of the 40 projectile slots in use
(active plus waiting to be erased); +1 when a projectile is spawned
(CODE 9 0x2132), −1 when an ended projectile has been erased and its slot
becomes free (CODE 4 0x2392, CODE 9 0x232c). It is read in one place only:
tile 222 (projectile enter handler CODE 4 0x4374) ends an incoming
projectile and fires a new one downwards, but only if the counter is not
40, so that the shot is not lost when no slot is free; otherwise the
projectile flies through.

Effect: after a checkpoint the counter is too low (it can even go
negative). With all slots really in use, tile 222 then ends the incoming
projectile and the spawn fails: the shot vanishes. Needs many projectiles
at once, so it is rare.

## Teleporting leaks projectile slots in the counter (kept)

CODE 4 0x2054 (teleporters, end of the magnifier) first marks visible
projectiles for erasing, draws, then sets the draw state of all 40 slots
to 0. Slots of ended projectiles that were waiting to be erased (draw
state 1) become free that way without the counter `-0x57ca` going down.

Effect: the counter is too high afterwards. If it happens to be exactly
40, tile 222 lets projectiles through although slots are free (see
above). Rare as well.

## Ricochets check a different box in one direction (kept)

CODE 4, projectiles hitting a wall (ricochet, kind 2). To decide which
velocity component to turn, the code tests neighbor boxes for walls
(tiles 40–94). For a projectile moving up and left it tests the box to the
right with "not empty" (CODE 9 0x1f94) instead of "wall" (0x1fe4), unlike
the three other directions.

Effect: moving up-left, any object or enemy to the right of the hit wall
counts as a wall, so the projectile can bounce differently than in the
mirrored situation.

## Moving enemies write outside the level (rewrite ignores the write)

Explosion handlers of enemies between two boxes (CODE 4, table `-0x22d0`)
set the second box without a bounds check. On the bottom edge the write
lands behind the map; the skip flag ends up in the world automaton's own
position variable. Real levels have walls on almost all edge boxes, so it
hardly matters. The rewrite ignores writes outside the level. The random
test levels have walls all around to avoid this case.

Similar: the mines check around the player (CODE 4 0x1c50) and other
neighbor tests compute the box as `x + 40·y`, so `x − 1` on the left edge
reads the last box of the previous row. The rewrite treats boxes outside
the level as empty. Unreachable in real levels (walls on the edges).

## The target counts even if the player dies afterwards (kept)

Main loop, CODE 9 0x0. After the target is destroyed, `-0x3a46` counts
down from 120. When it reaches 1 and the player is dying, the code sets
the death countdown `-0x5396` to 1 instead of ending the level. In the
next frame the countdown reaches 0, which looks like "alive", and the
level ends as won.

Effect: dying in the 2 seconds after destroying the target does not cost
a life; the level is won. Possibly intended (the target was destroyed), in
any case the way the game works.

## The magnifier leaves the player's screen position behind (kept)

CODE 4 0x1fde moves the view while the magnifier (tile 0x10c) is active,
but not the player's pixel position relative to the view
(`-0x57be`/`-0x57ba`). The player is hidden meanwhile and the position is
recomputed when the magnifier ends (CODE 4 0x2048), so nothing is visible.
The rewrite derives the position from the view; the comparison test
ignores it during the magnifier.

## A checkpoint saves a loaded turret but not the missile (kept)

CODE 5 0x301e saves the carried object as 0 but the turret lock
`-0x5788` and direction as they are. When the player dies after a
checkpoint taken while carrying a missile, the next life starts with a
loaded turret and empty hands: firing shoots a missile that is not there.
Taking an object meanwhile keeps the lock, and firing then loses that
object.

## A checkpoint can make double speed permanent (kept)

The checkpoint saves the current speed `-0x57ae`, which is 8 while the
accelerator (tile 0x10d) runs. The accelerator countdown `-0x538e` is not
saved and not reset. If it has run out when the player dies, the next life
starts with speed 8 and keeps it (nothing sets it back to 4).

## Dead code after the main loop (harmless)

CODE 7 handles the main loop results 4 (previous level) and 5 (next level)
with their own screens, but the main loop never returns them (debug keys
removed?). The 41 projectile slots the checkpoint saves at `-0x538a` are
never read either.

## Hits on reinforced walls are never reset (kept)

The hit counters of tile 218 (`-0x1ae8`, 20 hits break the wall) are not
cleared when a level is loaded or started. A reinforced wall on the same
box in a later level, or after losing a life, starts with the hits of
before.

## The big explosion sound restarts every frame (kept)

CODE 9 0x5550, sounds of things in view: for the big explosion 0x150 the
code checks whether sound 105 is playing but requests 107. While a big
explosion is in view, 107 is requested in every frame and the main loop
starts it again each time. The rewrite requests the same (the frontend
restarts the sound like the Mac does).

## Cannon sounds one box beyond the view (kept)

CODE 4 0x4374 …, a projectile fires a cannon (tiles 223–226): the sound
is played if the cannon is "in view", tested as `view ≤ x ≤ view + 14`
and `view ≤ y ≤ view + 10`; the view is 14 × 10 boxes, so the column
right of it and the row below count as well.

## Waiting for a sound that never ends (kept, harmless on real Macs)

CODE 7, after PERFECT, TRY AGAIN, ABORTED and GAME OVER: the game waits
in a loop until the sound channel is quiet (CODE 8 0x20e8), with no time
limit. If the sound never finishes, e.g. in Basilisk II under System 7.0
(no Sound Manager 3), the game hangs there. The rewrite waits the same
way; its own audio output always finishes.

## Unused sounds and data

- 'snd ' 115 and 119 are loaded at start (CODE 5 0x1218) but never played.
- `SHAP 5107`: the block letters at 16 × 16; nothing draws them. (The
  rewrite's settings used them for a while.)
- `SHAP 5200`: an earlier version of the music ("2ul"), packed, never
  loaded. Its sample names announce an "ingame jingle : pos24" and a "lost
  jingle : pos25", but the song ends at position 24 and those positions
  point at pattern 0: the jingles were never written.
- `SHAP 0`: factory settings, read only if `SHAP 1` is missing, which it
  never is in a shipped copy.
- The French menus, dialogs, alerts and screen texts (see below).
- The screens LEVEL UP and LEVEL DOWN, see "Dead code after the main loop".

## A condition that is always true (harmless)

CODE 9 0x1c68, scrolling down on arrival at view row 8: the code skips
scrolling if the view's top row `-0x57c2` is 15, which never happens (the
largest value is 14). The amount to scroll is computed as 0 in that case
anyway, so the result is the same.

## A leftover delay timer (not ported)

CODE 9 0x5586, a reinforced wall (tile 218) hit by a projectile: starts
`-0x6cfe` at 600 if it is 0 and clears it when the wall breaks. The main
loop counts it down every frame and at 0 runs an empty loop of 1000
iterations, a tiny delay. It has no effect on the game.

## Unreachable: firing with an invalid turret direction

CODE 9 0x174c, firing: for a turret direction outside 0–3 the result
register `d7` is tested without being set. The turret direction is always
0–3, so this cannot happen.

## Music player: effects that do not work (kept)

CODE 10, the music replay, compared with ProTracker:

- E commands are ignored (0x5ca2 has no case for them). The module uses
  E93 (retrigger every 3 ticks) 13 times; those notes play once.
- C (set volume) changes only the volume of the mixer voice, not the
  channel's volume, so a later volume slide starts again from the volume
  of the sample.
- The mixer uses volume − 1 as the row of its volume table: volume 1 is
  as silent as 0, every volume a little quieter.
- The loop start of a sample is ignored (an unconditional branch jumps
  over that code); a loop plays the last `replen` bytes of the sample. All
  loops of this module end at the sample end, so this is not audible.
- The fraction of the sample position starts at 0 on every tick, a tiny
  pitch error.
- The mixer takes the whole volume word, finetune × 256 + volume, as the
  volume: a sample with a finetune other than 0 would read the wrong part
  of the table. All samples of this module have finetune 0. (The finetune
  is not applied to the pitch either.)

The rewrite plays the module exactly like the original. With
`MusicPlayer::fixed` (`ouro-scout-extract --music fixed`, the default) it corrects all of the
above. The E commands it handles are those that need no extra state: E1 / E2
fine portamento, E9 retrigger, EA / EB fine volume slide and EC note cut.
Finetune stays unapplied.

## Settings: the fire key is not checked for duplicates (fixed)

CODE 2 0x137e, OK in the settings: compares the keys 0–4 with each other
(right, left, up, down, get / drop) but never the fire key (index 5), so
the fire key can be the same as another key without a warning. The rewrite
never lets two actions have the same key: assigning a key that is in use
swaps them.

## Password: NOWALL is a wrong password (kept)

CODE 2 0x4a8: the cheat code NOWALL switches the cheat on and is then
looked up like a level code; it is none, so the dialog says "Wrong
password !". The rewrite does the same. The dialog also copies six
characters whatever the length of the entry (CODE 2 0x48e), so a shorter
entry is compared with leftover bytes after it; a real code is always six
characters, so this only matters for wrong entries.

## The French version is unreachable (not ported)

All menus, dialogs and alerts exist twice, English (256–259, 128, 666) and
French (1256–1259, 1128, 1666), and so do the texts of the screens between
levels (ETAPE, VIES, JEU TERMINE, PARFAIT, …). The ID base is the global
`-0x6dc8` (0 or 1000), but nothing ever writes it, so it stays 0: the
French texts can never appear. Probably set by hand for a French build.
The rewrite has only the English texts.

## Version 1.4 beeps at every switch to and from the game (not ported)

CODE 7 0x1048, operating system events (suspend / resume, i.e. another
program comes to the front or the game comes back): 1.4 calls
`SysBeep(10)` first, for every such event; 1.3.1 does not. Most likely a
leftover from debugging. The rewrite does not beep.

## The settings are written into the program itself (not ported)

The settings live in the application's own resource `SHAP 1`. Every time
the game quits, CODE 5 0x16ce removes that resource and writes a new one
into the application file. So every copy of the game carries its own
settings, the shipped versions 1.3.1 and 1.4 differ in them (music off /
on, other keys), and on a locked disk or a read-only copy saving fails
with nothing but a beep. The rewrite keeps its settings in a file in the
user's settings folder.

## The screen dissolve changes the game's random numbers (not ported)

The block dissolve between screens (CODE 8 0x1354 and 0x1416) picks its
832 blocks with the Toolbox `Random`, the same generator the game logic
uses (CODE 4 0x2524 and CODE 9 0x221e, e.g. a homing projectile that hits
a wall picks the axis to undo). How the random numbers of
a level come out therefore depends on how many screens were shown before.
The rewrite has no dissolve yet and its own random numbers (Park-Miller like
the Toolbox, but not matched number for number; see TODO.md).

## The music skips its first pattern once (changed)

CODE 7 0x72a starts the music at order position 1 when the program starts,
but at position 0 when the music is switched on again or the game comes
back from the background (0x7d8). The song ends with a jump to position 0
(command B00), so after the first time through it plays from 0. On the
first run the first pattern is skipped - not to be heard. Coming back from
the background restarts the song. The rewrite plays the song from the
start and lets it go on where it was after a switch to another program.

## Pictures larger than where they are shown (kept)

The title and end pictures are 512 × 380 (1000) and 519 × 380 (1001,
1002, with a reddish column right of x 512); CODE 8 0x12d8 draws them at
the top left of the 512 × 364 frame area, clipped. The bottom 16 rows (and
the column of 1001 / 1002) are never seen. The rewrite clips the same way.

## The Codebook gives a wrong code for level 69 (kept)

The printed code list ("Codebook") and the old C++ port give MUFUMO for
level 69. The game's own data has MUFUPO, and the original accepts only
that (checked in Basilisk II, 2026-10-19; MUFUMO gives "Wrong password").
The rewrite also accepts only MUFUPO.

## Not bugs, but easy to get wrong

- Self destruct is Esc (virtual key code 0x35), not the character `5`: the
  keys are virtual key codes (KMAP resource), and the default fire key is
  keypad 5 (0x57).
- The level screen counts from 0: level 17 shows "LEVEL 016".
- Command + P and Command + Q in the game are Command, not Caps Lock (key
  code 0x37).
- The colors of the game screen are not the order of `pltt 128`: the
  Palette Manager places the entries (screen index i = entry
  (256 − swap_nibbles(i)) mod 256, see NOTES.md).
- Scrolling happens within one frame of the game logic, box by box with a
  redraw (one vertical blank) per box; the world does not move meanwhile.
- Teleporter 0x101 + n leads to the last box (row by row) with 0x105 + n;
  for 0x104 that is 0x108, the value of the start box before the start
  animation turns it into 0x10b.
