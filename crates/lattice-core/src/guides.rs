//! The 31 guide lattices and their refit variants.
//!
//! Every grid is generated, never authored: families of lines and circles that
//! the arrangement turns into cells. Ported from the reference bundle, keeping
//! iteration order, since cell indices follow it.

use crate::geom::{BOX, CENTER, EPS, Guide, Point, at, line, lp, sub};

const SQRT3: f64 = 1.7320508075688772;

fn family(angle: f64, spacing: f64, count: i32, offset: f64) -> Vec<Guide> {
    let theta = angle * std::f64::consts::PI / 180.0;
    let (sin, cos) = theta.sin_cos();
    let mut out = Vec::new();

    for step in -count..=count {
        let n = CENTER - sin * (step as f64 * spacing + offset);
        let i = CENTER + cos * (step as f64 * spacing + offset);
        let mut hits: Vec<Point> = Vec::new();

        let push = |p: Point, hits: &mut Vec<Point>| {
            if crate::geom::in_box(p) && !hits.iter().any(|h| (h.x - p.x).hypot(h.y - p.y) < EPS) {
                hits.push(p);
            }
        };

        if cos.abs() > EPS {
            for x in [0.0, BOX] {
                push(Point::new(x, i + (x - n) * sin / cos), &mut hits);
            }
        }
        if sin.abs() > EPS {
            for y in [0.0, BOX] {
                push(Point::new(n + (y - i) * cos / sin, y), &mut hits);
            }
        }

        let on_edge = [0.0, BOX].iter().any(|edge| {
            hits.iter().all(|p| (p.x - edge).abs() < EPS) || hits.iter().all(|p| (p.y - edge).abs() < EPS)
        });
        if hits.len() == 2 && !on_edge {
            out.push(line(hits[0], hits[1]));
        }
    }
    out
}

fn default_count(spacing: f64) -> i32 {
    (BOX / spacing).ceil() as i32
}

fn criss_cross(angle: f64, spacing: f64, count: i32) -> Vec<Guide> {
    let mut out = family(angle, spacing, count, 0.0);
    out.extend(family(-angle, spacing, count, 0.0));
    out
}

fn rings(count: usize, step: f64, cx: f64, cy: f64) -> Vec<Guide> {
    (0..count)
        .map(|i| Guide::Circle {
            center: Point::new(cx, cy),
            r: (i as f64 + 0.5) * step,
        })
        .collect()
}

fn square_mesh(spacing: f64) -> Vec<Guide> {
    let mut out = family(0.0, spacing, default_count(spacing), 0.0);
    out.extend(family(90.0, spacing, default_count(spacing), 0.0));
    out
}

fn iso_mesh(spacing: f64) -> Vec<Guide> {
    let mut out = family(90.0, spacing, default_count(spacing), 0.0);
    out.extend(criss_cross(30.0, spacing, default_count(spacing)));
    out
}

/// A 16 cell square lattice, the default.
fn squares_16() -> Vec<Guide> {
    square_mesh(15.0)
}

/// One circle inscribed in each square of the default lattice.
fn inscribed_squares() -> Vec<Guide> {
    let mut out = squares_16();
    out.extend((0..256).map(|i| Guide::Circle {
        center: Point::new((i % 16) as f64 * 15.0 + 7.5, (i / 16) as f64 * 15.0 + 7.5),
        r: 7.5,
    }));
    out
}

/// Concentric diamonds with a circle in each one.
fn inscribed_diamonds(spacing: f64) -> Vec<Guide> {
    let mut out = criss_cross(45.0, spacing, default_count(spacing));
    let half = spacing / 2.0;
    let reach = (BOX / (std::f64::consts::SQRT_2 * spacing)).ceil() as i32 + 1;
    for i in -reach..reach {
        for j in -reach..reach {
            let cx = CENTER + (j - i) as f64 * spacing / std::f64::consts::SQRT_2;
            let cy = CENTER + (j + i + 1) as f64 * spacing / std::f64::consts::SQRT_2;
            let over_x = (-cx).max(cx - BOX).max(0.0);
            let over_y = (-cy).max(cy - BOX).max(0.0);
            if over_x.hypot(over_y) < half {
                out.push(Guide::Circle {
                    center: Point::new(cx, cy),
                    r: half,
                });
            }
        }
    }
    out
}

