//! All entity creation. Every entity gets a `Name` component so the editor hierarchy
//! shows "Tong P1" instead of "Entity 7".
//!
//! Every art entity is built the same way: the sheet's first cell, the cell-sized
//! transform scale, and the sheet's measured anchor as the sprite offset, so the art
//! lands on the entity rather than at the cell's corner. Entities that change clip under
//! their own rules — the tongs, the ball, the candies, the one-shots — carry a
//! `ClipStateMachine` whose table names their states and clips.

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::*;

/// The components every art entity carries: the sheet's first cell as its sprite at
/// the sheet's depth, the sheet's clips as its animation, and the anchor offset that
/// puts the reference frame on the entity.
pub(crate) fn art_components(sheet: &SpriteSheet, spec: &SheetSpec, depth: f32) -> (Sprite, SpriteAnimation) {
    (sheet.sprite().with_offset(spec.sprite_offset()).with_depth(depth), sheet.animation())
}

/// A tong's ten states, Tong's five clips in each facing: resting `open` and `closed`,
/// the `closing` and `opening` between them, and the scowl a lost life plays.
///
/// One fixed table, never rebuilt: a rebuilt machine restarts its clip, and the engine
/// has no way to retarget a transition. The scowl always lands `closed`, because a serve
/// or a game over follows it and the serve holds the jaw shut. A clip in flight plays
/// out before any jaw or facing change: `gameplay::paddles` moves the machine only from
/// a rest.
pub(crate) fn tong_machine(facing: Facing) -> ClipStateMachine {
    let mut states = Vec::with_capacity(10);
    for side in [Facing::Up, Facing::Down] {
        let open = tong_state(TONG_OPEN, side);
        let closing = tong_state(TONG_CLOSING, side);
        let closed = tong_state(TONG_CLOSED, side);
        let opening = tong_state(TONG_OPENING, side);
        let scored_on = tong_state(TONG_SCORED_ON, side);
        states.push((open.clone(), ClipState::staying(open.clone())));
        states.push((closing.clone(), ClipState::new(closing, OnFinished::Next(closed.clone()))));
        states.push((closed.clone(), ClipState::staying(closed.clone())));
        states.push((opening.clone(), ClipState::new(opening, OnFinished::Next(open))));
        states.push((scored_on.clone(), ClipState::new(scored_on, OnFinished::Next(closed))));
    }
    ClipStateMachine::new(tong_state(TONG_CLOSED, facing), states)
}

/// The ball's one state: Deion sloshing. `hurt` belongs to the splash that replaces a
/// lost ball, not to the ball, whose body is destroyed the frame it is lost.
pub(crate) fn ball_machine() -> ClipStateMachine {
    ClipStateMachine::new(BALL_IDLE, vec![(BALL_IDLE.to_string(), ClipState::staying(BALL_IDLE))])
}

/// A falling candy's one state. `collect` belongs to the one-shot that replaces it:
/// the engine's `Pickups::collect` destroys the candy and its body in the same call.
pub(crate) fn candy_machine() -> ClipStateMachine {
    ClipStateMachine::new(CANDY_IDLE, vec![(CANDY_IDLE.to_string(), ClipState::staying(CANDY_IDLE))])
}

/// A detached effect's machine: it plays `clip` once and despawns.
pub(crate) fn one_shot_machine(clip: &str) -> ClipStateMachine {
    ClipStateMachine::new(clip, vec![(clip.to_string(), ClipState::new(clip, OnFinished::Despawn))])
}

/// Spawn a detached one-shot at `position`: a lost ball's splash, a caught candy's
/// collect. Sprite-only, and it ends itself; the caller keeps the handle in
/// `transient_visuals` so a match start or a quit can end it early.
pub(crate) fn spawn_effect(
    world: &mut World,
    name: &str,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
    depth: f32,
    position: Vec2,
    clip: &str,
) -> EntityId {
    let (sprite, animation) = art_components(sheet, spec, depth);
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(position, 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(one_shot_machine(clip))
        .id()
}

/// Spawn a paddle body at the given edge height (`PADDLE_Y` bottom, `PADDLE_TOP_Y`
/// top). It draws nothing — its tong is a separate entity placed on it every frame —
/// and it never turns: physics would turn the collider with the body.
///
/// It is born wearing the closed tong's box lying on its side, the jaw every match
/// starts with; from then on `gameplay::paddles` dresses it in the pose its tong draws.
pub(crate) fn spawn_paddle(world: &mut World, name: &str, y: f32) -> EntityId {
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::new(Vec2::new(0.0, y)))
        .with(RigidBody::new_kinematic().with_rotation_locked(true))
        .with(Collider::new(ColliderShape::capsule_x(PADDLE_W, PADDLE_H * 0.5))
            .with_friction(0.0)
            .with_restitution(1.0))
        .id()
}

