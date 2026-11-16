//! Pointer and keyboard behaviour, exercised through the real element tree.

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};
use lattice_app::studio::Studio;

fn setup<'a>(cx: &'a mut TestAppContext, home: &str) -> (Entity<Studio>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        // Safety: single threaded test process, set before any window exists.
        unsafe { std::env::set_var("LATTICE_DATA_DIR", home) };
        cx.activate(true);
    });
    cx.add_window_view(|window, cx| Studio::new(window, cx))
}

fn painted(cx: &mut VisualTestContext, entity: &Entity<Studio>) -> usize {
    cx.update(|_, cx| entity.read(cx).painted())
}

#[gpui::test]
fn clicking_paints_and_undo_reverses_it(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("lattice-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let (entity, cx) = setup(cx, dir.to_str().expect("utf8 temp dir"));

    let viewport = cx.update(|window, _| window.viewport_size());
    let target = point(px(f32::from(viewport.width) * 0.5), px(f32::from(viewport.height) * 0.5));

    cx.simulate_mouse_move(target, None, Modifiers::default());
    cx.simulate_click(target, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(painted(cx, &entity), 1, "one cell is painted");

    cx.simulate_keystrokes("ctrl-z");
    cx.run_until_parked();
    assert_eq!(painted(cx, &entity), 0, "undo empties it");
}
