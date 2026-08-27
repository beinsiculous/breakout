//! Ball lifecycle: serving glue, launching, velocity maintenance, and loss.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::types::*;
use super::{entity_position, entity_x};

/// Re-aim `dir` if it is too horizontal, preserving its left/right and
/// up/down senses. Keeps the ball from shuttling between the side walls.
pub(crate) fn enforce_min_vertical(dir: Vec2) -> Vec2 {
    if dir.y.abs() >= MIN_VERTICAL_FRACTION {
        return dir;
    }
    let y_sign = if dir.y == 0.0 { 1.0 } else { dir.y.signum() };
    Vec2::new(
        dir.x.signum() * (1.0 - MIN_VERTICAL_FRACTION * MIN_VERTICAL_FRACTION).sqrt(),
        y_sign * MIN_VERTICAL_FRACTION,
    )
}

/// Resting Y of a served ball: just inside the serving paddle, toward the
/// field.
pub(crate) fn serving_glue_y(side: PaddleSide) -> f32 {
    match side {
        PaddleSide::Bottom => PADDLE_Y + SERVE_OFFSET_Y,
        PaddleSide::Top => PADDLE_TOP_Y - SERVE_OFFSET_Y,
    }
}

impl BreakoutGame {
    /// The paddle currently holding the serve. Falls back to the bottom
    /// paddle if the serving side's paddle is missing (defensive: solo mode
    /// only ever serves from the bottom).
    fn serving_paddle(&self) -> Option<EntityId> {
        match self.serving_side {
            PaddleSide::Bottom => self.paddle,
            PaddleSide::Top => self.paddle_top.or(self.paddle),
        }
    }

    /// While serving, park the ball on the serving paddle every frame.
    /// Runs after `physics.update()`, so the paddle transform already
    /// reflects this frame's kinematic target.
    pub(super) fn glue_serving_ball(&mut self, world: &mut World) {
        if self.state != GameState::Serving { return; }
        let Some(ball) = self.ball else { return };
        let Some(paddle) = self.serving_paddle() else { return };
        let x = entity_x(world, paddle);
        // reset_body zeroes velocity too, so the ball just rides the paddle.
        self.physics.reset_body(ball, Vec2::new(x, serving_glue_y(self.serving_side)));
    }

    /// Fire the served ball toward the field at a slightly random angle
    /// (up from the bottom paddle, down from the top). Ridiculous mode
    /// launches a second ball mirrored the other way.
    pub(super) fn launch_balls(&mut self, ctx: &mut GameContext) {
        let Some(ball) = self.ball else { return };

        let y_sign = match self.serving_side {
            PaddleSide::Bottom => 1.0,
            PaddleSide::Top => -1.0,
        };
        let angle = (hash_f32(self.frame_count) - 0.5) * LAUNCH_ANGLE_SPREAD; // ±0.3 rad off vertical
        let dir = Vec2::new(angle.sin(), y_sign * angle.cos());
        let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        self.physics.set_velocity(ball, dir * speed, 0.0);

        if self.chaos_mode.is_ridiculous() {
            let pos = entity_position(ctx.world, ball)
                .unwrap_or(Vec2::new(0.0, serving_glue_y(self.serving_side)));
            let extra = self.spawn_ball(ctx.world);
            let theme = theme_for(self.chaos_mode);
            if let Some(t) = ctx.world.get_mut::<Transform2D>(extra) {
                t.position = pos;
            }
            if let Some(s) = ctx.world.get_mut::<Sprite>(extra) {
                s.color = theme.accent_color;
            }
            let dir2 = Vec2::new(-angle.sin(), y_sign * angle.cos());
            self.physics.set_velocity(extra, dir2 * speed, 0.0);
            self.extra_balls.push(extra);
        }

        self.state = GameState::Playing;
    }

    pub(super) fn all_balls(&self) -> Vec<EntityId> {
        self.ball.into_iter().chain(self.extra_balls.iter().copied()).collect()
    }

    /// Hold every live ball at its target speed and keep it from going
    /// fully horizontal.
    pub(super) fn maintain_all_ball_velocities(&mut self) {
        if self.state != GameState::Playing { return; }
        let target = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        for ball in self.all_balls() {
            let Some((vel, _)) = self.physics.get_body_velocity(ball) else { continue };
            let speed = vel.length();
            if speed < 1.0 { continue; }
            let dir = enforce_min_vertical(vel / speed);
            let new_vel = dir * target;
            if (new_vel - vel).length() > 1.0 {
                self.physics.set_velocity(ball, new_vel, 0.0);
            }
        }
    }