/// Spawn the tong drawn on a paddle: a quarter-turned sprite with no physics, resting
/// closed in `facing`. Its jaw moves under `gameplay::paddles`; the collider it draws is
/// its paddle's.
pub(crate) fn spawn_tong(
    world: &mut World,
    name: &str,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
    depth: f32,
    position: Vec2,
    facing: Facing,
) -> EntityId {
    let (sprite, animation) = art_components(sheet, spec, depth);
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(position, TONG_ROTATION, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(tong_machine(facing))
        .id()
}

/// Spawn one wall's collider: a single continuous box. The strips that draw it are
/// separate, visual-only entities — abutting strip-sized boxes would give the ball a
/// seam to catch on.
pub(crate) fn spawn_wall(world: &mut World, name: &str, pos: Vec2, w: f32, h: f32) -> EntityId {
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::new(pos))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(w, h).with_friction(0.0).with_restitution(1.0))
        .id()
}

/// Spawn the `ceil(length / cell)` counter edge strips that draw one wall, centred on
/// `center`. A side wall's strips are turned a quarter to run up the wall; the
/// outermost pair overhangs the window, so the rail reads as continuous.
pub(crate) fn spawn_wall_strips(
    world: &mut World,
    sheet: &SpriteSheet,
    name_prefix: &str,
    center: Vec2,
    length: f32,
    vertical: bool,
) -> Vec<EntityId> {
    let spec = &COURT_EDGE;
    let count = (length / spec.cell.x).ceil() as u32;
    let first = -(count as f32 - 1.0) * spec.cell.x / 2.0;
    let (step, rotation) = if vertical {
        (Vec2::Y, std::f32::consts::FRAC_PI_2)
    } else {
        (Vec2::X, 0.0)
    };
    (0..count)
        .map(|index| {
            let along = first + index as f32 * spec.cell.x;
            world.spawn()
                .with(Name::new(format!("{name_prefix} {index}")))
                .with(Transform2D::from_parts(center + step * along, rotation, spec.scale()))
                .with(sheet.sprite().with_offset(spec.sprite_offset()).with_depth(WALL_DEPTH))
                .id()
        })
        .collect()
}

/// Sensor strip outside the playfield's top or bottom edge — touching it
/// costs a ball. `y_sign` is -1.0 for the bottom strip, +1.0 for the top.
pub(crate) fn spawn_loss_sensor(world: &mut World, name: &str, y_sign: f32) -> EntityId {
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::new(Vec2::new(0.0, y_sign * (WIN_H / 2.0 + 30.0))))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(WIN_W + 200.0, 20.0).as_sensor())
        .id()
}

/// Spawn the counter: a `Tilemap` of Tong's court tile, `ceil(WIN / tile)` cells each
/// way, centred on the window. Tiles are placed in pixel units from the entity's
/// position, which anchors the centre of tile (0, 0) — the top-left cell, with rows
/// growing downward.
pub(crate) fn spawn_court(world: &mut World, sheet: &SpriteSheet) -> EntityId {
    let columns = (WIN_W / COURT_TILE_PX).ceil() as u32;
    let rows = (WIN_H / COURT_TILE_PX).ceil() as u32;

    let mut map = Tilemap::new(columns, rows, COURT_TILE_PX);
    map.tileset = sheet.texture.id;
    // The court sheet is one cell; a sheet with more would cut into that many.
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.tiles = vec![1; (columns * rows) as usize];
    map.depth = COURT_TILE_DEPTH;

    let anchor = Vec2::new(
        -(columns as f32 - 1.0) * COURT_TILE_PX / 2.0,
        (rows as f32 - 1.0) * COURT_TILE_PX / 2.0,
    );
    world.spawn().with(Name::new("Court")).with(Transform2D::new(anchor)).with(map).id()
}

