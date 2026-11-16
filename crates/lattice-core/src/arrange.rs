//! Planar arrangement over a guide set, producing one SVG path per face.
//!
//! Guides are cut at every intersection, each resulting segment becomes two
//! half edges, and faces are walked by always taking the previous edge in the
//! angular order around the arriving node. Outer faces are dropped and inner
//! holes are appended to their smallest containing face.

use std::collections::{HashMap, HashSet};

use crate::geom::{BOX, CENTER, EPS, Guide, Point, TAU, at, in_box, key, line, lp, sub};

const MIN_AREA: f64 = 1e-5;

/// The four square edges, in the order the arrangement adds them.
///
/// Painting uses this too: the guides of a lattice never include a line on the
/// square edge, so without these the tile boundary is invisible and adjacent
/// tiles merge.
pub fn box_edges() -> [Guide; 4] {
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(BOX, 0.0),
        Point::new(BOX, BOX),
        Point::new(0.0, BOX),
    ];
    std::array::from_fn(|i| line(corners[i], corners[(i + 1) % 4]))
}

pub fn intersect(a: &Guide, b: &Guide) -> Vec<Point> {
    match (a, b) {
        (Guide::Line { a: a0, b: a1 }, Guide::Line { a: b0, b: b1 }) => {
            let n = sub(*a1, *a0);
            let r = sub(*b1, *b0);
            let den = lp(n, r);
            if den.abs() < EPS {
                return Vec::new();
            }
            vec![at(a, lp(sub(*b0, *a0), r) / den)]
        }
        (Guide::Circle { center: c0, r: r0 }, Guide::Circle { center: c1, r: r1 }) => {
            let n = sub(*c1, *c0);
            let dist = n.x.hypot(n.y);
            if dist < EPS || dist > r0 + r1 + EPS || dist < (r0 - r1).abs() - EPS {
                return Vec::new();
            }
            let d = (r0 * r0 - r1 * r1 + dist * dist) / (2.0 * dist);
            let on_axis = Point::new(c0.x + d * n.x / dist, c0.y + d * n.y / dist);
            if (dist - r0 - r1).abs() < EPS || (dist - (r0 - r1).abs()).abs() < EPS {
                return vec![on_axis];
            }
            let h = (r0 * r0 - d * d).max(0.0).sqrt();
            vec![
                Point::new(c0.x + (d * n.x - h * n.y) / dist, c0.y + (d * n.y + h * n.x) / dist),
                Point::new(c0.x + (d * n.x + h * n.y) / dist, c0.y + (d * n.y - h * n.x) / dist),
            ]
        }
        (Guide::Circle { .. }, _) => intersect(b, a),
        (Guide::Line { a: p0, b: p1 }, Guide::Circle { center, r }) => {
            let n = sub(*p1, *p0);
            let rel = sub(*center, *p0);
            let den = n.x * n.x + n.y * n.y;
            let along = (rel.x * n.x + rel.y * n.y) / den;
            let off = r * r - lp(rel, n) * lp(rel, n) / den;
            if off < -EPS {
                return Vec::new();
            }
            if off <= EPS {
                return vec![at(a, along)];
            }
            let spread = (off / den).sqrt();
            vec![at(a, along - spread), at(a, along + spread)]
        }
    }
}

#[derive(Debug)]
struct HalfEdge {
    start: Point,
    end: Point,
    angle: f64,
    curvature: f64,
    area: f64,
    command: String,
    samples: Vec<Point>,
    twin: usize,
}

pub struct Face {
    pub path: String,
    pub area: f64,
    /// One entry per closed subpath. Holes appear as later entries, wound the
    /// opposite way, which is what makes parity hit testing work.
    pub subpaths: Vec<Vec<Point>>,
}

fn point_in_polygon(poly: &[Point], probe: Point) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut i = n - 1;
    for j in 0..n {
        let a = poly[j];
        let b = poly[i];
        if (a.y > probe.y) != (b.y > probe.y)
            && probe.x < (b.x - a.x) * (probe.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
        i = j;
    }
    inside
}

fn within_segment(guide: &Guide, p: Point) -> bool {
    match guide {
        Guide::Circle { .. } => true,
        Guide::Line { a, b } => {
            p.x >= a.x.min(b.x) - EPS
                && p.x <= a.x.max(b.x) + EPS
                && p.y >= a.y.min(b.y) - EPS
                && p.y <= a.y.max(b.y) + EPS
        }
    }
}

