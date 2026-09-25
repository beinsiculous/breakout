use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

/// An opaque colour from its `0xRRGGBB` hex — the form DEION_STYLE.md § 4 records
/// every ramp in, so a constant here reads the same as the style guide's row.
const fn rgb(hex: u32) -> Vec4 {
    Vec4::new(
        ((hex >> 16) & 0xFF) as f32 / 255.0,
        ((hex >> 8) & 0xFF) as f32 / 255.0,
        (hex & 0xFF) as f32 / 255.0,
        1.0,
    )
}

// --- the sheets ---------------------------------------------------------------------
// Every bound is measured from the synced PNG: the opaque box of the named reference
// frame inside its own cell, Y down, the bottom-right corner one past the last opaque
// pixel.

/// A food brick's cell. Every brick's collider and sprite are this cell, not the
/// food's measured box: the smaller foods sit inside it with a margin a ball bounces
/// off, the stated price of a wall with no gaps a 16 px ball can thread.
pub(crate) const BRICK_CELL: Vec2 = Vec2::new(64.0, 32.0);

const TONG_CELL: Vec2 = Vec2::new(64.0, 96.0);
/// The tong's `closed` frame — frame 5 of the sheet: 22 x 78, centred in its cell both
/// ways, the same bytes Tong measured.
const TONG_CLOSED_BOUNDS: (Vec2, Vec2) = (Vec2::new(21.0, 9.0), Vec2::new(43.0, 87.0));

const BALL_CELL: Vec2 = Vec2::new(16.0, 16.0);
/// Deion's body in every `idle` frame of both ball sheets, the mohawk above it
/// excluded: the outline never moves, so this box is the collider and the anchor on
/// either sheet, and swapping sheets moves nothing.
const BALL_BODY_BOUNDS: (Vec2, Vec2) = (Vec2::new(2.0, 4.0), Vec2::new(14.0, 16.0));

const CANDY_CELL: Vec2 = Vec2::new(48.0, 48.0);

const COURT_TILE_CELL: Vec2 = Vec2::new(64.0, 64.0);
const COURT_EDGE_CELL: Vec2 = Vec2::new(64.0, 16.0);

pub(crate) const TONG_LEFT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_left.png",
    cell: TONG_CELL,
    bounds: TONG_CLOSED_BOUNDS,
};
pub(crate) const TONG_RIGHT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_right.png",
    cell: TONG_CELL,
    bounds: TONG_CLOSED_BOUNDS,
};
/// Deion as the water ball — the form he plays in every chaos mode.
pub(crate) const BALL_WATER: SheetSpec = SheetSpec {
    path: "sprites/ai_deion_16.png",
    cell: BALL_CELL,
    bounds: BALL_BODY_BOUNDS,
};
/// Deion frozen — the form he takes while the wrecking candy runs.
pub(crate) const BALL_ICE: SheetSpec = SheetSpec {
    path: "sprites/ai_deion_ice_16.png",
    cell: BALL_CELL,
    bounds: BALL_BODY_BOUNDS,
};
/// The multiball candy's `idle` frames, their union: 39 x 21.
pub(crate) const CANDY_MULTIBALL: SheetSpec = SheetSpec {
    path: "sprites/ai_pyramid_multiball_candy_48x48.png",
    cell: CANDY_CELL,
    bounds: (Vec2::new(5.0, 14.0), Vec2::new(44.0, 35.0)),
};
/// The wrecking candy's `idle` frames, their union: 39 x 29.
pub(crate) const CANDY_WRECKING: SheetSpec = SheetSpec {
    path: "sprites/ai_pyramid_wrecking_candy_48x48.png",
    cell: CANDY_CELL,
    bounds: (Vec2::new(5.0, 9.0), Vec2::new(44.0, 38.0)),
};
/// The insiculous candy's `idle` frames, their union: 39 x 27.
pub(crate) const CANDY_INSICULOUS: SheetSpec = SheetSpec {
    path: "sprites/ai_pyramid_insiculous_candy_48x48.png",
    cell: CANDY_CELL,
    bounds: (Vec2::new(5.0, 11.0), Vec2::new(44.0, 38.0)),
};
pub(crate) const COURT_TILE: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_court_64x64.png",
    cell: COURT_TILE_CELL,
    bounds: (Vec2::ZERO, COURT_TILE_CELL),
};
pub(crate) const COURT_EDGE: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_court_edge_64x16.png",
    cell: COURT_EDGE_CELL,
    bounds: (Vec2::ZERO, COURT_EDGE_CELL),
};

