#!/usr/bin/env python3
"""Grabs the studio window from an X11 display and writes a PNG.

There is no screenshot tool installed on this machine, so this talks to libX11
through ctypes. It walks the window tree to the largest viewable top level
window, descends to its largest mapped child, which is the client area without
window manager decorations, and encodes that rectangle with zlib.

Reading the root window fails here with BadMatch, so every grab targets a
client window. A no-op X error handler keeps a failed request from aborting the
process; the script reports the null image instead.

Usage:
    python3 tools/screenshot/grab.py out.png [--pointer X Y]

`--pointer` places the mouse at that window relative point before the grab,
which is how hover states are captured. Without it the pointer is parked in the
corner of the screen so no hover tint appears.

Prints the size, the distinct colour count and the mean red channel of one row,
which is enough to tell a real capture from a black or empty one.
"""

import ctypes
import ctypes.util
import struct
import sys
import time
import zlib

ZPIXMAP = 2
IS_VIEWABLE = 2


class XWindowAttributes(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.c_int),
        ("y", ctypes.c_int),
        ("width", ctypes.c_int),
        ("height", ctypes.c_int),
        ("border_width", ctypes.c_int),
        ("depth", ctypes.c_int),
        ("visual", ctypes.c_void_p),
        ("root", ctypes.c_ulong),
        ("klass", ctypes.c_int),
        ("bit_gravity", ctypes.c_int),
        ("win_gravity", ctypes.c_int),
        ("backing_store", ctypes.c_int),
        ("backing_planes", ctypes.c_ulong),
        ("backing_pixel", ctypes.c_ulong),
        ("save_under", ctypes.c_int),
        ("colormap", ctypes.c_ulong),
        ("map_installed", ctypes.c_int),
        ("map_state", ctypes.c_int),
        ("all_event_masks", ctypes.c_long),
        ("your_event_mask", ctypes.c_long),
        ("do_not_propagate_mask", ctypes.c_long),
        ("override_redirect", ctypes.c_int),
        ("screen", ctypes.c_void_p),
    ]


class XImage(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_int),
        ("height", ctypes.c_int),
        ("xoffset", ctypes.c_int),
        ("format", ctypes.c_int),
        ("data", ctypes.POINTER(ctypes.c_char)),
        ("byte_order", ctypes.c_int),
        ("bitmap_unit", ctypes.c_int),
        ("bitmap_bit_order", ctypes.c_int),
        ("bitmap_pad", ctypes.c_int),
        ("depth", ctypes.c_int),
        ("bytes_per_line", ctypes.c_int),
        ("bits_per_pixel", ctypes.c_int),
        ("red_mask", ctypes.c_ulong),
        ("green_mask", ctypes.c_ulong),
        ("blue_mask", ctypes.c_ulong),
        ("obdata", ctypes.c_void_p),
        ("funcs", ctypes.c_void_p * 6),
    ]


def load():
    x11 = ctypes.CDLL(ctypes.util.find_library("X11"))
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XDefaultScreen.restype = ctypes.c_int
    x11.XDefaultScreen.argtypes = [ctypes.c_void_p]
    x11.XRootWindow.restype = ctypes.c_ulong
    x11.XRootWindow.argtypes = [ctypes.c_void_p, ctypes.c_int]
    x11.XQueryTree.restype = ctypes.c_int
    x11.XQueryTree.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),
        ctypes.POINTER(ctypes.c_uint),
    ]
    x11.XGetWindowAttributes.restype = ctypes.c_int
    x11.XGetWindowAttributes.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.POINTER(XWindowAttributes),
    ]
    x11.XGetImage.restype = ctypes.POINTER(XImage)
    x11.XGetImage.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_uint,
        ctypes.c_uint,
        ctypes.c_ulong,
        ctypes.c_int,
    ]
    x11.XFree.argtypes = [ctypes.c_void_p]
    x11.XSetErrorHandler.restype = ctypes.c_void_p
    x11.XFetchName.restype = ctypes.c_int
    x11.XFetchName.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.POINTER(ctypes.c_char_p),
    ]
    x11.XInternAtom.restype = ctypes.c_ulong
    x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    x11.XGetWindowProperty.restype = ctypes.c_int
    x11.XGetWindowProperty.argtypes = [
        ctypes.c_void_p,
        ctypes.c_ulong,
        ctypes.c_ulong,
        ctypes.c_long,
        ctypes.c_long,
        ctypes.c_int,
        ctypes.c_ulong,
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.c_int),
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.c_ulong),
        ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte)),
    ]
    return x11


def window_name(x11, display, window):
    name = ctypes.c_char_p()
    if x11.XFetchName(display, window, ctypes.byref(name)) and name.value:
        return name.value.decode("utf8", "replace")
    return ""


def net_name(x11, display, window):
    """_NET_WM_NAME, which is what most toolkits set instead of WM_NAME."""
    net_wm_name = x11.XInternAtom(display, b"_NET_WM_NAME", 0)
    utf8 = x11.XInternAtom(display, b"UTF8_STRING", 0)
    actual_type = ctypes.c_ulong()
    actual_format = ctypes.c_int()
    count = ctypes.c_ulong()
    after = ctypes.c_ulong()
    prop = ctypes.POINTER(ctypes.c_ubyte)()
    status = x11.XGetWindowProperty(
        display, window, net_wm_name, 0, 1024, 0, utf8,
        ctypes.byref(actual_type), ctypes.byref(actual_format), ctypes.byref(count),
        ctypes.byref(after), ctypes.byref(prop),
    )
    if status != 0 or not prop:
        return ""
    text = ctypes.string_at(prop, count.value).decode("utf8", "replace")
    x11.XFree(prop)
    return text


