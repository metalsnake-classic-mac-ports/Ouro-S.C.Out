"""Run the original world automaton in a 68020 emulator and record the level.

Loads all CODE segments with a resolved jump table, rebuilds the THINK C
globals, fills the handler tables with the original init (CODE 5 0x1b54),
puts a level into the map and then runs frames like the original main loop
does for the world: frame counter, the 30-step sweep counter and the world
automaton (CODE 9 0x23fa). The player is parked outside the level so it does
not interact. Sound functions are replaced by stubs.

The player stands still on the start box (doors and rails react to that).

Usage: python3 emu_world.py <work dir> <level 1..101 | all> <frames> <out dir>
Writes <out dir>/world_LLL.txt: one line per change, "frame box tile floor skip"
(frame 0 lists the start state of all boxes), and "g frame no_death dying
level_end s0 s1 s2 s3 s4" when these globals change.
(needs unicorn; uses the translation tables of scout-core via loader_tables.json)
"""
import json
import struct
import sys

from unicorn import UC_ARCH_M68K, UC_HOOK_CODE, UC_HOOK_INTR, UC_MODE_BIG_ENDIAN, Uc
from unicorn.m68k_const import UC_CPU_M68K_M68020, UC_M68K_REG_A5, UC_M68K_REG_A7, UC_M68K_REG_PC

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from thinkc_globals import expand  # noqa: E402

A5 = 0x800000
SEG_BASE = 0x100000
SEG_SIZE = 0x10000
STACK = 0x700000
RETURN = 0x0F0000
STUB = 0x0F1000  # RTS
W, H = 40, 24

# Jump table entries replaced by stubs (sound).
STUBBED_ENTRIES = {601, 602}
# Toolbox Random (A861) calls used by the game logic: (segment, offset).
RANDOM_TRAPS = [(4, 0x2524), (9, 0x221E)]
SLOTS = 40
SLOT_BYTES = 0x30


