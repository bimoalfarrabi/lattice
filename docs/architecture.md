# Architecture

## Shape of the system

```
lattice-core                 pure, testable, no window
  geom.rs                    points, cross product, epsilon, coordinate formatting
  guides.rs                  31 lattice generators and their refit variants
  arrange.rs                 planar arrangement: intersections to faces
  tiling.rs                  tile addressing, mirrors, hit index, shapes
  doc.rs                     drawing schema, validation, history, file I/O
  export.rs                  SVG writer and PNG rasterizer

lattice-app                  GPUI shell
  theme.rs                   both palettes
  icons.rs                   glyph paths
  canvas.rs                  painting order and pointer mapping
  studio.rs                  state, shell, pointer and key behaviour
```

The split is deliberate. Anything that can be decided without a window lives
in `lattice-core` and is covered by tests that need no GPU. The app layer holds
no geometry of its own; it asks `tiling::CellIndex` and `arrange` for shapes and
paints them.

## Data flow

1. `studio.rebuild` selects the current grid, builds a `CellIndex` for the
   active layout, and caches the guide list. It runs on start, on grid change,
   and after undo or redo.
2. Painting resolves a pointer position through `canvas::to_plane`, then
   `canvas::locate` undoes the tile mirror and the vertical squash. The result
   is a `(tile_x, tile_y, cell_index)` triple.
3. `studio.apply` converts that triple to a mark string and mutates
   `doc.current`. The first cell of a stroke records a history entry through
   `History::edit`; the rest of a pointer drag amend that same entry through
   `History::amend`, so one undo reverses the whole stroke.
4. Rendering rebuilds `MarkRef` values from the document, then paints paper,
   fills, rules, hover, and focus in that order.
5. Export serializes `doc.current` through `export::svg_for_with_ink` and writes
   next to the user's pictures directory.

## Invariants

- A mark is `"<index>"` or `"<tile_x>,<tile_y>:<index>"`. Plain indices always
  mean tile `(0, 0)`. `doc::Mark` owns the grammar.
- Cell indices are only valid for the grid and layout they were derived from.
  Switching to the original layout can change the cell count, so `rebuild`
  drops marks whose index falls out of range.
- The canvas paints rules after fills and after the hover and focus tints, so
  neither a fill nor a tint ever hides the lattice. The focus ring is a stroke
  and finishes on top.
- The guide list used for painting is the grid's guides plus `arrange::box_edges`.
  A lattice never includes a line on the square edge, so without those the tile
  boundary would be invisible and adjacent tiles would merge.
- Positive faces are kept; negative faces are holes and are appended to their
  smallest containing face. Hit testing relies on even-odd over the final
  rings.
- Every odd tile column and row is mirrored. `Placement::out` and
  `Placement::into` are exact inverses, and both are covered by a test.

## Rendering

`canvas::paint` receives a `Paint` struct with everything it needs and does no
lookups beyond slicing. The stage clones shapes, guides, and marks per frame at
present. That is measured fine at the current cell counts; the intended
optimization is a path cache keyed by `(tile, zoom bucket, kind)` with the
mirror baked in, so only the four parity variants are ever built.

Fills use `PathBuilder::fill` over every ring of a shape. Holes are later
subpaths wound the other way, so a nonzero fill cuts them out without extra
logic.

## Persistence

`paths::drawing_path` decides the location per platform: XDG data home on
Linux, Application Support on macOS, Roaming AppData on Windows. Exports use
`paths::export_dir`, which mirrors it with the pictures folder. Both accept a
`LATTICE_DATA_DIR` or `LATTICE_EXPORT_DIR` override, used by the tests.

`Drawing::save` writes a temp file and renames it, so a crash cannot leave a
half written document. Load falls back to the default drawing on any parse
error.

There is no debounce yet. Each edit writes the document once, and a drag
writes once per cell it crosses. Documents are
small and this is a local disk, but a debounce belongs here before the app is
used on network storage.

## Export

SVG output targets byte compatibility with the reference endpoint:

- `viewBox` is the bounding box of the used tiles, one tile is 240 units.
- Groups are ordered by first appearance of `(tile_x, tile_y, color)`.
- A `fill` attribute is written per path only for explicit colors; everything
  else inherits the group fill, which is the resolved ink.
- Odd tiles get `scale(-1)` on the matching axis.
- Negative tile coordinates are accepted. The reference server rejects them.

PNG is the same SVG rasterized by resvg with the long side capped at 2048.

## Failure handling

- Unknown grid: `CellIndex::build` returns `None`, the status line reports it,
  and the empty catalogue is never indexed blindly.
- Empty selection: export reports "paint a few cells first" and writes nothing.
- Export I/O errors surface in the status line with the underlying message.
- A corrupt drawing file cannot break startup: load falls back to default.
- A platform whose environment lacks the expected variables falls back to a
  relative path instead of failing to start.

## What the app does not own

Share links, accounts, and pricing belong to the original service and are
absent by design. The status line stands in for the four topbar actions until
those exist.
