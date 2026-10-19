"""Write the decompiled handlers of one table in a compact, normalized form.

X/Y = the handler's box (-0x53b2/-0x53ae, explosions -0x6d22/-0x6d1e), set(x, y, tile, skip) =
CODE 9 0x2046, free(x, y) = CODE 9 0x1f94, G[-0x...] = A5 globals.

Usage: python3 compact_handlers.py <work dir> <table name> <out file>
"""
import json
import re
import sys

work, name, out = sys.argv[1:4]
tables = json.load(open(f"{work}/handler_tables.json"))
src = {s: open(f"{work}/decomp/seg{s:02d}.c").read() for s in (4, 9)}


def body(seg, off):
    m = re.search(r"// ===== \S+ @ %08x\n(.*?)(?=// ===== |\Z)" % off, src[seg], re.S)
    return m.group(1) if m else ""


def compact(text):
    text = text[text.index("{") + 1:text.rindex("}")]
    text = re.sub(r"\s+", " ", text)
    # the box: -0x53b2/-0x53ae (tick handlers), -0x6d22/-0x6d1e (explosions)
    text = re.sub(r"\*\((?:undefined4|int) \*\)\(unaff_A5 \+ -0x(?:53b2|6d22)\)", "X", text)
    text = re.sub(r"\*\((?:undefined4|int) \*\)\(unaff_A5 \+ -0x(?:53ae|6d1e)\)", "Y", text)
    text = re.sub(r"\*\((\w+) \*\)\(unaff_A5 \+ (-?0x[0-9a-f]+)\)", r"G[\2]", text)

    def setcall(m):
        v = int(m.group(3), 16)
        return f"set({m.group(1)},{m.group(2)},{v >> 16},{(v >> 8) & 0xff})"

    # calls through the jump table: A5+0x8c2 = CODE 9 0x2046 (set),
    # A5+0x8b2 = CODE 9 0x1f94 (free), A5+0x3aa = CODE 4 0x0 (explode)
    text = re.sub(r"\(\*\*\(code \*\*\)\(unaff_A5 \+ 0x8c2\)\)\s*", "func_0x00002046", text)
    text = re.sub(r"\(\*\*\(code \*\*\)\(unaff_A5 \+ 0x8b2\)\)\s*", "FUN_00001f94", text)
    text = re.sub(r"\(\*\*\(code \*\*\)\(unaff_A5 \+ 0x3aa\)\)\s*", "explode", text)
    # projectile handlers: the current projectile is G[-0x53aa] (index), its
    # fields are at -0x6cf6 + 0x30 * index
    text = re.sub(r"\*\((\w+) \*\)\(unaff_A5 \+ (?:G\[-0x53aa\]|\*\(int \*\)\(unaff_A5 \+ -0x53aa\)) \* 0x30 \+ (-0x[0-9a-f]+)\)", r"P[\2]", text)
    text = re.sub(r"\*\((\w+) \*\)\(unaff_A5 \+ (-0x[0-9a-f]+) \+ (?:G\[-0x53aa\]|\*\(int \*\)\(unaff_A5 \+ -0x53aa\)) \* 0x30\)", r"P[\2]", text)
    text = re.sub(r"func_0x00002046\(([^,]+),([^,]+),\s*0x([0-9a-f]+)\s*\)", setcall, text)
    text = re.sub(r"FUN_00001f94\(([^,]+),([^)]+)\)", r"free(\1,\2)", text)
    text = re.sub(r"\b(int|char|short|undefined\d?|code|byte|uint|ushort|bool) \w+;", "", text)
    text = re.sub(r"\breturn;", "", text)
    return re.sub(r"\s+", " ", text).strip()


with open(out, "w") as f:
    for tile, (seg, off) in sorted(tables[name].items(), key=lambda kv: int(kv[0])):
        f.write(f"{tile} seg{seg}:{off:#x}: {compact(body(seg, off))}\n")
