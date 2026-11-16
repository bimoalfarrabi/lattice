//! The drawing plane is an endless field of 240 unit tiles. Every odd column
//! is mirrored on X and every odd row on Y, so a cell shape repeats with no
//! seams. Addressing is `(tile, index)`; a mark stores it as `"x,y:index"`.

use crate::geom::{BOX, EPS, Point};

/// Where a tile sits and which way it is flipped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub origin_x: f64,
    pub origin_y: f64,
    pub flip_x: f64,
    pub flip_y: f64,
}

impl Placement {
    pub fn of(tile_x: i32, tile_y: i32) -> Self {
        let odd_x = tile_x.rem_euclid(2) == 1;
        let odd_y = tile_y.rem_euclid(2) == 1;
        Self {
            origin_x: (tile_x + i32::from(odd_x)) as f64 * BOX,
            origin_y: (tile_y + i32::from(odd_y)) as f64 * BOX,
            flip_x: if odd_x { -1.0 } else { 1.0 },
            flip_y: if odd_y { -1.0 } else { 1.0 },
        }
    }

    /// Tile local units to plane units.
    pub fn out(self, local: Point) -> Point {
        Point::new(self.origin_x + self.flip_x * local.x, self.origin_y + self.flip_y * local.y)
    }

    /// Plane units to tile local units.
    pub fn into(self, plane: Point) -> Point {
        Point::new((plane.x - self.origin_x) * self.flip_x, (plane.y - self.origin_y) * self.flip_y)
    }

    /// The SVG transform the export writes for this tile.
    pub fn svg_transform(self) -> String {
        format!("translate({} {}) scale({} {})", self.origin_x, self.origin_y, self.flip_x, self.flip_y)
    }
}

/// Tiles that a plane rectangle touches.
pub fn covering(min_x: f64, min_y: f64, width: f64, height: f64) -> Vec<(i32, i32)> {
    let last_x = ((min_x + width) / BOX).ceil() as i32;
    let last_y = ((min_y + height) / BOX).ceil() as i32;
    let mut out = Vec::new();
    let mut y = (min_y / BOX).floor() as i32;
    while y < last_y {
        let mut x = (min_x / BOX).floor() as i32;
        while x < last_x {
            out.push((x, y));
            x += 1;
        }
        y += 1;
    }
    out
}

/// A cell prepared for hit testing: bounding box plus its closed rings.
#[derive(Clone)]
pub struct Shape {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub rings: Vec<Vec<Point>>,
    pub path: String,
}

impl Shape {
    fn from_face(face: crate::arrange::Face) -> Self {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for ring in &face.subpaths {
            for p in ring {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
            rings: face.subpaths,
            path: face.path,
        }
    }

    /// Even-odd test across every ring, which matches the SVG fill rule the
    /// reference uses for holes.
    pub fn contains(&self, p: Point) -> bool {
        if p.x < self.min_x - EPS || p.x > self.max_x + EPS || p.y < self.min_y - EPS || p.y > self.max_y + EPS {
            return false;
        }
        let mut inside = false;
        for ring in &self.rings {
            let n = ring.len();
            let mut j = n - 1;
            for i in 0..n {
                let a = ring[i];
                let b = ring[j];
                if (a.y > p.y) != (b.y > p.y)
                    && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
                {
                    inside = !inside;
                }
                j = i;
            }
        }
        inside
    }
}

const BUCKETS: i32 = 16;

/// A cell set with a coarse bucket index, so a pointer probe touches a handful
/// of candidates instead of every cell.
pub struct CellIndex {
    buckets: Vec<Vec<u32>>,
    pub shapes: Vec<Shape>,
}

impl CellIndex {
    pub fn build(original_layout: bool, id: &str) -> Option<Self> {
        let grid = crate::grid::find(id)?;
        let faces = crate::arrange::arrange(grid.guides(original_layout));
        let shapes: Vec<Shape> = faces.into_iter().map(Shape::from_face).collect();

        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); (BUCKETS * BUCKETS) as usize];
        for (index, shape) in shapes.iter().enumerate() {
            let step = BOX / BUCKETS as f64;
            let from_y = (shape.min_y / step).floor().max(0.0) as i32;
            let to_y = (shape.max_y / step).floor().min((BUCKETS - 1) as f64) as i32;
            let from_x = (shape.min_x / step).floor().max(0.0) as i32;
            let to_x = (shape.max_x / step).floor().min((BUCKETS - 1) as f64) as i32;
            for y in from_y..=to_y {
                for x in from_x..=to_x {
                    buckets[(x + y * BUCKETS) as usize].push(index as u32);
                }
            }
        }

        Some(Self { buckets, shapes })
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Cell under a tile local point.
    pub fn hit(&self, local: Point) -> Option<usize> {
        if !(0.0..=BOX).contains(&local.x) || !(0.0..=BOX).contains(&local.y) {
            return None;
        }
        let step = BOX / BUCKETS as f64;
        let bucket = ((local.x / step).floor() as i32).clamp(0, BUCKETS - 1)
            + ((local.y / step).floor() as i32).clamp(0, BUCKETS - 1) * BUCKETS;
        self.buckets[bucket as usize]
            .iter()
            .map(|i| *i as usize)
            .find(|i| self.shapes[*i].contains(local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_round_trip() {
        for y in -3..=3 {
            for x in -3..=3 {
                let placement = Placement::of(x, y);
                let local = Point::new(20.0, 30.0);
                let back = placement.into(placement.out(local));
                assert!((back.x - local.x).abs() < 1e-9);
                assert!((back.y - local.y).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn covering_reaches_the_edges() {
        assert_eq!(covering(0.0, 0.0, 240.0, 240.0), vec![(0, 0)]);
        assert_eq!(covering(-1.0, -1.0, 242.0, 242.0).len(), 9);
    }

    #[test]
    fn square_cells_hit_the_right_index() {
        let index = CellIndex::build(false, "square-16").expect("grid");
        assert_eq!(index.len(), 256);
        for (probe, left, top) in [
            (Point::new(7.5, 7.5), 0.0, 0.0),
            (Point::new(22.5, 7.5), 15.0, 0.0),
            (Point::new(7.5, 22.5), 0.0, 15.0),
        ] {
            let shape = &index.shapes[index.hit(probe).expect("a cell")];
            assert!((shape.min_x - left).abs() < 1e-6 && (shape.min_y - top).abs() < 1e-6);
        }
        assert_eq!(index.hit(Point::new(250.0, 7.5)), None);
    }

    #[test]
    fn holes_are_not_hits() {
        let index = CellIndex::build(false, "inscribed-squares").expect("grid");
        let outer = index.hit(Point::new(1.0, 1.0));
        assert!(outer.is_some(), "corner of the first square is filled");
        let hole = index.hit(Point::new(7.5, 7.5));
        assert!(hole.is_some(), "circle interior is still a face");
        assert_ne!(outer, hole, "the hole is its own face");
    }
}
