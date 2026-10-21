"""Run the original music player (CODE 10) in a 68020 emulator and record
its output: the reference for the rewrite's player.

CODE 10 0x0 mixes one tick of a ProTracker module into 279 8-bit samples
at 14161.98 Hz (the rate of the double buffer CODE 10 0x5e86 sets up):
called with mode 0 it starts the song at a position, with mode 1 it goes
on. It uses no Toolbox calls and keeps its state inside the segment.

Usage: python3 emu_music.py <work dir> <seconds> <out.raw> [out.wav] [start position]
Writes the raw samples (and a WAV); the module is 'SHAP' 200 as the game
loads it (uncompressed).
(needs unicorn)
"""
import struct
import sys

from unicorn import UC_ARCH_M68K, UC_HOOK_INTR, UC_MODE_BIG_ENDIAN, Uc
from unicorn.m68k_const import UC_CPU_M68K_M68020, UC_M68K_REG_A7, UC_M68K_REG_PC

CODE = 0x100000
MOD = 0x200000
OUT = 0x300000
STACK = 0x400000
RETURN = 0x0F0000
TICK_SAMPLES = 279
RATE = 14161.98


def main():
    work, seconds, raw_path = sys.argv[1], float(sys.argv[2]), sys.argv[3]
    wav_path = sys.argv[4] if len(sys.argv) > 4 else None
    start = int(sys.argv[5]) if len(sys.argv) > 5 else 0
    code = open(f"{work}/code/seg10.bin", "rb").read()
    module = open(f"{work}/rsrc/SHAP_200.bin", "rb").read()
    uc = Uc(UC_ARCH_M68K, UC_MODE_BIG_ENDIAN, UC_CPU_M68K_M68020)
    for base, size in ((0, 0x1000), (RETURN, 0x1000), (CODE, 0x10000), (MOD, 0x20000),
                       (OUT, 0x1000), (STACK - 0x10000, 0x10000)):
        uc.mem_map(base, size)
    uc.mem_write(CODE, code)
    uc.mem_write(MOD, module)

    def trap(uc, number, _):
        raise RuntimeError(f"exception {number} at {uc.reg_read(UC_M68K_REG_PC):#x}")
    uc.hook_add(UC_HOOK_INTR, trap)

    def tick(mode):
        # Pascal call: out (long), mode (word), position (word), module (long)
        sp = STACK - 0x100
        frame = struct.pack(">IIhhI", RETURN, OUT, mode, start if mode == 0 else 0, MOD)
        sp -= len(frame)
        uc.mem_write(sp, frame)
        uc.reg_write(UC_M68K_REG_A7, sp)
        uc.emu_start(CODE, RETURN, count=5_000_000)
        return bytes(uc.mem_read(OUT, TICK_SAMPLES))

    ticks = int(seconds * RATE / TICK_SAMPLES)
    out = bytearray()
    for n in range(ticks):
        out += tick(0 if n == 0 else 1)
    open(raw_path, "wb").write(out)
    if wav_path:
        rate = round(RATE)
        header = (b"RIFF" + struct.pack("<I", 36 + len(out)) + b"WAVEfmt "
                  + struct.pack("<IHHIIHH", 16, 1, 1, rate, rate, 1, 8)
                  + b"data" + struct.pack("<I", len(out)))
        open(wav_path, "wb").write(header + out)
    print(f"{ticks} ticks, {len(out)} samples, min {min(out)} max {max(out)}")


if __name__ == "__main__":
    main()
