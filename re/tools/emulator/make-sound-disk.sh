#!/bin/sh
# Build re/work/emu/system75-sound.dsk: the System Folder of the System 7.5
# "Disk Tools" floppy on a 20 MB volume with Sound Manager 3.1 added, so
# Basilisk II can play sound (it needs a Sound Manager 3.x component; with
# System 7.0 that does not work). run.sh uses it when it exists.
#
# Usage: make-sound-disk.sh MacOS75.ZIP Sound_Manager_3.1.sea_.hqx
# (MacOS75.ZIP: the System 7.5.0 floppies, SSW750_DiskTools.dsk is used)
# Needs: brew install unar hfsutils
set -e
ZIP="$1"
HQX="$2"
[ -f "$ZIP" ] && [ -f "$HQX" ] || {
    echo "usage: $0 MacOS75.ZIP Sound_Manager_3.1.sea_.hqx" >&2
    exit 1
}
EMU="$(cd "$(dirname "$0")/../.." && pwd)/work/emu"
TMP="$(mktemp -d)"
trap 'humount >/dev/null 2>&1 || true; rm -rf "$TMP"' EXIT

# Sound Manager and its control panel, with resource forks
unar -q -o "$TMP/sm" "$HQX"
SM="$TMP/sm/Sound Manager 3.1"
python3 -I - "$SM" "$TMP" <<'EOF'
import struct, subprocess, sys
sm, tmp = sys.argv[1], sys.argv[2]
def crc16(data):
    crc = 0
    for b in data:
        crc ^= b << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else crc << 1
            crc &= 0xFFFF
    return crc
def macbinary(name, out):
    path = f"{sm}/{name}"
    data = open(path, "rb").read()
    rsrc = open(path + "/..namedfork/rsrc", "rb").read()
    finder_info = subprocess.run(["xattr", "-px", "com.apple.FinderInfo", path],
                                 capture_output=True, text=True, check=True).stdout
    info = bytes.fromhex("".join(finder_info.split()))
    h = bytearray(128)
    n = name.encode("mac_roman")
    h[1] = len(n)
    h[2:2 + len(n)] = n
    h[65:73] = info[0:8]  # type, creator
    h[73] = info[8]  # Finder flags
    h[83:87] = struct.pack(">I", len(data))
    h[87:91] = struct.pack(">I", len(rsrc))
    h[122] = h[123] = 0x81  # MacBinary II
    h[124:126] = struct.pack(">H", crc16(h[:124]))
    pad = lambda b: b + bytes(-len(b) % 128)
    open(out, "wb").write(bytes(h) + pad(data) + pad(rsrc))
macbinary("Sound Manager", f"{tmp}/SoundManager.bin")
macbinary("Sound", f"{tmp}/Sound.bin")
EOF

# the System Folder of the Disk Tools floppy
unzip -q -o "$ZIP" SSW750_DiskTools.dsk -d "$TMP"
FLOPPY="$TMP/SSW750_DiskTools.dsk"
hmount "$FLOPPY" >/dev/null
hcopy -m ":System Folder:System" "$TMP/System.bin"
hcopy -m ":System Folder:Finder" "$TMP/Finder.bin"
hcopy -m ":System Folder:Preferences:Finder Preferences" "$TMP/FinderPrefs.bin"
humount

OUT="$TMP/system75-sound.dsk"
dd if=/dev/zero of="$OUT" bs=1m count=20 2>/dev/null
hformat -l "System 7.5" "$OUT" >/dev/null
for d in "" Extensions "Control Panels" Preferences "Apple Menu Items" "Startup Items" Fonts; do
    hmkdir ":System Folder${d:+:$d}"
done
hcopy -m "$TMP/System.bin" ":System Folder:"
hcopy -m "$TMP/Finder.bin" ":System Folder:"
hcopy -m "$TMP/FinderPrefs.bin" ":System Folder:Preferences:"
hcopy -m "$TMP/SoundManager.bin" ":System Folder:Extensions:"
hcopy -m "$TMP/Sound.bin" ":System Folder:Control Panels:"
hattrib -b ":System Folder"
humount
# boot blocks (a new volume has none)
python3 -I -c "
import sys
src = open(sys.argv[1], 'rb').read(1024)
with open(sys.argv[2], 'r+b') as f:
    f.write(src)
" "$FLOPPY" "$OUT"
cp "$OUT" "$EMU/system75-sound.dsk"
echo "built $EMU/system75-sound.dsk"