/// Hexagon ring clipped to the box, crossed by an iso mesh.
fn prism(spacing: f64) -> Vec<Guide> {
    let hex: Vec<Point> = (0..6)
        .map(|i| {
            let a = i as f64 * std::f64::consts::PI / 3.0 - std::f64::consts::FRAC_PI_2;
            Point::new(CENTER + 114.0 * a.cos(), CENTER + 114.0 * a.sin())
        })
        .collect();
    let ring: Vec<Guide> = (0..6).map(|i| line(hex[i], hex[(i + 1) % 6])).collect();

    let mut out = ring.clone();
    out.extend(criss_cross(30.0, spacing, 0));

    for candidate in iso_mesh(spacing) {
        let Guide::Line { a, b } = candidate else { continue };
        let dir = sub(b, a);
        if lp(dir, sub(Point::new(CENTER, CENTER), a)).abs() < EPS && dir.x.abs() > EPS {
            continue;
        }
        let mut lo = 0.0f64;
        let mut hi = 1.0f64;
        for edge in &ring {
            let Guide::Line { a: e0, b: e1 } = edge else { continue };
            let e = sub(*e1, *e0);
            let o = lp(e, sub(a, *e0));
            let s = lp(e, dir);
            if s.abs() < EPS {
                if o < 0.0 {
                    hi = -1.0;
                }
            } else if s > 0.0 {
                lo = lo.max(-o / s);
            } else {
                hi = hi.min(-o / s);
            }
        }
        if hi - lo > EPS {
            out.push(line(at(&candidate, lo), at(&candidate, hi)));
        }
    }
    out
}

fn octagons(count: i32) -> Vec<Guide> {
    let tile = BOX / count as f64;
    let cut = tile / (2.0 + std::f64::consts::SQRT_2);
    let mut out = Vec::new();

    for row in 0..count {
        for col in 0..count {
            let (x, y) = (col as f64 * tile, row as f64 * tile);
            let (x1, y1) = (x + tile, y + tile);
            out.push(line(Point::new(x + cut, y), Point::new(x, y + cut)));
            out.push(line(Point::new(x1 - cut, y), Point::new(x1, y + cut)));
            out.push(line(Point::new(x1, y1 - cut), Point::new(x1 - cut, y1)));
            out.push(line(Point::new(x + cut, y1), Point::new(x, y1 - cut)));
            if row > 0 {
                out.push(line(Point::new(x + cut, y), Point::new(x1 - cut, y)));
            }
            if col > 0 {
                out.push(line(Point::new(x, y + cut), Point::new(x, y1 - cut)));
            }
        }
    }
    out
}

fn running_bond() -> Vec<Guide> {
    let mut out = family(0.0, 20.0, default_count(20.0), 0.0);
    for row in 0..12 {
        let y = row as f64 * 20.0;
        let mut x = if row % 2 == 1 { 20.0 } else { 40.0 };
        while x < BOX {
            out.push(line(Point::new(x, y), Point::new(x, y + 20.0)));
            x += 40.0;
        }
    }
    out
}

fn polar_sectors() -> Vec<Guide> {
    let mut out = rings(6, 20.0, CENTER, CENTER);
    let mut angle = 0.0;
    while angle < 180.0 {
        out.extend(family(angle, BOX, 0, 0.0));
        angle += 15.0;
    }
    out
}

