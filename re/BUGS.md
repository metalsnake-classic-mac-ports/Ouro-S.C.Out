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

## Not bugs, but easy to get wrong

- Self destruct is Esc (virtual key code 0x35), not the character `5`: the
  keys are virtual key codes (KMAP resource), and the default fire key is
  keypad 5 (0x57).
- The level screen counts from 0: level 17 shows "LEVEL 016".
- Scrolling happens within one frame of the game logic, box by box with a
  redraw (one vertical blank) per box; the world does not move meanwhile.
- Teleporter 0x101 + n leads to the last box (row by row) with 0x105 + n;
  for 0x104 that is 0x108, the value of the start box before the start
  animation turns it into 0x10b.
