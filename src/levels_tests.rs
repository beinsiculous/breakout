//! Tests for the scene-driven level rosters (solo and co-op).
//!
//! Lives outside `levels.rs` so the roster tables and the test battery can
//! both grow without crowding the 600-line file budget.

use std::collections::HashMap;
use std::path::PathBuf;

use engine_core::prelude::*;

use crate::constants::{
    food_for_row, Food, BRICK_CELL, BRICK_COLS, BRICK_GAP, BRICK_ROWS, BRICK_VALUE_STEP, FOODS,
    PADDLE_TOP_Y, PADDLE_Y, PLAYFIELD_HALF_W,
};
use crate::levels::*;
use crate::spawning::{brick_value, brick_x, brick_y};
use crate::types::{GameMode, PickupKind, BRICK_ARMORED, BRICK_INTACT};

fn manifest_scene_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/scenes/level1.scene.ron")
}

fn load_scene() -> SceneData {
    SceneLoader::load_from_file(manifest_scene_path()).expect("level1.scene.ron should parse")
}

/// Effective components of an entity, merged as the engine merges them: the prefab's,
/// then the overrides, then the inline components an editor save writes.
fn merged_components(scene: &SceneData, entity: &EntityData) -> Vec<ComponentData> {
    let mut result: Vec<ComponentData> = entity
        .prefab
        .as_ref()
        .and_then(|p| scene.prefabs.get(p))
        .map(|p| p.components.clone())
        .unwrap_or_default();
    for over in entity.overrides.iter().chain(&entity.components) {
        let kind = std::mem::discriminant(over);
        if let Some(pos) = result.iter().position(|c| std::mem::discriminant(c) == kind) {
            result[pos] = over.clone();
        } else {
            result.push(over.clone());
        }
    }
    result
}

/// Load a roster level via a CARGO_MANIFEST_DIR-anchored path (tests can't
/// rely on exe-dir anchoring).
fn load_roster_level(def: &LevelDef) -> SceneData {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/scenes")
        .join(def.scene_file);
    SceneLoader::load_from_file(&path)
        .unwrap_or_else(|e| panic!("{} should parse: {e}", def.scene_file))
}

const BOTH_MODES: [GameMode; 2] = [GameMode::SinglePlayer, GameMode::TwoPlayerCoop];

#[test]
fn shipped_level_parses() {
    let scene = load_scene();
    assert_eq!(scene.name, "Breakout Level 1");
}

#[test]
fn every_roster_level_parses_with_valid_bricks() {
    for mode in BOTH_MODES {
        let levels = roster(mode);
        assert_eq!(levels.len(), ChaosMode::ALL.len(), "one level per chaos mode");
        for (i, def) in levels.iter().enumerate() {
            assert_eq!(def.mode, ChaosMode::ALL[i], "roster order follows ChaosMode::ALL");
            assert!(!level_hint(mode, i).is_empty());

            let scene = load_roster_level(def);
            let brick_names: Vec<&str> = scene
                .entities
                .iter()
                .filter_map(|e| e.name.as_deref())
                .filter(|n| n.starts_with("brick"))
                .collect();
            assert!(!brick_names.is_empty(), "{} has no bricks", def.scene_file);
            for name in &brick_names {
                brick_row_from_name(name)
                    .unwrap_or_else(|| panic!("unparsable brick name {name} in {}", def.scene_file));
            }
        }
    }
}

/// Solo levels stay above the bottom paddle; co-op levels must ALSO stay
/// below the top paddle (the both-sides analogue of the solo guarantee).
#[test]
fn every_roster_level_fits_the_playfield() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            let scene = load_roster_level(def);
            for entity in &scene.entities {
                let name = entity.name.as_deref().unwrap_or("<unnamed>");
                let (pos, scale) = merged_components(&scene, entity)
                    .into_iter()
                    .find_map(|c| match c {
                        ComponentData::Transform2D { position, scale, .. } => {
                            Some((position, scale))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{name} in {} has no Transform2D", def.scene_file));

                let half_w = scale.0 * RENDER_UNIT / 2.0;
                assert!(
                    pos.0.abs() + half_w < PLAYFIELD_HALF_W,
                    "{name} in {} pokes past a side wall",
                    def.scene_file
                );
                let bottom = pos.1 - scale.1 * RENDER_UNIT / 2.0;
                assert!(
                    bottom > PADDLE_Y + 100.0,
                    "{name} in {} sits too close to the bottom paddle",
                    def.scene_file
                );
                if mode == GameMode::TwoPlayerCoop {
                    let top = pos.1 + scale.1 * RENDER_UNIT / 2.0;
                    assert!(
                        top < PADDLE_TOP_Y - 100.0,
                        "{name} in {} sits too close to the top paddle",
                        def.scene_file
                    );
                }
            }
        }
    }
}

