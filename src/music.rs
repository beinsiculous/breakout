//! Shelf Breaker: original E-minor breakbeat with independently triggered feedback.
//! The backing plays under menus and pauses. Destroyed bricks and clear rewards
//! use the game's source; surviving armor and paddle bounces use their entity source.
//! Brick and armor share the march cursor; paddle follows a continuously tiled bass
//! line. Loss, menu and quiet tong movement have independent parts. Existing collision
//! order remains paddles, bricks (descending index), lost balls, then clear.

use engine_core::prelude::*;

use crate::constants::{GRID_IMPULSE_PADDLE_HIT_RADIUS, GRID_IMPULSE_PADDLE_HIT_STRENGTH};
use crate::gameplay::ripple_grid;

/// The song under the asset base: an agent's first cut, under `ai/music` until a person's
/// pass promotes it.
pub(crate) const SONG_PATH: &str = "ai/music/ai_food_pyramid.mid";
/// The song's seven bindings.
pub(crate) const BRICK_BINDING: &str = "brick";
pub(crate) const ARMOR_BINDING: &str = "armor";
pub(crate) const PADDLE_BINDING: &str = "paddle";
pub(crate) const LOSS_BINDING: &str = "loss";
pub(crate) const CLEAR_BINDING: &str = "clear";
pub(crate) const MOVE_BINDING: &str = "move";
pub(crate) const MENU_BINDING: &str = "menu";
/// A beat that is not a bar line ripples under the tongs at this share of a paddle hit;
/// zero pulses on the bars only.
pub(crate) const BEAT_PULSE_FRACTION: f32 = 0.25;
/// The pause menu's footer: the engine's, plus that a move plays.
pub(crate) const PAUSE_HINT: &str = "ESC resumes - SPACE/ENTER or click confirms - move to jam";

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against. The jam and the pulse are counted by `GameMusic`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) bricks: u64,
    pub(crate) armor_hits: u64,
    pub(crate) paddle_hits: u64,
    pub(crate) losses: u64,
    pub(crate) clears: u64,
    pub(crate) moves: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    movement: [crate::movement::MovementCue; 2],
    /// The bindings this frame triggered, in the order the game sent them: production's
    /// call order recorded for the same-frame order tests, never reconstructed.
    #[cfg(test)]
    pub(crate) triggered: Vec<&'static str>,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            game: GameMusic::new(SONG_PATH, "food pyramid", "FOOD_PYRAMID_TELEMETRY"),
            counts: MusicCounts::default(),
            movement: [crate::movement::MovementCue::default(); 2],
            #[cfg(test)]
            triggered: Vec::new(),
        }
    }
}

impl Music {
    pub(crate) fn reset_movement(&mut self) {
        self.movement = [crate::movement::MovementCue::default(); 2];
    }

    /// Completed displacement excludes held input at a clamped wall.
    pub(crate) fn movement(&mut self, ctx: &mut GameContext, index: usize, paddle: EntityId, displacement: f32, requested_speed: f32) {
        if self.movement[index].observe(displacement, ctx.delta_time, requested_speed) {
            self.counts.moves += 1;
            let source = ctx.sources.entity(paddle);
            self.send(ctx, MOVE_BINDING, source);
        }
    }

    /// Request the song before any gesture; on the web it starts on the first click or key.
    pub(crate) fn start(&mut self, ctx: &mut GameContext) {
        self.game.start(ctx);
    }

    /// A match starts: the engine's identity rule decides by the song in the slot.
    pub(crate) fn restart(&mut self, ctx: &mut GameContext) {
        self.reset_movement();
        self.game.restart(ctx);
    }

    /// A brick was destroyed: the hook's next unit on the brittle pluck, on the game's source —
    /// the brick is gone in the same call, and its note must ring on.
    pub(crate) fn brick(&mut self, ctx: &mut GameContext) {
        self.counts.bricks += 1;
        let source = ctx.sources.game();
        self.send(ctx, BRICK_BINDING, source);
    }

    /// A foil-wrapped brick survived a hit: the hook's next unit on the heavier foil synth, on the
    /// brick's own source, so a second ball that kills it mid-note releases the note.
    pub(crate) fn armor(&mut self, ctx: &mut GameContext, brick: EntityId) {
        self.counts.armor_hits += 1;
        let source = ctx.sources.entity(brick);
        self.send(ctx, ARMOR_BINDING, source);
    }

    /// A tong returned a ball (a chomp included): the bass note written for this moment of
    /// the score, on the paddle's own source.
    pub(crate) fn paddle(&mut self, ctx: &mut GameContext, paddle: EntityId) {
        self.counts.paddle_hits += 1;
        let source = ctx.sources.entity(paddle);
        self.send(ctx, PADDLE_BINDING, source);
    }

    /// A ball was lost: one stroke of the dirge, on the game's source.
    pub(crate) fn loss(&mut self, ctx: &mut GameContext) {
        self.counts.losses += 1;
        let source = ctx.sources.game();
        self.send(ctx, LOSS_BINDING, source);
    }

    /// The board is cleared: the bright reward chord, on the game's source.
    pub(crate) fn clear(&mut self, ctx: &mut GameContext) {
        self.counts.clears += 1;
        let source = ctx.sources.game();
        self.send(ctx, CLEAR_BINDING, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if paused { self.reset_movement(); }
        if self.game.follow_pause(ctx, paused, MENU_BINDING) {
            self.record(MENU_BINDING);
        }
    }

    /// A running gameplay frame: on a bar line the paddle-hit ripple under each live tong,
    /// on another beat a share of it. Never under the pause menu, whose frozen grid discards
    /// what it is given.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, paddles: [Option<Vec2>; 2]) {
        let strength = match self.game.beat_pulse(ctx) {
            Some(BeatPulse::Bar) => GRID_IMPULSE_PADDLE_HIT_STRENGTH,
            Some(BeatPulse::Beat) => GRID_IMPULSE_PADDLE_HIT_STRENGTH * BEAT_PULSE_FRACTION,
            None => return,
        };
        for paddle in paddles.into_iter().flatten() {
            ripple_grid(ctx.world, paddle, strength, GRID_IMPULSE_PADDLE_HIT_RADIUS);
        }
    }

    /// Every frame, whatever the state: the engine's frame ring, telemetry and refusal
    /// lines, the telemetry ending in the game's counts.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        #[cfg(test)]
        self.triggered.clear();
        let (counts, jam_units, pulsed) = (self.counts, self.game.jam_units(), self.game.pulsed());
        self.game.update(ctx, || {
            format!(
                "bricks={} armor_hits={} paddle_hits={} losses={} clears={} moves={} jam_units={jam_units} pulsed={pulsed}",
                counts.bricks, counts.armor_hits, counts.paddle_hits, counts.losses, counts.clears, counts.moves
            )
        });
    }

    fn send(&mut self, ctx: &mut GameContext, binding: &'static str, source: SourceId) {
        self.record(binding);
        ctx.sources.trigger(ctx.audio, binding, source);
    }

    #[cfg(test)]
    fn record(&mut self, binding: &'static str) {
        self.triggered.push(binding);
    }

    #[cfg(not(test))]
    fn record(&mut self, _binding: &'static str) {}
}