// --- the foods ----------------------------------------------------------------------

/// The six food groups, one per row of the wall and tier of the pyramid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Food {
    Watermelon,
    Broccoli,
    Baguette,
    CheeseWheel,
    Steak,
    Donut,
}

/// Everything a food brick draws with: its sheet, the tint of its shatter, and the
/// one depth its whole sheet is drawn at.
pub(crate) struct FoodSpec {
    pub(crate) food: Food,
    pub(crate) sheet: SheetSpec,
    /// The most-used second or third shade of the food's § 4 ramp in its `intact`
    /// frame — the colour the brick reads as, measured rather than chosen.
    pub(crate) burst_color: Vec4,
    pub(crate) depth: f32,
}

const fn brick_sheet(path: &'static str) -> SheetSpec {
    SheetSpec { path, cell: BRICK_CELL, bounds: (Vec2::ZERO, BRICK_CELL) }
}

/// The pyramid from its base up. `scripts/regenerate_levels.py` writes each sheet's
/// depth into the scenes' prefabs, and `levels_tests.rs` holds the two together.
pub(crate) const FOODS: [FoodSpec; 6] = [
    FoodSpec {
        food: Food::Watermelon,
        sheet: brick_sheet("sprites/ai_pyramid_watermelon_64x32.png"),
        burst_color: rgb(0xD33F4C),
        depth: -0.60,
    },
    FoodSpec {
        food: Food::Broccoli,
        sheet: brick_sheet("sprites/ai_pyramid_broccoli_64x32.png"),
        burst_color: rgb(0x3FA45C),
        depth: -0.55,
    },
    FoodSpec {
        food: Food::Baguette,
        sheet: brick_sheet("sprites/ai_pyramid_baguette_64x32.png"),
        burst_color: rgb(0xD8A05C),
        depth: -0.50,
    },
    FoodSpec {
        food: Food::CheeseWheel,
        sheet: brick_sheet("sprites/ai_pyramid_cheese_wheel_64x32.png"),
        burst_color: rgb(0xFFE08A),
        depth: -0.45,
    },
    FoodSpec {
        food: Food::Steak,
        sheet: brick_sheet("sprites/ai_pyramid_steak_64x32.png"),
        burst_color: rgb(0xD33F4C),
        depth: -0.40,
    },
    FoodSpec {
        food: Food::Donut,
        sheet: brick_sheet("sprites/ai_pyramid_donut_64x32.png"),
        burst_color: rgb(0xD8A05C),
        depth: -0.35,
    },
];

impl Food {
    pub(crate) fn spec(self) -> &'static FoodSpec {
        FOODS
            .iter()
            .find(|spec| spec.food == self)
            .expect("every food has a row in FOODS")
    }

    /// The food whose sheet `path` names, matched by file name so a scene may spell
    /// the directory either way.
    pub(crate) fn from_sheet_path(path: &str) -> Option<Food> {
        let file_name = |full: &str| full.rsplit('/').next().unwrap_or(full).to_string();
        let wanted = file_name(path);
        FOODS.iter().find(|spec| file_name(spec.sheet.path) == wanted).map(|spec| spec.food)
    }
}

/// The food a row of the wall is made of: the top row is the pyramid's peak, sweets,
/// and the bottom row its base, fruit. A row past the base is the base.
pub(crate) fn food_for_row(row: usize) -> Food {
    let tiers_from_the_base = BRICK_ROWS - 1 - row.min(BRICK_ROWS - 1);
    FOODS[tiers_from_the_base].food
}

// --- the playfield ------------------------------------------------------------------
// Re-derived from the measured art above, in the order the geometry nests: the window,
// the walls, then the things standing inside them.

