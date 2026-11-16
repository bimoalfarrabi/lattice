//! Scalar geometry shared by the guide generators and the arrangement.
//!
//! Ported line for line from the reference studio bundle. Order of arithmetic
//! and the epsilon are load bearing: the arrangement keys nodes by a rounded
//! coordinate string, so a different order changes cell identity.

pub const EPS: f64 = 1e-7;
pub const TAU: f64 = std::f64::consts::TAU;
pub const CENTER: f64 = 120.0;
pub const BOX: f64 = 240.0;
pub const BOX_HI: f64 = 240.0000001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub fn lp(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}

pub fn sub(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}

/// JS `Number(v.toFixed(5))` then string interpolation: five decimals, then
/// trailing zeros dropped. Integer math, so there is no second rounding.
pub fn num(v: f64) -> String {
    let scaled = (v * 100_000.0).round();
    if scaled == 0.0 {
        return "0".to_string();
    }
    let negative = scaled < 0.0;
    let n = scaled.abs() as u128;
    let whole = n / 100_000;
    let frac = n % 100_000;

    let mut out = String::with_capacity(12);
    if negative {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    if frac != 0 {
        let mut tail = format!("{frac:05}");
        while tail.ends_with('0') {
            tail.pop();
        }
        out.push('.');
        out.push_str(&tail);
    }
    out
}

pub fn key(p: Point) -> String {
    format!("{} {}", num(p.x), num(p.y))
}

pub fn in_box(p: Point) -> bool {
    p.x >= -EPS && p.y >= -EPS && p.x <= BOX_HI && p.y <= BOX_HI
}

#[derive(Clone, Debug)]
pub enum Guide {
    Line { a: Point, b: Point },
    Circle { center: Point, r: f64 },
}

pub fn line(a: Point, b: Point) -> Guide {
    Guide::Line { a, b }
}

pub fn at(guide: &Guide, t: f64) -> Point {
    match guide {
        Guide::Line { a, b } => Point::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)),
        Guide::Circle { center, r } => {
            Point::new(center.x + r * t.cos(), center.y + r * t.sin())
        }
    }
}

pub fn is_line(guide: &Guide) -> bool {
    matches!(guide, Guide::Line { .. })
}
