"""Disassemble a range of a CODE segment with the names the decompiler
loses: Toolbox traps (A-line words) by name, calls through the jump table
(`jsr $off(a5)`) as "CODE n 0xoffset". Data between routines (names,
strings, jump tables) comes out as nonsense instructions.

Usage: python3 disasm.py <work dir> <segment> <start hex> <end hex>
(needs capstone; the segments are re/work/code/segNN.bin, CODE 0 is
re/work/rsrc/CODE_0.bin)
"""
import re
import struct
import sys

import capstone

# Toolbox and OS traps (Inside Macintosh). Auto-pop and flag bits (0x0400,
# 0x0200, 0x0100 in OS traps) are ignored where they do not change the name.
TRAPS = {
    0xA000: "Open", 0xA001: "Close", 0xA002: "Read", 0xA003: "Write",
    0xA004: "Control", 0xA005: "Status", 0xA00C: "GetFileInfo",
    0xA011: "GetEOF", 0xA012: "SetEOF", 0xA013: "FlushVol", 0xA014: "GetVol",
    0xA015: "SetVol", 0xA018: "GetFPos", 0xA01B: "SetZone",
    0xA01D: "MaxMem", 0xA01E: "NewPtr", 0xA01F: "DisposePtr",
    0xA020: "SetPtrSize", 0xA021: "GetPtrSize", 0xA022: "NewHandle",
    0xA023: "DisposeHandle", 0xA024: "SetHandleSize", 0xA025: "GetHandleSize",
    0xA027: "ReallocHandle", 0xA029: "HLock", 0xA02A: "HUnlock",
    0xA02E: "BlockMove", 0xA036: "MoreMasters", 0xA063: "MaxApplZone", 0xA02F: "PostEvent", 0xA030: "OSEventAvail",
    0xA031: "GetOSEvent", 0xA032: "FlushEvents", 0xA03C: "CmpString",
    0xA040: "ResrvMem", 0xA044: "SetFPos", 0xA049: "HPurge",
    0xA04A: "HNoPurge", 0xA051: "ReadXPRam", 0xA055: "StripAddress",
    0xA064: "MoveHHi", 0xA069: "HGetState", 0xA06A: "HSetState",
    0xA05D: "SwapMMUMode", 0xA060: "FSDispatch", 0xA0BD: "CacheFlush",
    0xA1AD: "Gestalt", 0xA260: "HFSDispatch", 0xA11E: "NewPtr",
    0xA122: "NewHandle", 0xA31E: "NewPtrClear", 0xA322: "NewHandleClear",
    0xA800: "SoundDispatch", 0xA801: "SndDisposeChannel",
    0xA802: "SndAddModifier", 0xA803: "SndDoCommand",
    0xA804: "SndDoImmediate", 0xA805: "SndPlay", 0xA806: "SndControl",
    0xA807: "SndNewChannel", 0xA850: "InitCursor", 0xA851: "SetCursor",
    0xA852: "HideCursor", 0xA853: "ShowCursor", 0xA855: "ShieldCursor",
    0xA856: "ObscureCursor", 0xA858: "BitAnd", 0xA85D: "BitTst",
    0xA85E: "BitSet", 0xA85F: "BitClr", 0xA860: "WaitNextEvent",
    0xA861: "Random", 0xA862: "ForeColor", 0xA863: "BackColor",
    0xA86A: "HiWord", 0xA86B: "LoWord", 0xA86E: "InitGraf",
    0xA86F: "OpenPort", 0xA870: "LocalToGlobal", 0xA871: "GlobalToLocal",
    0xA873: "SetPort", 0xA874: "GetPort", 0xA875: "SetPortBits",
    0xA876: "PortSize", 0xA877: "MovePortTo", 0xA878: "SetOrigin",
    0xA879: "SetClip", 0xA87A: "GetClip", 0xA87B: "ClipRect",
    0xA87C: "BackPat", 0xA87D: "ClosePort", 0xA883: "DrawChar",
    0xA884: "DrawString", 0xA885: "DrawText", 0xA886: "TextWidth",
    0xA887: "TextFont", 0xA888: "TextFace", 0xA889: "TextMode",
    0xA88A: "TextSize", 0xA88B: "GetFontInfo", 0xA88C: "StringWidth",
    0xA88D: "CharWidth", 0xA891: "LineTo", 0xA892: "Line", 0xA893: "MoveTo",
    0xA894: "Move", 0xA896: "HidePen", 0xA897: "ShowPen",
    0xA898: "GetPenState", 0xA899: "SetPenState", 0xA89A: "GetPen",
    0xA89B: "PenSize", 0xA89C: "PenMode", 0xA89D: "PenPat",
    0xA89E: "PenNormal", 0xA8A1: "FrameRect", 0xA8A2: "PaintRect",
    0xA8A3: "EraseRect", 0xA8A4: "InvertRect", 0xA8A5: "FillRect",
    0xA8A6: "EqualRect", 0xA8A7: "SetRect", 0xA8A8: "OffsetRect",
    0xA8A9: "InsetRect", 0xA8AA: "SectRect", 0xA8AB: "UnionRect",
    0xA8AD: "PtInRect", 0xA8AE: "EmptyRect", 0xA8B0: "FrameRoundRect",
    0xA8B1: "PaintRoundRect", 0xA8D8: "NewRgn", 0xA8D9: "DisposeRgn",
    0xA8DA: "OpenRgn", 0xA8DB: "CloseRgn", 0xA8DF: "RectRgn",
    0xA8E0: "OffsetRgn", 0xA8EC: "CopyBits", 0xA8F3: "OpenPicture",
    0xA8F4: "ClosePicture", 0xA8F5: "KillPicture", 0xA8F6: "DrawPicture",
    0xA8FE: "InitFonts", 0xA8FF: "GetFName", 0xA900: "GetFNum",
    0xA90A: "SetFontLock", 0xA910: "GetWMgrPort", 0xA912: "InitWindows",
    0xA913: "NewWindow", 0xA914: "DisposeWindow", 0xA915: "ShowWindow",
    0xA916: "HideWindow", 0xA917: "GetWRefCon", 0xA918: "SetWRefCon",
    0xA919: "GetWTitle", 0xA91A: "SetWTitle", 0xA91B: "MoveWindow",
    0xA91C: "HiliteWindow", 0xA91D: "SizeWindow", 0xA91E: "TrackGoAway",
    0xA91F: "SelectWindow", 0xA920: "BringToFront", 0xA921: "SendBehind",
    0xA922: "BeginUpdate", 0xA923: "EndUpdate", 0xA924: "FrontWindow",
    0xA925: "DragWindow", 0xA927: "InvalRgn", 0xA928: "InvalRect",
    0xA929: "ValidRgn", 0xA92A: "ValidRect", 0xA92B: "GrowWindow",
    0xA92C: "FindWindow", 0xA92D: "CloseWindow", 0xA930: "InitMenus",
    0xA931: "NewMenu", 0xA932: "DisposeMenu", 0xA933: "AppendMenu",
    0xA934: "ClearMenuBar", 0xA935: "InsertMenu", 0xA936: "DeleteMenu",
    0xA937: "DrawMenuBar", 0xA938: "HiliteMenu", 0xA939: "EnableItem",
    0xA93A: "DisableItem", 0xA93B: "GetMenuBar", 0xA93C: "SetMenuBar",
    0xA93D: "MenuSelect", 0xA93E: "MenuKey", 0xA945: "CheckItem",
    0xA946: "GetMenuItemText", 0xA947: "SetMenuItemText",
    0xA949: "GetMenuHandle", 0xA94C: "FlashMenuBar", 0xA94D: "AddResMenu",
    0xA950: "CountMItems", 0xA954: "NumToString/StringToNum (Pack7)",
    0xA95D: "HiliteControl", 0xA960: "GetCtlValue", 0xA963: "SetCtlValue", 0xA964: "SetCtlMin",
    0xA968: "TrackControl", 0xA96C: "FindControl", 0xA970: "GetNextEvent",
    0xA971: "EventAvail", 0xA972: "GetMouse", 0xA973: "StillDown",
    0xA974: "Button", 0xA975: "TickCount", 0xA976: "GetKeys",
    0xA977: "WaitMouseUp", 0xA97B: "InitDialogs", 0xA97C: "GetNewDialog",
    0xA97D: "NewDialog", 0xA97E: "SelectDialogItemText",
    0xA97F: "IsDialogEvent", 0xA980: "DialogSelect", 0xA981: "DrawDialog",
    0xA982: "CloseDialog", 0xA983: "DisposeDialog", 0xA985: "Alert",
    0xA986: "StopAlert", 0xA987: "NoteAlert", 0xA988: "CautionAlert",
    0xA98B: "ParamText", 0xA98D: "GetDialogItem", 0xA98E: "SetDialogItem",
    0xA98F: "SetDialogItemText", 0xA990: "GetDialogItemText",
    0xA991: "ModalDialog", 0xA992: "DetachResource",
    0xA994: "CurResFile", 0xA995: "InitResources", 0xA997: "OpenResFile",
    0xA998: "UseResFile", 0xA999: "UpdateResFile", 0xA99A: "CloseResFile",
    0xA99B: "SetResLoad", 0xA99C: "CountResources",
    0xA99D: "GetIndResource", 0xA9A0: "GetResource",
    0xA9A1: "GetNamedResource", 0xA9A2: "LoadResource",
    0xA9A3: "ReleaseResource", 0xA9A5: "SizeResource",
    0xA9A6: "GetResAttrs", 0xA9A8: "GetResInfo", 0xA9AA: "ChangedResource",
    0xA9AB: "AddResource", 0xA9AD: "RemoveResource", 0xA9AF: "ResError",
    0xA9B0: "WriteResource", 0xA9B9: "GetCursor", 0xA9BA: "GetString",
    0xA9BB: "GetIcon", 0xA9BC: "GetPicture", 0xA9BD: "GetNewWindow",
    0xA9BE: "GetNewControl", 0xA9BF: "GetMenu", 0xA9C0: "GetNewMBar",
    0xA9C6: "SecondsToDate", 0xA9C8: "SysBeep", 0xA9C9: "SysError",
    0xA9B3: "SystemClick", 0xA9B4: "SystemTask", 0xA9B6: "OpenDeskAcc",
    0xA9C2: "SystemEdit", 0xA9CC: "TEInit", 0xA9D5: "TECopy", 0xA9D6: "TECut",
    0xA9DB: "TEPaste", 0xA9DC: "TEKey", 0xA9E7: "Pack0", 0xA9E8: "Pack1", 0xA9E9: "Pack2",
    0xA9EA: "Pack3 (Standard File)", 0xA9EB: "Pack4 (SANE)", 0xA9EC: "Pack5",
    0xA9ED: "Pack6", 0xA9EE: "Pack7 (NumToString/StringToNum)",
    0xA9EF: "PtrAndHand", 0xA9F0: "LoadSeg", 0xA9F1: "UnloadSeg",
    0xA9F4: "ExitToShell", 0xA9F5: "GetAppParms", 0xA9FF: "Debugger",
    0xAA00: "OpenCPort", 0xAA01: "InitCPort", 0xAA02: "CloseCPort",
    0xAA03: "NewPixMap", 0xAA04: "DisposePixMap", 0xAA06: "SetPortPix",
    0xAA08: "RGBForeColor", 0xAA09: "RGBBackColor", 0xAA0A: "SetCPixel",
    0xAA0B: "GetCPixel", 0xAA0C: "GetForeColor", 0xAA0D: "GetBackColor",
    0xAA18: "GetCTable", 0xAA19: "DisposeCTable", 0xAA1E: "GetCIcon",
    0xAA1F: "PlotCIcon", 0xAA27: "NewGDevice", 0xAA29: "GetDeviceList",
    0xAA2A: "GetMainDevice", 0xAA2B: "GetNextDevice", 0xAA2C: "TestDeviceAttribute",
    0xAA2D: "SetDeviceAttribute", 0xAA32: "GetGDevice", 0xAA31: "SetGDevice",
    0xAA33: "Color2Index", 0xAA34: "Index2Color", 0xAA38: "SetEntries",
    0xAA45: "NewCWindow", 0xAA4C: "PaintOne", 0xAA90: "InitPalettes",
    0xAA91: "NewPalette", 0xAA92: "GetNewPalette", 0xAA93: "DisposePalette",
    0xAA94: "ActivatePalette", 0xAA95: "SetPalette", 0xAA96: "GetPalette",
    0xAA97: "PmForeColor", 0xAA98: "PmBackColor", 0xAA99: "AnimateEntry",
    0xAA9A: "AnimatePalette", 0xAA9B: "GetEntryColor", 0xAA9C: "SetEntryColor",
    0xAAA2: "PaletteDispatch", 0xAB1D: "QDExtensions",
}


