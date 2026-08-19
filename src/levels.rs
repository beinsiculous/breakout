//! Scene-driven brick layouts.
//!
//! The brick grid is authored in `<asset base>/scenes/level1.scene.ron`
//! (editable in the engine editor) and instantiated at every match start.
//! Every read goes through the configured asset base, so the same code
//! reads the game dir natively and the VFS on the web. Bricks are
//! identified by the `brick_r{row}_c{col}` naming convention; the row digit
//! drives the score payout, which stays a game rule in Rust
//! (`spawning::brick_value`). If the scene is missing or yields no bricks,
//! the caller falls back to the generated grid in `spawning::spawn_bricks`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use engine_core::prelude::*;

use crate::constants::{BRICK_ROWS, BRICK_VALUE_STEP};
use crate::spawning::brick_value;
use crate::types::{Brick, GameMode, PickupKind};

/// A selectable level: display name, scene file, and the chaos mode it
/// plays in. The layouts live in the scene files (editor-authorable); this
/// table is deliberately a Rust const — breakout has no serde/ron deps.
pub(crate) struct LevelDef {
    pub(crate) title: &'static str,
    pub(crate) scene_file: &'static str,
    pub(crate) mode: ChaosMode,
}

/// The solo level roster, one per chaos mode.
pub(crate) const LEVELS: [LevelDef; 4] = [
    LevelDef { title: "CLASSIC", scene_file: "level1.scene.ron", mode: ChaosMode::Normal },
    LevelDef { title: "THE VAULT", scene_file: "level2.scene.ron", mode: ChaosMode::Insane },
    LevelDef { title: "PINATA", scene_file: "level3.scene.ron", mode: ChaosMode::Ridiculous },
    LevelDef { title: "THE GAUNTLET", scene_file: "level4.scene.ron", mode: ChaosMode::Insiculous },
];

/// The co-op roster: dedicated layouts designed for the middle band between
/// the two paddles (ball in play off both edges), one per chaos mode.
pub(crate) const LEVELS_2P: [LevelDef; 4] = [
    LevelDef { title: "CLASSIC DUO", scene_file: "level1_2p.scene.ron", mode: ChaosMode::Normal },
    LevelDef { title: "THE VAULT DUO", scene_file: "level2_2p.scene.ron", mode: ChaosMode::Insane },
    LevelDef { title: "PINATA DUO", scene_file: "level3_2p.scene.ron", mode: ChaosMode::Ridiculous },
    LevelDef { title: "GAUNTLET DUO", scene_file: "level4_2p.scene.ron", mode: ChaosMode::Insiculous },
];

/// The level roster for a game mode.
pub(crate) fn roster(mode: GameMode) -> &'static [LevelDef] {
    match mode {
        GameMode::SinglePlayer => &LEVELS,
        GameMode::TwoPlayerCoop => &LEVELS_2P,
    }
}

/// One-line flavor text per level, shown under the roster.
pub(crate) fn level_hint(mode: GameMode, index: usize) -> &'static str {
    match (mode, index) {
        (GameMode::SinglePlayer, 0) => "The classic wall. A gentle taste of armor.",
        (GameMode::SinglePlayer, 1) => "A fortress of steel. Ball speeds up per paddle hit.",
        (GameMode::SinglePlayer, 2) => "Crack it open - it rains power-ups. Two-ball serves.",
        (GameMode::SinglePlayer, 3) => "Armor, chaos, and everything at once.",
        (GameMode::TwoPlayerCoop, 0) => "The wall between you. Guard both edges.",
        (GameMode::TwoPlayerCoop, 1) => "An armored core - and the ball keeps speeding up.",
        (GameMode::TwoPlayerCoop, 2) => "A prize band ripe for the cracking. Two-ball serves.",
        (GameMode::TwoPlayerCoop, 3) => "Everything at once, coming from both sides.",
        _ => "",
    }
}

/// Path of a level scene file, anchored to the configured asset base.
///
/// `SceneLoader::load_from_file` takes a path rather than an asset-base
/// relative name, so the join happens here. `asset_base` must be the same
/// string the game passed to `GameConfig::with_asset_base_path`
/// (`ctx.assets.base_path()`): natively that is the absolute `assets` dir,
/// on the web it is the deploy URL base and the joined string is the
/// scene's exact VFS key (`{base}/scenes/level1.scene.ron`).
pub(crate) fn level_scene_path(asset_base: &str, scene_file: &str) -> PathBuf {
    Path::new(asset_base).join("scenes").join(scene_file)
}

