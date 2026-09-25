//! Paddle control (per-player input, mouse takeover), paddle bounces, and the tongs
//! drawn on the paddles: where each lies and which way its mouth points.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::types::*;
use super::{entity_position, entity_x, ripple_grid, set_clip_state};

/// Direction a ball leaves the bottom paddle, from where it struck.
///
/// `offset_frac` is the hit position relative to the paddle center in
/// half-widths: 0 = dead center, ±1 = the very edge. Center hits return
/// the ball straight up; edge hits deflect up to `PADDLE_MAX_BOUNCE_ANGLE`.
pub(crate) fn paddle_bounce_direction(offset_frac: f32) -> Vec2 {
    let angle = offset_frac.clamp(-1.0, 1.0) * PADDLE_MAX_BOUNCE_ANGLE;
    Vec2::new(angle.sin(), angle.cos())
}

/// Side-aware bounce: a paddle always returns the ball toward the field —
/// up off the bottom paddle, DOWN off the top one. Without the top-side
/// flip, the top paddle would fire balls into itself/the top sensor.
pub(crate) fn paddle_bounce_direction_for(offset_frac: f32, side: PaddleSide) -> Vec2 {
    let dir = paddle_bounce_direction(offset_frac);
    match side {
        PaddleSide::Bottom => dir,
        PaddleSide::Top => Vec2::new(dir.x, -dir.y),
    }
}

/// The state a tong's machine should be moved to this frame, or `None` to leave it.
///
/// A facing change lands only on a tong at rest: a scowl in flight plays out first,
/// returns to its own facing's `closed`, and the pending facing lands the frame after —
/// a transition under a clip would restart it.
pub(crate) fn tong_rest_for_facing(state: &str, wanted: Facing) -> Option<String> {
    let (clip, facing) = Facing::split(state)?;
    if clip != TONG_CLOSED || facing == wanted {
        return None;
    }
    Some(tong_state(TONG_CLOSED, wanted))
}

impl BreakoutGame {
    /// Both paddles' worth of side/entity/tong/burst colour, for uniform loops.
    pub(super) fn paddle_roster(&self) -> [(Option<EntityId>, Option<EntityId>, PaddleSide, Vec4); 2] {
        [
            (self.paddle, self.tong, PaddleSide::Bottom, P1_BURST_COLOR),
            (self.paddle_top, self.tong_top, PaddleSide::Top, P2_BURST_COLOR),
        ]
    }

    pub(super) fn update_paddles(&mut self, ctx: &mut GameContext) {
        if let Some(paddle) = self.paddle {
            self.update_paddle(ctx, paddle, PlayerId::P1, PADDLE_Y, PaddleSide::Bottom);
        }
        if let Some(paddle) = self.paddle_top {
            self.update_paddle(ctx, paddle, PlayerId::P2, PADDLE_TOP_Y, PaddleSide::Top);
        }
        self.face_tongs(ctx.world);
    }

    /// Move one paddle from its player's bindings (keys, dpad, or stick) or
    /// the mouse. Mouse takes over whenever it moves and belongs to the
    /// bottom paddle only (the mouse is player 1's device); bound input
    /// takes over whenever it's active. Whatever moved it, the move asks the
    /// paddle's tong to face the way it went.
    fn update_paddle(&mut self, ctx: &GameContext, paddle: EntityId, player: PlayerId, y: f32, side: PaddleSide) {
        let x = entity_x(ctx.world, paddle);

        // Solo: the lone paddle listens to both players' devices, so WASD,
        // arrows, and either pad all work.
        let axis = match self.mode {
            GameMode::SinglePlayer => (ctx.players.move_x(PlayerId::P1, ctx.input)
                + ctx.players.move_x(PlayerId::P2, ctx.input))
            .clamp(-1.0, 1.0),
            GameMode::TwoPlayerCoop => ctx.players.move_x(player, ctx.input),
        };

        let mouse_moved =
            player == PlayerId::P1 && ctx.input.mouse_movement_delta().0.abs() > 0.0;
        let new_x = if axis != 0.0 {
            x + axis * PADDLE_SPEED * ctx.delta_time
        } else if mouse_moved {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-PADDLE_MAX_X, PADDLE_MAX_X);
        if let Some(facing) = Facing::from_displacement(new_x - x) {
            self.tong_facing[side.index()] = facing;
        }
        self.physics.set_kinematic_target(paddle, Vec2::new(new_x, y), 0.0);
    }

    /// Turn each resting tong to the facing it was last asked for.
    pub(super) fn face_tongs(&self, world: &mut World) {
        for (_, tong, side, _) in self.paddle_roster() {
            let Some(tong) = tong else { continue };
            let Some(state) = world.get::<ClipStateMachine>(tong).map(|machine| machine.state().to_string()) else {
                continue;
            };
            if let Some(target) = tong_rest_for_facing(&state, self.tong_facing[side.index()]) {
                set_clip_state(world, tong, &target);
            }
        }
    }