#[test]
fn shipped_level_has_full_brick_grid() {
    let scene = load_scene();
    let brick_names: Vec<&str> = scene
        .entities
        .iter()
        .filter_map(|e| e.name.as_deref())
        .filter(|n| n.starts_with("brick"))
        .collect();
    assert_eq!(brick_names.len(), BRICK_ROWS * BRICK_COLS);
    for name in &brick_names {
        let row = brick_row_from_name(name).expect("brick name should parse");
        assert!(row < BRICK_ROWS, "row out of range in {name}");
    }
}

#[test]
fn shipped_level_positions_match_generated_grid() {
    let scene = load_scene();
    for entity in &scene.entities {
        let name = entity.name.as_deref().expect("all level entities are named");
        let row = brick_row_from_name(name).expect("brick name should parse");
        let col: usize = name
            .split("_c")
            .nth(1)
            .and_then(|s| s.parse().ok())
            .expect("brick name has a column");

        let transform = merged_components(&scene, entity)
            .into_iter()
            .find_map(|c| match c {
                ComponentData::Transform2D { position, scale, .. } => Some((position, scale)),
                _ => None,
            })
            .expect("brick has a Transform2D");

        assert_eq!(transform.0, (brick_x(col), brick_y(row)), "position of {name}");
        assert_eq!(
            transform.1,
            (BRICK_CELL.x / RENDER_UNIT, BRICK_CELL.y / RENDER_UNIT),
            "scale of {name}: the cell drawn at 1x"
        );
    }
}

/// Guards the sprite/collider size footgun: physics ignores
/// Transform2D.scale, so every brick's collider must be its cell — the
/// same 64 x 32 its sprite draws — in EVERY roster level, both modes.
#[test]
fn every_brick_collider_is_its_cell() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            let scene = load_roster_level(def);
            for entity in &scene.entities {
                let name = entity.name.as_deref().expect("all level entities are named");
                let half_extents = merged_components(&scene, entity)
                    .into_iter()
                    .find_map(|c| match c {
                        ComponentData::Collider {
                            shape: engine_core::scene_data::ColliderShapeData::Box { half_extents },
                            ..
                        } => Some(half_extents),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{name} in {} has no box collider", def.scene_file));
                assert_eq!(
                    half_extents,
                    (BRICK_CELL.x / 2.0, BRICK_CELL.y / 2.0),
                    "collider of {name} in {}",
                    def.scene_file
                );
            }
        }
    }
}

#[test]
fn plain_bricks_do_not_glow_and_the_grid_fits_between_the_walls() {
    // The foods are art, not light: emissive belongs to the candy-dropping bricks'
    // pulse, which the game drives at runtime, never to the scene.
    let total_width = BRICK_COLS as f32 * BRICK_CELL.x + (BRICK_COLS as f32 - 1.0) * BRICK_GAP;
    assert!(total_width < 2.0 * PLAYFIELD_HALF_W);
    for mode in BOTH_MODES {
        for def in roster(mode) {
            let scene = load_roster_level(def);
            for entity in &scene.entities {
                let emissive = merged_components(&scene, entity)
                    .iter()
                    .find_map(|c| match c {
                        ComponentData::Sprite { emissive, .. } => Some(*emissive),
                        _ => None,
                    })
                    .expect("brick has a sprite");
                assert_eq!(emissive, 0.0, "{:?} in {}", entity.name, def.scene_file);
            }
        }
    }
}

/// A scene brick's sheet, tag, autoplay clip, sprite offset and depth.
struct SceneBrick {
    name: String,
    sheet: Option<String>,
    tag: Option<String>,
    autoplay: Option<String>,
    offset: (f32, f32),
    depth: f32,
}