/// The backdrop grid's colour for a chaos theme: its grid colour at the low alpha that
/// lets the lattice read over the counter without veiling it.
pub(crate) fn backdrop_color(theme: &ChaosTheme) -> Vec4 {
    let grid = theme.grid_color;
    Vec4::new(grid.x, grid.y, grid.z, BACKDROP_ALPHA)
}

/// Spawn the deforming grid drawn over the counter: the engine simulates and draws
/// it, gameplay events ripple it.
pub(crate) fn spawn_backdrop(world: &mut World, theme: &ChaosTheme) -> EntityId {
    world.spawn()
        .with(Name::new("Grid Backdrop"))
        .with(Transform2D::new(Vec2::ZERO))
        .with(GridBackdrop {
            draw_order: GridDrawOrder::OverSprites,
            color: backdrop_color(theme),
            ..GridBackdrop::default()
        })
        .id()
}

impl BreakoutGame {
    /// Tear down and respawn the playfield structure for the given mode.
    ///
    /// Solo: solid top wall, two side walls, bottom loss sensor, one paddle.
    /// Co-op: the top wall is GONE (destroying the collider matters — a hidden static
    /// wall would still block) and replaced by a top loss sensor plus player 2's
    /// paddle. Each wall's strips and each paddle's tong are spawned and drained with
    /// it, so a rebuild never leaves art standing where the last match ended.
    pub(crate) fn rebuild_playfield(&mut self, world: &mut World, mode: GameMode) {
        for wall in self.walls.drain(..) {
            self.physics.destroy_entity(world, wall);
        }
        for sensor in [self.bottom_sensor.take(), self.top_sensor.take()].into_iter().flatten() {
            self.physics.destroy_entity(world, sensor);
        }
        for paddle in [self.paddle.take(), self.paddle_top.take()].into_iter().flatten() {
            self.physics.destroy_entity(world, paddle);
        }
        for art in self.wall_strips.drain(..).chain([self.tong.take(), self.tong_top.take()].into_iter().flatten()) {
            world.remove_entity(&art).ok();
        }

        let side_x = WIN_W / 2.0 - WALL_THICKNESS / 2.0;
        for (name, x) in [("Wall left", -side_x), ("Wall right", side_x)] {
            self.walls.push(spawn_wall(world, name, Vec2::new(x, 0.0), WALL_THICKNESS, WIN_H));
            self.wall_strips.extend(spawn_wall_strips(
                world, &self.sheets.court_edge, name, Vec2::new(x, 0.0), WIN_H, true));
        }
        self.bottom_sensor = Some(spawn_loss_sensor(world, "Loss Sensor bottom", -1.0));
        self.tongs = DEFAULT_TONGS;
        self.paddle = Some(spawn_paddle(world, "Paddle P1", PADDLE_Y));
        self.tong = Some(spawn_tong(
            world, "Tong P1", &TONG_LEFT, &self.sheets.tong_left, TONG_LEFT_DEPTH,
            Vec2::new(0.0, PADDLE_Y), self.tongs[PaddleSide::Bottom.index()].facing));

        match mode {
            GameMode::SinglePlayer => {
                let top_y = WIN_H / 2.0 - WALL_THICKNESS / 2.0;
                self.walls.push(spawn_wall(world, "Wall top", Vec2::new(0.0, top_y), WIN_W, WALL_THICKNESS));
                self.wall_strips.extend(spawn_wall_strips(
                    world, &self.sheets.court_edge, "Wall top", Vec2::new(0.0, top_y), WIN_W, false));
            }
            GameMode::TwoPlayerCoop => {
                self.top_sensor = Some(spawn_loss_sensor(world, "Loss Sensor top", 1.0));
                self.paddle_top = Some(spawn_paddle(world, "Paddle P2", PADDLE_TOP_Y));
                self.tong_top = Some(spawn_tong(
                    world, "Tong P2", &TONG_RIGHT, &self.sheets.tong_right, TONG_RIGHT_DEPTH,
                    Vec2::new(0.0, PADDLE_TOP_Y), self.tongs[PaddleSide::Top.index()].facing));
            }
        }
    }
}

/// Center X position of brick `col` (0-based, left to right).
pub(crate) fn brick_x(col: usize) -> f32 {
    let total = BRICK_COLS as f32 * BRICK_CELL.x + (BRICK_COLS as f32 - 1.0) * BRICK_GAP;
    -(total - BRICK_CELL.x) / 2.0 + col as f32 * (BRICK_CELL.x + BRICK_GAP)
}

