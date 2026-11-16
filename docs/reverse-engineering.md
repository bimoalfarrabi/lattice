# Reverse engineering notes

## Target

`https://logofx.app/createx`, reached from a Threads post by `deadrabbbbit` on
16 November 2026. The product is CreateX, one of three tools by LogoFX. It
sells the full grid set, which is why this reimplementation exists.

## Method

Browser and static analysis with REA plus a scripted Playwright session.

- A Chromium instance ran with a CDP endpoint on loopback, driven by
  `rea.list_browser_targets`, `rea.inspect_web_page` and
  `rea.capture_web_screenshot`.
- Runtime evidence: `ev_fea3e9329f6cb0c9fd4b1833e174bd636a3c22363cde169035cec4e45d16d1aa`.
- Static analysis of the extracted bundle: `ev_8af41d9a3fe0ed30f90572737f27e76c739defd3902ed10bb67c192d4078ee41`.

Recon artifacts live in `/tmp/opencode/recon-logofx`: the SSR HTML, the control
and body text dumps, formatted copies of the studio and engine modules, the
style sheets, and a golden export. Regenerate that directory if it is gone.

The app is a TanStack Start SSR build with client hydration. The studio module
is `/assets/CreateX-*.js`; the geometry and document rules all sit in it. The
rendering stack is React with Tailwind style sheets, but none of that mattered
for the port beyond reading the cascade for tokens.

## Findings that shaped the port

### Document schema

`createx-drawing-v2` stores `grid`, `ink` (`auto` or six hex digits),
`originalLayouts`, `marks` keyed by grid, and `cellColors` keyed by grid and
mark. A mark is `"<index>"` or `"<tile_x>,<tile_y>:<index>"`.

Marks are per grid, not per tile only. One drawing therefore re-projects onto
every lattice, which is the product's core idea.

### Endless mirrored plane

The canvas is not one tile. It repeats a 240 unit tile with a mirror on odd
columns and rows, and a pointer position is mapped back through that mirror
before hit testing. This is why a mark needs tile coordinates.

### Geometry as code

All 31 lattices are generated from line and circle guides and cut by a
half-edge arrangement. Six grids never appear in the gallery and are reachable
only through a loaded drawing.

### Gating

Nine grids are free. Export of any other grid answers `401` without a
membership, and the panel lists only the free ones. Export of free grids is
clean, with no watermark.

### Export format

`POST /api/createx/export` takes `grid`, `originalLayout`, `marks`, `fill` and
`colors`, and returns `image/svg+xml`. The SVG is a bounding box of the used
tiles, one group with the ink, and one path per tile and colour with a mirror
transform. PNG is that SVG rasterized in the browser at a 2048 maximum.

`fill` must be six hex digits. Three digit values answer `400`.

### Defect found

Negative tile coordinates answer `400` with an empty body, so a drawing painted
left of or above the origin cannot be exported. Lattice accepts them.

### Layout and tokens

Measured through computed styles, the DOM, and a screenshot audit. The full
table is in [ui-spec.md](ui-spec.md).

## Coverage and limits

Observed directly:

- bundle contents, module graph, and every geometry and document function
- the export contract, including a saved golden response
- UI structure, sizing, colours, and control inventory
- the free and paid split, and the panel filtering rule

Inferred, not executed:

- server side validation beyond the checks hit during testing
- share link storage; it is an opaque server ID, so the shape was not needed
- behaviour for signed in accounts, since no session was created

Unknown:

- exact production CDN and cache headers
- server side PNG sizing, because PNG is client side

## Licensing posture

The geometry is reimplemented from observable behaviour, not copied as source.
The UI is reproduced from measurements. Icons were redrawn after the Lucide set
(ISC). Inter is bundled under the SIL Open Font License. The name Lattice is new.