fn scene_bricks(scene: &SceneData) -> Vec<SceneBrick> {
    scene
        .entities
        .iter()
        .map(|entity| {
            let components = merged_components(scene, entity);
            let (sheet, autoplay) = components
                .iter()
                .find_map(|c| match c {
                    ComponentData::SpriteAnimation { sheet, autoplay, .. } => {
                        Some((sheet.clone(), autoplay.clone()))
                    }
                    _ => None,
                })
                .unwrap_or((None, None));
            let (offset, depth) = components
                .iter()
                .find_map(|c| match c {
                    ComponentData::Sprite { offset, depth, .. } => Some((*offset, *depth)),
                    _ => None,
                })
                .expect("brick has a sprite");
            let tag = components.iter().find_map(|c| match c {
                ComponentData::EntityTag { tag } => Some(tag.clone()),
                _ => None,
            });
            SceneBrick {
                name: entity.name.clone().expect("all level entities are named"),
                sheet,
                tag,
                autoplay,
                offset,
                depth,
            }
        })
        .collect()
}

/// Each row of every shipped wall is its tier of the pyramid, read from the brick's
/// own sheet — the same path the game resolves a scene brick's food through.
#[test]
fn every_shipped_brick_in_row_n_is_row_ns_food() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            for brick in scene_bricks(&load_roster_level(def)) {
                let row = brick_row_from_name(&brick.name).expect("brick name should parse");
                let food = brick.sheet.as_deref().and_then(Food::from_sheet_path);
                assert_eq!(
                    food,
                    Some(food_for_row(row)),
                    "{} in {} plays {:?}",
                    brick.name,
                    def.scene_file,
                    brick.sheet
                );
                assert_eq!(brick_food(&brick.name, brick.sheet.as_deref()), food_for_row(row));
            }
        }
    }
}

/// A shipped scene's file, its brick count, and how often each tag appears in it.
type SceneBrickCensus = (&'static str, usize, &'static [(&'static str, usize)]);

/// The re-skin moved the bricks, never added, lost or re-tagged one: each shipped
/// scene's brick count and tag multiset are the four-level game's.
#[test]
fn every_shipped_scene_keeps_its_bricks_and_tags() {
    let expected: [SceneBrickCensus; 8] = [
        ("level1.scene.ron", 60, &[("drop_multiball", 1), ("armored2", 2)]),
        ("level2.scene.ron", 51, &[("drop_multiball", 2), ("armored2", 21), ("armored3", 6)]),
        (
            "level3.scene.ron",
            31,
            &[("drop_multiball", 6), ("drop_wrecking", 3), ("armored2", 4), ("drop_insiculous", 1)],
        ),
        (
            "level4.scene.ron",
            54,
            &[
                ("drop_insiculous", 2),
                ("armored3", 16),
                ("armored2", 10),
                ("armored2+drop_wrecking", 2),
                ("drop_multiball", 3),
                ("drop_wrecking", 1),
            ],
        ),
        ("level1_2p.scene.ron", 60, &[("armored2", 8)]),
        ("level2_2p.scene.ron", 60, &[("armored2", 16), ("armored3", 20)]),
        (
            "level3_2p.scene.ron",
            60,
            &[("drop_insiculous", 2), ("drop_multiball", 6), ("drop_wrecking", 2)],
        ),
        (
            "level4_2p.scene.ron",
            60,
            &[
                ("drop_multiball", 4),
                ("armored2", 16),
                ("armored2+drop_wrecking", 4),
                ("armored3", 16),
                ("drop_insiculous", 4),
            ],
        ),
    ];
    let all_levels: Vec<&LevelDef> = BOTH_MODES.iter().flat_map(|&mode| roster(mode)).collect();
    assert_eq!(all_levels.len(), expected.len());

    for (scene_file, count, tags) in expected {
        let def = all_levels
            .iter()
            .find(|def| def.scene_file == scene_file)
            .unwrap_or_else(|| panic!("{scene_file} is on a roster"));
        let bricks = scene_bricks(&load_roster_level(def));
        assert_eq!(bricks.len(), count, "{scene_file}'s brick count");

        let mut found: HashMap<String, usize> = HashMap::new();
        for brick in bricks.iter().filter_map(|brick| brick.tag.clone()) {
            *found.entry(brick).or_default() += 1;
        }
        let wanted: HashMap<String, usize> =
            tags.iter().map(|(tag, times)| (tag.to_string(), *times)).collect();
        assert_eq!(found, wanted, "{scene_file}'s tags");
    }
}

/// Armor is foil from the first frame: every `armored{N}` brick autoplays the foil
/// frame, and every other brick the bare food. The scene is the one mechanism —
/// nothing at runtime sets the starting frame.
#[test]
fn every_armored_brick_starts_in_foil_and_every_other_bare() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            for brick in scene_bricks(&load_roster_level(def)) {
                let armored = brick.tag.as_deref().is_some_and(|tag| parse_brick_tag(tag).hits > 1);
                let clip = if armored { BRICK_ARMORED } else { BRICK_INTACT };
                assert_eq!(
                    brick.autoplay.as_deref(),
                    Some(clip),
                    "{} in {} (tag {:?})",
                    brick.name,
                    def.scene_file,
                    brick.tag
                );
            }
        }
    }
}

