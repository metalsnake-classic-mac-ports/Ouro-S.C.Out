"""Decode the 8-bit PackBits pixmap of a Mac PICT v2 resource to PNG.

Only handles what S.C.OUT uses: one PackBitsRect/PackBitsRgn (0x0098/0x0099)
or DirectBits-free indexed pixmap with an embedded color table.

Usage: python3 pict.py <PICT.bin> <out.png>
"""
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from topng import write_png


def unpack_bits(src, length):
    out = bytearray()
    i = 0
    while len(out) < length:
        n = src[i]
        i += 1
        if n < 128:
            out += src[i:i + n + 1]
            i += n + 1
        elif n > 128:
            out += bytes([src[i]]) * (257 - n)
            i += 1
    return bytes(out[:length]), i


def decode(p):
    for m in re.finditer(rb"\x00[\x98-\x9b]", p):
        op = m.start()
        pm = op + 2 + (4 if p[op + 1] in (0x9A, 0x9B) else 0)
        row_bytes = struct.unpack(">H", p[pm:pm + 2])[0]
        if not row_bytes & 0x8000 or pm + 54 > len(p):
            continue
        pixel_size = struct.unpack(">H", p[pm + 28:pm + 30])[0]
        ct = pm + 46
        _, flags, size = struct.unpack(">IHH", p[ct:ct + 8])
        if pixel_size != 8 or size > 255:
            continue
        break
    else:
        raise ValueError("no 8-bit pixmap found")
    row_bytes &= 0x3FFF
    top, left, bottom, right = struct.unpack(">hhhh", p[pm + 2:pm + 10])
    width, height = right - left, bottom - top
    palette = [(0, 0, 0)] * 256
    for k in range(size + 1):
        value, r, g, b = struct.unpack(">HHHH", p[ct + 8 + k * 8:ct + 16 + k * 8])
        palette[k if flags & 0x8000 else value] = (r >> 8, g >> 8, b >> 8)
    pos = ct + 8 + (size + 1) * 8 + 8 + 8 + 2  # srcRect, dstRect, mode
    if p[op + 1] in (0x99, 0x9B):
        pos += struct.unpack(">H", p[pos:pos + 2])[0]  # mask region
    rows = []
    for _ in range(height):
        if row_bytes > 250:
            count = struct.unpack(">H", p[pos:pos + 2])[0]
            pos += 2
        else:
            count = p[pos]
            pos += 1
        line, _ = unpack_bits(p[pos:pos + count], row_bytes)
        pos += count
        rows.append(line[:width])
    return width, height, rows, palette


if __name__ == "__main__":
    w, h, rows, pal = decode(open(sys.argv[1], "rb").read())
    write_png(sys.argv[2], w, h, [b"".join(bytes(pal[i]) for i in row) for row in rows])
    print(w, h)
