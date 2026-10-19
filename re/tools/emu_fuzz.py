"""Differential test scenarios for the world automaton.

Builds random levels from the tick handler tiles (plus walls and empty
boxes), a random player position and random global flags, runs them in the
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
from emu_world import A5, H, SEG_BASE, SEG_SIZE, W, Original  # noqa: E402
from unicorn.m68k_const import UC_M68K_REG_PC  # noqa: E402

WALL = 48
FLOORS = [0, 4, 5, 6, 7, 8, 9, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]


def scenario(orig, n, frames, pool):
    rnd = random.Random(n)
    tiles = []
    for _ in range(W * H):
        r = rnd.random()
        tiles.append(0 if r < 0.45 else WALL if r < 0.5 else rnd.choice(pool))
    floor = [rnd.choice(FLOORS) for _ in range(W * H)]
    px, py = rnd.randrange(W), rnd.randrange(H)
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
    head = f"# player {px} {py} switches {' '.join(str(int(s)) for s in switches)} " \
           f"carried {carried} pulse {pulse} frame {frame}"
    lines = [head] + [f"0 {i} {t} {f} 0" for i, (t, f) in enumerate(zip(tiles, floor))]
    state = (tiles, floor, [0] * (W * H))
    for k in range(1, frames + 1):
        orig.frame()
        new = (orig.tiles(), orig.floor(), orig.skips())
        for i in range(W * H):
            if any(a[i] != b[i] for a, b in zip(state, new)):
                lines.append(f"{k} {i} {new[0][i]} {new[1][i]} {new[2][i]}")
        state = new
    return "\n".join(lines) + "\n"


def main():
    work, count, frames, out_dir = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    exclude = {int(t) for t in sys.argv[5:]}
    tick = json.load(open(f"{work}/handler_tables.json"))["tick"]
    pool = sorted(int(t) for t in tick if int(t) not in exclude)
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
