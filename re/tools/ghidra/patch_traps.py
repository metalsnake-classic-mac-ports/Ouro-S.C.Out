"""Replace Mac Toolbox A-line traps with TRAP #15 for Ghidra.

Ghidra's 68000 language has no A-line traps and stops decoding at them. A
linear capstone sweep finds the trap words at instruction boundaries; they
are replaced by TRAP #15 (0x4E4F) in a copy of each segment, and the original
trap words are listed per segment (offset and trap word) so the decompiled
code can be read with the real Toolbox calls.

Usage: python3 patch_traps.py <work dir>   (needs capstone)
"""
import glob
import os
import sys

from capstone import CS_ARCH_M68K, CS_MODE_BIG_ENDIAN, CS_MODE_M68K_030, Cs

work = sys.argv[1]
md = Cs(CS_ARCH_M68K, CS_MODE_BIG_ENDIAN | CS_MODE_M68K_030)
md.skipdata = True
os.makedirs(f"{work}/code_patched", exist_ok=True)
for path in sorted(glob.glob(f"{work}/code/seg*.bin")):
    data = bytearray(open(path, "rb").read())
    traps = []
    for ins in md.disasm(bytes(data), 0):
        if ins.size == 2 and ins.bytes[0] & 0xF0 == 0xA0:
            traps.append((ins.address, int.from_bytes(ins.bytes, "big")))
            data[ins.address:ins.address + 2] = b"\x4e\x4f"
    name = os.path.basename(path)
    open(f"{work}/code_patched/{name}", "wb").write(data)
    with open(f"{work}/code_patched/{name[:-4]}.traps.txt", "w") as f:
        for offset, word in traps:
            f.write(f"{offset:06x} {word:04X}\n")
    print(name, len(traps), "traps")