/// Center Y position of brick `row` (0-based, top to bottom).
pub(crate) fn brick_y(row: usize) -> f32 {
    BRICK_TOP_Y - row as f32 * (BRICK_CELL.y + BRICK_GAP)
}

/// Center Y position of brick `row` in the co-op middle band.
pub(crate) fn brick_y_2p(row: usize) -> f32 {
    BRICK_TOP_Y_2P - row as f32 * (BRICK_CELL.y + BRICK_GAP)
}

/// Score paid out by a brick in `row` — top rows are worth the most.
pub(crate) fn brick_value(row: usize) -> u32 {
    (BRICK_ROWS - row) as u32 * BRICK_VALUE_STEP
}

/// Spawn the full brick grid: `BRICK_ROWS` rows of `BRICK_COLS` food bricks, each row
/// its tier's food, with row Y positions supplied by `row_y`. Each brick's collider is
/// its whole cell, and its sprite is that cell playing `intact`.
fn spawn_brick_grid(world: &mut World, sheets: &Sheets, row_y: impl Fn(usize) -> f32) -> Vec<Brick> {
    let mut bricks = Vec::with_capacity(BRICK_ROWS * BRICK_COLS);
    for row in 0..BRICK_ROWS {
        let food = food_for_row(row);
        let spec = food.spec();
        for col in 0..BRICK_COLS {
            let pos = Vec2::new(brick_x(col), row_y(row));
            let (sprite, mut animation) = art_components(sheets.food(food), &spec.sheet, spec.depth);
            let _ = animation.play(BRICK_INTACT);
            let entity = world.spawn()
                .with(Name::new(format!("brick_r{row}_c{col}")))
                .with(Transform2D::from_parts(pos, 0.0, spec.sheet.scale()))
                .with(sprite)
                .with(animation)
                .with(RigidBody::new_static())
                .with(Collider::box_collider(BRICK_CELL.x, BRICK_CELL.y)
                    .with_friction(0.0)
                    .with_restitution(1.0))
                .id();
            bricks.push(Brick { entity, value: brick_value(row), food, hits_left: 1, drop: None });
        }
    }
    bricks
}

/// Solo fallback grid in the classic upper region.
pub(crate) fn spawn_bricks(world: &mut World, sheets: &Sheets) -> Vec<Brick> {
    spawn_brick_grid(world, sheets, brick_y)
}

/// Co-op fallback grid centered on the middle band, between the paddles.
pub(crate) fn spawn_bricks_2p(world: &mut World, sheets: &Sheets) -> Vec<Brick> {
    spawn_brick_grid(world, sheets, brick_y_2p)
}

