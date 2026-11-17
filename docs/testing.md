# Testing

## Running

```sh
cargo test
```

The suite is 28 tests and needs no display, network, or GPU. Tests that write
files point `LATTICE_DATA_DIR` and `LATTICE_EXPORT_DIR` at a temporary folder,
so a run never touches the real home directory.

## Coverage

Geometry, in `lattice-core`:

- `parity::matches_the_reference_cells` compares every grid in both layouts
  against the extracted reference, 62 cases and 19 982 cells, byte for byte.
- `tiling_parity::matches_the_reference_placements` compares 25 placements and
  4 coverage rectangles.
- `tiling::tests` covers mirror round trips, coverage at boundaries, index hits,
  and hole separation.

Document, in `lattice-core`:

- round trip through JSON
- canonical mark formatting
- out of range index dropping
- legacy ink collapse
- fallback to default on bad input
- undo history capped at 80

Export, in `lattice-core`:

- golden file reproduced byte for byte
- negative tiles accepted
- `auto` ink resolved through the caller
- unknown grid and empty marks rejected
- PNG rasterization
- undo and redo round trip

Application, in `lattice-app`:

- `controls_change_their_state` clicks every toolbar, panel, and corner control
  by element id and asserts the state change: tools, swatch, grid switch, zoom,
  preview, panel toggle, theme.
- `painting_then_exporting_writes_a_file` clicks the canvas, asserts one mark,
  presses `Ctrl+Z`, then exports and reads the written SVG.
- `clicking_paints_and_undo_reverses_it` is the same click path without export.
- `behaviour.rs` covers what a single click cannot: hex entry with valid,
  invalid and cancelled input, the two step trash confirmation, a drag painting
  a run of cells and one undo reversing the whole stroke, the drawing surviving
  a restart, PNG export contents, and the zoom steps.
- `paths::tests` checks that the environment overrides win over the platform
  defaults.

The behaviour tests hold a process wide mutex, because `LATTICE_DATA_DIR` and
`LATTICE_EXPORT_DIR` are environment variables and the tests run on parallel
threads. Each test also points both at its own temporary directory, so a run
never reads the real drawing.

## Screenshots

No screenshot tool is installed on this machine, so `tools/screenshot/grab.py`
talks to libX11 through ctypes: it picks the client window, reads it with
`XGetImage` and writes a PNG with zlib. Reading the root window fails here with
BadMatch, which is why the script targets the client.

```sh
# in one shell
env -u WAYLAND_DISPLAY LATTICE_SHOT=dark \
  LATTICE_DATA_DIR=/tmp/shots/data LATTICE_EXPORT_DIR=/tmp/shots/out \
  ./target/debug/lattice-app &

# in another
python3 tools/screenshot/grab.py /tmp/shots/dark.png
```

`LATTICE_SHOT` paints a fixed drawing so captures are comparable. Tokens:
`light`, `preview`, `nopanel`, `voronoi`, `empty`, combined with commas.
`LATTICE_SIZE=WxH` sets the window, which is how the narrow layouts are
captured. The grabber parks the pointer in the corner of the screen first, so a
capture never contains a hover tint that depends on where the mouse was.

`tools/screenshot/compare.py` prints the layout geometry of a capture, so a
difference against the reference is a number. `tools/screenshot/pngread.py` is
the decoder it shares with the grabber.

Six states were captured and reviewed on 16 November 2026: dark, light,
preview, voronoi, empty, and dark without the panel. That pass found five
defects that the test suite could not see, all now fixed and listed in
[decisions.md](decisions.md) entry 18.

Captures at 1280x820, 1000x700 and 860x640 then found a sixth: the transport row
is fixed width and clips its ends below roughly 1100 pixels, and the detached
corner buttons overlap it. The responsive pass that fixes this is described in
entry 19.

## Regenerating fixtures

Only when the geometry is meant to change.

```sh
cd tools/parity
node extract.mjs
node dump.mjs
node dump_tiling.mjs
cd ../..
cargo test -p lattice-core
```

`extract.mjs` slices functions out of
`/tmp/opencode/recon-logofx/pretty-CreateX-BY5jD11_.js`. If the recon directory
is gone, recapture it first.

## Manual checks the suite cannot do

1. Open the app and look at it. Nothing here has been checked by eye: the theme
   pairs, the gallery density, the shadow depth, the icon weights.
2. Resize the window narrow and confirm the stage does not collapse into
   nonsense before the panel does.
3. Try a drag across a filled shape and confirm it erases for the whole drag.
4. Zoom to 800 percent, pan away, and reset with the readout.
5. Export a grid with a non-square aspect, for example `voronoi` with marks on
   two rows, and open the SVG.
6. Toggle both themes with the gallery open and check contrast on the selected
   thumbnail border.

## Packaging

`cargo test` does not cover the installers. What was checked, and where:

- Linux, locally: the `.deb` is built, extracted with `dpkg-deb`, and the
  binary runs from the extracted path. The desktop entry passes
  `desktop-file-validate`. The tarball is unpacked and run the same way, and
  the released tarball was downloaded again and run once more.
- Windows and macOS: built in CI only. The setup is a valid NSIS
  self-extractor, both disk images carry a valid UDF trailer, and the bundle
  passes `codesign --verify` during the build. Nothing has launched on either
  system yet, so treat those two installers as untested by hand.

The CI matrix also refuses a `v*` tag whose name does not match the version in
`Cargo.toml`, so a release cannot be published under the wrong number.

The Windows package was run once from WSL against the installed binary: it
started, painted the screenshot state, and wrote `%APPDATA%\Lattice\drawing.json`
with the expected marks. That is what caught the font panic described in
[decisions.md](decisions.md) entry 22.

## Tracing pointer problems

Setting `LATTICE_TRACE` to a file path makes the studio append a line for every
pointer move, hover change and recomputed hover cell. It is the only way to see
what a window on another platform is doing when the symptom only appears there.

```sh
LATTICE_TRACE=/tmp/lattice-trace.log ./target/release/lattice-app
```

## Build size

`[profile.dev]` sets `debug = "line-tables-only"`. Full debug info pushed
`target/debug` to 79 GB on this machine, which filled the Windows disk behind
WSL. Line tables keep backtraces readable and the same build is 2.8 GB.

## Not covered

- IME input, since there is no text element.
- Multi monitor and HiDPI scale factors.
- Wayland sessions, because the local compositor is too old for GPUI.
- Windows and macOS builds. `lattice-core` is type-checked for
  `x86_64-pc-windows-msvc`, the app crate is not, and neither macOS path has
  been compiled.