/// Every brick draws its cell on its collider (no anchor), at its food sheet's one
/// depth — a sheet split across depths would punch holes in the sheets between.
#[test]
fn every_scene_brick_is_drawn_on_its_cell_at_its_sheets_depth() {
    for mode in BOTH_MODES {
        for def in roster(mode) {
            for brick in scene_bricks(&load_roster_level(def)) {
                assert_eq!(brick.offset, (0.0, 0.0), "{} in {}", brick.name, def.scene_file);
                let food = brick.sheet.as_deref().and_then(Food::from_sheet_path).expect("a food sheet");
                assert_eq!(brick.depth, food.spec().depth, "{} in {}", brick.name, def.scene_file);
            }
        }
    }
    for spec in &FOODS {
        assert_eq!(spec.sheet.sprite_offset(), Vec2::ZERO, "{}", spec.sheet.path);
    }
}

#[test]
fn a_brick_whose_sheet_names_no_food_falls_back_to_its_row() {
    assert_eq!(brick_food("brick_r0_c3", Some("sprites/ai_pyramid_steak_64x32.png")), Food::Steak);
    assert_eq!(brick_food("brick_r0_c3", Some("sprites/unknown.png")), food_for_row(0));
    assert_eq!(brick_food("brick_r5_c3", None), food_for_row(5));
    assert_eq!(food_for_row(0), Food::Donut, "the top row is the pyramid's peak");
    assert_eq!(food_for_row(BRICK_ROWS - 1), Food::Watermelon, "the bottom row its base");
}

#[test]
fn parse_brick_tag_grammar_table() {
    let d = BrickSpec::default();
    assert_eq!(d, BrickSpec { hits: 1, drop: None });

    assert_eq!(parse_brick_tag("armored2"), BrickSpec { hits: 2, drop: None });
    assert_eq!(parse_brick_tag("armored9"), BrickSpec { hits: 9, drop: None });
    assert_eq!(
        parse_brick_tag("drop_multiball"),
        BrickSpec { hits: 1, drop: Some(PickupKind::Multiball) }
    );
    assert_eq!(
        parse_brick_tag("drop_wrecking"),
        BrickSpec { hits: 1, drop: Some(PickupKind::Wrecking) }
    );
    assert_eq!(
        parse_brick_tag("drop_insiculous"),
        BrickSpec { hits: 1, drop: Some(PickupKind::Insiculous) }
    );
    assert_eq!(
        parse_brick_tag("armored2+drop_wrecking"),
        BrickSpec { hits: 2, drop: Some(PickupKind::Wrecking) }
    );
    // Token order doesn't matter; whitespace tolerated
    assert_eq!(
        parse_brick_tag(" drop_wrecking + armored3 "),
        BrickSpec { hits: 3, drop: Some(PickupKind::Wrecking) }
    );
    // Duplicates: last wins
    assert_eq!(
        parse_brick_tag("armored2+armored3"),
        BrickSpec { hits: 3, drop: None }
    );
    // Unknown/malformed tokens degrade to defaults, never panic
    assert_eq!(parse_brick_tag(""), d);
    assert_eq!(parse_brick_tag("bogus"), d);
    assert_eq!(parse_brick_tag("armored1"), d, "1-hit armor is not armor");
    assert_eq!(parse_brick_tag("armored99"), d, "out-of-range armor rejected");
    assert_eq!(parse_brick_tag("armoredX"), d);
    assert_eq!(
        parse_brick_tag("bogus+drop_multiball"),
        BrickSpec { hits: 1, drop: Some(PickupKind::Multiball) },
        "unknown tokens are skipped, not fatal"
    );
}

