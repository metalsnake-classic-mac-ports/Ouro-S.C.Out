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


def read_png(path):
    """Read an 8-bit RGB, RGBA or indexed PNG (non-interlaced).
    Returns (width, height, rows of (r, g, b))."""
    data = open(path, "rb").read()
    pos, idat = 8, b""
    while pos < len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        tag, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        if tag == b"IHDR":
            width, height, depth, color_type, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or color_type not in (2, 3, 6) or interlace:
                raise ValueError("only 8-bit RGB/RGBA/indexed, non-interlaced PNG supported")
        elif tag == b"PLTE":
            plte = [tuple(body[i:i + 3]) for i in range(0, len(body), 3)]
        elif tag == b"IDAT":
            idat += body
        pos += 12 + length
    bpp = {2: 3, 3: 1, 6: 4}[color_type]
    raw = zlib.decompress(idat)
    stride = width * bpp
    prev = bytearray(stride)
    rows = []
    for y in range(height):
        ftype = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if ftype == 1:
                line[i] = (line[i] + a) & 0xFF
            elif ftype == 2:
                line[i] = (line[i] + b) & 0xFF
            elif ftype == 3:
                line[i] = (line[i] + (a + b) // 2) & 0xFF
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 0xFF
        if color_type == 3:
            rows.append([plte[v] for v in line])
        else:
            rows.append([tuple(line[x * bpp:x * bpp + 3]) for x in range(width)])
        prev = line
    return width, height, rows


if __name__ == "__main__":
    pixels = open(sys.argv[1], "rb").read()
    palette = read_pltt(open(sys.argv[3], "rb").read())
    convert(pixels, int(sys.argv[2]), palette, sys.argv[4])
