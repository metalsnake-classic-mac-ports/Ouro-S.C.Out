"""Convert raw 8-bit pixel data with a Mac 'pltt' palette to PNG.

Usage: python3 topng.py <pixels.bin> <width> <pltt.bin> <out.png>
"""
import struct
import sys
import zlib


def read_pltt(data):
    count = struct.unpack(">H", data[:2])[0]
    colors = []
    for i in range(count):
        r, g, b = struct.unpack(">HHH", data[16 + i * 16:22 + i * 16])
        colors.append((r >> 8, g >> 8, b >> 8))
    return colors + [(255, 0, 255)] * (256 - len(colors))


def write_png(path, width, height, rgb_rows):
    def chunk(tag, payload):
        return struct.pack(">I", len(payload)) + tag + payload + struct.pack(">I", zlib.crc32(tag + payload))
    raw = b"".join(b"\x00" + row for row in rgb_rows)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 9)))
        f.write(chunk(b"IEND", b""))


def convert(pixels, width, palette, out):
    height = len(pixels) // width
    rows = []
    for y in range(height):
        line = pixels[y * width:(y + 1) * width]
        rows.append(b"".join(bytes(palette[p]) for p in line))
    write_png(out, width, height, rows)


if __name__ == "__main__":
    pixels = open(sys.argv[1], "rb").read()
    palette = read_pltt(open(sys.argv[3], "rb").read())
    convert(pixels, int(sys.argv[2]), palette, sys.argv[4])
