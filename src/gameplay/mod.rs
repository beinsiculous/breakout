//! Match gameplay, split by concern: paddle control and bounces
//! (`paddles`), ball serving/velocity/loss (`balls`), brick hits and
//! destruction (`bricks`), state transitions and visibility (`flow`).

mod balls;
mod bricks;
mod flow;
mod paddles;

// Pure gameplay rules, re-exported for the test battery in gameplay_tests.rs
// (the game itself calls them through their home modules).
#[cfg(test)]
pub(crate) use balls::{enforce_min_vertical, serving_glue_y};
#[cfg(test)]
pub(crate) use bricks::{brick_bounce_velocity, brick_hit_outcome, BrickHitOutcome};
#[cfg(test)]
pub(crate) use flow::serve_side_after_loss;
#[cfg(test)]
pub(crate) use paddles::{paddle_bounce_direction, paddle_bounce_direction_for};

use engine_core::prelude::*;
use crate::types::*;

pub(super) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

pub(super) fn entity_x(world: &World, entity: EntityId) -> f32 {
    world.get::<Transform2D>(entity).map(|t| t.position.x).unwrap_or(0.0)
}

impl BreakoutGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        if self.paddle.is_none() { return; }

        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top of sprites so any sprite-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: while paused the whole match is frozen — no physics
        // step, no collision drain, no input; the overlay draws in the UI pass.
        if matches!(self.state, GameState::Serving | GameState::Playing) {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay (Space must not also launch the ball).
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // Keep the frozen scene visible under the pause overlay:
                // re-emit the grid without advancing it (dt 0).
                engine_core::grid::step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
        }

        self.update_paddles(ctx);
        self.physics.update(ctx.world, ctx.delta_time);

        // Drain this frame's collision events once (take = the buffer is
        // consumed, not borrowed). Every consumer below shares this Vec, and
        // no borrow of `self.physics` is held while reacting.
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.glue_serving_ball(ctx.world);
        self.handle_state_input(ctx);
        self.maintain_all_ball_velocities();
        self.check_paddle_hits(ctx, &collisions);
        self.check_brick_hits(ctx, &collisions);
        self.check_pickup_catches(ctx, &collisions);
        self.despawn_missed_pickups(ctx, &collisions);
        self.update_wrecking(ctx);
        self.pulse_drop_bricks(ctx.world);
        self.check_ball_loss(ctx, &collisions);
        self.check_win_condition(ctx);

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's collisions.
        self.step_and_emit_grid(ctx);
    }

    /// Advance the spring-mass grid and push its line vertices into the
    /// engine's per-frame line buffer. When the collider-debug overlay is
    /// enabled, the collider outlines are pushed on top.
    fn step_and_emit_grid(&mut self, ctx: &mut GameContext) {
        engine_core::grid::step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }
}