/// The court edge strip's thickness, and so the wall's: the collider's inner face is
/// the drawn rail's.
pub(crate) const WALL_THICKNESS: f32 = COURT_EDGE_CELL.y;
/// Inner edge of the side walls — the playfield half-width.
pub(crate) const PLAYFIELD_HALF_W: f32 = WIN_W / 2.0 - WALL_THICKNESS;

/// The paddle body is the closed tong lying on its side: the art's length along x, its
/// thickness along y.
pub(crate) const PADDLE_W: f32 = TONG_CLOSED_BOUNDS.1.y - TONG_CLOSED_BOUNDS.0.y;
pub(crate) const PADDLE_H: f32 = TONG_CLOSED_BOUNDS.1.x - TONG_CLOSED_BOUNDS.0.x;
/// A quarter turn counter-clockwise lays a tong on the counter with the arm that faced
/// Tong's court facing the field and its mouth along the direction of travel.
pub(crate) const TONG_ROTATION: f32 = std::f32::consts::FRAC_PI_2;
pub(crate) const PADDLE_Y: f32 = -260.0;
/// Player 2's paddle in co-op guards the top edge, mirroring the bottom one.
pub(crate) const PADDLE_TOP_Y: f32 = -PADDLE_Y;
/// How far a tong may travel before its art would cross a wall.
pub(crate) const PADDLE_MAX_X: f32 = PLAYFIELD_HALF_W - PADDLE_W / 2.0;
pub(crate) const PADDLE_SPEED: f32 = 520.0;
/// Maximum bounce deflection off the paddle, in radians from straight up.
/// Hitting the paddle dead center returns the ball vertically; the very
/// edge sends it out at this angle.
pub(crate) const PADDLE_MAX_BOUNCE_ANGLE: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees

/// The ball's collider: Deion's body's measured width, and one circle on it.
pub(crate) const BALL_SIZE: f32 = BALL_BODY_BOUNDS.1.x - BALL_BODY_BOUNDS.0.x;
pub(crate) const BALL_RADIUS: f32 = BALL_SIZE / 2.0;
pub(crate) const BALL_SPEED: f32 = 360.0;
pub(crate) const BALL_MAX_SPEED: f32 = 760.0;
/// Minimum fraction of the ball's speed that must be vertical. Prevents the
/// ball ping-ponging horizontally between the side walls forever.
pub(crate) const MIN_VERTICAL_FRACTION: f32 = 0.25;
/// Insane mode: ball speed multiplier gained on every paddle hit.
pub(crate) const INSANE_SPEED_GAIN: f32 = 1.15;

/// Resting offset of a served ball above the paddle center.
pub(crate) const SERVE_OFFSET_Y: f32 = PADDLE_H / 2.0 + BALL_RADIUS + 2.0;
/// Full width of the random launch-angle spread, in radians. A served ball
/// leaves within ±half this off vertical.
pub(crate) const LAUNCH_ANGLE_SPREAD: f32 = 0.6;

pub(crate) const BRICK_COLS: usize = 10;
pub(crate) const BRICK_ROWS: usize = 6;
pub(crate) const BRICK_GAP: f32 = 4.0;
/// Y position of the center of the top brick row.
pub(crate) const BRICK_TOP_Y: f32 = 240.0;
/// Co-op: the top row of the middle band, placed so the band is centred on the
/// window and each paddle faces the same reaction room.
pub(crate) const BRICK_TOP_Y_2P: f32 = (BRICK_ROWS as f32 - 1.0) * (BRICK_CELL.y + BRICK_GAP) / 2.0;
/// Points awarded per brick = (rows from the bottom of the grid) * this.
pub(crate) const BRICK_VALUE_STEP: u32 = 10;

// Falling candies dropped by special bricks.
pub(crate) const PICKUP_FALL_SPEED: f32 = 180.0;
/// Wrecking-ball effect length; catching another pickup refreshes it.
pub(crate) const WRECKING_DURATION: f32 = 10.0;
/// Cap on simultaneous extra balls (multiball grants beyond it fizzle).
pub(crate) const MAX_EXTRA_BALLS: usize = 6;

pub(crate) const STARTING_LIVES: u32 = 3;
/// Bricks destroyed in one volley (without touching the paddle) to unlock
/// the combo achievement.
pub(crate) const COMBO_TARGET: u32 = 5;