    /// Remove balls that fell past either paddle (loss-sensor hit or escaped
    /// the playfield entirely). When none remain, spend a shared life and
    /// hand the serve to the side that lost the ball.
    pub(super) fn check_ball_loss(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        if self.state != GameState::Playing { return; }
        let Some(bottom) = self.bottom_sensor else { return };

        let bound_x = WIN_W / 2.0 + BALL_LOST_BOUNDS_PAD;
        let bound_y = WIN_H / 2.0 + BALL_LOST_BOUNDS_PAD;
        let mut lost: Vec<(EntityId, PaddleSide)> = Vec::new();
        for &ball in &self.all_balls() {
            let sensor_side = collisions.iter().find_map(|c| {
                if !c.event.started { return None; }
                if c.event.involves(ball, bottom) {
                    Some(PaddleSide::Bottom)
                } else if self.top_sensor.is_some_and(|s| c.event.involves(ball, s)) {
                    Some(PaddleSide::Top)
                } else {
                    None
                }
            });
            // Safety net: a CCD miss or NaN position also counts as lost;
            // attribute the side by which half the ball vanished in.
            let escaped_side = match entity_position(ctx.world, ball) {
                Some(p) if p.x.is_finite() && p.y.is_finite()
                    && p.x.abs() <= bound_x && p.y.abs() <= bound_y => None,
                Some(p) if p.y.is_finite() && p.y > 0.0 => Some(PaddleSide::Top),
                _ => Some(PaddleSide::Bottom),
            };
            if let Some(side) = sensor_side.or(escaped_side) {
                lost.push((ball, side));
            }
        }
        if lost.is_empty() { return; }

        let theme = theme_for(self.chaos_mode);
        let mut last_lost_side = PaddleSide::Bottom;
        for (ball, side) in lost {
            last_lost_side = side;
            let fallback_y = match side {
                PaddleSide::Bottom => -WIN_H / 2.0,
                PaddleSide::Top => WIN_H / 2.0,
            };
            let pos = entity_position(ctx.world, ball).unwrap_or(Vec2::new(0.0, fallback_y));
            ctx.particles.spawn_burst(pos, &effects::ball_lost_burst(&theme, self.tex_id));
            if let Some(grid) = self.grid.as_mut() {
                grid.apply_impulse(&GridImpulse::Radial {
                    position: pos,
                    strength: GRID_IMPULSE_BALL_LOST_STRENGTH,
                    radius: GRID_IMPULSE_BALL_LOST_RADIUS,
                    attractive: false,
                });
            }

            if Some(ball) == self.ball {
                self.ball = self.extra_balls.pop();
            } else {
                self.extra_balls.retain(|&e| e != ball);
            }
            self.physics.destroy_entity(ctx.world, ball);
        }

        if self.ball.is_some() { return; }

        // All balls gone — spend a life. Wrecking dies with the volley
        // (consistent with the speed_mult reset below).
        self.lives = self.lives.saturating_sub(1);
        self.combo = 0;
        self.speed_mult = 1.0;
        self.wrecking.stop();
        if self.lives == 0 {
            self.destroy_all_pickups(ctx.world);
            ctx.scores.submit(super::flow::score_mode(self.mode), self.score as u64);
            self.state = GameState::GameOver { won: false };
            return;
        }

        self.serving_side = super::flow::serve_side_after_loss(self.mode, last_lost_side);
        let fresh = self.spawn_ball(ctx.world);
        let theme = theme_for(self.chaos_mode);
        if let Some(s) = ctx.world.get_mut::<Sprite>(fresh) {
            s.color = theme.accent_color;
        }
        self.ball = Some(fresh);
        self.state = GameState::Serving;
    }

    pub(crate) fn destroy_all_balls(&mut self, world: &mut World) {
        if let Some(ball) = self.ball.take() {
            self.physics.destroy_entity(world, ball);
        }
        for ball in self.extra_balls.drain(..) {
            self.physics.destroy_entity(world, ball);
        }
    }
}
