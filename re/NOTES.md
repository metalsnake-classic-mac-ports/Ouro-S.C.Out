# How the original works (Mac version 1.4)

Findings from the 68k code. Offsets like `CODE 9 0xb70` are offsets in the
segment without its 4-byte header (`re/work/code/segNN.bin`). Globals are
offsets from A5 (application globals).

## Tools

- `re/tools/macres.py`, `lzrw1a.py`: read resources, unpack `SHAP`
- `re/tools/ghidra/patch_traps.py`: replace Toolbox A-line traps by TRAP #15
  (Ghidra stops decoding at A-line traps); writes `re/work/code_patched/`
- `re/tools/ghidra/decompile_all.sh`: Ghidra headless, 68020, function starts
  from the jump table; writes `re/work/decomp/segNN.c`
- REA (MCP) works on the patched segments: `open_binary` with
  `format: raw-binary`, `architecture: m68k`, `entry_point: 0x0`
- `re/tools/xref.py`: callers through the jump table
- `re/tools/emulate_level_loader.py`: runs original code in Unicorn (68020)
- `re/work/handler_tables.json`: tile value → handler (segment, offset) for
  the four tables below

Jump table: `JSR d16(A5)` with `d16 = 32 + 8 × entry + 2`; CODE 0 maps the
entry to (segment, offset). Function pointers stored in globals are
`A5 + d16` of the same form.

## Main loop (CODE 9 0x0)

Single-threaded; one pass is one frame:

1. swap view buffers (`-0x57d6` / `-0x57d2`)
2. frame counter `-0x6d08` = (`-0x6d08` + 1) mod (960 / `-0x6d04`)
3. read input (CODE 6 0x7a8) unless the player is dying
4. if no special state (`-0x538c` = 0, e.g. magnifier): player update
   (CODE 9 0xb70, skipped during the self-destruct countdown `-0x5396`),
   world automaton (CODE 9 0x23fa), projectiles (jump table, CODE 4),
   CODE 9 0x25a, draw (`-0x1b00`)
5. sounds/music, end conditions (`-0x6d0c`: 1 pause, 0xfd quit, 2/3 won/lost …)

## Input (CODE 6 0x7a8)

Only the first pressed key (GetKeys + KCHR) counts. Command `-0x5790`:
0–3 configured direction keys (left, up, right, down), 7 and 4 actions
(edge-triggered), 8 = key `5` (self destruct after 300 frames),
9 / 6 = Caps Lock + Q / P, −1 nothing.

## Level map and tile values

`-0x6546`: 40 × 24 words, values translated from the level file by the
loader (CODE 5 0x48c; tables in `crates/scout-core/src/translate.rs`).
Values < 40: object on floor, ≥ 40: full tile. The start box gets 0x108.

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
(x, y), not invincible (`-0x5392`) and the tile is not 0x10e: player death
animation (tile 0x150) and `-0x5396` = 300.

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
- view scrolls one box per frame when the player arrives one box from the
  edge (up to 6 sideways, 4 up/down)
- switch on the tile under the player: rails 0x8c–0xa9 and conveyors
  0xf7–0xfd force the next direction (`-0x53a2`) when `-0x539e` is set by
  the rail/conveyor tick handlers; steering is ignored on 0x8c–0xaa;
  cannons 0xdf–0xe2 fire once; 0x10c magnifier (300 frames); 0x10d speed
- the forced move does not check passability itself
