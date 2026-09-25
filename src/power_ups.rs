//! Falling power-up candies and the effects they grant.
//!
//! Special bricks (tagged `drop_*` in the level scenes) drop a wrapped candy
//! when destroyed; the player must catch it with a tong.
//! Tracking/collection mechanics come from the engine's `Pickups` /
//! `EffectTimer`; this module owns what the pickups DO.

use engine_core::prelude::*;

use crate::constants::*;
use crate::spawning::{art_components, candy_machine, spawn_effect};
use crate::types::*;

/// What a candy kind is drawn with: its sheet's spec and the one depth that sheet —
/// the falling candies and the collects they leave — is drawn at.
pub(crate) fn candy_spec(kind: PickupKind) -> (&'static SheetSpec, f32) {
    match kind {
        PickupKind::Multiball => (&CANDY_MULTIBALL, CANDY_MULTIBALL_DEPTH),
        PickupKind::Wrecking => (&CANDY_WRECKING, CANDY_WRECKING_DEPTH),
        PickupKind::Insiculous => (&CANDY_INSICULOUS, CANDY_INSICULOUS_DEPTH),
    }
}

/// Editor-hierarchy display name for a falling candy.
fn candy_entity_name(kind: PickupKind) -> &'static str {
    match kind {
        PickupKind::Multiball => "Candy (Multiball)",
        PickupKind::Wrecking => "Candy (Wrecking Ball)",
        PickupKind::Insiculous => "Candy (Insiculous)",
    }
}

/// The positions of the candies a collection took, in the tracker's own order.
///
/// `Pickups::collect` reports and removes exactly the same candies in the same order,
/// so a caller zips the two to learn where each caught candy was falling — by the time
/// the call returns, that entity and its position are gone from the world.
fn collected_positions(tracked: &[(EntityId, Vec2)], remaining: &[EntityId]) -> Vec<Vec2> {
    tracked
        .iter()
        .filter(|(entity, _)| !remaining.contains(entity))
        .map(|(_, position)| *position)
        .collect()
}

/// What catching a pickup grants: (extra balls spawned, wrecking refresh).
/// Insiculous is the trio's "both at once" — the pattern the whole engine
/// theme is named after.
pub(crate) fn pickup_effects(kind: PickupKind) -> (u32, bool) {
    match kind {
        PickupKind::Multiball => (1, false),
        PickupKind::Wrecking => (0, true),
        PickupKind::Insiculous => (1, true),
    }
}

/// Whether another extra ball may spawn given how many are live.
pub(crate) fn multiball_allowed(extra_count: usize) -> bool {
    extra_count < MAX_EXTRA_BALLS
}

impl BreakoutGame {
    pub(crate) fn wrecking_active(&self) -> bool {
        self.wrecking.active()
    }

    /// The loaded sheet a candy kind wears.
    pub(crate) fn candy_sheet(&self, kind: PickupKind) -> &SpriteSheet {
        match kind {
            PickupKind::Multiball => &self.sheets.candy_multiball,
            PickupKind::Wrecking => &self.sheets.candy_wrecking,
            PickupKind::Insiculous => &self.sheets.candy_insiculous,
        }
    }

    /// Drop a candy at a destroyed brick's position; it falls toward the paddle as a
    /// dynamic sensor (no ball interference). Its sensor is the candy's drawn body, the
    /// union of its `idle` frames, and the sheet's anchor centres that body on it.
    pub(crate) fn spawn_pickup(&mut self, world: &mut World, kind: PickupKind, pos: Vec2) {
        let (spec, depth) = candy_spec(kind);
        let (sprite, animation) = art_components(self.candy_sheet(kind), spec, depth);
        let body = spec.bounds.1 - spec.bounds.0;
        let entity = world
            .spawn()
            .with(Name::new(candy_entity_name(kind)))
            .with(Transform2D::from_parts(pos, 0.0, spec.scale()))
            .with(sprite)
            .with(animation)
            .with(candy_machine())
            .with(
                RigidBody::new_dynamic()
                    .with_gravity_scale(0.0)
                    .with_rotation_locked(true),
            )
            .with(Collider::box_collider(body.x, body.y).as_sensor())
            .id();
        // Buffered-safe on the spawn frame; applied once the body syncs.
        self.physics.set_velocity(entity, Vec2::new(0.0, -PICKUP_FALL_SPEED), 0.0);
        self.pickups.track(entity, kind);
    }

    /// Resolve paddle catches from this frame's collision snapshot and grant
    /// the effects. In co-op either paddle catches; candies fall toward the
    /// bottom paddle, but a top-paddle graze on the way down still counts. A
    /// caught candy unwraps where it was caught.
    pub(crate) fn check_pickup_catches(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
    ) {
        let catchers: Vec<EntityId> =
            [self.paddle, self.paddle_top].into_iter().flatten().collect();
        // Where the live candies are, taken before the collection destroys the ones it
        // reaches.
        let tracked: Vec<(EntityId, Vec2)> = self
            .pickups
            .entities()
            .filter_map(|entity| ctx.world.get::<Transform2D>(entity).map(|t| (entity, t.position)))
            .collect();
        let caught = self
            .pickups
            .collect(collisions, &catchers, &mut self.physics, ctx.world);
        if caught.is_empty() {
            return;
        }

        for &(kind, _) in &caught {
            let (extra_balls, wrecking) = pickup_effects(kind);
            for _ in 0..extra_balls {
                self.try_spawn_extra_ball(ctx);
            }
            if wrecking {
                // Catching a second one refreshes the clock — no stacking.
                self.wrecking.start(WRECKING_DURATION);
                self.apply_ball_visuals(ctx.world);
            }
        }

        // `collect` reports in the tracker's order and `collected_positions` keeps it,
        // and every candy `spawn_pickup` tracks carries a `Transform2D`, so each kind
        // pairs with its own position.
        let remaining: Vec<EntityId> = self.pickups.entities().collect();
        let taken = collected_positions(&tracked, &remaining);
        for ((kind, _), position) in caught.iter().zip(taken) {
            let (spec, depth) = candy_spec(*kind);
            let collect = spawn_effect(
                ctx.world, "Candy Collect", spec, self.candy_sheet(*kind), depth, position, CANDY_COLLECT);
            self.transient_visuals.push(collect);
        }
    }

