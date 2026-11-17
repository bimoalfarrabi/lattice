# UI specification

Every number here was measured from the reference build with a computed style
dump and a DOM scan at a 1680 x 1000 viewport, then rounded to the size the
cascade actually resolved. Deviations are listed at the end with their reasons.

## Frame

- Outer gutter is the page colour itself, padding `8`, gap `8` between stage
  and panel.
- Stage and panel are separate rounded surfaces. Panel radius `18`, border
  `1px`, panel border colour `#ffffff17`.
- Panel width is fixed at `348`, inner padding `22` vertical and `18`
  horizontal. Stage takes the remaining width.
- Panel collapse removes it from layout and the stage expands. The toggle sits
  in the bottom right corner of the stage.

## Stage

- Canvas paper fills the stage, radius `18`, border `1px`.
- No dot pattern. The reference disables it for this tool.
- Reference framing: one 240 unit tile renders at `640` pixels, so a 15 unit
  cell is 40 pixels. Grids with two repeats render a tile at `320` pixels to
  keep cell density equal.
- Zoom indicator starts at `100%` and means the zoom multiplier, not a fit
  ratio.

## Canvas paint order

1. Paper.
2. Fills, per visible tile.
3. Hover tint `#9265dc` at `0.27` alpha.
4. Focus tint at `0.31` alpha.
5. Rules, one pixel, over the fills and the tints.
6. Focus ring, one pixel `#764cb6`, last.

Rules stay above fills and tints on purpose: adjacent cells must remain
separated, and a hovered cell must still show its lattice. The rules include the
four tile edges, so the lattice is continuous across the mirrored repeats.
Preview mode drops rules and hover and keeps only fills.

## Transport

Floating row, `22` above the bottom edge, centred, `10` gap between groups.
Every control is `44` high and the pills hug them with `5` pixels of horizontal
padding and no vertical padding, so a pill is `44` high too. Radius `999`,
border `#ffffff1a`, translucent float surface and a soft shadow.

Left to right:

1. Tool pill: pointer, hand, eraser, colour swatch.
2. History pill: undo, redo, trash.
3. Preview circle, `54` wide, accent tint when active.
4. Zoom pill: a magnifier with a minus, the percentage readout, a magnifier
   with a plus. The readout resets the view.
5. Export pill, split: `Export` and a chevron that exports PNG. Fill
   `#7950c8`, label `#f5f3ec`.

Active tools use the accent soft surface. Disabled controls use the muted ink.

## Top bar

Absolute, `16` from the top, `34` from the left, `22` from the right, `50` high.
Left: `LogoFX`, `BrandX` with a small crown, `CreateX`, at `18.5px` semibold in
`Inter Variable`, with `31` between the outer labels and `6` between `BrandX`
and the crown. Active label is ink; the others are muted.

The family is `Inter Variable`, not `Inter`. The bundled file is the variable
font and its name table calls the family `Inter Variable`. Asking for `Inter`
matches nothing: on Linux the text silently fell back to a system face, and on
Windows it panicked because the release build carried a test-only feature.

Right: four circular `42` buttons at `6` gap for share, contact, pricing, and
account.

## Panel

No heading. A three column gallery starts at the top padding. Thumbnails are
square, radius `5`, border `3` transparent, accent border when selected, line
art in the muted ink at `1.45` stroke. The gallery scrolls.

Below the gallery, a `44` high informational row: all 31 grids are unlocked.

## Corner buttons

Two detached circles, `42` each, `29` above the bottom edge: theme toggle at
`22` from the left, panel toggle at `22` from the right.

The window refuses to go below `1280` by `800`. At that size the transport is
`750` wide and the corner buttons clear it by about `15` pixels on each side.

## Tip

A capsule, `50` high, centred `84` from the top, `14` text, close button on the
right. Shown only while the current grid is empty.

## Colour palette

Anchored above the tool pill, `196` wide, radius `14`, panel surface. Nine `28`
swatches in a five column grid: automatic, then `#808080`, `#9470ed`, `#4a91ee`,
`#37be91`, `#f3cc4b`, `#fa9d4d`, `#f16e60`, `#e96da9`. Under the swatches sits
a full width hex field, `30` high, that accepts six hex digits from the
keyboard: click it, type, Enter applies, Escape cancels, Backspace edits.

The automatic swatch renders as a split circle of the paper and its contrast
partner.

## Interaction

