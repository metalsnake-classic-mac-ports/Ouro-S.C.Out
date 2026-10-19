"""Read classic Mac resource forks from a MacBinary file.

Usage:
    python3 macres.py list <file.bin>
    python3 macres.py dump <file.bin> <outdir>     # writes TYPE_ID.bin per resource
"""
import os
import struct
import sys


def resource_fork(path):
    data = open(path, "rb").read()
    data_len, rsrc_len = struct.unpack(">II", data[83:91])
    start = 128 + ((data_len + 127) // 128) * 128
    return data[start:start + rsrc_len]


def parse(fork):
    """Return a list of (type, id, name, data)."""
    data_off, map_off, _, map_len = struct.unpack(">IIII", fork[:16])
    rmap = fork[map_off:map_off + map_len]
    type_list_off, name_list_off = struct.unpack(">HH", rmap[24:28])
    type_list = rmap[type_list_off:]
    type_count = struct.unpack(">H", type_list[:2])[0] + 1
    resources = []
    for i in range(type_count):
        rtype, count, ref_off = struct.unpack(">4sHH", type_list[2 + i * 8:10 + i * 8])
        for j in range(count + 1):
            entry = type_list[ref_off + j * 12:ref_off + j * 12 + 8]
            rid, name_off, attr_and_off = struct.unpack(">hHI", entry)
            off = data_off + (attr_and_off & 0xFFFFFF)
            length = struct.unpack(">I", fork[off:off + 4])[0]
            name = ""
            if name_off != 0xFFFF:
                p = name_list_off + name_off
                name = rmap[p + 1:p + 1 + rmap[p]].decode("mac_roman")
            resources.append((rtype.decode("mac_roman"), rid, name, fork[off + 4:off + 4 + length]))
    return resources


def safe(rtype):
    return "".join(c if c.isalnum() else "_" for c in rtype)


def main():
    cmd, path = sys.argv[1], sys.argv[2]
    resources = parse(resource_fork(path))
    if cmd == "list":
        for rtype, rid, name, data in sorted(resources, key=lambda r: (r[0], r[1])):
            print(f"{rtype!r:8} {rid:6} {len(data):8}  {name}")
    elif cmd == "dump":
        out = sys.argv[3]
        os.makedirs(out, exist_ok=True)
        for rtype, rid, name, data in resources:
            with open(os.path.join(out, f"{safe(rtype)}_{rid}.bin"), "wb") as f:
                f.write(data)


if __name__ == "__main__":
    main()