impl BreakoutGame {
    /// Spawn Deion as the water ball at `position`. The collider is a true circle on his
    /// body, so reflections match what the player sees; the mohawk above it is not
    /// collided. The position is the transform the ball is drawn at on the frame it is
    /// spawned — a body repositioned later lands only at the next physics step.
    pub(crate) fn spawn_ball(&self, world: &mut World, name: &str, position: Vec2) -> EntityId {
        let (sprite, animation) = art_components(&self.sheets.ball_water, &BALL_WATER, BALL_WATER_DEPTH);
        world.spawn()
            .with(Name::new(name))
            .with(Transform2D::from_parts(position, 0.0, BALL_WATER.scale()))
            .with(sprite)
            .with(animation)
            .with(ball_machine())
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0)
                .with_ccd(true))
            .with(Collider::circle_collider(BALL_RADIUS)
                .with_friction(0.0)
                .with_restitution(1.0))
            .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brick_grid_spawns_every_tier_of_the_pyramid() {
        let mut world = World::new();
        let bricks = spawn_bricks(&mut world, &Sheets::default());
        assert_eq!(bricks.len(), BRICK_ROWS * BRICK_COLS);
        for (index, brick) in bricks.iter().enumerate() {
            assert_eq!(brick.food, food_for_row(index / BRICK_COLS), "row {}", index / BRICK_COLS);
        }
    }

    #[test]
    fn brick_grid_fits_inside_playfield_walls() {
        let left_edge = brick_x(0) - BRICK_CELL.x / 2.0;
        let right_edge = brick_x(BRICK_COLS - 1) + BRICK_CELL.x / 2.0;
        assert!(left_edge > -PLAYFIELD_HALF_W, "grid pokes past left wall: {left_edge}");
        assert!(right_edge < PLAYFIELD_HALF_W, "grid pokes past right wall: {right_edge}");
        // Symmetric layout
        assert!((left_edge + right_edge).abs() < 0.001);
    }

    #[test]
    fn brick_rows_descend_and_stay_above_paddle() {
        assert!(brick_y(0) > brick_y(BRICK_ROWS - 1));
        let lowest = brick_y(BRICK_ROWS - 1) - BRICK_CELL.y / 2.0;
        assert!(lowest > PADDLE_Y + 100.0, "bricks too close to paddle: {lowest}");
    }

    #[test]
    fn top_row_bricks_pay_the_most() {
        assert_eq!(brick_value(0), BRICK_ROWS as u32 * BRICK_VALUE_STEP);
        assert_eq!(brick_value(BRICK_ROWS - 1), BRICK_VALUE_STEP);
        for row in 1..BRICK_ROWS {
            assert!(brick_value(row - 1) > brick_value(row));
        }
    }

    #[test]
    fn generated_2p_grid_stays_between_both_paddles() {
        let top = brick_y_2p(0) + BRICK_CELL.y / 2.0;
        let bottom = brick_y_2p(BRICK_ROWS - 1) - BRICK_CELL.y / 2.0;
        assert!(top < PADDLE_TOP_Y - 100.0, "2P grid too close to the top paddle: {top}");
        assert!(bottom > PADDLE_Y + 100.0, "2P grid too close to the bottom paddle: {bottom}");
        // Band is vertically centered, mirroring both players' reaction room
        assert!((top + bottom).abs() < 0.001, "2P band not centered: {top}..{bottom}");
        assert_eq!((top, bottom), (106.0, -106.0));
    }

    #[test]
    fn coop_playfield_swaps_top_wall_for_sensor_and_paddle() {
        let mut world = World::new();
        let mut game = BreakoutGame::default();

        game.rebuild_playfield(&mut world, GameMode::SinglePlayer);
        assert_eq!(game.walls.len(), 3, "solo keeps the top wall");
        assert!(game.top_sensor.is_none());
        assert!(game.paddle_top.is_none() && game.tong_top.is_none(), "solo spawns no top tong");
        assert!(game.paddle.is_some() && game.tong.is_some() && game.bottom_sensor.is_some());

        game.rebuild_playfield(&mut world, GameMode::TwoPlayerCoop);
        assert_eq!(game.walls.len(), 2, "co-op opens the top edge");
        assert!(game.top_sensor.is_some());
        assert!(game.paddle_top.is_some() && game.tong_top.is_some());
        assert!(game.paddle.is_some() && game.bottom_sensor.is_some());

        // Rebuilding back to solo never leaks co-op structure
        game.rebuild_playfield(&mut world, GameMode::SinglePlayer);
        assert_eq!(game.walls.len(), 3);
        assert!(game.top_sensor.is_none() && game.paddle_top.is_none() && game.tong_top.is_none());
    }

    #[test]
    fn no_tong_or_strip_survives_a_rebuild() {
        let mut world = World::new();
        let mut game = BreakoutGame::default();

        game.rebuild_playfield(&mut world, GameMode::TwoPlayerCoop);
        let old_art: Vec<EntityId> = game.wall_strips.iter().copied()
            .chain([game.tong, game.tong_top].into_iter().flatten())
            .collect();
        game.rebuild_playfield(&mut world, GameMode::TwoPlayerCoop);

        let live = world.entities();
        for art in old_art {
            assert!(!live.contains(&art), "{art:?} outlived the rebuild");
        }
        let names = |prefix: &str| live.iter()
            .filter(|&&entity| world.get::<Name>(entity).is_some_and(|name| name.0.starts_with(prefix)))
            .count();
        assert_eq!(names("Tong"), 2, "exactly the two new tongs");
        assert_eq!(names("Wall top"), 0, "co-op draws no top rail over its open edge");
    }

    #[test]
    fn the_counter_covers_the_window() {
        let mut world = World::new();
        let court = spawn_court(&mut world, &placeholder_sheet());
        let map = world.get::<Tilemap>(court).expect("the court is a tilemap");
        assert_eq!((map.width, map.height), (13, 10));
        assert!(map.width as f32 * COURT_TILE_PX >= WIN_W && map.height as f32 * COURT_TILE_PX >= WIN_H);
    }
}
