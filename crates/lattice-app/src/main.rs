//! Lattice studio, a desktop grid logo maker.

use gpui::{App, Application};

fn main() {
    Application::new().run(|cx: &mut App| lattice_app::studio::open(cx));
}