/// Every EntityTag authored in a roster level must parse to a meaningful
/// spec — a tag that parses to the plain-brick default is a typo.
#[test]
fn every_roster_level_tag_is_meaningful() {
    for mode in BOTH_MODES {
        let mut tagged_total = 0;
        for def in roster(mode) {
            let scene = load_roster_level(def);
            for entity in &scene.entities {
                let name = entity.name.as_deref().unwrap_or("<unnamed>");
                for c in merged_components(&scene, entity) {
                    if let ComponentData::EntityTag { tag } = c {
                        assert_ne!(
                            parse_brick_tag(&tag),
                            BrickSpec::default(),
                            "tag '{tag}' on {name} in {} means nothing",
                            def.scene_file
                        );
                        tagged_total += 1;
                    }
                }
            }
        }
        assert!(
            tagged_total > 20,
            "expected plenty of special bricks in the {mode:?} roster, got {tagged_total}"
        );
    }
}

/// Prize levels (index 2 and 3) must actually rain power-ups, in both
/// rosters.
#[test]
fn prize_levels_have_drop_bricks() {
    for mode in BOTH_MODES {
        for i in [2usize, 3] {
            let def = &roster(mode)[i];
            let scene = load_roster_level(def);
            let drops = scene
                .entities
                .iter()
                .flat_map(|e| merged_components(&scene, e))
                .filter_map(|c| match c {
                    ComponentData::EntityTag { tag } => parse_brick_tag(&tag).drop,
                    _ => None,
                })
                .count();
            assert!(drops >= 4, "{} has only {drops} drop bricks", def.scene_file);
        }
    }
}

#[test]
fn brick_row_from_name_parses_valid_and_rejects_invalid() {
    assert_eq!(brick_row_from_name("brick_r0_c0"), Some(0));
    assert_eq!(brick_row_from_name("brick_r5_c9"), Some(5));
    assert_eq!(brick_row_from_name("brick_r12_c3"), Some(12));
    assert_eq!(brick_row_from_name("brick_rX_c0"), None);
    assert_eq!(brick_row_from_name("brick"), None);
    assert_eq!(brick_row_from_name("paddle"), None);
}

#[test]
fn brick_value_from_name_maps_rows_and_defaults_minimum() {
    assert_eq!(brick_value_from_name("brick_r0_c0"), brick_value(0));
    assert_eq!(brick_value_from_name("brick_r5_c9"), brick_value(5));
    // Renamed or out-of-range bricks score the minimum instead of panicking
    assert_eq!(brick_value_from_name("brick_r99_c0"), BRICK_VALUE_STEP);
    assert_eq!(brick_value_from_name("brick_custom"), BRICK_VALUE_STEP);
}

#[test]
fn bricks_from_names_builds_bookkeeping_from_world() {
    let mut world = World::new();
    let mut named = HashMap::new();

    // A cheese wheel placed on the top row: the sheet, not the row, says what it is.
    let brick = world.create_entity();
    let animation = SpriteAnimation {
        sheet: Some("sprites/ai_pyramid_cheese_wheel_64x32.png".to_string()),
        ..SpriteAnimation::default()
    };
    world.add_component(&brick, animation).ok();
    named.insert("brick_r0_c0".to_string(), brick);

    // Non-brick entities are ignored
    let paddle = world.create_entity();
    named.insert("paddle".to_string(), paddle);

    let bricks = bricks_from_names(&named, &world);
    assert_eq!(bricks.len(), 1);
    assert_eq!(bricks[0].entity, brick);
    assert_eq!(bricks[0].value, brick_value(0));
    assert_eq!(bricks[0].food, Food::CheeseWheel);
    // Untagged brick gets the plain defaults
    assert_eq!(bricks[0].hits_left, 1);
    assert_eq!(bricks[0].drop, None);
}

#[test]
fn bricks_from_names_reads_entity_tags() {
    let mut world = World::new();
    let mut named = HashMap::new();

    let brick = world.create_entity();
    world.add_component(&brick, Sprite::new(0)).ok();
    world
        .add_component(&brick, EntityTag::new("armored3+drop_insiculous"))
        .ok();
    named.insert("brick_r1_c1".to_string(), brick);

    let bricks = bricks_from_names(&named, &world);
    assert_eq!(bricks.len(), 1);
    assert_eq!(bricks[0].hits_left, 3);
    assert_eq!(bricks[0].drop, Some(PickupKind::Insiculous));
}
