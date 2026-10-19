"""Run the original level loader (CODE 5, 0x48c) in a 68020 emulator to get
its tile and floor translation tables.

The level files use the numbering of the MS-DOS version. While loading a
level the Mac version translates every tile value and every floor value to
an index into its own graphics. Instead of reading the long switch statements,
we feed the function a level whose boxes contain 0, 1, 2, ... and read back
what it wrote.

Usage: python3 emulate_level_loader.py <seg05.bin> <out.json>
(seg05.bin = CODE 5 without its 4-byte segment header; needs `unicorn`)
"""
import json
import struct
import sys

from unicorn import UC_ARCH_M68K, UC_HOOK_INTR, UC_MODE_BIG_ENDIAN, Uc
from unicorn.m68k_const import UC_CPU_M68K_M68020, UC_M68K_REG_A5, UC_M68K_REG_A7, UC_M68K_REG_PC

LOADER = 0x48C
CODE_BASE = 0x100000
A5 = 0x800000
STACK = 0x600000
DATA = 0x400000  # level data
HANDLE = 0x480000
RETURN = 0x200000
CELLS = 960


def run(code, tiles, floor):
    uc = Uc(UC_ARCH_M68K, UC_MODE_BIG_ENDIAN, UC_CPU_M68K_M68020)
    uc.mem_map(0x000000, 0x1000)  # exception vectors
    uc.mem_map(CODE_BASE, 0x10000)
    uc.mem_map(RETURN, 0x1000)
    uc.mem_map(DATA, 0x100000)
    uc.mem_map(STACK - 0x10000, 0x10000)
    uc.mem_map(A5 - 0x10000, 0x20000)
    uc.mem_write(CODE_BASE, code)

    level = b"".join(struct.pack("<H", t) for t in tiles) + bytes(floor)
    uc.mem_write(DATA, level)
    uc.mem_write(HANDLE, struct.pack(">I", DATA))
    uc.mem_write(A5 - 0x128C, struct.pack(">I", HANDLE))  # handle to the level data
    uc.mem_write(A5 - 0x6DAE, struct.pack(">h", 0))  # level number

    sp = STACK - 0x100
    uc.mem_write(sp, struct.pack(">I", RETURN))
    uc.reg_write(UC_M68K_REG_A7, sp)
    uc.reg_write(UC_M68K_REG_A5, A5)

    def on_interrupt(uc, number, _):
        pc = uc.reg_read(UC_M68K_REG_PC)
        raise RuntimeError(f"exception {number} at {pc - CODE_BASE:#x} (Toolbox call?)")

    uc.hook_add(UC_HOOK_INTR, on_interrupt)
    uc.emu_start(CODE_BASE + LOADER, RETURN, count=5_000_000)
    out_tiles = struct.unpack(f">{CELLS}H", uc.mem_read(A5 - 0x481A, 2 * CELLS))
    out_floor = list(uc.mem_read(A5 - 0x4BDA, CELLS))
    return out_tiles, out_floor


def main():
    code = open(sys.argv[1], "rb").read()
    tiles_in = list(range(CELLS))
    floor_in = [i % 256 for i in range(CELLS)]
    tiles_out, floor_out = run(code, tiles_in, floor_in)
    result = {
        "tiles": {str(v): t for v, t in zip(tiles_in, tiles_out)},
        "floor": {str(v): f for v, f in zip(floor_in[:256], floor_out[:256])},
    }
    json.dump(result, open(sys.argv[2], "w"), indent=1)
    changed = sum(1 for v, t in zip(tiles_in, tiles_out) if v != t)
    print(f"tiles: {changed} of {CELLS} values changed; floor: {sum(1 for i in range(256) if floor_out[i] != i)} of 256 changed")


if __name__ == "__main__":
    main()
