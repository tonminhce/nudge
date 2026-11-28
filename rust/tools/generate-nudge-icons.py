#!/usr/bin/env python3
import argparse
import struct
import zlib
from pathlib import Path

BRAND = (0x47, 0x62, 0xD9)
WHITE = (0xFF, 0xFF, 0xFF)
VIEWBOX = 32
RADIUS = 8
BARS = ((8, 10, 15, 22), (17, 10, 24, 22))
SS = 4


def _inside_rounded(x, y, size):
    px, py = x * VIEWBOX / size, y * VIEWBOX / size
    lo, hi = 0.0, VIEWBOX
    r = RADIUS
    if lo + r <= px <= hi - r or lo + r <= py <= hi - r:
        return lo <= px <= hi and lo <= py <= hi
    cx = lo + r if px < lo + r else hi - r
    cy = lo + r if py < lo + r else hi - r
    return (px - cx) ** 2 + (py - cy) ** 2 <= r * r


def _inside_bar(x, y, size):
    px, py = x * VIEWBOX / size, y * VIEWBOX / size
    return any(x0 <= px <= x1 and y0 <= py <= y1 for x0, y0, x1, y1 in BARS)


def render(size):
    pixels = bytearray()
    for row in range(size):
        pixels.append(0)
        for col in range(size):
            hits = 0
            brand = 0
            for sy in range(SS):
                for sx in range(SS):
                    x = col + (sx + 0.5) / SS
                    y = row + (sy + 0.5) / SS
                    if _inside_rounded(x, y, size):
                        hits += 1
                        if _inside_bar(x, y, size):
                            brand += 1
            total = SS * SS
            if hits == 0:
                pixels.extend((0, 0, 0, 0))
                continue
            alpha = int(round(255 * hits / total))
            white = brand / hits if hits else 0
            red = int(round(BRAND[0] + (WHITE[0] - BRAND[0]) * white))
            green = int(round(BRAND[1] + (WHITE[1] - BRAND[1]) * white))
            blue = int(round(BRAND[2] + (WHITE[2] - BRAND[2]) * white))
            pixels.extend((red, green, blue, alpha))
    return pixels


def png(size):
    raw = bytes(render(size))
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    chunks = [
        b"IHDR" + header,
        b"IDAT" + zlib.compress(raw, 9),
        b"IEND" + b"",
    ]
    out = bytearray(b"\x89PNG\r\n\x1a\n")
    for chunk in chunks:
        out.extend(struct.pack(">I", len(chunk) - 4))
        out.extend(chunk)
        out.extend(struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF))
    return bytes(out)


def ico(sizes, path):
    images = [(size, png(size)) for size in sizes]
    count = len(images)
    header_size = 6 + 16 * count
    out = bytearray(struct.pack("<HHH", 0, 1, count))
    offset = header_size
    for size, data in images:
        width = 0 if size >= 256 else size
        height = 0 if size >= 256 else size
        out.extend(struct.pack("<BBBBHHII", width, height, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    for _, data in images:
        out.extend(data)
    path.write_bytes(bytes(out))
    return count


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--sizes", default="48,72,96,144,152,167,180,192,384,512,1024")
    parser.add_argument("--ico-sizes", default="32,48")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    for size in (int(value) for value in args.sizes.split(",")):
        (args.out / f"nudge-mark-{size}.png").write_bytes(png(size))
        print(f"png {size}")
    count = ico([int(value) for value in args.ico_sizes.split(",")], args.out / "favicon.ico")
    print(f"ico entries {count}")


if __name__ == "__main__":
    main()
