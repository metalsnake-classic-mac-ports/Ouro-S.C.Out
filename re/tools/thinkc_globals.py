"""Reconstruct the initialized globals of a THINK C application.

The THINK C startup code (CODE 1) copies the 'DATA' resource word by word to
the area just below A5; after every copied zero word it inserts as many zero
bytes as the next word of the 'ZERO' resource says, until it reaches A5.
'DREL' lists A5 offsets of pointers that are then made absolute.

Usage: python3 thinkc_globals.py <rsrc dir> [A5 offset hex ...]
       prints the 32-bit value at each A5 offset (e.g. -6d04)
"""
import struct
import sys


def expand(data, zero):
    out = bytearray()
    zi = 0
    for i in range(0, len(data) - 1, 2):
        word = data[i:i + 2]
        out += word
        if word == b"\x00\x00":
            count = struct.unpack(">H", zero[zi:zi + 2])[0]
            zi += 2
            out += bytes(count)
    return bytes(out)


def main():
    rsrc = sys.argv[1]
    data = open(f"{rsrc}/DATA_0.bin", "rb").read()
    zero = open(f"{rsrc}/ZERO_0.bin", "rb").read()
    g = expand(data, zero)
    print(f"globals: {len(g)} bytes, A5-{len(g):#x} .. A5")
    for arg in sys.argv[2:]:
        off = -int(arg.lstrip("-"), 16)
        at = len(g) + off
        print(f"A5{off:+#x}: long {struct.unpack('>i', g[at:at + 4])[0]}  word {struct.unpack('>h', g[at:at + 2])[0]}")


if __name__ == "__main__":
    main()