/// Extra margin past the window edge before an off-screen ball counts as
/// lost (safety net for CCD misses / NaN positions).
pub(crate) const BALL_LOST_BOUNDS_PAD: f32 = 60.0;
/// How far inside the window's edge a lost ball's splash is drawn: the loss sensor
/// fires with the ball's centre outside the window, where a splash would not show.
pub(crate) const SPLASH_EDGE_INSET: f32 = 8.0;

// Radial impulses rippled into the backdrop grid, one strength and radius per event
// that disturbs it.
pub(crate) const GRID_IMPULSE_PADDLE_HIT_STRENGTH: f32 = 200.0;
pub(crate) const GRID_IMPULSE_PADDLE_HIT_RADIUS: f32 = 70.0;
pub(crate) const GRID_IMPULSE_BRICK_DESTROY_STRENGTH: f32 = 260.0;
pub(crate) const GRID_IMPULSE_BRICK_DESTROY_RADIUS: f32 = 90.0;
pub(crate) const GRID_IMPULSE_BALL_LOST_STRENGTH: f32 = 700.0;
pub(crate) const GRID_IMPULSE_BALL_LOST_RADIUS: f32 = 160.0;

/// The backdrop grid's alpha, well under the preset's resting value: the lattice
/// reads over the counter art without veiling it.
pub(crate) const BACKDROP_ALPHA: f32 = 0.25;

/// Collider outlines for the F1 overlay: bright magenta at a high emissive so they
/// bloom over the art.
pub(crate) const DEBUG_COLLIDER_COLOR: Vec4 = Vec4::new(1.0, 0.2, 1.0, 0.9);
pub(crate) const DEBUG_COLLIDER_EMISSIVE: f32 = 2.0;

// Particle tints — every art sprite is drawn white, so these colour only the bursts.
/// Player 1's paddle-hit spray: the red of the left tong's grip.
pub(crate) const P1_BURST_COLOR: Vec4 = rgb(0xD33F4C);
/// Player 2's paddle-hit spray: the water blue of the right tong's grip.
pub(crate) const P2_BURST_COLOR: Vec4 = rgb(0x29D1EA);
/// An armor hit's spark: the foil's plate neutral.
pub(crate) const FOIL_BURST_COLOR: Vec4 = rgb(0xCFC9DC);
/// A lost ball's burst under the splash: Deion's body core.
pub(crate) const SPLASH_BURST_COLOR: Vec4 = rgb(0x29D1EA);
/// The wrecking countdown label: the ice ramp's pale highlight, the frozen ball's.
pub(crate) const WRECKING_LABEL_COLOR: Vec4 = rgb(0x8DF3F8);

// --- draw depths --------------------------------------------------------------------
// One depth per sheet, never per entity: the batcher draws a texture's batch whole,
// ordered by its lowest depth, and transparent texels write depth — so a sheet with
// entities both under and over another sheet's would punch holes in it. Nested, from
// the counter up; every one stays under the particles the engine draws at 0.5. The six
// food sheets' depths are in `FOODS`, between the strips and the tongs.

pub(crate) const COURT_TILE_DEPTH: f32 = -2.0;
pub(crate) const WALL_DEPTH: f32 = -1.0;
pub(crate) const TONG_LEFT_DEPTH: f32 = -0.20;
pub(crate) const TONG_RIGHT_DEPTH: f32 = -0.15;
/// The water ball, and the splash a water ball leaves.
pub(crate) const BALL_WATER_DEPTH: f32 = 0.0;
/// The frozen ball, and the shatter a frozen ball leaves.
pub(crate) const BALL_ICE_DEPTH: f32 = 0.05;
/// The candies and the collects they leave: over the tongs, because a candy is caught
/// the frame it touches one.
pub(crate) const CANDY_MULTIBALL_DEPTH: f32 = 0.20;
pub(crate) const CANDY_WRECKING_DEPTH: f32 = 0.25;
pub(crate) const CANDY_INSICULOUS_DEPTH: f32 = 0.30;

