"""Find calls to a function of the original game across all CODE segments.

Calls between segments go through the jump table: JSR d16(A5) with
d16 = 32 + 8 * entry + 2. Calls inside a segment use JSR/BSR d16(PC).

Usage: python3 xref.py <work dir> <segment> <offset hex>
"""
import glob
import struct
import sys

work, seg, target = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 16)
c0 = open(f"{work}/rsrc/CODE_0.bin", "rb").read()
jt_len = struct.unpack(">I", c0[8:12])[0]
entries = [k for k in range(jt_len // 8)
           if struct.unpack(">HHHH", c0[16 + k * 8:24 + k * 8])[0::2] == (target, seg)]
print(f"seg{seg:02d}:{target:#x} jump table entries: {entries}")
for path in sorted(glob.glob(f"{work}/code/seg*.bin")):
    n = int(path[-6:-4])
    d = open(path, "rb").read()
    for k in entries:
        pat = b"\x4e\xad" + struct.pack(">h", 32 + 8 * k + 2)
        i = d.find(pat)
        while i != -1:
            print(f"  seg{n:02d}:{i:#06x}  jsr via jump table")
            i = d.find(pat, i + 2)
    if n == seg:
        for i in range(0, len(d) - 3, 2):
            op = d[i:i + 2]
            disp = struct.unpack(">h", d[i + 2:i + 4])[0]
            if op in (b"\x4e\xba", b"\x61\x00") and i + 2 + disp == target:
                kind = "jsr" if op == b"\x4e\xba" else "bsr"
                print(f"  seg{n:02d}:{i:#06x}  {kind} (pc)")