    /// Put each tong where physics left its paddle this frame.
    pub(super) fn place_tongs(&self, world: &mut World) {
        for (paddle, tong, _, _) in self.paddle_roster() {
            let (Some(paddle), Some(tong)) = (paddle, tong) else { continue };
            let Some(position) = entity_position(world, paddle) else { continue };
            if let Some(transform) = world.get_mut::<Transform2D>(tong) {
                transform.position = position;
            }
        }
    }

    /// Paddle bounces: aim the ball by hit offset (toward the field, per
    /// side), reset the combo, apply the Insane speed gain, spray particles.
    pub(super) fn check_paddle_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let theme = theme_for(self.chaos_mode);

        for (paddle, _, side, color) in self.paddle_roster() {
            let Some(paddle) = paddle else { continue };
            let paddle_x = entity_x(ctx.world, paddle);

            for &ball in &self.all_balls() {
                let hit = collisions.iter()
                    .any(|c| c.event.started && c.event.involves(ball, paddle));
                if !hit { continue; }

                self.combo = 0;
                if self.chaos_mode.is_insane() {
                    self.speed_mult *= INSANE_SPEED_GAIN;
                }

                let Some(pos) = entity_position(ctx.world, ball) else { continue };
                // Override the physical reflection with offset-based aim —
                // this is what makes Breakout controllable rather than
                // deterministic.
                let offset = (pos.x - paddle_x) / (PADDLE_W / 2.0);
                let dir = paddle_bounce_direction_for(offset, side);
                let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
                self.physics.set_velocity(ball, dir * speed, 0.0);

                ctx.particles.spawn_burst(pos, &effects::paddle_hit_burst(color, &theme, self.sheets.white));
                ripple_grid(ctx.world, pos, GRID_IMPULSE_PADDLE_HIT_STRENGTH, GRID_IMPULSE_PADDLE_HIT_RADIUS);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::spawn_tong;

    /// A sheet carrying the tong's four clip names: `closed` one frame, `scored_on` four
    /// frames at 10 fps played once — the shape the synced sidecar gives them.
    fn tong_sheet() -> SpriteSheet {
        let mut clips = Vec::new();
        for facing in [Facing::Up, Facing::Down] {
            clips.push((tong_state(TONG_CLOSED, facing), AnimationClip::new(vec![0], 10.0)));
            clips.push((
                tong_state(TONG_SCORED_ON, facing),
                AnimationClip::new(vec![0, 1, 2, 3], 10.0).with_looping(false),
            ));
        }
        SpriteSheet { texture: TextureHandle { id: 1 }, grid: SheetGrid::new(4, 1), clips, path: String::new() }
    }

    fn state(world: &World, tong: EntityId) -> String {
        world.get::<ClipStateMachine>(tong).expect("machine").state().to_string()
    }

    /// The frame tail as the engine runs it after `update`, for the one tong: its clip
    /// advances, then the machine applies a finished clip's `Next`.
    fn tail(world: &mut World, tong: EntityId, seconds: f32) {
        if let Some(animation) = world.get_mut::<SpriteAnimation>(tong) {
            animation.update(seconds);
        }
        ClipStateMachineSystem.update(world, seconds);
    }

    #[test]
    fn a_facing_asked_for_during_a_scowl_lands_when_the_scowl_ends_and_not_before() {
        let mut world = World::new();
        let mut game = BreakoutGame::default();
        let tong = spawn_tong(
            &mut world, "Tong P1", &TONG_LEFT, &tong_sheet(), TONG_LEFT_DEPTH, Vec2::ZERO, Facing::Up);
        game.tong = Some(tong);
        tail(&mut world, tong, 0.0);

        set_clip_state(&mut world, tong, &tong_state(TONG_SCORED_ON, Facing::Up));
        tail(&mut world, tong, 0.0);
        game.tong_facing[PaddleSide::Bottom.index()] = Facing::Down;

        for _ in 0..3 {
            game.face_tongs(&mut world);
            assert_eq!(state(&world, tong), tong_state(TONG_SCORED_ON, Facing::Up), "the scowl plays out");
            tail(&mut world, tong, 0.1);
        }
        tail(&mut world, tong, 0.2);
        assert_eq!(state(&world, tong), tong_state(TONG_CLOSED, Facing::Up), "it ends in its own facing");

        game.face_tongs(&mut world);
        assert_eq!(state(&world, tong), tong_state(TONG_CLOSED, Facing::Down), "then the pending facing lands");
    }

    #[test]
    fn a_resting_tong_turns_at_once_and_a_turned_one_stays() {
        assert_eq!(tong_rest_for_facing("closed_up", Facing::Down), Some("closed_down".to_string()));
        assert_eq!(tong_rest_for_facing("closed_up", Facing::Up), None);
        assert_eq!(tong_rest_for_facing("scored_on_up", Facing::Down), None);
        assert_eq!(Facing::from_displacement(-3.0), Some(Facing::Up), "moving left, the mouth leads left");
        assert_eq!(Facing::from_displacement(3.0), Some(Facing::Down));
        assert_eq!(Facing::from_displacement(0.0), None, "a stop keeps the last facing");
    }
}