class Original:
    def __init__(self, work):
        self.work = work
        uc = Uc(UC_ARCH_M68K, UC_MODE_BIG_ENDIAN, UC_CPU_M68K_M68020)
        self.uc = uc
        uc.mem_map(0, 0x1000)
        uc.mem_map(RETURN, 0x2000)
        uc.mem_write(STUB, b"\x4e\x75")
        uc.mem_map(STACK - 0x20000, 0x20000)
        uc.mem_map(A5 - 0x10000, 0x20000)
        for n in range(1, 11):
            uc.mem_map(SEG_BASE + n * SEG_SIZE, SEG_SIZE)
            code = open(f"{work}/code/seg{n:02d}.bin", "rb").read()
            uc.mem_write(SEG_BASE + n * SEG_SIZE, code)
        # loaded jump table: [segment word][JMP abs.l target]
        c0 = open(f"{work}/rsrc/CODE_0.bin", "rb").read()
        jt_len = struct.unpack(">I", c0[8:12])[0]
        for k in range(jt_len // 8):
            off, _, seg, _ = struct.unpack(">HHHH", c0[16 + k * 8:24 + k * 8])
            target = STUB if k in STUBBED_ENTRIES else SEG_BASE + seg * SEG_SIZE + off
            uc.mem_write(A5 + 32 + 8 * k, struct.pack(">HHI", seg, 0x4EF9, target))
        # globals
        data = open(f"{work}/rsrc/DATA_0.bin", "rb").read()
        zero = open(f"{work}/rsrc/ZERO_0.bin", "rb").read()
        g = bytearray(expand(data, zero))
        drel = open(f"{work}/rsrc/DREL_0.bin", "rb").read()
        for i in range(0, len(drel), 2):
            off = struct.unpack(">h", drel[i:i + 2])[0]
            at = len(g) + off
            v = struct.unpack(">I", g[at:at + 4])[0]
            g[at:at + 4] = struct.pack(">I", (v + A5) & 0xFFFFFFFF)
        uc.mem_write(A5 - len(g), bytes(g))
        uc.hook_add(UC_HOOK_INTR, self._on_trap)
        self.traps = []
        # Random: replace the trap by NOP and put the result on the stack,
        # like the Toolbox (Park-Miller, see scout-core world/random.rs)
        self.seed = 1
        for seg, off in RANDOM_TRAPS:
            addr = SEG_BASE + seg * SEG_SIZE + off
            assert uc.mem_read(addr, 2) == b"\xa8\x61"
            uc.mem_write(addr, b"\x4e\x71")
            uc.hook_add(UC_HOOK_CODE, self._random, begin=addr, end=addr)
        self.call(5, 0x1B54)  # fill the handler tables

    def _random(self, uc, addr, size, _):
        self.seed = self.seed * 16807 % 0x7FFFFFFF
        value = self.seed & 0xFFFF
        if value == 0x8000:
            value = 0
        uc.mem_write(uc.reg_read(UC_M68K_REG_A7), struct.pack(">H", value))

    def _on_trap(self, uc, number, _):
        pc = uc.reg_read(UC_M68K_REG_PC)
        self.traps.append((number, pc))
        raise RuntimeError(f"exception {number} at {pc:#x}")

    def call(self, seg, off, *args):
        uc = self.uc
        sp = STACK - 0x100
        frame = struct.pack(">I", RETURN) + b"".join(struct.pack(">i", a) for a in args)
        sp -= len(frame)
        uc.mem_write(sp, frame)
        uc.reg_write(UC_M68K_REG_A7, sp)
        uc.reg_write(UC_M68K_REG_A5, A5)
        uc.emu_start(SEG_BASE + seg * SEG_SIZE + off, RETURN, count=50_000_000)

    def g32(self, off, value=None):
        if value is None:
            return struct.unpack(">i", self.uc.mem_read(A5 + off, 4))[0]
        self.uc.mem_write(A5 + off, struct.pack(">i", value))

    def g16(self, off, value=None):
        if value is None:
            return struct.unpack(">h", self.uc.mem_read(A5 + off, 2))[0]
        self.uc.mem_write(A5 + off, struct.pack(">h", value))

    def load_level(self, tiles, floor, start):
        """tiles/floor: translated values, 960 each; start: (x, y)."""
        self.uc.mem_write(A5 - 0x6546, b"".join(struct.pack(">H", t) for t in tiles))
        self.uc.mem_write(A5 - 0x5DC6, bytes(floor))
        self.uc.mem_write(A5 - 0x5780, bytes(W * H))
        # the player stands on the start box
        for off, v in ((-0x579A, start[0]), (-0x5796, start[1]), (-0x53C0, start[0]), (-0x53BC, start[1])):
            self.g32(off, v)
        self.g32(-0x57C6, 0)
        self.g32(-0x57C2, 0)
        self.g32(-0x6D08, 0)
        self.g16(-0x5390, 0)
        # projectiles and the state around them
        self.uc.mem_write(A5 - 0x6CF6, bytes(SLOTS * SLOT_BYTES))
        self.uc.mem_write(A5 - 0x1AE8, bytes(2 * W * H))
        self.g32(-0x57CA, 0)
        self.g16(-0x3A42, -1)
        self.uc.mem_write(A5 - 0x6D1A, b"\x00")
        self.uc.mem_write(A5 - 0x184, b"\x01")
        self.uc.mem_write(A5 - 0x5392, b"\x00")
        self.g32(-0x5396, 0)
        self.g32(-0x3A46, 0)

    def frame(self):
        """One frame of the world, as in the main loop (CODE 9 0x0)."""
        n = self.g32(-0x6D04)
        counter = (self.g32(-0x6D08) + 1) % (960 // n)
        self.g32(-0x6D08, counter)
        if counter == 0:
            self.g16(-0x5390, (self.g16(-0x5390) + 1) % 30)
        self.call(9, 0x23FA)
        self.call(4, 0x2392)  # projectiles

    def tiles(self):
        return list(struct.unpack(">960h", self.uc.mem_read(A5 - 0x6546, 1920)))

    def floor(self):
        return list(self.uc.mem_read(A5 - 0x5DC6, 960))

    def skips(self):
        return list(self.uc.mem_read(A5 - 0x5780, 960))

    def flags(self):
        """Global state the world changes: no_death, dying, level_end,
        switches, projectile count, forced command, player hit, random seed."""
        switches = list(self.uc.mem_read(A5 - 0x53B8, 5))
        return (self.uc.mem_read(A5 - 0x5392, 1)[0], self.g32(-0x5396), self.g32(-0x3A46),
                *[1 if s else 0 for s in switches], self.g32(-0x57CA), self.g16(-0x3A42),
                1 if self.uc.mem_read(A5 - 0x6D1A, 1)[0] else 0, self.seed)

    def projectiles(self):
        """(active, draw, life, x, y, vx, vy, kind) per slot."""
        raw = self.uc.mem_read(A5 - 0x6CF6, SLOTS * SLOT_BYTES)
        out = []
        for i in range(SLOTS):
            b = raw[i * SLOT_BYTES:(i + 1) * SLOT_BYTES]
            active, draw, life, x, y, vx, vy, kind = struct.unpack(">BBhiiiii", bytes(b[:0x18]))
            out.append((1 if active else 0, draw, life, x, y, vx, vy, kind))
        return out

    def set_projectile(self, slot, active, draw, life, x, y, vx, vy, kind):
        self.uc.mem_write(A5 - 0x6CF6 + slot * SLOT_BYTES,
                          struct.pack(">BBhiiiii", active, draw, life, x, y, vx, vy, kind))


def level_from_disk(work, number):
    lv = open(f"{work}/rsrc/unpacked_SHAP_6642.bin", "rb").read()
    t = json.load(open(f"{work}/loader_tables.json"))
    chunk = lv[(number - 1) * 2880:number * 2880]
    file_tiles = struct.unpack("<960H", chunk[:1920])
    starts = {1, 2, 3, 10, 11, 12, 13, 20, 21, 22, 23, 30, 31, 32, 33}
    tiles = [0 if v in starts else t["tiles"].get(str(v), 255) for v in file_tiles]
    start = max(i for i, v in enumerate(file_tiles) if v in starts)
    # the loader puts 0x108 there; the start animation (CODE 5 ≈ 0x2ee0)
    # leaves the closed hatch 0x10b when the game begins
    tiles[start] = 0x10B
    floor = [t["floor"][str(v)] for v in chunk[1920:]]
    return tiles, floor, (start % W, start // W)


def record(work, level, frames, out_dir):
    orig = Original(work)
    tiles, floor, start = level_from_disk(work, level)
    orig.load_level(tiles, floor, start)
    orig.uc.mem_write(A5 - 0x53B4, b"\x01")  # switch 4 starts set (CODE 5 0x1a0a)
    lines = [header(orig, start)] + [f"0 {i} {t} {f} 0" for i, (t, f) in enumerate(zip(tiles, floor))]
    lines += run_frames(orig, frames, (tiles, floor, [0] * (W * H)))
    open(f"{out_dir}/world_{level:03d}.txt", "w").write("\n".join(lines) + "\n")
    return len(lines) - W * H


def header(orig, player):
    """Header line: player, switches, carried, pulse, frame, view, seed."""
    switches = [1 if s else 0 for s in orig.uc.mem_read(A5 - 0x53B8, 5)]
    return (f"# player {player[0]} {player[1]} switches {' '.join(map(str, switches))} "
            f"carried {orig.g32(-0x578C)} pulse {orig.g16(-0x5390)} frame {orig.g32(-0x6D08)} "
            f"view {orig.g32(-0x57C6)} {orig.g32(-0x57C2)} seed {orig.seed}")


def run_frames(orig, frames, state):
    """Run frames and return the change lines (boxes, flags, projectiles)."""
    lines = []
    flags = orig.flags()
    lines.append("g 0 " + " ".join(map(str, flags)))
    shots = orig.projectiles()
    lines += [f"p 0 {i} " + " ".join(map(str, p)) for i, p in enumerate(shots)]
    for n in range(1, frames + 1):
        orig.frame()
        new = (orig.tiles(), orig.floor(), orig.skips())
        for i in range(W * H):
            if any(a[i] != b[i] for a, b in zip(state, new)):
                lines.append(f"{n} {i} {new[0][i]} {new[1][i]} {new[2][i]}")
        state = new
        if orig.flags() != flags:
            flags = orig.flags()
            lines.append(f"g {n} " + " ".join(map(str, flags)))
        new_shots = orig.projectiles()
        lines += [f"p {n} {i} " + " ".join(map(str, p))
                  for i, (a, p) in enumerate(zip(shots, new_shots)) if a != p]
        shots = new_shots
    return lines


def main():
    import os
    work, which, frames, out_dir = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    os.makedirs(out_dir, exist_ok=True)
    levels = range(1, 102) if which == "all" else [int(which)]
    for level in levels:
        print(f"level {level}: {record(work, level, frames, out_dir)} changes")


if __name__ == "__main__":
    main()
