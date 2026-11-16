#!/usr/bin/env python3
"""Minimal PNG reader for the comparison scripts.

Handles 8 bit RGB and RGBA, no interlacing, and all five row filters. That
covers what the grabber writes and what Playwright writes, which is all this
repository needs. Returns rows of RGB tuples.
"""

import struct
import zlib


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def read(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")

    pos = 8
    idat = b""
    width = height = depth = color = interlace = 0
    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        tag = data[pos + 4 : pos + 8]
        payload = data[pos + 8 : pos + 8 + length]
        if tag == b"IHDR":
            width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", payload)
        elif tag == b"IDAT":
            idat += payload
        elif tag == b"IEND":
            break
        pos += 12 + length

    if depth != 8 or interlace != 0 or color not in (2, 6):
        raise ValueError(f"{path}: unsupported depth {depth} color {color} interlace {interlace}")

    channels = 3 if color == 2 else 4
    stride = width * channels
    raw = zlib.decompress(idat)

    rows = []
    previous = bytearray(stride)
    offset = 0
    for _ in range(height):
        filter_type = raw[offset]
        offset += 1
        line = bytearray(raw[offset : offset + stride])
        offset += stride

        if filter_type == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 0xFF
        elif filter_type == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 0xFF
        elif filter_type == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 0xFF
        elif filter_type == 4:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                up_left = previous[i - channels] if i >= channels else 0
                line[i] = (line[i] + _paeth(left, previous[i], up_left)) & 0xFF
        elif filter_type != 0:
            raise ValueError(f"{path}: unknown filter {filter_type}")

        if channels == 3:
            rows.append([tuple(line[i : i + 3]) for i in range(0, stride, 3)])
        else:
            rows.append([tuple(line[i : i + 3]) for i in range(0, stride, 4)])
        previous = line

    return width, height, rows
