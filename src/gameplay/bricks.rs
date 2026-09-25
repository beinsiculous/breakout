//! Brick hit resolution and destruction.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::types::*;
use super::{entity_position, ripple_grid};

/// Velocity a ball should leave a destroyed brick with.
///
/// Bricks are destroyed the instant a contact starts, which cancels rapier's
/// contact impulse whenever the ball catches a brick corner or squeezes into
/// the gap between two bricks (the contact normal is horizontal there, so
/// nothing pushes the ball back) — the ball would plough straight through
/// the grid. So, like paddle bounces, the game supplies the reflection:
/// push the velocity *away from the brick* on the dominant contact axis.
/// Direction-agnostic (covers balls arriving upward off the top paddle) and
/// idempotent — if rapier already reflected the ball, this changes nothing.
pub(crate) fn brick_bounce_velocity(ball_pos: Vec2, vel: Vec2, brick_pos: Vec2) -> Vec2 {
    let d = ball_pos - brick_pos;
    let away_x = if d.x >= 0.0 { 1.0 } else { -1.0 };
    let away_y = if d.y >= 0.0 { 1.0 } else { -1.0 };
    // Normalize the offset by the brick's half-extents to find which face
    // the ball is closest to; ties go to vertical (breakout balls travel
    // mostly vertically).
    let mut v = vel;
    if d.y.abs() / (BRICK_CELL.y / 2.0) >= d.x.abs() / (BRICK_CELL.x / 2.0) {
        v.y = v.y.abs() * away_y;
    } else {
        v.x = v.x.abs() * away_x;
    }
    v
}

/// What a brick hit does, given the brick's remaining hits and whether the
/// wrecking ball is active (wrecking one-hit-kills anything).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrickHitOutcome {
    Destroyed,
    Damaged { hits_left: u32 },
}

pub(crate) fn brick_hit_outcome(hits_left: u32, wrecking_active: bool) -> BrickHitOutcome {
    if wrecking_active || hits_left <= 1 {
        BrickHitOutcome::Destroyed
    } else {
        BrickHitOutcome::Damaged { hits_left: hits_left - 1 }
    }
}

impl BreakoutGame {
    /// Resolve brick hits: armored bricks take damage (rapier reflects the
    /// ball naturally — the brick survives, so its contact impulse lands);
    /// destroyed bricks score, grow the combo, and may drop a pickup.
    pub(super) fn check_brick_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let all_balls = self.all_balls();

        let mut hit: Vec<usize> = Vec::new();
        for collision in collisions {
            if !collision.event.started { continue; }
            for (i, brick) in self.bricks.iter().enumerate() {
                if all_balls.iter().any(|&b| collision.event.involves(b, brick.entity)) {
                    hit.push(i);
                }
            }
        }
        // A brick can appear once per ball that touched it this frame —
        // dedup so it only takes one hit. Ascending order for reverse removal.
        hit.sort_unstable();
        hit.dedup();
        if hit.is_empty() { return; }

        let wrecking_active = self.wrecking_active();
        let theme = theme_for(self.chaos_mode);
        // Descending order: removals at index i never shift lower indices.
        for &i in hit.iter().rev() {
            match brick_hit_outcome(self.bricks[i].hits_left, wrecking_active) {
                BrickHitOutcome::Damaged { hits_left } => {
                    self.bricks[i].hits_left = hits_left;
                    let entity = self.bricks[i].entity;
                    // The foil tears when one hit is left.
                    if hits_left == 1 {
                        if let Some(animation) = ctx.world.get_mut::<SpriteAnimation>(entity) {
                            let _ = animation.play(BRICK_ARMOR_DAMAGED);
                        }
                    }
                    if let Some(pos) = entity_position(ctx.world, entity) {
                        ctx.particles.spawn_burst(
                            pos,
                            &effects::armor_hit_burst(FOIL_BURST_COLOR, &theme, self.sheets.white),
                        );
                    }
                }
                BrickHitOutcome::Destroyed => {
                    let brick = self.bricks.remove(i);
                    self.score += brick.value;
                    self.combo += 1;
                    if self.combo == COMBO_TARGET {
                        ctx.achievements.unlock(crate::achievements::COMBO_VOLLEY);
                    }
                    self.destroy_brick_entity(ctx, collisions, &all_balls, &brick, &theme);
                }
            }
        }
    }

    /// Tear down a destroyed brick: reflect the balls that hit it, burst
    /// particles in its food's colour, ripple the grid, drop its candy, and
    /// remove the entity.
    fn destroy_brick_entity(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
        all_balls: &[EntityId],
        brick: &Brick,
        theme: &ChaosTheme,
    ) {
        if let Some(pos) = entity_position(ctx.world, brick.entity) {
            // Destroying the brick cancels rapier's contact impulse, so
            // reflect every ball that hit it ourselves (see
            // brick_bounce_velocity for why corner/gap hits need this).
            for &ball in all_balls {
                let hit_this = collisions.iter()
                    .any(|c| c.event.started && c.event.involves(ball, brick.entity));
                if !hit_this { continue; }
                if let (Some(ball_pos), Some((vel, _))) =
                    (entity_position(ctx.world, ball), self.physics.get_body_velocity(ball))
                {
                    let new_vel = brick_bounce_velocity(ball_pos, vel, pos);
                    if new_vel != vel {
                        self.physics.set_velocity(ball, new_vel, 0.0);
                    }
                }
            }

            let burst_color = brick.food.spec().burst_color;
            ctx.particles.spawn_burst(pos, &effects::brick_burst(burst_color, theme, self.sheets.white));
            ripple_grid(ctx.world, pos, GRID_IMPULSE_BRICK_DESTROY_STRENGTH, GRID_IMPULSE_BRICK_DESTROY_RADIUS);

            if let Some(kind) = brick.drop {
                self.spawn_pickup(ctx.world, kind, pos);
            }
        }
        self.physics.destroy_entity(ctx.world, brick.entity);
    }
}
