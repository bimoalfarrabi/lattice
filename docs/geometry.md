# Geometry

The lattice is not stored. It is computed from a small set of lines and circles
and cut into faces by a planar arrangement. This page documents that pipeline
precisely, because any drift from the reference changes cell identity and
breaks the parity fixtures.

## Units and constants

- The working square is `240 x 240` units, origin top left, y down.
- `EPS = 1e-7` decides degeneracy everywhere. There is no other tolerance.
- `CENTER = 120`, the middle of the square, used by radial generators.
- `BOX_HI = 240.0000001`, the upper bound for the containment test.

Coordinates are formatted with five decimals and trailing zeros dropped, by
`geom::num`. It scales by 100000, rounds, and rebuilds the digit string with
integer math, so there is no second rounding. `geom::key` joins two such
coordinates with a space. Node identity in the arrangement is this string,
which is why arithmetic order matters: two points that round to the same key
are the same node.

## Guides

A guide is either a line segment or a circle, both clipped to the square.

```rust
enum Guide {
    Line { a: Point, b: Point },
    Circle { center: Point, r: f64 },
}
```

`guides.rs` builds all 31 lattices from five primitives:

| Primitive | Meaning |
|---|---|
| `family(angle, spacing, count, offset)` | Parallel lines at an angle, clipped by the square, duplicates and edge coincidences dropped |
| `criss_cross(angle, spacing, count)` | `family` at the angle and at its negation |
| `rings(count, step, cx, cy)` | Concentric circles spaced by `step` |
| `square_mesh(spacing)` | Two perpendicular families |
| `iso_mesh(spacing)` | One vertical family plus a 30 degree cross |

`family` walks `-count..=count`, computes the line offset, drops candidate
endpoints that are out of bounds or within `EPS` of an existing one, requires
exactly two surviving endpoints, and rejects lines that lie on a square edge.

Grids that need more (Voronoi, honeycomb, spiral, quadtree, and the rest) have
their own generator. They follow the same rules: clipped to the square, ordered
deterministically.

## Refit variants

Fifteen grids carry a second guide set. The default set is the loose refit at a
15 unit module; the original set is the denser authored layout. The document
records which grids use their original layout, and export and rendering pick
the same set the index was built from.

## The arrangement

`arrange::arrange` takes the guide list and adds the four square edges as guides
first. The edge order is top, right, bottom, left, and that order decides the
order faces are discovered, which is the order cells are indexed in.

### 1. Cut placement

Each guide starts with seeding cuts: `[0, 1]` for a line, the four quadrants for
a circle. Every pair of guides is intersected:

- line and line: one point when the determinant exceeds `EPS`
- circle and circle: zero, one, or two points, with tangent cases collapsed
- line and circle: zero, one, or two points along the line parameter

A candidate is kept only when it is inside the square and inside both source
segments. Kept points are registered as cuts: a line gets a clamped parameter, a
circle gets its angle.

### 2. Splitting

Cuts are sorted and deduplicated to `EPS`. Circles get a full turn appended so
the last arc closes. Every consecutive pair becomes two half edges with opposite
direction, sampling circles every 0.08 radians so hit testing and hole probing
have points to work with.

### 3. Nodes

Each half edge is filed in a bucket keyed by the formatted start point. A node
bucket therefore holds every edge leaving that point.

### 4. Face walking

Buckets are sorted by angle, then by curvature. Starting from each unused edge,
the walk repeatedly:

1. marks the edge visited and appends its command to the path,
2. looks up the bucket of its end point,
3. takes the edge immediately before its twin in that bucket,
4. stops when it reaches an already visited edge.

Taking the previous edge keeps the traced face on one side, which is what makes
the walk produce faces rather than cross through an intersection.

### 5. Area and holes

Areas sum while walking. Faces under `1e-5` are dropped. Positive faces are
kept. Each negative face is a hole and is appended to the smallest positive
face that is bigger than it and contains its first point. The hole keeps its
opposite winding, so a nonzero fill renders it as a hole. Hit testing uses
even-odd, which agrees for opposite winding.

## Tiling

The plane repeats the square with mirrors on odd columns and rows.

```
origin_x = (tile_x + odd(tile_x)) * 240
flip_x   = -1 when tile_x is odd, else 1
```

`Placement::out` maps tile local coordinates into the plane and `Placement::into`
maps back. Both are tested to round trip over `-3..=3`.

`covering` lists the tiles a plane rectangle touches, using `floor` for the
start and `ceil` for the end, so a boundary pixel belongs to two tiles.

Every face is clipped to the box, so a tile boundary is a genuine cell edge.
Painting therefore draws `arrange::box_edges` alongside the lattice guides. On
grids whose pattern meets the box (the square families) it is invisible because
a guide already sits there. On a mosaic such as Voronoi it shows as a straight
rule, which is correct: the cell on the other side is a different, mirrored
cell, and without the rule the two would read as one polygon.

## Hit testing

`CellIndex` buckets cell bounding boxes into a `16 x 16` grid over the square.
A probe picks one bucket and tests candidates in index order with an even-odd
crossing count over every ring, after a bounding box rejection.

Holes therefore behave correctly: the area inside a circle cut out of a square
is a different cell, and a probe there hits that cell, not the square.

## Parity

`tools/parity/extract.mjs` slices the geometry functions out of the deployed
bundle into `engine.mjs`. `dump.mjs` and `dump_tiling.mjs` write golden fixtures
for all grids in both layouts and for tile transforms and coverage.

`cargo test -p lattice-core` compares cell path strings byte for byte. Any
geometry change must keep those fixtures green or regenerate them on purpose.