/// The counter's tile, which also sets how many cells cover the window — the counts
/// are `ceil(WIN / COURT_TILE_PX)` each way, computed where the map is built.
pub(crate) const COURT_TILE_PX: f32 = COURT_TILE_CELL.x;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::{ball_machine, candy_machine, one_shot_machine, tong_machine};
    use crate::test_support::sidecar;
    use crate::types::*;

    /// Every sheet the game declares, beside the machines that play it and the grid its
    /// declared cell must cut the synced PNG into.
    fn every_sheet() -> Vec<(&'static SheetSpec, Vec<ClipStateMachine>, (u32, u32))> {
        let tongs = || vec![tong_machine(Facing::Up), tong_machine(Facing::Down)];
        let ball = || vec![ball_machine(), one_shot_machine(BALL_HURT)];
        let candy = || vec![candy_machine(), one_shot_machine(CANDY_COLLECT)];
        let mut sheets = vec![
            (&TONG_LEFT, tongs(), (4, 7)),
            (&TONG_RIGHT, tongs(), (4, 7)),
            (&BALL_WATER, ball(), (4, 2)),
            (&BALL_ICE, ball(), (4, 2)),
            (&CANDY_MULTIBALL, candy(), (4, 2)),
            (&CANDY_WRECKING, candy(), (4, 2)),
            (&CANDY_INSICULOUS, candy(), (4, 2)),
            (&COURT_TILE, Vec::new(), (1, 1)),
            (&COURT_EDGE, Vec::new(), (1, 1)),
        ];
        for spec in &FOODS {
            sheets.push((&spec.sheet, Vec::new(), (3, 1)));
        }
        sheets
    }

    #[test]
    fn every_sheet_is_measured_inside_its_own_cell() {
        // A transposed or overflowing box still yields a plausible offset, so the
        // measurement itself is checked before anything is derived from it.
        for (spec, _, _) in every_sheet() {
            assert!(spec.is_within_cell(), "{} is measured outside its cell", spec.path);
        }
    }

    #[test]
    fn every_state_plays_a_clip_its_sheet_really_has() {
        // The synced PNGs and their sidecars, read through the engine's own GPU-free load
        // path — the one check that ties the game's tables to the committed art. The
        // declared cell over the real PNG must also cut the sheet into the grid the art
        // really has.
        for (spec, machines, expected_grid) in every_sheet() {
            let prepared = sidecar(spec);
            assert_eq!(
                (prepared.sheet.grid.cols, prepared.sheet.grid.rows),
                expected_grid,
                "{}: the declared {}x{} cell does not cut the PNG into {expected_grid:?}",
                spec.path, spec.cell.x, spec.cell.y
            );
            assert_eq!(prepared.filter, TextureFilter::Nearest, "{}: pixel art", spec.path);

            let clips: Vec<&str> = prepared.sheet.clips.iter().map(|(name, _)| name.as_str()).collect();
            for machine in &machines {
                for (state, row) in machine.states() {
                    assert!(
                        clips.contains(&row.clip.as_str()),
                        "{}: state '{state}' plays '{}', which the sheet does not have ({clips:?})",
                        spec.path, row.clip
                    );
                }
            }
        }
        for spec in &FOODS {
            let clips: Vec<String> = sidecar(&spec.sheet).sheet.clips.into_iter().map(|(name, _)| name).collect();
            for clip in [BRICK_INTACT, BRICK_ARMORED, BRICK_ARMOR_DAMAGED] {
                assert!(clips.iter().any(|name| name == clip), "{} has no '{clip}'", spec.sheet.path);
            }
        }
    }

    #[test]
    fn every_one_shot_clip_plays_once() {
        // A one-shot despawns when its clip finishes, and a looping clip never does: a
        // splash or a collect whose sidecar loops would stay on screen until the next
        // match start drained it.
        let one_shots = [
            (&BALL_WATER, BALL_HURT),
            (&BALL_ICE, BALL_HURT),
            (&CANDY_MULTIBALL, CANDY_COLLECT),
            (&CANDY_WRECKING, CANDY_COLLECT),
            (&CANDY_INSICULOUS, CANDY_COLLECT),
        ];
        for (spec, clip) in one_shots {
            let prepared = sidecar(spec);
            let (_, declared) = prepared
                .sheet
                .clips
                .iter()
                .find(|(name, _)| name == clip)
                .unwrap_or_else(|| panic!("{} has no '{clip}'", spec.path));
            assert!(!declared.looping, "{}: '{clip}' loops", spec.path);
        }
    }

    #[test]
    fn the_two_balls_share_one_body_and_every_brick_is_its_cell() {
        // Spec-level: the pixels behind these numbers are the art batch's gate.
        assert_eq!(BALL_WATER.bounds, BALL_ICE.bounds, "a sheet swap must move nothing");
        assert_eq!(BALL_WATER.cell, BALL_ICE.cell);
        assert_eq!(BALL_WATER.sprite_offset(), BALL_ICE.sprite_offset());
        for spec in &FOODS {
            assert_eq!(spec.sheet.bounds, (Vec2::ZERO, BRICK_CELL), "{}", spec.sheet.path);
            assert_eq!(spec.sheet.sprite_offset(), Vec2::ZERO);
        }
    }

    #[test]
    fn the_colliders_are_the_measured_art() {
        let footprint = |spec: &SheetSpec| spec.bounds.1 - spec.bounds.0;
        // The tong lies on its side: its drawn length is the paddle's width.
        assert_eq!(footprint(&TONG_LEFT), Vec2::new(PADDLE_H, PADDLE_W));
        assert_eq!(footprint(&TONG_RIGHT), Vec2::new(PADDLE_H, PADDLE_W));
        assert_eq!(TONG_LEFT.sprite_offset(), Vec2::ZERO, "the closed tong is centred both ways");
        assert_eq!(footprint(&BALL_WATER).x, BALL_SIZE);
        assert_eq!(BALL_SIZE, BALL_RADIUS * 2.0);
    }

    #[test]
    fn the_derived_playfield_numbers() {
        assert_eq!((PADDLE_W, PADDLE_H), (78.0, 22.0));
        assert_eq!(WALL_THICKNESS, 16.0);
        assert_eq!(PLAYFIELD_HALF_W, 384.0);
        assert_eq!(PADDLE_MAX_X, 345.0);
        assert_eq!(BRICK_TOP_Y_2P, 90.0);
        // The tong's art reaches the wall's inner face and stops there.
        assert_eq!(PADDLE_MAX_X + PADDLE_W / 2.0, PLAYFIELD_HALF_W);
    }

    #[test]
    fn each_sheet_draws_at_one_depth_nested_from_the_counter_up() {
        // Batches are drawn whole, ordered by their lowest depth: the nesting order is
        // the draw order, and every depth stays under the engine's particles at 0.5.
        let brick_depths: Vec<f32> = FOODS.iter().map(|spec| spec.depth).collect();
        let highest_brick = brick_depths.iter().copied().fold(f32::MIN, f32::max);
        let lowest_brick = brick_depths.iter().copied().fold(f32::MAX, f32::min);
        let ladder = [
            COURT_TILE_DEPTH,
            WALL_DEPTH,
            lowest_brick,
            highest_brick,
            TONG_LEFT_DEPTH,
            TONG_RIGHT_DEPTH,
            BALL_WATER_DEPTH,
            BALL_ICE_DEPTH,
            CANDY_MULTIBALL_DEPTH,
            CANDY_WRECKING_DEPTH,
            CANDY_INSICULOUS_DEPTH,
        ];
        assert!(ladder.windows(2).all(|pair| pair[0] < pair[1]), "{ladder:?}");
        const { assert!(CANDY_INSICULOUS_DEPTH < 0.5) };
        let mut distinct = brick_depths.clone();
        distinct.sort_by(f32::total_cmp);
        distinct.dedup();
        assert_eq!(distinct.len(), FOODS.len(), "one depth per food sheet");
    }

    #[test]
    fn a_sheet_path_names_its_food() {
        for spec in &FOODS {
            assert_eq!(Food::from_sheet_path(spec.sheet.path), Some(spec.food));
            assert_eq!(spec.food.spec().sheet.path, spec.sheet.path);
        }
        assert_eq!(Food::from_sheet_path("elsewhere/ai_pyramid_donut_64x32.png"), Some(Food::Donut));
        assert_eq!(Food::from_sheet_path("sprites/ai_deion_16.png"), None);
    }
}
