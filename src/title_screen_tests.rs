//! The title art on this game's title screen: drawn there only, and laid out with
//! the menu inside the window on both row counts, from the synced art itself.

use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness};

use crate::constants::{TITLE_MENU, WIN_H, WIN_W};
use crate::types::{BreakoutGame, GameState};

/// The lowest row the art's content may reach and leave the menu room under it.
const CONTENT_BOTTOM_CEILING: f32 = 234.0;
/// Where the menu panel's top lands under this game's art.
const PANEL_TOP: f32 = 252.0;

fn harness() -> GameHarness<BreakoutGame> {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let config = crate::game_config(assets.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(BreakoutGame::default(), config);
    harness.step(1.0 / 60.0, &[]);
    harness
}

#[test]
fn the_synced_art_and_the_menu_fit_the_window_with_the_menu_under_the_arts_content() {
    let harness = harness();
    let art = harness.game().title_art.expect("the synced title art loads");
    assert_eq!(art.content_bottom(), 234.0, "measured from the synced PNG: a new value means the art changed, so look at the title before updating it");
    assert!(art.content_bottom() <= CONTENT_BOTTOM_CEILING);
    let window = Vec2::new(WIN_W, WIN_H);
    for rows in [TITLE_MENU.fit_rows, TITLE_MENU.fit_rows - 1] {
        let layout = title_layout("", window, Some(&art), &TitleMenu { rows, ..TITLE_MENU });
        let art_box = layout.art.expect("the window has room for the art");
        assert!(art_box.x >= 0.0 && art_box.x + art_box.width <= window.x && art_box.y == 0.0, "{rows} rows");
        let panel = layout.panel.panel_rect();
        assert_eq!(panel.y, PANEL_TOP, "{rows} rows");
        assert!(panel.y + panel.height + TITLE_MENU.lines_below_panel <= window.y,
            "{rows} rows: the panel and the lines under it are inside the window: {panel:?}");
    }
}

#[test]
fn the_art_draws_on_the_title_screen_only_and_again_on_returning_to_it() {
    let mut harness = harness();
    let art = harness.game().title_art.expect("the synced title art loads");
    let draws_art = |harness: &GameHarness<BreakoutGame>| {
        harness.ui_commands().iter().any(|command| {
            matches!(command, DrawCommand::Image { texture_id, .. } if *texture_id == art.texture_id())
        })
    };
    assert!(draws_art(&harness), "the title screen draws its art");

    harness.game_mut().state = GameState::LevelSelect { selection: 0 };
    harness.step(1.0 / 60.0, &[]);
    assert!(!draws_art(&harness), "another menu does not");

    harness.game_mut().state = GameState::TitleScreen { selection: 0 };
    harness.step(1.0 / 60.0, &[]);
    assert!(draws_art(&harness), "back on the title screen, it draws again");
}