/// Builds the faces of `guides` inside the 240 unit box.
pub fn arrange(guides: &[Guide]) -> Vec<Face> {
    let mut all: Vec<Guide> = box_edges().to_vec();
    all.extend(guides.iter().cloned());

    // Angles already known to split each guide: the corners of a line, the
    // quadrants of a circle. Intersections add to these.
    let mut cuts: Vec<Vec<f64>> = all
        .iter()
        .map(|g| match g {
            Guide::Line { .. } => vec![0.0, 1.0],
            Guide::Circle { .. } => vec![0.0, TAU / 4.0, TAU / 2.0, 3.0 * TAU / 4.0],
        })
        .collect();

    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            for p in intersect(&all[i], &all[j]) {
                if !in_box(p) || !within_segment(&all[i], p) || !within_segment(&all[j], p) {
                    continue;
                }
                register(&all[i], p, &mut cuts[i]);
                register(&all[j], p, &mut cuts[j]);
            }
        }
    }

    let mut edges: Vec<HalfEdge> = Vec::new();
    let mut nodes: HashMap<String, Vec<usize>> = HashMap::new();

    for (index, guide) in all.iter().enumerate() {
        let mut sorted = cuts[index].clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted.dedup_by(|a, b| (*a - *b).abs() <= EPS);
        if matches!(guide, Guide::Circle { .. }) {
            sorted.push(TAU);
        }
        for pair in sorted.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if to - from < EPS || !in_box(at(guide, (from + to) / 2.0)) {
                continue;
            }
            let here = edges.len();
            emit(guide, from, to, here + 1, &mut edges, &mut nodes);
            emit(guide, to, from, here, &mut edges, &mut nodes);
        }
    }

    for bucket in nodes.values_mut() {
        bucket.sort_by(|a, b| {
            let delta = edges[*a].angle - edges[*b].angle;
            if delta.abs() < EPS {
                edges[*a].curvature.partial_cmp(&edges[*b].curvature).unwrap()
            } else {
                delta.partial_cmp(&0.0).unwrap()
            }
        });
    }

    let mut visited: HashSet<usize> = HashSet::new();
    let mut faces: Vec<Face> = Vec::new();

    for seed in 0..edges.len() {
        if visited.contains(&seed) {
            continue;
        }
        let mut current = seed;
        let mut area = 0.0;
        let mut path = format!("M{}", key(edges[current].start));
        let mut subpaths: Vec<Vec<Point>> = Vec::new();
        let mut previous_end: Option<Point> = None;
        loop {
            visited.insert(current);
            let edge = &edges[current];
            path.push_str(&edge.command);
            area += edge.area;

            let continues = previous_end
                .map(|end| (end.x - edge.start.x).abs() < EPS && (end.y - edge.start.y).abs() < EPS)
                .unwrap_or(false);
            if continues {
                subpaths.last_mut().expect("open subpath").extend_from_slice(&edge.samples);
            } else {
                subpaths.push(edge.samples.clone());
            }
            previous_end = Some(edge.end);

            let bucket = nodes.get(&key(edge.end)).expect("node of an endpoint");
            let twin_at = bucket.iter().position(|e| *e == edge.twin).expect("twin in bucket");
            current = bucket[(twin_at + bucket.len() - 1) % bucket.len()];
            if visited.contains(&current) {
                break;
            }
        }
        if area.abs() > MIN_AREA {
            path.push('Z');
            faces.push(Face { path, area, subpaths });
        }
    }

    let solid: Vec<usize> = (0..faces.len()).filter(|i| faces[*i].area > 0.0).collect();
    let holes: Vec<usize> = (0..faces.len()).filter(|i| faces[*i].area < 0.0).collect();
    for hole in holes {
        let probe = faces[hole].subpaths[0][0];
        let mut host = solid
            .iter()
            .copied()
            .filter(|i| faces[*i].area > -faces[hole].area + EPS && point_in_polygon(&faces[*i].subpaths[0], probe))
            .collect::<Vec<_>>();
        host.sort_by(|a, b| faces[*a].area.partial_cmp(&faces[*b].area).unwrap());
        if let Some(target) = host.first() {
            let suffix = faces[hole].path.clone();
            let shapes = faces[hole].subpaths.clone();
            faces[*target].path.push_str(&suffix);
            faces[*target].subpaths.extend(shapes);
        }
    }

    let mut slots: Vec<Option<Face>> = faces.into_iter().map(Some).collect();
    solid
        .into_iter()
        .map(|i| slots[i].take().expect("face taken once"))
        .collect()
}

fn register(guide: &Guide, p: Point, cuts: &mut Vec<f64>) {
    match guide {
        Guide::Circle { center, .. } => {
            let angle = (p.y - center.y).atan2(p.x - center.x);
            cuts.push((angle + TAU) % TAU);
        }
        Guide::Line { a, b } => {
            let n = sub(*b, *a);
            let t = if n.x.abs() > n.y.abs() {
                (p.x - a.x) / n.x
            } else {
                (p.y - a.y) / n.y
            };
            cuts.push(t.clamp(0.0, 1.0));
        }
    }
}

fn emit(
    guide: &Guide,
    from: f64,
    to: f64,
    twin: usize,
    edges: &mut Vec<HalfEdge>,
    nodes: &mut HashMap<String, Vec<usize>>,
) {
    let start = at(guide, from);
    let end = at(guide, to);
    let step = to - from;

    let (angle, area, samples, command, curvature) = match guide {
        Guide::Circle { center, r } => {
            let sweep = if step > 0.0 { 1 } else { 0 };
            let angle = from + step.signum() * TAU / 4.0;
            let area = (lp(*center, sub(end, start)) + r * r * step) / 2.0;
            let count = ((step.abs() / 0.08).ceil() as usize).max(1);
            let samples = (0..count).map(|k| at(guide, from + step * k as f64 / count as f64)).collect();
            (
                angle,
                area,
                samples,
                format!("A{} {} 0 0 {} {}", crate::geom::num(*r), crate::geom::num(*r), sweep, key(end)),
                step.signum() / r,
            )
        }
        Guide::Line { .. } => {
            let angle = (end.y - start.y).atan2(end.x - start.x);
            (
                angle,
                lp(start, end) / 2.0,
                vec![start, end],
                format!("L{}", key(end)),
                0.0,
            )
        }
    };

    let wrapped = (angle + TAU) % TAU;
    let angle = if TAU - wrapped < EPS { 0.0 } else { wrapped };

    let index = edges.len();
    nodes.entry(key(start)).or_default().push(index);
    edges.push(HalfEdge {
        start,
        end,
        angle,
        curvature,
        area,
        command,
        samples,
        twin,
    });
}

/// Rounds a value the way the reference output does, for tests.
pub fn round5(v: f64) -> f64 {
    (v * 100_000.0).round() / 100_000.0
}

#[allow(dead_code)]
pub const CENTER_HINT: f64 = CENTER;
