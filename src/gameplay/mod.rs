//! Match gameplay, split by concern: paddle control, bounces and the tongs drawn on
//! them (`paddles`), ball serving/velocity/loss (`balls`), brick hits and destruction
//! (`bricks`), state transitions and visibility (`flow`).

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
use crate::constants::*;
use crate::types::*;

pub(super) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

pub(super) fn entity_x(world: &World, entity: EntityId) -> f32 {
    world.get::<Transform2D>(entity).map(|t| t.position.x).unwrap_or(0.0)
}

/// Move `entity`'s clip machine to `state`. An entity without one is left alone, as is
/// an unknown state name (the machine warns and holds its ground). Re-asserting the
/// state the machine is already in does not restart its clip.
pub(crate) fn set_clip_state(world: &mut World, entity: EntityId, state: &str) {
    if let Some(machine) = world.get_mut::<ClipStateMachine>(entity) {
        let _ = machine.transition_to(state);
    }
}

/// Push a radial shockwave into the backdrop grid. The engine applies it to every
/// backdrop on its next running frame.
pub(super) fn ripple_grid(world: &mut World, position: Vec2, strength: f32, radius: f32) {
    ripple(world, GridImpulse::Radial { position, strength, radius, attractive: false });
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
                PauseAction::ExitGame => { ctx.request_exit(); return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay (Space must not also launch the ball).
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // The engine holds the backdrop grid still with the rest of the world
                // (it steps on the time-scaled delta), so only the collider overlay is
                // still ours to draw under the pause overlay.
                self.emit_collider_overlay(ctx);
                return;
            }
        }

        self.update_paddles(ctx);
        self.physics.update(ctx.world, ctx.delta_time);
        // The tongs are art beside their bodies: placed where physics left each
        // paddle, so the drawn tong is never a frame behind its collider.
        self.place_tongs(ctx.world);

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

        self.emit_collider_overlay(ctx);
    }

    /// Outline every collider in bright magenta while F1 is on. The backdrop grid is
    /// the engine's to draw, so the collider overlay is the only line-buffer work the
    /// game owns — and it draws last, over the grid and the art.
    pub(crate) fn emit_collider_overlay(&self, ctx: &mut GameContext) {
        if self.debug_colliders {
            debug::draw_colliders(ctx.world, ctx.lines, DEBUG_COLLIDER_COLOR, DEBUG_COLLIDER_EMISSIVE);
        }
    }
}
