//! Grid catalogue and cell derivation.

use crate::arrange::arrange;
use crate::guides;

#[derive(Clone)]
pub struct Grid {
    pub id: &'static str,
    pub name: &'static str,
    original: Vec<crate::geom::Guide>,
    refit: Option<Vec<crate::geom::Guide>>,
}

impl Grid {
    pub fn guides(&self, original_layout: bool) -> &[crate::geom::Guide] {
        match (&self.refit, original_layout) {
            (_, true) => &self.original,
            (Some(refit), false) => refit,
            (None, false) => &self.original,
        }
    }

    /// Cells in the layout the drawing asks for. `None` when the id is unknown.
    pub fn cells(&self, original_layout: bool) -> Vec<String> {
        arrange(self.guides(original_layout))
            .into_iter()
            .map(|face| face.path)
            .collect()
    }

    pub fn scale_y(&self) -> f64 {
        guides::scale_y(self.id)
    }

    pub fn repeats(&self) -> usize {
        guides::repeats(self.id)
    }
}

pub fn all() -> Vec<Grid> {
    guides::table()
        .into_iter()
        .map(|def| Grid {
            id: def.id,
            name: def.name,
            original: def.guides,
            refit: def.refit_guides,
        })
        .collect()
}

pub fn find(id: &str) -> Option<Grid> {
    all().into_iter().find(|grid| grid.id == id)
}

pub fn is_free(id: &str) -> bool {
    guides::FREE.contains(&id)
}

pub fn is_derived(id: &str) -> bool {
    guides::DERIVED.contains(&id)
}

/// Grids the gallery lists: the derived ones are reachable only through a
/// loaded drawing, and the plain 20 by 20 only once it is in use.
pub fn listed(current: Option<&str>, marks: impl Fn(&str) -> bool) -> Vec<Grid> {
    all()
        .into_iter()
        .filter(|grid| {
            !is_derived(grid.id) && (grid.id != "square" || current == Some(grid.id) || marks(grid.id))
        })
        .collect()
}