def find_named(x11, display, window, wanted, depth=0):
    for child in children(x11, display, window):
        name = net_name(x11, display, child) or window_name(x11, display, child)
        if wanted.lower() in name.lower():
            return child
        if depth < 3:
            found = find_named(x11, display, child, wanted, depth + 1)
            if found is not None:
                return found
    return None


def silence_errors(x11):
    """A failed request must not end the process, Xlib's default handler exits."""
    handler = ctypes.CFUNCTYPE(ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p)
    keep = handler(lambda display, event: 0)
    x11.XSetErrorHandler(keep)
    return keep


def children(x11, display, window):
    root = ctypes.c_ulong()
    parent = ctypes.c_ulong()
    kids = ctypes.POINTER(ctypes.c_ulong)()
    count = ctypes.c_uint()
    x11.XQueryTree(
        display,
        window,
        ctypes.byref(root),
        ctypes.byref(parent),
        ctypes.byref(kids),
        ctypes.byref(count),
    )
    return [kids[index] for index in range(count.value)]


def viewable(x11, display, window):
    attributes = XWindowAttributes()
    if not x11.XGetWindowAttributes(display, window, ctypes.byref(attributes)):
        return None
    if attributes.map_state != IS_VIEWABLE:
        return None
    return attributes


def root_size(x11, display, root):
    attributes = XWindowAttributes()
    x11.XGetWindowAttributes(display, root, ctypes.byref(attributes))
    return attributes.width, attributes.height


def pick_client(x11, display, root):
    """Client area of the window titled for this app, else the largest on screen."""
    named = find_named(x11, display, root, "lattice")
    if named is not None:
        attributes = viewable(x11, display, named)
        if attributes and attributes.width * attributes.height > 20000:
            window = named
            while True:
                inner = [
                    (a.width * a.height, w, a)
                    for w in children(x11, display, window)
                    if (a := viewable(x11, display, w)) and a.width * a.height > 20000
                ]
                if not inner:
                    break
                _, window, attributes = max(inner, key=lambda entry: entry[0])
            return window, attributes.width, attributes.height

    top = [
        (a.width * a.height, w, a)
        for w in children(x11, display, root)
        if (a := viewable(x11, display, w)) and a.width * a.height > 100000
    ]
    if not top:
        return None

    _, window, attributes = max(top, key=lambda entry: entry[0])
    while True:
        inner = [
            (a.width * a.height, w, a)
            for w in children(x11, display, window)
            if (a := viewable(x11, display, w)) and a.width * a.height > 100000
        ]
        if not inner:
            break
        _, window, attributes = max(inner, key=lambda entry: entry[0])

    return window, attributes.width, attributes.height


def chunk(tag, payload):
    return (
        struct.pack(">I", len(payload))
        + tag
        + payload
        + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
    )


def encode_png(width, height, rows):
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    body = b"".join(b"\x00" + row for row in rows)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(body, 6))
        + chunk(b"IEND", b"")
    )


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: grab.py out.png [--pointer X Y]")

    pointer = None
    if "--pointer" in sys.argv:
        index = sys.argv.index("--pointer")
        pointer = (int(sys.argv[index + 1]), int(sys.argv[index + 2]))
        del sys.argv[index : index + 3]

    x11 = load()
    keep = silence_errors(x11)
    display = x11.XOpenDisplay(None)
    if not display:
        raise SystemExit("cannot open display, check DISPLAY")

    root = x11.XRootWindow(display, x11.XDefaultScreen(display))

    # Park the pointer in the bottom right corner, outside the window, so the
    # capture does not include a hover tint that depends on where the mouse was.
    root_width, root_height = root_size(x11, display, root)
    if pointer is None:
        x11.XWarpPointer(display, 0, root, 0, 0, 0, 0, root_width - 2, root_height - 2)
        x11.XFlush(display)
        time.sleep(0.4)
    target = pick_client(x11, display, root)
    if not target:
        raise SystemExit("no window large enough to grab")
    window, width, height = target

    if pointer is not None:
        at_x = ctypes.c_int()
        at_y = ctypes.c_int()
        child = ctypes.c_ulong()
        x11.XTranslateCoordinates(
            display, window, root, pointer[0], pointer[1],
            ctypes.byref(at_x), ctypes.byref(at_y), ctypes.byref(child),
        )
        x11.XWarpPointer(display, 0, root, 0, 0, 0, 0, at_x.value, at_y.value)
        x11.XFlush(display)
        time.sleep(0.8)

    image = x11.XGetImage(
        display, window, 0, 0, width, height, ctypes.c_ulong(0xFFFFFFFFFFFFFFFF), ZPIXMAP
    )
    if not image:
        raise SystemExit("XGetImage returned null for the client window")

    stride = image.contents.bytes_per_line
    pixels = ctypes.string_at(image.contents.data, stride * height)
    x11.XFree(image)
    del keep

    rows = []
    for row in range(height):
        line = pixels[row * stride : row * stride + width * 4]
        interleaved = bytearray(width * 3)
        interleaved[0::3] = line[2::4]
        interleaved[1::3] = line[1::4]
        interleaved[2::3] = line[0::4]
        rows.append(bytes(interleaved))

    with open(sys.argv[1], "wb") as handle:
        handle.write(encode_png(width, height, rows))

    sample = rows[len(rows) // 2][::3]
    print(
        f"{sys.argv[1]} {width}x{height} "
        f"distinct={len(set(sample))} mean={sum(sample) / max(1, len(sample)):.0f}"
    )


if __name__ == "__main__":
    main()