def trap_name(word):
    if word in TRAPS:
        return TRAPS[word]
    if word & 0x0800 == 0:  # OS trap: flag bits 8–10
        base = word & 0xF8FF
        if base in TRAPS:
            return TRAPS[base]
    else:
        base = word & 0xFBFF  # Toolbox: auto-pop bit 10
        if base in TRAPS:
            return TRAPS[base] + " (auto-pop)"
    return f"trap {word:#06x}"


def jump_table(work):
    code0 = open(f"{work}/rsrc/CODE_0.bin", "rb").read()
    table = {}
    for i in range((len(code0) - 16) // 8):
        offset, _, segment, _ = struct.unpack(">HHHH", code0[16 + 8 * i : 24 + 8 * i])
        table[34 + 8 * i] = (segment, offset)
    return table


def main():
    work, seg = sys.argv[1], int(sys.argv[2])
    start, end = int(sys.argv[3], 16), int(sys.argv[4], 16)
    code = open(f"{work}/code/seg{seg:02d}.bin", "rb").read()
    end = min(end, len(code) - 1)
    table = jump_table(work)
    md = capstone.Cs(capstone.CS_ARCH_M68K, capstone.CS_MODE_M68K_020 | capstone.CS_MODE_BIG_ENDIAN)
    md.skipdata = True
    at = start
    while at < end:
        word = struct.unpack(">H", code[at : at + 2])[0]
        if word >> 12 == 0xA:
            print(f"{at:5x} _{trap_name(word)}")
            at += 2
            continue
        ins = next(md.disasm(code[at:end], at, count=1), None)
        if ins is None:
            print(f"{at:5x} dc.w     ${word:04x}")
            at += 2
            continue
        op = ins.op_str
        m = re.fullmatch(r"\$([0-9a-f]+)\(a5\)", op)
        if ins.mnemonic in ("jsr", "jmp", "pea") and m and int(m.group(1), 16) in table:
            s, o = table[int(m.group(1), 16)]
            op += f"    ; CODE {s} {o:#x}"
        print(f"{ins.address:5x} {ins.mnemonic:8} {op}")
        at += ins.size


if __name__ == "__main__":
    main()
