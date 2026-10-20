"""Run the original main loop (CODE 9 0x0) with scripted keys in a 68020
emulator and record what happens, frame by frame.

Builds on emu_world.Original. The real main loop runs: player (CODE 9 0xb70),
world, projectiles, view buffer, scrolling, teleporters, the magnifier and
the countdowns of the main loop. Replaced are only the parts that need the
Mac: the keyboard (jump table entry 550, CODE 6 0x7a8: re-implemented here
with the same rules, fed with one key per frame), sound and blitting (entries
601-607 and the drawing hook -0x1b00 become RTS) and the wait for the
vertical blank in the magnifier (CODE 4 0x1fe6). The emulation stops at the
top of the loop body (CODE 9 0x28) after every frame.

Usage: python3 emu_play.py <work dir> <level 1..101 | all> <frames> <out dir>
       python3 emu_play.py <work dir> fuzz <count> <frames> <out dir>
Writes <out dir>/play_LLL.txt (or fuzz_NNNN.txt for random levels full of
the tiles the player deals with) with the header line, the keys
("k frame key"), box changes ("frame box tile floor skip"), the world flags
("g ..."), projectile slots ("p ...") and the player state ("s frame ..."
when it changes, see PLAYER_STATE).
"""
import json
import random
import struct
import sys

from unicorn import UC_HOOK_CODE

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from emu_world import (  # noqa: E402
    A5, H, SEG_BASE, SEG_SIZE, STUB, W, Original, level_from_disk)

INPUT_STUB = STUB + 0x10
LOOP_BODY = SEG_BASE + 9 * SEG_SIZE + 0x28
# Keys the game knows, in the order the original tests them (CODE 6 0x7a8).
# "use" (command 7) takes, drops and teleports, "fire" (command 4) shoots.
KEYS = ["none", "right", "left", "up", "down", "self_destruct", "use", "fire"]
# Player state recorded per frame: (name, offset, size).
PLAYER_STATE = [
    ("x", -0x579A, 4), ("y", -0x5796, 4), ("view_x", -0x57C6, 4), ("view_y", -0x57C2, 4),
    ("px", -0x57BE, 4), ("py", -0x57BA, 4), ("remaining", -0x57A6, 4), ("dir", -0x57AA, 4),
    ("turret", -0x5786, 4), ("turret_locked", -0x5788, 1), ("speed", -0x57AE, 4),
    ("carried", -0x578C, 4), ("command", -0x5790, 4), ("target_x", -0x53C0, 4),
    ("target_y", -0x53BC, 4), ("on_rail", -0x539E, 1), ("rail_dir", -0x53A2, 4),
    ("cannon_fired", -0x5398, 1), ("accelerator", -0x538E, 2), ("magnifier", -0x538C, 2),
    ("exit", -0x6D0C, 4), ("self_destructed", -0x6CFC, 2), ("visible", -0x184, 1),
    ("lives", -0x6DAC, 2), ("clear_shots", -0x6D00, 2),
]
LIVES = 5


def start_view(x, y):
    """The view the loader picks for the start box (CODE 5, case 10 …)."""
    vx = x // 14 * 14
    if x == vx:
        vx -= 2
    if x - vx == 1:
        vx -= 1
    if x - vx == 12:
        vx += 1
    if x - vx == 13:
        vx += 2
    vy = y // 10 * 10
    if y - vy == 1:
        vy -= 1
    if y - vy == 8:
        vy += 1
    if y == vy:
        vy -= 2
    if y - vy == 9:
        vy += 2
    return max(0, min(vx, 26)), max(0, min(vy, 14))


