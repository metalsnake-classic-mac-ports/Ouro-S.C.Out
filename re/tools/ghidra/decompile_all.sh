#!/bin/sh
# Decompile all CODE segments of the original into re/work/decomp/segNN.c.
# Input: re/work/code_patched/segNN.bin (segment without its 4-byte header,
# A-line traps replaced by TRAP #15, see patch_traps.py). Function starts are
# seeded from the jump table in CODE 0 (re/work/rsrc/CODE_0.bin).
set -e
W="$(cd "$(dirname "$0")/../.." && pwd)/work"
python3 -I - "$W" <<'PY'
import struct, sys, os
w = sys.argv[1]
c0 = open(f"{w}/rsrc/CODE_0.bin", "rb").read()
jtlen = struct.unpack(">I", c0[8:12])[0]
seeds = {}
for k in range(jtlen // 8):
    off, _, seg, _ = struct.unpack(">HHHH", c0[16 + k * 8:24 + k * 8])
    seeds.setdefault(seg, set()).add(off)
os.makedirs(f"{w}/seeds", exist_ok=True)
for seg, offs in seeds.items():
    open(f"{w}/seeds/seg{seg:02d}.txt", "w").write("\n".join(f"{o:x}" for o in sorted(offs)) + "\n")
PY
for f in "$W"/code_patched/seg*.bin; do
    n=$(basename "$f" .bin)
    /opt/homebrew/opt/ghidra/libexec/support/analyzeHeadless "$W/ghidra-proj" scout -import "$f" -overwrite \
        -processor 68000:BE:32:MC68020 -loader BinaryLoader \
        -scriptPath "$(dirname "$0")" \
        -postScript CreateFuncs.java "$W/seeds/$n.txt" \
        -postScript DecompileAll.java "$W/decomp/$n.c" 2>&1 | grep 'created functions' | sed "s/^/$n: /"
done