/// Parse a level's scene from the asset base. Returns `None` (with a console
/// warning) if the file is missing or malformed — the game then uses the
/// generated grid instead of failing to start.
pub(crate) fn load_level_data(
    asset_base: &str,
    mode: GameMode,
    index: usize,
) -> Option<SceneData> {
    let Some(def) = roster(mode).get(index) else {
        eprintln!("breakout: level index {index} out of range; using generated brick grid");
        return None;
    };
    let path = level_scene_path(asset_base, def.scene_file);
    match SceneLoader::load_from_file(&path) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!(
                "breakout: could not load level scene {}: {e}; using generated brick grid",
                path.display()
            );
            None
        }
    }
}

/// Extract the row index from a `brick_r{row}_c{col}` entity name.
pub(crate) fn brick_row_from_name(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("brick_r")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Score paid out by a brick with the given entity name. Bricks whose name
/// doesn't yield an in-range row still score the minimum payout, so renamed
/// bricks degrade gracefully instead of breaking the game.
pub(crate) fn brick_value_from_name(name: &str) -> u32 {
    match brick_row_from_name(name) {
        Some(row) if row < BRICK_ROWS => brick_value(row),
        _ => BRICK_VALUE_STEP,
    }
}

/// What a brick's `EntityTag` says about it. Untagged bricks get the
/// defaults: one hit, no drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrickSpec {
    pub(crate) hits: u32,
    pub(crate) drop: Option<PickupKind>,
}

impl Default for BrickSpec {
    fn default() -> Self {
        Self { hits: 1, drop: None }
    }
}

/// Parse a brick's `EntityTag` string: tokens joined by `+`.
///
/// - `armored{N}` — N total hits to destroy (2..=9)
/// - `drop_multiball` / `drop_wrecking` / `drop_insiculous` — pickup dropped
///
/// Unknown or malformed tokens warn and are skipped (graceful degradation,
/// same philosophy as `brick_value_from_name`); duplicates: last wins.
pub(crate) fn parse_brick_tag(tag: &str) -> BrickSpec {
    let mut spec = BrickSpec::default();
    for token in tag.split('+') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(n) = token.strip_prefix("armored") {
            match n.parse::<u32>() {
                Ok(hits) if (2..=9).contains(&hits) => spec.hits = hits,
                _ => eprintln!("breakout: ignoring malformed armor token '{token}'"),
            }
        } else {
            match token {
                "drop_multiball" => spec.drop = Some(PickupKind::Multiball),
                "drop_wrecking" => spec.drop = Some(PickupKind::Wrecking),
                "drop_insiculous" => spec.drop = Some(PickupKind::Insiculous),
                _ => eprintln!("breakout: ignoring unknown brick tag token '{token}'"),
            }
        }
    }
    spec
}

/// Build the game's `Brick` bookkeeping from a scene instance's named
/// entities: every entity named `brick*` becomes a brick. The particle-burst
/// color is read from the entity's live `Sprite`, so bricks retinted in the
/// editor keep matching effects; armor/drop behavior comes from the
/// entity's `EntityTag` (see `parse_brick_tag`).
pub(crate) fn bricks_from_names(
    named_entities: &HashMap<String, EntityId>,
    world: &World,
) -> Vec<Brick> {
    named_entities
        .iter()
        .filter(|(name, _)| name.starts_with("brick"))
        .map(|(name, &entity)| {
            let spec = world
                .get::<EntityTag>(entity)
                .map(|t| parse_brick_tag(&t.0))
                .unwrap_or_default();
            Brick {
                entity,
                value: brick_value_from_name(name),
                color: world
                    .get::<Sprite>(entity)
                    .map(|s| s.color)
                    .unwrap_or(Vec4::ONE),
                hits_left: spec.hits,
                drop: spec.drop,
            }
        })
        .collect()
}

/// Instantiate the level scene into the world and return the brick list.
pub(crate) fn spawn_bricks_from_scene(
    data: &SceneData,
    world: &mut World,
    assets: &mut AssetManager,
) -> Result<Vec<Brick>, SceneLoadError> {
    let instance = SceneLoader::instantiate(data, world, assets)?;
    Ok(bricks_from_names(&instance.named_entities, world))
}