class Player(Original):
    def __init__(self, work):
        super().__init__(work)
        uc = self.uc
        # keyboard, sound, blitting
        for k, target in [(550, INPUT_STUB)] + [(k, STUB) for k in range(601, 608)]:
            seg = struct.unpack(">H", uc.mem_read(A5 + 32 + 8 * k, 2))[0]
            uc.mem_write(A5 + 32 + 8 * k, struct.pack(">HHI", seg, 0x4EF9, target))
        uc.mem_write(INPUT_STUB, b"\x4e\x75")
        uc.hook_add(UC_HOOK_CODE, self._input, begin=INPUT_STUB, end=INPUT_STUB)
        # no wait for the vertical blank in the magnifier
        wait = SEG_BASE + 4 * SEG_SIZE + 0x1FE6
        assert uc.mem_read(wait, 2) == b"\x67\xfa"
        uc.mem_write(wait, b"\x4e\x71")
        uc.hook_add(UC_HOOK_CODE, self._loop_top, begin=LOOP_BODY, end=LOOP_BODY)
        self.key = "none"
        self.fire_held = self.use_held = False
        self.stop_at_top = False

    def _input(self, uc, addr, size, _):
        """CODE 6 0x7a8: the command from the first pressed key."""
        key = self.key
        command = {"right": 2, "left": 0, "up": 1, "down": 3, "self_destruct": 8}.get(key, -1)
        if key == "use":
            if not self.use_held:
                command = 7
            self.use_held = True
        else:
            self.use_held = False
        if key == "fire":
            if not self.fire_held:
                command = 4
            self.fire_held = True
        else:
            self.fire_held = False
        self.g32(-0x5790, command)

    def _loop_top(self, uc, addr, size, _):
        if self.stop_at_top:
            uc.emu_stop()
        self.stop_at_top = True

    def start(self, tiles, floor, start, carried=0, switches=(0,) * 5):
        """Put the level in place like the level start (CODE 5 ≈ 0x2c00) and
        enter the main loop; the start animation is over."""
        self.load_level(tiles, floor, start)
        self.uc.mem_write(A5 - 0x53B8, bytes(switches))
        x, y = start
        vx, vy = start_view(x, y)
        for off, v in [(-0x57C6, vx), (-0x57C2, vy), (-0x57A2, x - vx), (-0x579E, y - vy),
                       (-0x57BE, 32 * (x - vx)), (-0x57BA, 32 * (y - vy)),
                       (-0x57B6, 32 * (x - vx)), (-0x57B2, 32 * (y - vy)),
                       (-0x57AE, 4), (-0x57AA, 3), (-0x57A6, 0), (-0x578C, 0), (-0x5786, 3),
                       (-0x53A2, -1), (-0x5790, -1), (-0x6DC0, 0), (-0x6DBC, 1), (-0x6D0C, 0)]:
            self.g32(off, v)
        for off in (-0x5792, -0x5788, -0x539E, -0x5398, -0x5782):
            self.uc.mem_write(A5 + off, b"\x00")
        self.g32(-0x578C, carried)
        if 16 <= carried <= 19:
            self.uc.mem_write(A5 - 0x5788, b"\x01")
            self.g32(-0x5786, carried - 16)
        for off in (-0x538E, -0x538C, -0x6CFC, -0x6D00, -0x6CFE, -0x1154, -0x1156):
            self.g16(off, 0)
        self.g16(-0x6DAC, LIVES)
        self.g32(-0x1B00, STUB)
        # view buffers (CODE 5 ≈ 0x2c40)
        view = b"".join(struct.pack(">h", tiles[(vy + r) * W + vx + c])
                        for r in range(10) for c in range(14))
        self.uc.mem_write(A5 - 0x5A06, view)
        self.uc.mem_write(A5 - 0x58EE, view)
        self.g32(-0x57D6, A5 - 0x5A06)
        self.g32(-0x57D2, A5 - 0x58EE)
        self.stop_at_top = True
        self.running = True
        self.call(9, 0x0)  # runs up to the top of the loop body

    def play_frame(self, key):
        """One pass of the main loop; False when the loop has ended."""
        if not self.running:
            return False
        self.key = key
        self.stop_at_top = False
        uc = self.uc
        uc.emu_start(LOOP_BODY, 0x0F0000, count=50_000_000)
        self.running = uc.reg_read(__import__("unicorn").m68k_const.UC_M68K_REG_PC) == LOOP_BODY
        return self.running

    def player_state(self):
        out = []
        for _, off, size in PLAYER_STATE:
            raw = bytes(self.uc.mem_read(A5 + off, size))
            out.append(struct.unpack({1: ">b", 2: ">h", 4: ">i"}[size], raw)[0])
        return out


def random_keys(rnd, frames):
    """Held keys like a player: runs of the same key, often none."""
    keys = []
    while len(keys) < frames:
        key = rnd.choices(KEYS, weights=[6, 4, 4, 4, 4, 0.02, 1.5, 1.5])[0]
        keys += [key] * rnd.randrange(1, 40)
    return keys[:frames]


