"""Render a raw 8-bit tile resource (32x32 tiles, stored one after another) as a sheet.

Usage: python3 tiles.py <tiles.bin> <pltt.bin> <out.png> [tiles_per_row] [tile_h]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from topng import read_pltt, write_png

data = open(sys.argv[1], "rb").read()
pal = read_pltt(open(sys.argv[2], "rb").read())
per_row = int(sys.argv[4]) if len(sys.argv) > 4 else 16
th = int(sys.argv[5]) if len(sys.argv) > 5 else 32
tw = 32
count = len(data) // (tw * th)
rows_of_tiles = (count + per_row - 1) // per_row
gap = 2
width = per_row * (tw + gap)
out = []
for ty in range(rows_of_tiles):
    for y in range(th):
        line = b""
        for tx in range(per_row):
            k = ty * per_row + tx
            if k < count:
                base = k * tw * th + y * tw
                line += b"".join(bytes(pal[p]) for p in data[base:base + tw])
            else:
                line += bytes((40, 40, 40)) * tw
            line += bytes((255, 255, 255)) * gap
        out.append(line)
    out.extend([bytes((255, 255, 255)) * width] * gap)
write_png(sys.argv[3], width, len(out), out)