fn honeycomb() -> Vec<Guide> {
    let mut out = Vec::new();
    let vertex = |col: i32, row: i32| -> Point {
        let drift = if (col + row) % 2 == 0 { 15.0 / 6.0 } else { -15.0 / 6.0 };
        Point::new((col as f64 + 0.5) * 15.0 + drift, row as f64 * 10.0)
    };
    for row in 0..24 {
        for col in 0..16 {
            out.push(line(vertex(col, row), vertex(col, row + 1)));
        }
        if row != 0 {
            let mut col = -(row % 2);
            while col < 16 {
                let a = vertex(col, row);
                let b = vertex(col + 1, row);
                out.push(line(
                    Point::new(a.x.max(0.0), a.y),
                    Point::new(b.x.min(BOX), b.y),
                ));
                col += 2;
            }
        }
    }
    out
}

fn basketweave() -> Vec<Guide> {
    let mut out = square_mesh(30.0);
    let mut y = 0.0;
    while y < BOX {
        let mut x = 0.0;
        while x < BOX {
            let horizontal = ((x + y) / 30.0) as i32 % 2 == 0;
            for inset in [10.0, 20.0] {
                out.push(if horizontal {
                    line(Point::new(x, y + inset), Point::new(x + 30.0, y + inset))
                } else {
                    line(Point::new(x + inset, y), Point::new(x + inset, y + 30.0))
                });
            }
            x += 30.0;
        }
        y += 30.0;
    }
    out
}

fn shippo() -> Vec<Guide> {
    let mut out = Vec::new();
    let mut row = 0;
    while row as f64 * 20.0 <= BOX {
        let mut col = (row % 2) as f64 * 20.0;
        while col <= BOX {
            out.push(Guide::Circle {
                center: Point::new(col, row as f64 * 20.0),
                r: 20.0,
            });
            col += 40.0;
        }
        row += 1;
    }
    out
}

fn petals() -> Vec<Guide> {
    let mut out = Vec::new();
    let mut y = 0.0;
    while y <= BOX {
        let mut x = 0.0;
        while x <= BOX {
            out.push(Guide::Circle {
                center: Point::new(x, y),
                r: 24.0 * 0.65,
            });
            x += 24.0;
        }
        y += 24.0;
    }
    out
}

fn trihexagonal() -> Vec<Guide> {
    let step = 10.0 * SQRT3;
    let mut out = criss_cross(60.0, step, default_count(step));
    out.extend(family(0.0, step, default_count(step), step / 2.0));
    out
}

fn cross_tiling() -> Vec<Guide> {
    let offsets = [(0, 0), (-1, 0), (0, -1), (0, 1), (1, 0)];
    let shifted = |x: i32, y: i32| -> (i32, i32) {
        let n = ((x + 2 * y) % 5 + 5) % 5;
        let (dx, dy) = offsets[n as usize];
        (x + dx, y + dy)
    };
    let mut out = Vec::new();
    for row in 0..20 {
        for col in 0..20 {
            let (x, y) = (col as f64 * 12.0, row as f64 * 12.0);
            let here = shifted(col, row);
            if col > 0 && here != shifted(col - 1, row) {
                out.push(line(Point::new(x, y), Point::new(x, y + 12.0)));
            }
            if row > 0 && here != shifted(col, row - 1) {
                out.push(line(Point::new(x, y), Point::new(x + 12.0, y)));
            }
        }
    }
    out
}

fn quadtree() -> Vec<Guide> {
    fn subdivide(out: &mut Vec<Guide>, x: f64, y: f64, size: f64, depth: u8) {
        let half = size / 2.0;
        out.push(line(Point::new(x, y + half), Point::new(x + size, y + half)));
        out.push(line(Point::new(x + half, y), Point::new(x + half, y + size)));
        if depth == 4 {
            return;
        }
        for quadrant in 0..4 {
            if depth >= 2 && (quadrant == 0 || quadrant == 3) {
                continue;
            }
            subdivide(
                out,
                x + (quadrant % 2) as f64 * half,
                y + (quadrant / 2) as f64 * half,
                half,
                depth + 1,
            );
        }
    }
    let mut out = Vec::new();
    subdivide(&mut out, 0.0, 0.0, BOX, 0);
    out
}

