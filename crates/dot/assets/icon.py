import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

CANVAS = 1024
SQUARE_HALF = 412
CORNER = 185
BORDER = 2
DOT_RADIUS = 112
BORDER_INK = 0x38
ICONSET = [(16, 1), (16, 2), (32, 1), (32, 2), (128, 1), (128, 2), (256, 1), (256, 2), (512, 1), (512, 2)]

SVG = f"""<svg xmlns="http://www.w3.org/2000/svg" width="{CANVAS}" height="{CANVAS}" viewBox="0 0 {CANVAS} {CANVAS}">
  <rect x="{CANVAS // 2 - SQUARE_HALF}" y="{CANVAS // 2 - SQUARE_HALF}" width="{2 * SQUARE_HALF}" height="{2 * SQUARE_HALF}" rx="{CORNER}" fill="#000000"/>
  <rect x="{CANVAS // 2 - SQUARE_HALF + BORDER / 2}" y="{CANVAS // 2 - SQUARE_HALF + BORDER / 2}" width="{2 * SQUARE_HALF - BORDER}" height="{2 * SQUARE_HALF - BORDER}" rx="{CORNER - BORDER / 2}" fill="none" stroke="#{BORDER_INK:02x}{BORDER_INK:02x}{BORDER_INK:02x}" stroke-width="{BORDER}"/>
  <circle cx="{CANVAS // 2}" cy="{CANVAS // 2}" r="{DOT_RADIUS}" fill="#ffffff"/>
</svg>
"""


def coverage(distance_in_pixels):
    return min(1.0, max(0.0, 0.5 - distance_in_pixels))


def square_distance(x, y):
    inner = SQUARE_HALF - CORNER
    qx = abs(x - CANVAS / 2) - inner
    qy = abs(y - CANVAS / 2) - inner
    outside = (max(qx, 0.0) ** 2 + max(qy, 0.0) ** 2) ** 0.5
    return outside + min(max(qx, qy), 0.0) - CORNER


def pixel(column, row, scale):
    x = (column + 0.5) / scale
    y = (row + 0.5) / scale
    edge = square_distance(x, y)
    alpha = coverage(edge * scale)
    inside_border = coverage((edge + BORDER) * scale)
    ink = BORDER_INK * (1.0 - inside_border)
    dot = coverage((((x - CANVAS / 2) ** 2 + (y - CANVAS / 2) ** 2) ** 0.5 - DOT_RADIUS) * scale)
    ink = ink * (1.0 - dot) + 255.0 * dot
    level = round(ink)
    return bytes((level, level, level, round(alpha * 255.0)))


def png(size):
    scale = size / CANVAS
    rows = b"".join(b"\x00" + b"".join(pixel(column, row, scale) for column in range(size)) for row in range(size))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def main():
    svg_path, icns_path = Path(sys.argv[1]), Path(sys.argv[2])
    svg_path.write_text(SVG)
    with tempfile.TemporaryDirectory() as work:
        iconset = Path(work) / "Dot.iconset"
        iconset.mkdir()
        rendered = {}
        for points, density in ICONSET:
            size = points * density
            if size not in rendered:
                rendered[size] = png(size)
            suffix = "@2x" if density == 2 else ""
            (iconset / f"icon_{points}x{points}{suffix}.png").write_bytes(rendered[size])
        subprocess.run(["iconutil", "--convert", "icns", "--output", str(icns_path), str(iconset)], check=True)


main()
