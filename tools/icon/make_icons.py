#!/usr/bin/env python3
"""Draws the application icon and writes PNG, ICO and ICNS versions.

No image library is available on this machine, so the mark is rasterised here:
shapes are drawn at four times the final size and box filtered down, which is
enough anti-aliasing for a flat geometric icon. Everything is pure Python plus
zlib.

The mark is the app's own vocabulary: a lattice tile, a few filled cells, the
accent purple and the paper colour from the theme.

Usage:
    python3 tools/icon/make_icons.py
"""

import os
import struct
import zlib

BASE = 1024
SCALE = 4
CANVAS = BASE * SCALE

PAPER = (0x17, 0x18, 0x19, 255)
RULE = (0x3a, 0x3b, 0x3d, 255)
ACCENT = (0xb9, 0x9a, 0xff, 255)
INK = (0xf0, 0xf0, 0xf1, 255)
CLEAR = (0, 0, 0, 0)


def new_canvas(color):
    return bytearray(bytes(color) * (CANVAS * CANVAS))


def blend(canvas, x, y, color, coverage):
    if coverage <= 0.0:
        return
    index = (y * CANVAS + x) * 4
    alpha = coverage * (color[3] / 255.0)
    if alpha >= 0.999:
        canvas[index : index + 4] = bytes(color)
        return
    for channel in range(3):
        under = canvas[index + channel]
        canvas[index + channel] = int(round(under + (color[channel] - under) * alpha))
    canvas[index + 3] = min(255, int(round(canvas[index + 3] + 255 * alpha)))


def rounded_rect(canvas, x0, y0, x1, y1, radius, color):
    for y in range(max(0, y0), min(CANVAS, y1)):
        for x in range(max(0, x0), min(CANVAS, x1)):
            dx = 0
            if x < x0 + radius:
                dx = x0 + radius - x
            elif x > x1 - radius - 1:
                dx = x - (x1 - radius - 1)
            dy = 0
            if y < y0 + radius:
                dy = y0 + radius - y
            elif y > y1 - radius - 1:
                dy = y - (y1 - radius - 1)
            if dx and dy and dx * dx + dy * dy > radius * radius:
                continue
            blend(canvas, x, y, color, 1.0)


def rect(canvas, x0, y0, x1, y1, color):
    for y in range(max(0, y0), min(CANVAS, y1)):
        row = y * CANVAS
        for x in range(max(0, x0), min(CANVAS, x1)):
            blend(canvas, x, y, color, 1.0)


def downsample(canvas):
    out = bytearray(BASE * BASE * 4)
    total = SCALE * SCALE
    for y in range(BASE):
        for x in range(BASE):
            r = g = b = a = 0
            for sy in range(SCALE):
                base = ((y * SCALE + sy) * CANVAS + x * SCALE) * 4
                for sx in range(SCALE):
                    index = base + sx * 4
                    r += canvas[index]
                    g += canvas[index + 1]
                    b += canvas[index + 2]
                    a += canvas[index + 3]
            index = (y * BASE + x) * 4
            out[index] = r // total
            out[index + 1] = g // total
            out[index + 2] = b // total
            out[index + 3] = a // total
    return out