fn chevron() -> Vec<Guide> {
    let mut out = family(90.0, 20.0, default_count(20.0), 0.0);
    for row in 0..12 {
        let y = row as f64 * 20.0;
        let mut x = 0.0;
        while x < BOX {
            let (left, right) = if row % 2 == 0 { (x, x + 10.0) } else { (x + 10.0, x) };
            out.push(line(Point::new(y, left), Point::new(y + 20.0, right)));
            x += 10.0;
        }
    }
    out
}

fn spiral() -> Vec<Guide> {
    let step = std::f64::consts::TAU / 16.0;
    let mut out = Vec::new();
    let mut previous: Vec<Point> = Vec::new();
    for ring in 1..=9 {
        let points: Vec<Point> = (0..16)
            .map(|i| {
                let angle = i as f64 * step + ((ring - 9) as f64 * step) / 3.0;
                let (sin, cos) = angle.sin_cos();
                let radius = if ring == 9 {
                    CENTER / cos.abs().max(sin.abs())
                } else {
                    CENTER * ring as f64 / 9.0
                };
                Point::new(CENTER + radius * cos, CENTER + radius * sin)
            })
            .collect();
        for (i, p) in points.iter().enumerate() {
            out.push(line(*previous.get(i).unwrap_or(&Point::new(CENTER, CENTER)), *p));
            if ring < 9 {
                out.push(line(*p, points[(i + 1) % 16]));
            }
        }
        previous = points;
    }
    out
}

/// Bisectors between scattered seeds, clipped by the other seeds and the box.
fn bound(n: Point, offset: f64, mid: Point, dir: Point, lo: &mut f64, hi: &mut f64) {
    let den = n.x * dir.x + n.y * dir.y;
    let num = offset - n.x * mid.x - n.y * mid.y;
    if den > 0.0 {
        *hi = hi.min(num / den);
    } else if den < 0.0 {
        *lo = lo.max(num / den);
    } else if num < 0.0 {
        *hi = f64::NEG_INFINITY;
    }
}

fn voronoi() -> Vec<Guide> {
    let seeds: Vec<Point> = [
        (17.0, 22.0),
        (73.0, 12.0),
        (121.0, 31.0),
        (160.0, 16.0),
        (220.0, 25.0),
        (33.0, 71.0),
        (84.0, 84.0),
        (138.0, 64.0),
        (191.0, 78.0),
        (232.0, 93.0),
        (11.0, 125.0),
        (62.0, 118.0),
        (113.0, 137.0),
        (157.0, 108.0),
        (212.0, 132.0),
        (26.0, 177.0),
        (81.0, 163.0),
        (132.0, 182.0),
        (178.0, 158.0),
        (226.0, 175.0),
        (14.0, 223.0),
        (69.0, 214.0),
        (110.0, 231.0),
        (167.0, 209.0),
        (210.0, 227.0),
    ]
    .iter()
    .map(|(x, y)| Point::new(*x, *y))
    .collect();

    let mut out = Vec::new();
    for (i, a) in seeds.iter().enumerate() {
        for b in seeds.iter().skip(i + 1) {
            let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let dir = Point::new(a.y - b.y, b.x - a.x);
            let mut lo = f64::NEG_INFINITY;
            let mut hi = f64::INFINITY;

            bound(Point::new(-1.0, 0.0), 0.0, mid, dir, &mut lo, &mut hi);
            bound(Point::new(1.0, 0.0), BOX, mid, dir, &mut lo, &mut hi);
            bound(Point::new(0.0, -1.0), 0.0, mid, dir, &mut lo, &mut hi);
            bound(Point::new(0.0, 1.0), BOX, mid, dir, &mut lo, &mut hi);

            for c in &seeds {
                if c == a || c == b {
                    continue;
                }
                let n = Point::new(c.x - a.x, c.y - a.y);
                bound(
                    n,
                    (n.x * (c.x + a.x) + n.y * (c.y + a.y)) / 2.0,
                    mid,
                    dir,
                    &mut lo,
                    &mut hi,
                );
                if lo >= hi {
                    break;
                }
            }

            if hi - lo > EPS {
                let at_t = |t: f64| Point::new(mid.x + t * dir.x, mid.y + t * dir.y);
                out.push(line(at_t(lo), at_t(hi)));
            }
        }
    }
    out
}

