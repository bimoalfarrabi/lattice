# Decisions

Recording why, so a later reader does not have to guess.

## 1. Clone CreateX and not BrandX

The Threads post is CreateX. BrandX is the sibling with 29 effects, mockups, and
three.js rendering. Reimplementing both would triple the surface for little
gain, so BrandX is out of scope until CreateX stands on its own.

## 2. All 31 lattices unlocked

The paid gate exists because the original sells grids. Here there is no server,
so there is nothing to check and nothing to sell. The panel lists everything and
the informational row says so.

## 3. Local only, no account, no share link

Share in the original is a server blob behind a membership. A local app can
export a file instead. Account, pricing, and share are represented by the four
topbar controls reporting status until they have real destinations.

## 4. Port the geometry exactly instead of redesigning it

Writing a cleaner tessellator would have produced different cells and lost the
document compatibility. The engine is a line by line port, and
`tools/parity` proves it against the original in 19 982 cells.

## 5. Parity fixtures are golden files

`dump.mjs` and `dump_tiling.mjs` bake the reference output into the repository.
A geometry change must keep them green or regenerate them with a reason. This
turns "looks the same" into a byte comparison.

## 6. Pure core, thin shell

All geometry, document rules, and export live in `lattice-core` with no window
dependency. The app only wires state to elements. This is what lets the suite
run without a GPU.

## 7. The canvas is an infinite mirrored plane

It follows from the document format: marks carry tile coordinates. Rendering a
single tile would have flattened the data model.

## 8. Rules paint above fills

Cells that touch must stay separated, which is how the reference reads. It also
means a fill never erases the lattice.

## 9. Preview is a paint mode, not a second view

Preview simply omits rules and hover and keeps the fills. A second tree would
have duplicated state for no benefit.

## 10. X11 on this machine, Wayland kept compiled in

The local compositor advertises `xdg_wm_base` version 1 and GPUI requires at
least 2. Both backends stay in the feature list so a newer session works.

## 11. `libxkbcommon-x11.so` symlinked into `vendor/lib`

The runtime package ships `libxkbcommon-x11.so.0` without the development
symlink and the linker wants `.so`. A repo local symlink plus a
`rustc-link-search` flag avoids requiring a system change. Installing the
development package makes the symlink harmless.

## 12. Inter is bundled

The reference is set in Inter and nothing like it is installed here. The
variable font ships with the app under the OFL, loaded at startup, and the root
element sets the family.

## 13. Export label colour moved to `#f5f3ec`

The reference pair `#21162e` on `#7950c8` measures 3.11:1 and fails AA at 13
pixels. Keeping the purple and switching the label to `#f5f3ec` reaches 4.98:1.
The fill token is untouched, so the button still reads like the reference.

## 14. Hex colour entry is keyboard driven

GPUI 0.2.2 has no text element and no IME plumbing at the app layer. Six hex
digits through the key handler cover the need without pulling in a fork of a
larger editor component.

## 15. Persistence without a debounce, for now

Each edit writes once with a temp file and rename. Documents are a few
kilobytes. A debounce is the next step before use on network storage.

## 16. Handwritten icons

GPUI has an SVG element that needs an asset source. Redrawing twenty glyphs as
paths kept the bundle small and gave control over stroke weight without a build
step.

## 17. Platform work sits behind small gates, not a rewrite

The engine and the UI never touch the operating system directly. The two
places that do are isolated:

- `crates/lattice-app/build.rs` adds the vendored `libxkbcommon-x11` search path
  only when the target OS is Linux, so `.cargo/config.toml` could be dropped.
  A Windows or macOS build never sees a Linux path.
- `crates/lattice-app/src/paths.rs` resolves the drawing and export folders
  with one `cfg` branch per platform, plus environment overrides for tests.

`lattice-core` type-checks for `x86_64-pc-windows-msvc`. The app crate cannot
be cross-checked here, because GPUI pulls `ring` and `libz-sys`, whose build
scripts want a C compiler for the target.

## 18. The first visual pass changed five things

Capturing the running app and reading the pixels found defects that no unit
test could catch. Each is now covered by a number in the spec, not by taste.

1. Translucent tokens went through `gpui::rgb`, which keeps only the low 24
   bits, so `rgb(0xffffff17)` rendered as opaque yellow and every panel border
   was bright yellow. They use `gpui::rgba` now.
2. The painted guide list did not include the square edges, so the tile
   boundary, which is 640 pixels on the default scale, had no rule and adjacent
   tiles merged into one another. `arrange::box_edges` is now shared by the
   arrangement and the painter.
3. Hover and focus tints were painted above the rules, which turned a tinted
   cell into a block with no lattice inside it. Tints moved below the rules and
   the focus ring stayed on top.
4. A pointer drag pushed one history entry per cell, so `Ctrl+Z` undid a single
   cell of a stroke. `History::amend` mutates the current drawing without
   recording, and `apply` records only the first cell.
5. Thumbnails of the two-repeat grids drew one half-size tile in the corner of
   the gallery box. The preview now tiles the mirrored plane to fill the box at
   the same cell density as the stage.