def draw_mark():
    canvas = new_canvas(CLEAR)

    # Rounded plate, the shape macOS and Windows both read as an app icon.
    inset = int(CANVAS * 0.055)
    rounded_rect(
        canvas,
        inset,
        inset,
        CANVAS - inset,
        CANVAS - inset,
        int(CANVAS * 0.208),
        PAPER,
    )

    # A three by three tile inside the plate.
    span = CANVAS * 0.56
    origin = (CANVAS - span) / 2.0
    cell = span / 3.0
    line = max(2, int(CANVAS * 0.006))

    filled = {(0, 0), (0, 1), (1, 0), (1, 1)}
    for row in range(3):
        for column in range(3):
            x0 = int(origin + column * cell)
            y0 = int(origin + row * cell)
            x1 = int(origin + (column + 1) * cell)
            y1 = int(origin + (row + 1) * cell)
            if (row, column) in filled:
                colour = ACCENT if (row, column) != (1, 1) else INK
                rect(canvas, x0, y0, x1, y1, colour)

    # Rules, drawn over the fills so cells stay separated like the canvas does.
    for step in range(4):
        across = int(origin + step * cell)
        rect(canvas, across - line // 2, int(origin), across + line // 2, int(origin + span), RULE)
        rect(canvas, int(origin), across - line // 2, int(origin + span), across + line // 2, RULE)

    return downsample(canvas)


def chunk(tag, payload):
    return (
        struct.pack(">I", len(payload))
        + tag
        + payload
        + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
    )


def png_bytes(size, pixels):
    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)
        raw += pixels[y * stride : (y + 1) * stride]
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def resize(pixels, source_size, target_size):
    """Box filter with alpha weighted colour, so edges do not fringe."""
    out = bytearray(target_size * target_size * 4)
    ratio = source_size / target_size
    for y in range(target_size):
        y0 = int(y * ratio)
        y1 = max(y0 + 1, min(source_size, int((y + 1) * ratio)))
        for x in range(target_size):
            x0 = int(x * ratio)
            x1 = max(x0 + 1, min(source_size, int((x + 1) * ratio)))
            red = green = blue = alpha = count = 0
            for sy in range(y0, y1):
                base = sy * source_size * 4
                for sx in range(x0, x1):
                    index = base + sx * 4
                    a = pixels[index + 3]
                    red += pixels[index] * a
                    green += pixels[index + 1] * a
                    blue += pixels[index + 2] * a
                    alpha += a
                    count += 1
            index = (y * target_size + x) * 4
            if alpha:
                out[index] = red // alpha
                out[index + 1] = green // alpha
                out[index + 2] = blue // alpha
            out[index + 3] = alpha // count
    return out


def bmp_entry(pixels, size):
    """A 32 bit bottom-up DIB with its AND mask, which is what NSIS reads.

    PNG compressed entries are valid ICO, but the NSIS icon loader refuses
    them, so every size is stored uncompressed.
    """
    header = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0, size * size * 4, 0, 0, 0, 0)
    body = bytearray()
    for y in range(size - 1, -1, -1):
        row = pixels[y * size * 4 : (y + 1) * size * 4]
        for x in range(size):
            red, green, blue, alpha = row[x * 4 : x * 4 + 4]
            body += bytes((blue, green, red, alpha))
    mask_stride = ((size + 31) // 32) * 4
    return header + bytes(body) + bytes(mask_stride * size)


def write_ico(path, images):
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    payload = b""
    for size, data in images:
        entries += struct.pack(
            "<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset
        )
        offset += len(data)
        payload += data
    with open(path, "wb") as handle:
        handle.write(header + entries + payload)


def write_icns(path, images):
    body = b""
    for tag, data in images:
        body += tag + struct.pack(">I", len(data) + 8) + data
    with open(path, "wb") as handle:
        handle.write(b"icns" + struct.pack(">I", len(body) + 8) + body)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "..", "packaging", "icons")
    os.makedirs(out, exist_ok=True)

    master = draw_mark()
    with open(os.path.join(out, "lattice-1024.png"), "wb") as handle:
        handle.write(png_bytes(BASE, master))

    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    ico_images = [(size, bmp_entry(resize(master, BASE, size), size)) for size in ico_sizes]
    write_ico(os.path.join(out, "lattice.ico"), ico_images)

    icns_images = []
    for tag, size in ((b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)):
        icns_images.append((tag, png_bytes(size, resize(master, BASE, size))))
    write_icns(os.path.join(out, "lattice.icns"), icns_images)

    for size in (256,):
        with open(os.path.join(out, f"lattice-{size}.png"), "wb") as handle:
            handle.write(png_bytes(size, resize(master, BASE, size)))

    print(f"wrote icons to {os.path.normpath(out)}")


if __name__ == "__main__":
    main()