- Pointer: toggles a cell. Starting a drag on a filled cell erases for the rest
  of the drag.
- Eraser: removes.
- Colour: paints the brush colour.
- Hand: pans. Space pans with any tool held.
- Wheel zooms around the pointer across ten steps from `25%` to `800%`.
- Keys: `1` pointer, `2` hand, `3` eraser, `4` colour, `Space` pan, arrows nudge,
  `Esc` leaves preview, `Ctrl+Z` and `Ctrl+Shift+Z` undo and redo,
  `Enter` toggles the focused cell.
- Trash arms on first click and clears on the second.

## Palettes

Dark, the default:

| Token | Value |
|---|---|
| gutter | `#101112` |
| paper | `#171819` |
| panel | `#1b1c1e` |
| panel control | `#292a2d` |
| border | `#ffffff17` |
| toolbar control | `#27282b` |
| toolbar hover | `#36373b` |
| accent | `#b99aff` |
| accent soft | `#332c40` |
| ink | `#f0f0f1` |
| muted | `#96989d` |
| crown | `#f2c66d` |
| export fill | `#7950c8` |

Light:

| Token | Value |
|---|---|
| gutter | `#e9e9eb` |
| paper | `#fafafa` |
| panel | `#f7f7f8` |
| panel control | `#e9e9ed` |
| border | `#1a1b1f1f` |
| toolbar control | `#e8e8ec` |
| toolbar hover | `#dadade` |
| accent | `#8960d7` |
| accent soft | `#eee7fb` |
| ink | `#202027` |
| muted | `#686a74` |
| crown | `#ae7417` |

Rules are mixed at 22 percent of ink into paper, the same formula the
reference cascade uses.

## Measured against the reference

A real screenshot of the target at 1680x1000 is kept at
`/tmp/opencode/recon-logofx/shot-01.png`. `tools/screenshot/compare.py` prints
the same geometry for any capture, so a difference is a number rather than an
impression.

| Element | Reference, window pixels | This build |
|---|---|---|
| Top bar text | LogoFX 43 to 107, BrandX 142 to 203, crown 212 to 225, CreateX 260 to 328 | 43 to 108, 142 to 203, 213 to 226, 260 to 328 |
| Top bar gaps | 35, 9, 35 | 34, 10, 34 |
| Transport row | 287 to 1042 | 286 to 1037 |
| Export pill | 903 to 1034 by 926 to 969, so 131 by 44 | 895 to 1037 by 925 to 968, so 142 by 44 |
| Zoom pill | 726 to 890, so 164 wide | 724 to 884, so 161 wide |
| Left corner button | 30 to 71 by 921 to 962 | 30 to 71 by 921 to 962 |
| Right corner button | 1252 to 1293 | 1252 to 1293 |
| Panel left edge | 1325 | 1325 |
| Floating pill surface | about `#242424` over the `#171819` paper, so roughly 5 percent white |
| Tool buttons | 44 square, 6 apart, about 5 pixels of horizontal padding inside the pill |

The reference zoom control is a magnifier with a minus and a magnifier with a
plus. The reference Export label is dark on purple, which measures 3.11:1 and
fails AA, so this build keeps the light label described below.

The Export pill is `142` wide here against `143` in the reference. The purple
fill measures `131` in the reference because its pill carries an inset ring;
this build fills the whole pill, so the fill and the pill are the same width.

## Deliberate deviations

| Deviation | Reason |
|---|---|
| Export label is `#f5f3ec` instead of `#21162e` | The reference pair measures 3.11:1, below AA. The new pair measures 4.98:1 with the fill unchanged |
| Cost of grid unlock is not shown | Everything is unlocked offline, so a price claim would be stale |
| Hex field instead of a native colour input | GPUI 0.2 has no text element; six keyboard digits cover the same need. The field takes its own row because seven characters do not fit a 28 pixel swatch |
| Rules above the hover and focus tints | A tint above the rules turned the hovered cell into a floating block with no lattice |
| The tile boundary is drawn on every lattice | The arrangement clips every face to the 240 unit box, so the boundary is a real cell edge. On a mosaic such as Voronoi the mirrored tile makes it read as a straight rule over the pattern. Removing it would join two distinct cells, and it measures the same weight as every other rule (`#2f3030` over paper) |
| No dot pattern on the stage | Matches the reference for this tool |
| No icon for the automatic swatch | A split circle reads better than a label at 28 pixels |