    /// Spawn a multiball-granted extra ball above the paddle, launched
    /// upward at a lightly randomized angle. Fizzles silently at the cap or
    /// outside active play.
    fn try_spawn_extra_ball(&mut self, ctx: &mut GameContext) {
        if !multiball_allowed(self.extra_balls.len()) || self.state != GameState::Playing {
            return;
        }
        let paddle_x = self
            .paddle
            .and_then(|p| ctx.world.get::<Transform2D>(p).map(|t| t.position.x))
            .unwrap_or(0.0);

        let ball = self.spawn_ball(ctx.world, "Deion (extra)", Vec2::new(paddle_x, PADDLE_Y + SERVE_OFFSET_Y));
        let angle = (hash_f32(self.frame_count.wrapping_add(7)) - 0.5) * 0.8;
        let dir = Vec2::new(angle.sin(), angle.cos());
        let speed = (BALL_SPEED * self.speed_mult).min(BALL_MAX_SPEED);
        self.physics.set_velocity(ball, dir * speed, 0.0);

        self.extra_balls.push(ball);
        // A new ball adopts the current form: frozen while wrecking runs.
        self.apply_ball_visuals(ctx.world);
    }

    /// Despawn pickups that fell past the paddle: bottom-sensor hit, or the
    /// y-threshold safety net in case a sensor event is ever missed.
    pub(crate) fn despawn_missed_pickups(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
    ) {
        if self.pickups.is_empty() {
            return;
        }
        let cutoff = -(WIN_H / 2.0 + 60.0);
        let doomed: Vec<EntityId> = self
            .pickups
            .entities()
            .filter(|&e| {
                let sensor_hit = self.bottom_sensor.is_some_and(|s| {
                    collisions.iter().any(|c| c.event.started && c.event.involves(e, s))
                });
                let fell = ctx
                    .world
                    .get::<Transform2D>(e)
                    .is_none_or(|t| !t.position.y.is_finite() || t.position.y < cutoff);
                sensor_hit || fell
            })
            .collect();
        if !doomed.is_empty() {
            self.pickups
                .remove_where(&mut self.physics, ctx.world, |p| doomed.contains(&p.entity));
        }
    }

    /// Tick the wrecking clock; when it expires, thaw the balls.
    pub(crate) fn update_wrecking(&mut self, ctx: &mut GameContext) {
        if self.wrecking.tick(ctx.delta_time) {
            self.apply_ball_visuals(ctx.world);
        }
    }

    /// Dress every live ball in the current form's sheet: frozen while wrecking runs,
    /// water otherwise. The swap writes the texture, the depth and the animation's grid
    /// and clips — never a colour — and the two sheets' bodies are the same box, so the
    /// ball's anchor and collider are untouched. Called on effect start/expiry AND on
    /// every ball spawn, so nothing can keep a stale form.
    pub(crate) fn apply_ball_visuals(&self, world: &mut World) {
        let (sheet, depth) = if self.wrecking.active() {
            (&self.sheets.ball_ice, BALL_ICE_DEPTH)
        } else {
            (&self.sheets.ball_water, BALL_WATER_DEPTH)
        };
        for ball in self.ball.into_iter().chain(self.extra_balls.iter().copied()) {
            if let Some(sprite) = world.get_mut::<Sprite>(ball) {
                sprite.texture_handle = sheet.texture.id;
                sprite.depth = depth;
            }
            if let Some(animation) = world.get_mut::<SpriteAnimation>(ball) {
                animation.grid = sheet.grid;
                animation.clips = sheet.clips.clone();
                animation.sheet = Some(sheet.path.clone());
            }
        }
    }

    /// Remove every in-flight pickup (match end / reset).
    pub(crate) fn destroy_all_pickups(&mut self, world: &mut World) {
        self.pickups.clear(&mut self.physics, world);
    }

    /// Make drop-bricks visibly pulse so players can spot the prizes.
    /// Owns the emissive channel for drop bricks only — armor damage owns
    /// the animation's frames, so the two never fight.
    pub(crate) fn pulse_drop_bricks(&self, world: &mut World) {
        let glow = 1.2 + 0.6 * (self.frame_count as f32 * 0.12).sin();
        for brick in self.bricks.iter().filter(|b| b.drop.is_some()) {
            if let Some(s) = world.get_mut::<Sprite>(brick.entity) {
                s.emissive = glow;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickup_effects_form_the_insiculous_trio() {
        assert_eq!(pickup_effects(PickupKind::Multiball), (1, false));
        assert_eq!(pickup_effects(PickupKind::Wrecking), (0, true));
        // Insiculous = both base powers at once.
        let (balls, wrecking) = pickup_effects(PickupKind::Insiculous);
        assert_eq!(balls, pickup_effects(PickupKind::Multiball).0);
        assert!(wrecking);
    }

    #[test]
    fn multiball_cap_blocks_at_limit() {
        assert!(multiball_allowed(0));
        assert!(multiball_allowed(MAX_EXTRA_BALLS - 1));
        assert!(!multiball_allowed(MAX_EXTRA_BALLS));
        assert!(!multiball_allowed(MAX_EXTRA_BALLS + 1));
    }
}
