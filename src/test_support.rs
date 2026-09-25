//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load path, so a
//! test can check the game's tables against the committed art without a window, a GPU
//! or a `GameContext`. The fixtures also drive the whole game through the engine's
//! harness: the real frame, with no window.

use std::path::{Path, PathBuf};

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;
use engine_core::test_support::GameHarness;

use crate::types::*;

pub(crate) const FRAME: f32 = 1.0 / 60.0;

/// The synced art's directory, anchored to the crate so the working directory never
/// matters.
fn asset_base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// A synced sheet read through the engine's own GPU-free load path.
pub(crate) fn sidecar(spec: &SheetSpec) -> PreparedSheet {
    prepare_sheet(&asset_base(), spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

/// The title screen's world: the game's own config over the synced art, with no save
/// paths so achievements and scores stay in memory, and one frame run so `init` has
/// loaded the sheets and spawned what the title shows.
pub(crate) fn title_harness() -> GameHarness<BreakoutGame> {
    let base = asset_base();
    let config = crate::game_config(base.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(BreakoutGame::default(), config);
    harness.step(FRAME, &[]);
    harness
}

/// Start a match as the level select does: the mode, the level and its chaos mode on
/// the game and the context, then `start_game`.
pub(crate) fn start_match(harness: &mut GameHarness<BreakoutGame>, mode: GameMode, level: usize) {
    harness.context(|game, ctx| {
        game.mode = mode;
        game.selected_level = level;
        game.chaos_mode = crate::levels::roster(mode)[level].mode;
        ctx.chaos_mode = game.chaos_mode;
        game.start_game(ctx)
    });
}

/// A match just started from the title screen, through the real frame.
pub(crate) fn harness(mode: GameMode, level: usize) -> GameHarness<BreakoutGame> {
    let mut harness = title_harness();
    start_match(&mut harness, mode, level);
    harness
}

pub(crate) fn position_of(world: &World, entity: EntityId) -> Vec2 {
    world.get::<Transform2D>(entity).expect("the entity has a transform").position
}

pub(crate) fn state_of(world: &World, entity: EntityId) -> String {
    world
        .get::<ClipStateMachine>(entity)
        .expect("the entity carries a machine")
        .state()
        .to_string()
}

/// The live entities whose `Name` is exactly `name`.
pub(crate) fn named(world: &World, name: &str) -> Vec<EntityId> {
    world
        .entities()
        .into_iter()
        .filter(|&entity| world.get::<Name>(entity).is_some_and(|found| found.0 == name))
        .collect()
}
