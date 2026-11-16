//! Lattice studio, a desktop grid logo maker.

// A GUI application on Windows should not open a console window. Debug builds
// keep it, so panics and the demo hook stay visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gpui::{App, Application};

fn main() {
    Application::new().run(|cx: &mut App| lattice_app::studio::open(cx));
}
