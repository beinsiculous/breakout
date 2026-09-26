//! The title screen's art: the game's title composition at 1×, at the top of the
//! window, on its own backdrop, with the menu panel starting just under the art's
//! content — over the art's blank lower margin, which is the backdrop's colour.
//!
//! One function lays the title screen out for both of its halves, the drawing and
//! the mouse hit-test, so a click lands on the row it appears to. A window too small
//! for art and menu together, or art that will not load, keeps the plain centred
//! menu with no art and no fill.

use engine_core::prelude::*;

use crate::constants::{TITLE_ART, TITLE_ART_CONTENT_BOTTOM, TITLE_LINES_BELOW_PANEL, TITLE_PANEL_WIDTH};
use crate::menu::TITLE_ITEMS;

/// Space between the art's last content row and the menu panel's top.
const PANEL_GAP_UNDER_ART: f32 = 18.0;
/// Room kept between the lowest thing on the title screen and the window's bottom.
const BOTTOM_MARGIN: f32 = 16.0;

/// The loaded title art: its texture, its size, and its backdrop — the top-left
/// pixel's colour, which the window is filled with around it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TitleArt {
    texture_id: u32,
    size: Vec2,
    backdrop: Color,
}

impl TitleArt {
    /// Load the synced title art, or `None` — logged — when it will not read or
    /// load; the title screen then keeps its plain centred menu.
    pub(crate) fn load(assets: &mut AssetManager) -> Option<Self> {
        let backdrop = assets.image_backdrop(TITLE_ART)?;
        match assets.load_texture_filtered(TITLE_ART, TextureFilter::Nearest) {
            Ok(texture) => Some(Self {
                texture_id: texture.id,
                size: Vec2::new(backdrop.size.x as f32, backdrop.size.y as f32),
                backdrop: backdrop.corner,
            }),
            Err(error) => {
                log::warn!("The title art {TITLE_ART} did not load, so the title keeps its plain menu: {error}");
                None
            }
        }
    }
}

/// Where the title screen's parts go: the menu panel, and the art's box when the
/// window has room for it (`None` in the fallback).
pub(crate) struct TitleLayout {
    pub(crate) panel: MenuPanel,
    pub(crate) art: Option<Rect>,
}

/// The title screen's layout in this window, for this build's rows.
pub(crate) fn title_layout(title: &str, window_size: Vec2, art: Option<&TitleArt>) -> TitleLayout {
    title_layout_for_rows(title, window_size, art, TITLE_ITEMS.len())
}

/// The native build's row count. The web build drops the Achievements row, and the
/// fit is decided on the native count on every target, so a window either shows
/// the art or falls back the same way on the web as natively.
fn native_title_rows() -> usize {
    TITLE_ITEMS.len() + usize::from(cfg!(target_arch = "wasm32"))
}

/// [`title_layout`] for a given row count.
pub(crate) fn title_layout_for_rows(title: &str, window_size: Vec2, art: Option<&TitleArt>, rows: usize) -> TitleLayout {
    let centred = || TitleLayout {
        panel: MenuPanel::new(title, window_size / 2.0, TITLE_PANEL_WIDTH, rows),
        art: None,
    };
    let Some(art) = art else { return centred() };
    let panel_top = TITLE_ART_CONTENT_BOTTOM + PANEL_GAP_UNDER_ART;
    let lowest = panel_top + panel_height(native_title_rows()) + TITLE_LINES_BELOW_PANEL + BOTTOM_MARGIN;
    if window_size.x < art.size.x || window_size.y < lowest {
        return centred();
    }
    let panel_center = Vec2::new(window_size.x / 2.0, panel_top + panel_height(rows) / 2.0);
    TitleLayout {
        panel: MenuPanel::new(title, panel_center, TITLE_PANEL_WIDTH, rows),
        art: Some(Rect::new(((window_size.x - art.size.x) / 2.0).floor(), 0.0, art.size.x, art.size.y)),
    }
}

/// A title panel's height for `rows` rows.
fn panel_height(rows: usize) -> f32 {
    MenuPanel::new("", Vec2::ZERO, TITLE_PANEL_WIDTH, rows).panel_rect().height
}

/// Fill the window with the art's backdrop and draw the art, when the layout has
/// room for it. Drawn before the menu panel, which then sits over the art's blank
/// lower margin.
pub(crate) fn draw_title_art(ui: &mut UIContext, window_size: Vec2, layout: &TitleLayout, art: Option<&TitleArt>) {
    let (Some(bounds), Some(art)) = (layout.art, art) else { return };
    ui.rect(Rect::new(0.0, 0.0, window_size.x, window_size.y), art.backdrop);
    ui.image(bounds, art.texture_id, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art() -> TitleArt {
        TitleArt { texture_id: 1, size: Vec2::new(480.0, 288.0), backdrop: Color::BLACK }
    }

    fn overlaps(first: Rect, second: Rect) -> bool {
        first.x < second.x + second.width
            && second.x < first.x + first.width
            && first.y < second.y + second.height
            && second.y < first.y + first.height
    }

    #[test]
    fn the_art_and_the_menu_fit_the_window_with_the_menu_under_the_arts_content() {
        let window = Vec2::new(crate::constants::WIN_W, crate::constants::WIN_H);
        for rows in [native_title_rows(), native_title_rows() - 1] {
            let layout = title_layout_for_rows("", window, Some(&art()), rows);
            let art_box = layout.art.expect("the window has room for the art");
            let panel = layout.panel.panel_rect();
            let whole_window = Rect::new(0.0, 0.0, window.x, window.y);
            assert!(art_box.x >= 0.0 && art_box.x + art_box.width <= window.x && art_box.y == 0.0, "{rows} rows");
            assert!(overlaps(panel, whole_window) && panel.y + panel.height + TITLE_LINES_BELOW_PANEL <= window.y,
                "{rows} rows: the panel and the lines under it are inside the window: {panel:?}");
            assert_eq!(panel.y, TITLE_ART_CONTENT_BOTTOM + PANEL_GAP_UNDER_ART, "{rows} rows");
        }
    }

    #[test]
    fn a_window_too_small_or_no_art_keeps_the_plain_centred_menu() {
        let small = Vec2::new(640.0, 480.0);
        for art in [Some(art()), None] {
            let layout = title_layout_for_rows("", small, art.as_ref(), native_title_rows());
            assert!(layout.art.is_none(), "no art at 640x480, art loaded: {}", art.is_some());
            let panel = layout.panel.panel_rect();
            assert_eq!(panel.y + panel.height / 2.0, small.y / 2.0, "centred as before");
        }
        let full = Vec2::new(crate::constants::WIN_W, crate::constants::WIN_H);
        assert!(title_layout_for_rows("", full, None, native_title_rows()).art.is_none(), "art that did not load");
    }

    #[test]
    fn the_art_draws_on_the_title_screen_only_and_again_on_returning_to_it() {
        use crate::types::{GameState, BreakoutGame};
        use engine_core::test_support::{DrawCommand, GameHarness};

        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let config = crate::game_config(assets.to_str().expect("the asset path is UTF-8"));
        let mut harness = GameHarness::new(BreakoutGame::default(), config);
        harness.step(1.0 / 60.0, &[]);
        let art = harness.game().title_art.expect("the synced title art loads");
        let draws_art = |harness: &GameHarness<BreakoutGame>| {
            harness.ui_commands().iter().any(|command| {
                matches!(command, DrawCommand::Image { texture_id, .. } if *texture_id == art.texture_id)
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
}
