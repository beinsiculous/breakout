//! Insiculous Breakout — game crate.
//!
//! The library owns the whole game (`BreakoutGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! behind the `editor` feature in both entry points — `main.rs` for the native
//! window, `web_entry.rs` for the browser's editor bundle.

mod achievements;
mod chaos_theme;
mod constants;
mod drawing;
mod effects;
#[cfg(test)]
mod flow_tests;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod levels;
#[cfg(test)]
mod levels_tests;
mod menu;
mod power_ups;
mod spawning;
#[cfg(test)]
mod test_support;
mod types;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use chaos_theme::theme_for;
use constants::*;
use engine_core::prelude::*;
use types::*;

pub use types::BreakoutGame;

/// The shared `GameConfig` for every target. Entry points add their own
/// platform extras on top (native: save paths anchored to the game dir;
/// web: nothing — no save paths means in-memory achievements and default
/// input bindings).
///
/// `asset_base` must be an ANCHORED base: native callers pass an absolute
/// path (`main.rs` derives it from `game_root!()` so the cwd never
/// matters); the web entry passes the deploy URL base. Passing a bare
/// relative path like `"assets"` would silently resolve against the
/// current working directory — and the level scenes are read through the
/// same base, so a wrong base costs every brick layout too.
pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("The Food Pyramid")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        // The art is 1x with nearest filtering, so snapping every sprite's origin to a
        // whole device pixel is what keeps it crisp at this window size.
        .with_pixel_snap(true)
        .with_asset_base_path(asset_base)
}

/// Load one synced sheet by the path its spec names. Fail-loud: the sheets ship with
/// the game, so a missing or malformed one is a broken build rather than a game that
/// silently draws nothing.
fn load_sheet(assets: &mut AssetManager, spec: &SheetSpec) -> SpriteSheet {
    assets
        .load_sprite_sheet(spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

impl Game for BreakoutGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, _strings: &Strings) {
        achievements::register_all(achievements);
    }

    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.sheets.white = tex.id;

        // Every sheet's path, cell and measured anchor is in `constants.rs`; the PNG and
        // its sidecar are the synced copies under `assets/sprites/`.
        self.sheets.foods = std::array::from_fn(|index| load_sheet(ctx.assets, &FOODS[index].sheet));
        self.sheets.ball_water = load_sheet(ctx.assets, &BALL_WATER);
        self.sheets.ball_ice = load_sheet(ctx.assets, &BALL_ICE);
        self.sheets.candy_multiball = load_sheet(ctx.assets, &CANDY_MULTIBALL);
        self.sheets.candy_wrecking = load_sheet(ctx.assets, &CANDY_WRECKING);
        self.sheets.candy_insiculous = load_sheet(ctx.assets, &CANDY_INSICULOUS);
        self.sheets.tong_left = load_sheet(ctx.assets, &TONG_LEFT);
        self.sheets.tong_right = load_sheet(ctx.assets, &TONG_RIGHT);
        self.sheets.court = load_sheet(ctx.assets, &COURT_TILE);
        self.sheets.court_edge = load_sheet(ctx.assets, &COURT_EDGE);

        // The counter first: the surface everything else stands on. It and the grid
        // over it stay up under the menus, as Tong's court does.
        self.court = Some(spawning::spawn_court(ctx.world, &self.sheets.court));
        self.backdrop = Some(spawning::spawn_backdrop(ctx.world, &theme_for(self.chaos_mode)));

        // Walls, sensors, paddles and their tongs are mode-dependent (co-op opens the
        // top edge for a second paddle), so the whole playfield structure is built by
        // rebuild_playfield() at every match start. Solo layout here just so the
        // editor sees a populated scene before play.
        self.rebuild_playfield(ctx.world, GameMode::SinglePlayer);

        // Brick layouts are authored in per-level scenes (editor-editable);
        // start_game() loads the selected level each match, falling back to
        // the generated grid if the file is missing.
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::LevelSelect { selection } => self.update_level_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.update_entity_visibility(ctx);
        self.draw_ui(ctx);
    }
}