## 19. The window has a floor, and the layout was matched to the target

A real screenshot of the target at 1680x1000 was compared against a capture of
this build with `tools/screenshot/compare.py`. The transport and the top bar
were off: pills 54 high instead of 44, Export 110 wide instead of 131, top bar
text 18px with 8 pixel gaps instead of 20px with 35, and the floating surface
opaque `#1b1c1e` instead of a translucent white over paper. All of it is
corrected and the numbers are recorded in [ui-spec.md](ui-spec.md).

Below roughly 1100 pixels the transport clipped its ends and the detached
corner buttons overlapped it. Rather than build a second layout, the window now
refuses to go below 1280 by 800, where the transport is 750 wide and clears the
corner buttons by about 15 pixels on each side. A narrower layout is a real
feature, not a bug fix, and it is not built.

## 20. Mouse events are dispatched to every listener, so the chrome stops them

GPUI sends a mouse event to every listener in the frame, in reverse paint order,
and only stops when a handler calls `cx.stop_propagation()`. The canvas covers
the whole stage, so every click on the floating transport also painted or
erased the cell underneath it. The behaviour tests caught it: clicking zoom-in
added a mark.

The transport, top bar, panel, palette, tip and both corner buttons now stop
propagation on mouse down. They are painted after the canvas, so their handlers
run first and the canvas never sees a click that belongs to the chrome.

## 21. Installers wrap the release binary that CI already builds

Linux gets a Debian package and a portable tarball, Windows an NSIS setup, and
macOS an ad hoc signed bundle in a disk image. Each one wraps the binary its
build job already produced, so there is one build and no second pipeline.

- The Debian package is assembled with `dpkg-deb` instead of a Rust packaging
  crate, so the only tool needed is one every Debian system already has.
- The Windows setup installs per user under `LOCALAPPDATA`. No administrator
  prompt and no machine wide registry writes, which is what a single user
  desktop app wants.
- The macOS bundle is signed ad hoc with `codesign -s -`. Apple Silicon
  requires a signature before a binary runs at all. It is not notarised, so the
  first launch needs a right click and Open. Notarising needs an Apple
  Developer account this project does not have, and a fake claim of a trusted
  signature would be worse than an honest one.
- `tools/icon/make_icons.py` draws the application icon and writes PNG, ICO and
  ICNS. ICO entries are stored as uncompressed bitmaps because the NSIS loader
  refuses PNG compressed ones, and the downscale is a box filter so 16 and 32
  pixels stay clean.
- `makensis` changes its working directory to the script's directory, so
  `lattice.nsi` runs `!cd` back to the repository root before touching any
  relative path. Without it the icon resolves inside `packaging/windows`.

## 22. The font family is Inter Variable, and test features stay out of the release

The bundled file is `InterVariable.ttf`, and its name table calls the family
`Inter Variable`. The app asked for `Inter`. Two different failures came from
that one mistake:

- On Linux the lookup matched nothing and the text fell back to a system face
  without a warning. Every layout measurement in this document was therefore
  tuned against the wrong font, which is how the top bar ended at 20 pixels
  while the reference is nearer 18.5.
- On Windows the same lookup panicked instead of falling back, because gpui's
  `test-support` feature was enabled in the release build, and that feature
  turns a missing font into a panic. The installed application closed the
  moment it was launched.

Both are fixed. The family is `Inter Variable`, the release no longer carries
`test-support` (it moved to `[dev-dependencies]`, where the test context
needs it), and the top bar is `18.5px`, which now matches the reference within
a pixel on every label.

The lesson worth keeping: check what font actually resolved rather than what
was requested. `cx.text_system().all_font_names()` answers that in one line.

## 23. The hover cell is derived, not pushed, and one case remains

The highlight was written only by mouse move events, so its accuracy depended
on the order in which the platform delivered move and hover notifications. On
Windows that produced a highlight several cells away from the pointer: with the
0.1.1 build a pointer at 700,500 lit the cell at 943,261.

The studio now keeps the last pointer position and whether the pointer is over
the canvas, and recomputes the hovered cell from both whenever either changes.
On Windows the highlight now sits on the cell under the pointer for every
position tested.

One case is still open. When the pointer leaves the window without entering
another element, this build does not receive a mouse leave event, so the last
cell stays tinted until the pointer returns. `Window::is_window_hovered` and the
element hover callbacks both depend on that same message, and the raw cursor
position is not exposed, so the app cannot detect it without polling the
platform directly. `LATTICE_TRACE` exists to confirm this on a machine that can
reproduce it.

## Open decisions

1. The name Lattice and the icon style were chosen during implementation and
   have not been confirmed (see R-23 of the delivery gate).
2. The requested extensions are only partly built: custom lattice density,
   saved presets, drawing import, PNG size choice, and background colour remain.
3. No visual review has happened yet, since this machine has no screenshot
   tool. Someone has to look before the UI claims to be finished.
4. The visual pass was made at 1680x1000 and 1280x800. HiDPI scaling and the
   800 percent zoom have not been looked at.
5. Windows and macOS app builds are unverified. The code is plumbed for both,
   but a native toolchain on either machine is the only way to confirm.
