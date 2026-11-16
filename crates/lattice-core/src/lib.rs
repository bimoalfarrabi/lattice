//! Lattice geometry, documents and export. No windowing, no I/O beyond files.

pub mod arrange;
pub mod doc;
pub mod geom;
pub mod grid;
pub mod guides;
pub mod export;
pub mod tiling;

pub use geom::{Guide, Point};

#[cfg(test)]
mod parity {
    use super::grid;

    #[test]
    fn matches_the_reference_cells() {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/parity.json"
        ))
        .expect("fixtures/parity.json, run tools/parity/dump.mjs");

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Case {
            id: String,
            original: bool,
            scale_y: f64,
            cells: Vec<String>,
        }

        let cases: Vec<Case> = serde_json::from_str(&raw).expect("fixture json");
        assert_eq!(cases.len(), 62, "every grid in both layouts");

        let mut failures = Vec::new();
        for case in cases {
            let grid = grid::find(&case.id).expect("known grid");
            let cells = grid.cells(case.original);

            if cells.len() != case.cells.len() {
                failures.push(format!(
                    "{} original={}: {} cells, expected {}",
                    case.id,
                    case.original,
                    cells.len(),
                    case.cells.len()
                ));
                continue;
            }
            if (grid.scale_y() - case.scale_y).abs() > f64::EPSILON {
                failures.push(format!("{}: scaleY {} != {}", case.id, grid.scale_y(), case.scale_y));
            }
            for (index, expected) in case.cells.iter().enumerate() {
                if &cells[index] != expected {
                    failures.push(format!(
                        "{} original={} cell {}:\n  got      {}\n  expected {}",
                        case.id, case.original, index, cells[index], expected
                    ));
                    break;
                }
            }
        }

        assert!(
            failures.is_empty(),
            "{} mismatching cases\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

#[cfg(test)]
mod tiling_parity {
    use super::tiling::{Placement, covering};
    use super::Point;

    #[test]
    fn matches_the_reference_placements() {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/tiling.json"
        ))
        .expect("fixtures/tiling.json, run tools/parity/dump_tiling.mjs");

        #[derive(serde::Deserialize)]
        struct Tile {
            x: i32,
            y: i32,
            transform: String,
            world: World,
        }
        #[derive(serde::Deserialize)]
        struct World {
            x: f64,
            y: f64,
        }
        #[derive(serde::Deserialize)]
        struct Cover {
            #[serde(rename = "box")]
            extent: Rect,
            tiles: Vec<World>,
        }
        #[derive(serde::Deserialize)]
        struct Rect {
            x: f64,
            y: f64,
            width: f64,
            height: f64,
        }
        #[derive(serde::Deserialize)]
        struct Fixture {
            tiles: Vec<Tile>,
            boxes: Vec<Cover>,
        }

        let fixture: Fixture = serde_json::from_str(&raw).expect("fixture json");
        for tile in &fixture.tiles {
            let placement = Placement::of(tile.x, tile.y);
            assert_eq!(placement.svg_transform(), tile.transform, "tile {},{}", tile.x, tile.y);
            assert_eq!(placement.out(Point::new(20.0, 30.0)), Point::new(tile.world.x, tile.world.y));
        }
        for case in &fixture.boxes {
            let rect = &case.extent;
            let expected: Vec<(i32, i32)> = case.tiles.iter().map(|t| (t.x as i32, t.y as i32)).collect();
            assert_eq!(
                covering(rect.x, rect.y, rect.width, rect.height),
                expected,
                "box {:?}",
                (rect.x, rect.y, rect.width, rect.height)
            );
        }
    }
}
