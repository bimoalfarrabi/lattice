#!/usr/bin/env python3
"""Prints the layout geometry of a studio screenshot, for comparison.

Run it on a capture and on the reference shot to see the deltas as numbers
instead of impressions. The reference values are recorded in
docs/ui-spec.md under "Measured against the reference".

Usage:
    python3 tools/screenshot/compare.py shot.png
"""

import sys

import pngread

PAPER = (23, 24, 25)
PANEL = (27, 28, 30)


def bright_runs(img, y0, y1, x0, x1, threshold=250, gap=8):
    _, _, rows = img
    columns = []
    for x in range(x0, x1):
        if any(sum(rows[y][x]) > threshold for y in range(y0, y1)):
            columns.append(x)
    out = []
    for x in columns:
        if out and x - out[-1][1] <= gap:
            out[-1] = (out[-1][0], x)
        else:
            out.append((x, x))
    return out


def not_paper_runs(img, y, x0, x1, tolerance=2, minimum=15):
    _, _, rows = img
    out = []
    start = None
    for x in range(x0, x1):
        hit = any(abs(rows[y][x][i] - PAPER[i]) > tolerance for i in range(3))
        if hit and start is None:
            start = x
        if not hit and start is not None:
            if x - start >= minimum:
                out.append((start, x - 1))
            start = None
    if start is not None and x1 - start >= minimum:
        out.append((start, x1 - 1))
    return out


def bbox(img, x0, x1, y0, y1, predicate):
    _, _, rows = img
    xs = []
    ys = []
    for y in range(y0, y1):
        for x in range(x0, x1):
            if predicate(rows[y][x]):
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return (min(xs), max(xs), min(ys), max(ys))


def purple(color):
    return color[2] > color[0] + 20 and 80 < color[0] < 170


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: compare.py shot.png")

    img = pngread.read(sys.argv[1])
    width, height, rows = img
    print(f"{sys.argv[1]} {width}x{height}")

    print("topbar text      ", bright_runs(img, 30, 80, 0, 700))
    print("transport pills  ", not_paper_runs(img, 947, 0, 1310))
    print("export purple    ", bbox(img, 860, 1100, 900, 1000, purple))
    print("selected thumb   ", bbox(img, 1315, 1680, 20, 140, purple))
    print("zoom icons       ", bright_runs(img, 935, 965, 720, 910, threshold=200))
    print("tool icons       ", bright_runs(img, 930, 970, 280, 510, threshold=200))

    edge = None
    for x in range(1200, width):
        if rows[500][x] == PANEL:
            edge = x
            break
    print("panel left edge  ", edge)

    for label, x in (("tool pill", 350), ("export", 960), ("corner left", 50)):
        column = []
        start = None
        for y in range(900, 1000):
            hit = any(abs(rows[y][x][i] - PAPER[i]) > 2 for i in range(3))
            if hit and start is None:
                start = y
            if not hit and start is not None:
                column.append((start, y - 1))
                start = None
        if start is not None:
            column.append((start, 999))
        print(f"{label:16} x={x} vertical runs {column}")


if __name__ == "__main__":
    main()