def record(orig, tiles, floor, start, keys, head, **state):
    """Play the keys; return the lines."""
    orig.start(tiles, floor, start, **state)
    lines = [head] + [f"0 {i} {t} {f} 0" for i, (t, f) in enumerate(zip(tiles, floor))]
    state = (orig.tiles(), orig.floor(), orig.skips())
    flags = orig.flags()
    lines.append("g 0 " + " ".join(map(str, flags)))
    shots = orig.projectiles()
    lines += [f"p 0 {i} " + " ".join(map(str, p)) for i, p in enumerate(shots)]
    player = orig.player_state()
    lines.append("s 0 " + " ".join(map(str, player)))
    for n, key in enumerate(keys, 1):
        lines.append(f"k {n} {key}")
        alive = orig.play_frame(key)
        new = (orig.tiles(), orig.floor(), orig.skips())
        lines += [f"{n} {i} {new[0][i]} {new[1][i]} {new[2][i]}"
                  for i in range(W * H) if any(a[i] != b[i] for a, b in zip(state, new))]
        state = new
        if orig.flags() != flags:
            flags = orig.flags()
            lines.append(f"g {n} " + " ".join(map(str, flags)))
        new_shots = orig.projectiles()
        lines += [f"p {n} {i} " + " ".join(map(str, p))
                  for i, (a, p) in enumerate(zip(shots, new_shots)) if a != p]
        shots = new_shots
        if orig.player_state() != player:
            player = orig.player_state()
            lines.append(f"s {n} " + " ".join(map(str, player)))
        if not alive:
            lines.append(f"e {n}")
            break
    return lines


WALL = 48
FLOORS = [0, 4, 5, 6, 7, 8, 9, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]
# Tiles the player code handles (CODE 9 0xb70 and what it calls).
PLAYER_TILES = (list(range(1, 40)) + [16, 17, 18, 19] * 3 + list(range(0x101, 0x10F))
                + [0xFF, 0x100, 0x110, 0x111, 0x112, 0x114, 0xE8, 0xE7, 0x56]
                + list(range(0x63, 0x68)) + list(range(0x8C, 0xAB)) + list(range(0xEF, 0xFF))
                + list(range(0xDF, 0xE3)) + [0xB6, 0xB7, 0xB8, 0xB9] + [0xB3, 0x124, 0x150]
                + [292, 303, 314, 325, 400, 415, 430, 445, 179, 33, 32, 166])


def fuzz_scenario(work, n, frames, pool):
    rnd = random.Random(n)
    px, py = rnd.randrange(2, W - 2), rnd.randrange(2, H - 2)
    tiles = []
    for i in range(W * H):
        x, y = i % W, i // W
        if x in (0, W - 1) or y in (0, H - 1):
            tiles.append(WALL)
            continue
        near = abs(x - px) <= 3 and abs(y - py) <= 3
        r = rnd.random()
        if r < (0.35 if near else 0.5):
            tiles.append(0)
        elif r < 0.55:
            tiles.append(WALL if rnd.random() < 0.3 else rnd.choice(pool))
        else:
            tiles.append(rnd.choice(PLAYER_TILES))
    tiles[py * W + px] = rnd.choice([0, 0] + PLAYER_TILES)
    floor = [rnd.choice(FLOORS) for _ in range(W * H)]
    carried = rnd.choice([0, 0, 0, 16, 17, 18, 19, 25, 30, 31, 36, 37, 38, 33])
    switches = tuple(rnd.randrange(2) for _ in range(5))
    keys = random_keys(rnd, frames)
    orig = Player(work)
    orig.seed = rnd.randrange(1, 0x7FFFFFFF)
    head = (f"# fuzz {n} start {px} {py} carried {carried} switches "
            f"{' '.join(map(str, switches))} seed {orig.seed}")
    return record(orig, tiles, floor, (px, py), keys, head, carried=carried, switches=switches)


def main():
    import os
    if sys.argv[2] == "fuzz":
        work, count, frames, out_dir = sys.argv[1], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
        os.makedirs(out_dir, exist_ok=True)
        tables = json.load(open(f"{work}/handler_tables.json"))
        pool = sorted({int(t) for table in tables.values() for t in table})
        failed = []
        for n in range(count):
            try:
                lines = fuzz_scenario(work, n, frames, pool)
            except Exception as e:  # report and go on
                failed.append(f"scenario {n}: {e}")
                continue
            open(f"{out_dir}/fuzz_{n:04d}.txt", "w").write("\n".join(lines) + "\n")
        print(f"{count - len(failed)} of {count} scenarios")
        for f in failed:
            print(f)
        return
    work, which, frames, out_dir = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    os.makedirs(out_dir, exist_ok=True)
    levels = range(1, 102) if which == "all" else [int(which)]
    for level in levels:
        orig = Player(work)
        tiles, floor, start = level_from_disk(work, level)
        keys = random_keys(random.Random(level), frames)
        head = f"# level {level} start {start[0]} {start[1]}"
        lines = record(orig, tiles, floor, start, keys, head)
        open(f"{out_dir}/play_{level:03d}.txt", "w").write("\n".join(lines) + "\n")
        end = [line for line in lines if line.startswith("e ")]
        print(f"level {level}: {len(lines)} lines{', ends at frame ' + end[0][2:] if end else ''}")


if __name__ == "__main__":
    main()
