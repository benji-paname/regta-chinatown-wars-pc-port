#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
import struct, zlib, sys

pak_path = r"C:\Users\BENJAMIN\Documents\Codex\2026-10-17\tu-voit-le-projet-vcs-rust\work\Chinatown-Wars-PSP.iso"
with open(pak_path, "rb") as f:
    data = f.read()

# Find OVERLAYS.PAK inside ISO (magic 20KP)
idx = data.find(b"20KP")
if idx == -1:
    print("20KP not found")
    sys.exit(1)

count, sec = struct.unpack_from("<II", data, idx + 0x24)
print(f"OVERLAYS.PAK at offset {idx:#x}: {count} entries, sector {sec} bytes")

starts = struct.unpack_from(f"<{count}H", data, idx + 0x2C)
names = []
for i in range(1, count):
    off = idx + 0x800 + starts[i] * sec
    if data[off:off+4] != b"10SA":
        continue
    packed, unpacked = struct.unpack_from("<II", data, off + 4)
    zlib_bytes = data[off + 12:off + 12 + packed]
    try:
        elf = zlib.decompress(zlib_bytes)
        # Parse ELF shstrtab to get section name .overlay_<name>
        e_shoff = struct.unpack_from("<I", elf, 0x20)[0]
        e_shentsize = struct.unpack_from("<H", elf, 0x2E)[0]
        e_shnum = struct.unpack_from("<H", elf, 0x30)[0]
        e_shstrndx = struct.unpack_from("<H", elf, 0x32)[0]
        
        shstr_hdr = e_shoff + e_shstrndx * e_shentsize
        str_offset = struct.unpack_from("<I", elf, shstr_hdr + 0x10)[0]
        
        # Section 1 header
        sh1 = e_shoff + 1 * e_shentsize
        sh1_name_idx = struct.unpack_from("<I", elf, sh1)[0]
        end = elf.find(b"\x00", str_offset + sh1_name_idx)
        sec_name = elf[str_offset + sh1_name_idx:end].decode("ascii", "replace")
        ovl_name = sec_name.replace(".overlay_", "")
        names.append((i, ovl_name, packed, unpacked))
    except Exception as e:
        names.append((i, f"error: {e}", packed, unpacked))

print(f"Successfully extracted {len(names)} overlay names:")
for i, name, packed, unpacked in names:
    slot = "App (Slot 1)" if "app" in name or "hud" in name or "trading" in name or "safehouse" in name else "Mission (Slot 0)"
    print(f"[{i:03d}] {name:<25} | {slot:<15} | Packed: {packed:>6} B | Unpacked: {unpacked:>6} B")
