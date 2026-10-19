"""Decompress the 'SHAP' ("secret stuff") resources of S.C.OUT.

They use LZRW1-A by Ross N. Williams (public domain, 1991); the original
DATA resource contains the string "LZRW1-A 1.0 22-Jun-1991 Public Domain".

Resource layout:
    4 bytes  big-endian unpacked size
    4 bytes  LZRW1-A flag: 0 = compressed, 1 = stored (copy)
    ...      LZRW1-A stream: a 16-bit control word (little-endian, LSB first)
             before every 16 items. Bit 0 = literal byte, bit 1 = copy item
             of 2 bytes: high nibble of byte 0 and byte 1 = offset (12 bits),
             low nibble of byte 0 = length - 3.
"""
import struct


def decompress(resource):
    size, flag = struct.unpack(">II", resource[:8])
    src = resource[8:]
    if flag == 1:
        out = bytes(src)
    elif flag == 0:
        out = bytearray()
        i = 0
        control = 0
        bits = 0
        while i < len(src):
            if bits == 0:
                control = src[i] | (src[i + 1] << 8)
                i += 2
                bits = 16
            if control & 1:
                offset = ((src[i] & 0xF0) << 4) | src[i + 1]
                length = (src[i] & 0x0F) + 3
                i += 2
                start = len(out) - offset
                for k in range(length):
                    out.append(out[start + k])
            else:
                out.append(src[i])
                i += 1
            control >>= 1
            bits -= 1
    else:
        raise ValueError(f"unknown LZRW1-A flag {flag}")
    if len(out) != size:
        raise ValueError(f"unpacked {len(out)} bytes, header says {size}")
    return bytes(out)


if __name__ == "__main__":
    import sys
    data = decompress(open(sys.argv[1], "rb").read())
    open(sys.argv[2], "wb").write(data)
