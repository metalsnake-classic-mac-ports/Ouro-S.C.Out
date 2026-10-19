"""Differential test scenarios for the world automaton.

Builds random levels from the tiles that have handlers (plus walls and
empty boxes), a random player position and random global flags, runs them in the
original code (emulated, see emu_world.py) and writes the changes like
emu_world.py, with a header line:

    # player X Y switches S0 S1 S2 S3 S4 carried C pulse P frame F

switches S0..S4 are the flags -0x53b8 … -0x53b4. Scenarios are reproducible
from their number.

Usage: python3 emu_fuzz.py <work dir> <count> <frames> <out dir> [exclude tiles...]
"""
import json
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from emu_world import A5, H, SEG_BASE, SEG_SIZE, SLOTS, W, Original, header, run_frames  # noqa: E402
from unicorn.m68k_const import UC_M68K_REG_PC  # noqa: E402

WALL = 48
FLOORS = [0, 4, 5, 6, 7, 8, 9, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]


def scenario(orig, n, frames, pool):
    rnd = random.Random(n)
    tiles = []
    for i in range(W * H):
        r = rnd.random()
        border = i % W in (0, W - 1) or i // W in (0, H - 1)
        # walls all around, like nearly all boxes on the edge of the
        # original levels: the original writes outside the map (into its
        # globals) when an enemy explodes on the edge; Rust ignores that
        tiles.append(WALL if border else 0 if r < 0.45 else WALL if r < 0.5 else rnd.choice(pool))
    floor = [rnd.choice(FLOORS) for _ in range(W * H)]
    # projectiles aimed at a box with a free box before it, so that the
    # rarer projectile handlers come into play
    aimed = []
    for _ in range(rnd.randrange(6)):
        tx, ty = rnd.randrange(3, W - 3), rnd.randrange(3, H - 3)
        dx, dy = rnd.choice([(-1, 0), (1, 0), (0, -1), (0, 1)])
        tiles[(ty + dy) * W + tx + dx] = 0
        aimed.append((tx + 2 * dx, ty + 2 * dy, -dx, -dy))
    # a computer with program chips around it, and the doors it switches
    if rnd.random() < 0.5:
        cx, cy = rnd.randrange(3, W - 3), rnd.randrange(3, H - 3)
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                tiles[(cy + dy) * W + cx + dx] = rnd.choice([0x110, 0x111, 0x112, 0])
        tiles[cy * W + cx] = 166
        dx, dy = rnd.choice([(-1, 0), (1, 0), (0, -1), (0, 1)])
        tiles[(cy + dy) * W + cx + dx] = 0
        aimed.append((cx + 2 * dx, cy + 2 * dy, -dx, -dy))
        for _ in range(4):
            tiles[rnd.randrange(1, H - 1) * W + rnd.randrange(1, W - 1)] = rnd.choice([0xE7, 0x56])
    px, py = rnd.randrange(1, W - 1), rnd.randrange(1, H - 1)
    switches = [rnd.random() < 0.5 for _ in range(5)]
    carried = rnd.choice([0, 0, 16, 17, 18, 19, 25])
    pulse = rnd.randrange(30)
    frame = rnd.randrange(10)
    orig.load_level(tiles, floor, (px, py))
    for i, s in enumerate(switches):
        orig.uc.mem_write(A5 - 0x53B8 + i, bytes([1 if s else 0]))
    orig.g32(-0x578C, carried)
    orig.g16(-0x5390, pulse)
    orig.g32(-0x6D08, frame)
    orig.seed = rnd.randrange(1, 0x7FFFFFFF)
    # projectiles: some active ones of all kinds, some waiting to be erased
    count = 0
    for slot in rnd.sample(range(SLOTS), rnd.randrange(0, 14)):
        if rnd.random() < 0.2:
            orig.set_projectile(slot, 0, 1, 0, 0, 0, 0, 0, 1)
            count += 1
            continue
        kind = rnd.choice([1, 1, 2, 3])
        x = rnd.randrange(1, W - 1) * 32 + 2 * rnd.randrange(16)
        y = rnd.randrange(1, H - 1) * 32 + 2 * rnd.randrange(16)
        if kind == 2:
            vx, vy = rnd.choice([-2, 2]), rnd.choice([-2, 2])
        else:
            vx, vy = rnd.choice([(-2, 0), (2, 0), (0, -2), (0, 2)])
        life = rnd.randrange(1, 400) if kind == 3 else 0
        orig.set_projectile(slot, 1, rnd.choice([0, 2, 3]), life, x, y, vx, vy, kind)
        count += 1
    free_slots = [i for i, p in enumerate(orig.projectiles()) if not p[0] and not p[1]]
    for slot, (bx, by, dx, dy) in zip(free_slots, aimed):
        kind = rnd.choice([1, 1, 2])
        if kind == 2:
            vx = 2 * (dx or rnd.choice([-1, 1]))
            vy = 2 * (dy or rnd.choice([-1, 1]))
        else:
            vx, vy = 2 * dx, 2 * dy
        orig.set_projectile(slot, 1, 0, 0, bx * 32 + 16, by * 32 + 16, vx, vy, kind)
        count += 1
    orig.g32(-0x57CA, count)
    # the visible part decides the drawing state of projectiles
    orig.g32(-0x57C6, rnd.randrange(W - 13))
    orig.g32(-0x57C2, rnd.randrange(H - 9))
    lines = [header(orig, (px, py))] + [f"0 {i} {t} {f} 0" for i, (t, f) in enumerate(zip(tiles, floor))]
    # earlier hits on reinforced walls (tile 218, 20 hits destroy them)
    for i, t in enumerate(tiles):
        if t == 218 and rnd.random() < 0.7:
            hits = rnd.randrange(rnd.choice([0, 15]), 20)
            orig.g16(-0x1AE8 + 2 * i, hits)
            lines.append(f"w {i} {hits}")
    lines += run_frames(orig, frames, (tiles, floor, [0] * (W * H)))
    return "\n".join(lines) + "\n"


def main():
    work, count, frames, out_dir = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    exclude = {int(t) for t in sys.argv[5:]}
    tables = json.load(open(f"{work}/handler_tables.json"))
    # every tile the original has a handler for (tick, projectiles, explosions)
    tiles = {int(t) for table in tables.values() for t in table}
    pool = sorted(t for t in tiles if t not in exclude)
    os.makedirs(out_dir, exist_ok=True)
    failed = []
    for n in range(count):
        # a fresh original per scenario, so no state leaks between them
        orig = Original(work)
        try:
            text = scenario(orig, n, frames, pool)
        except Exception as e:  # report and go on
            pc = orig.uc.reg_read(UC_M68K_REG_PC)
            seg, off = divmod(pc - SEG_BASE, SEG_SIZE)
            failed.append(f"scenario {n}: {e} at seg{seg}:{off:#x}, box ({orig.g32(-0x53B2)}, {orig.g32(-0x53AE)})")
            continue
        open(f"{out_dir}/fuzz_{n:04d}.txt", "w").write(text)
    print(f"{count - len(failed)} of {count} scenarios, pool of {len(pool)} tiles")
    for f in failed:
        print(f)


if __name__ == "__main__":
    main()