pub struct GridDef {
    pub id: &'static str,
    pub name: &'static str,
    pub guides: Vec<Guide>,
    pub refit_guides: Option<Vec<Guide>>,
}

pub fn table() -> Vec<GridDef> {
    let mesh = 15.0;
    let iso_honey = mesh * 1.5 * SQRT3 / 2.0;
    let crossed = || {
        let mut out = square_mesh(30.0);
        out.extend(criss_cross(
            45.0,
            mesh * std::f64::consts::SQRT_2,
            default_count(mesh * std::f64::consts::SQRT_2),
        ));
        out
    };

    let def = |id, name, guides: Vec<Guide>| GridDef {
        id,
        name,
        guides,
        refit_guides: None,
    };

    vec![
        def("square-16", "Square grid", squares_16()),
        def("inscribed-squares", "Circles in squares", inscribed_squares()),
        GridDef {
            refit_guides: Some({
                let mut out = family(90.0, 30.0, 3, 0.0);
                out.extend(criss_cross(30.0, 30.0, default_count(30.0)));
                out.extend(family(30.0, 30.0, 5, mesh / 2.0));
                out
            }),
            ..def("weave", "Woven grid", {
                let mut out = family(90.0, 30.0, 3, 0.0);
                out.extend(criss_cross(30.0, 24.0, default_count(24.0)));
                out.extend(family(30.0, 24.0, 5, 7.0));
                out
            })
        },
        GridDef {
            refit_guides: Some(crossed()),
            ..def("diagonal", "Crossed squares", {
                let mut out = square_mesh(24.0);
                out.extend(criss_cross(45.0, 24.0 * std::f64::consts::SQRT_2, default_count(24.0)));
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let step = iso_honey;
                let mut out = family(0.0, step, default_count(step), 0.0);
                out.extend(criss_cross(60.0, step, default_count(step)));
                out
            }),
            ..def("triangles", "Triangle grid", {
                let step = 12.0 * SQRT3;
                let mut out = family(0.0, step, default_count(step), 0.0);
                out.extend(criss_cross(60.0, step, default_count(step)));
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let mut out = rings(9, mesh, 57.0, 73.0);
                out.extend(rings(9, mesh, 183.0, 73.0));
                out.extend(rings(9, mesh, 120.0, 182.0));
                out
            }),
            ..def("trefoil", "Three centers", {
                let mut out = rings(12, 11.0, 57.0, 73.0);
                out.extend(rings(12, 11.0, 183.0, 73.0));
                out.extend(rings(12, 11.0, 120.0, 182.0));
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let mut out = criss_cross(60.0, iso_honey, default_count(iso_honey));
                out.extend(family(0.0, iso_honey, 5, 0.0));
                out.extend(
                    rings(2, mesh, CENTER, CENTER)
                        .into_iter()
                        .map(|g| match g {
                            Guide::Circle { center, r } => Guide::Circle { center, r: r + 90.0 },
                            other => other,
                        }),
                );
                out
            }),
            ..def("halo", "Triangle halo", {
                let mut out = criss_cross(60.0, 18.0, default_count(18.0));
                out.extend(family(0.0, 18.0, 5, 0.0));
                out.extend(
                    rings(2, 15.0, CENTER, CENTER)
                        .into_iter()
                        .map(|g| match g {
                            Guide::Circle { center, r } => Guide::Circle { center, r: r + 87.0 },
                            other => other,
                        }),
                );
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let mut out = crossed();
                out.extend(rings(4, 30.0, CENTER, CENTER));
                out
            }),
            ..def("compass", "Compass grid", {
                let mut out = square_mesh(24.0);
                out.extend(criss_cross(45.0, 24.0 * std::f64::consts::SQRT_2, default_count(24.0)));
                out.extend(rings(5, 24.0, CENTER, CENTER));
                out
            })
        },
        GridDef {
            refit_guides: Some(iso_mesh(iso_honey)),
            ..def("isometric", "Isometric grid", iso_mesh(18.0))
        },
        GridDef {
            refit_guides: Some(criss_cross(45.0, mesh, default_count(mesh))),
            ..def("diamond", "Diamond grid", criss_cross(45.0, 24.0, default_count(24.0)))
        },
        GridDef {
            refit_guides: Some(inscribed_diamonds(mesh)),
            ..def("inscribed-diamonds", "Inscribed circles", inscribed_diamonds(24.0))
        },
        GridDef {
            refit_guides: Some(criss_cross(60.0, mesh, default_count(mesh))),
            ..def("steep", "Tall diamonds", criss_cross(60.0, 18.0, default_count(18.0)))
        },
        GridDef {
            refit_guides: Some(criss_cross(30.0, mesh, default_count(mesh))),
            ..def("wide", "Wide diamonds", criss_cross(30.0, 18.0, default_count(18.0)))
        },
        GridDef {
            refit_guides: Some({
                let mut out = square_mesh(30.0);
                out.extend(rings(8, mesh, CENTER, CENTER));
                out
            }),
            ..def("orbit", "Concentric grid", {
                let mut out = square_mesh(24.0);
                out.extend(rings(6, 22.0, CENTER, CENTER));
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let mut out = criss_cross(45.0, mesh, 5);
                out.extend(family(0.0, mesh, 2, 0.0));
                out
            }),
            ..def("ribbon", "Crossed bands", {
                let mut out = criss_cross(45.0, 12.0, 5);
                out.extend(family(0.0, 18.0, 2, 0.0));
                out
            })
        },
        GridDef {
            refit_guides: Some({
                let mut out = iso_mesh(iso_honey);
                out.extend(rings(7, mesh, CENTER, CENTER));
                out
            }),
            ..def("seal", "Circular lattice", {
                let mut out = iso_mesh(22.0);
                out.extend(rings(5, 21.0, CENTER, CENTER));
                out
            })
        },
        GridDef {
            refit_guides: Some(prism(iso_honey)),
            ..def("prism", "Prism grid", prism(18.0))
        },
        def("octagons", "Octagon grid", octagons(8)),
        def("running-bond", "Running bond", running_bond()),
        def("polar", "Polar sectors", polar_sectors()),
        def("voronoi", "Voronoi mosaic", voronoi()),
        def("honeycomb", "Honeycomb grid", honeycomb()),
        def("basketweave", "Basketweave grid", basketweave()),
        def("shippo", "Shippō linked circles", shippo()),
        def("petals", "Petal lattice", petals()),
        def("trihexagonal", "Trihexagonal tiling", trihexagonal()),
        def("cross-tiling", "Cross tiling", cross_tiling()),
        def("quadtree", "Quadtree grid", quadtree()),
        def("square", "20 × 20", {
            let mut out = family(0.0, 12.0, default_count(12.0), 0.0);
            out.extend(family(90.0, 12.0, default_count(12.0), 0.0));
            out
        }),
        def("chevron", "Chevron grid", chevron()),
        def("spiral", "Spiral grid", spiral()),
    ]
}

pub fn scale_y(id: &str) -> f64 {
    if id == "honeycomb" {
        SQRT3 / 2.0
    } else {
        1.0
    }
}

pub fn repeats(id: &str) -> usize {
    if id == "running-bond" || id == "voronoi" {
        2
    } else {
        1
    }
}

pub const FREE: [&str; 9] = [
    "square-16",
    "inscribed-squares",
    "weave",
    "diagonal",
    "triangles",
    "trefoil",
    "halo",
    "compass",
    "isometric",
];

pub const DERIVED: [&str; 6] = [
    "cross-tiling",
    "quadtree",
    "petals",
    "trihexagonal",
    "chevron",
    "spiral",
];
