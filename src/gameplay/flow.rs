//! Match state transitions, win detection, and entity visibility.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::spawning::backdrop_color;
use crate::types::*;

/// High-score board a session records into (see docs/WEB_SAVES.md: mode
/// strings are game-defined, lowercase, stable).
pub(crate) fn score_mode(mode: GameMode) -> &'static str {
    match mode {
        GameMode::SinglePlayer => "single",
        GameMode::TwoPlayerCoop => "coop",
    }
}

/// Who serves after a lost ball: in co-op the side that lost it redeems;
/// solo always serves from the bottom (there is no top paddle).
pub(crate) fn serve_side_after_loss(mode: GameMode, lost_past: PaddleSide) -> PaddleSide {
    match mode {
        GameMode::SinglePlayer => PaddleSide::Bottom,
        GameMode::TwoPlayerCoop => lost_past,
    }
}

impl BreakoutGame {
    /// State transitions during a match. Either player's primary action
    /// (Space/Enter/click/pad A) launches or restarts. Menu (Escape/pad
    /// Start) during Serving/Playing is handled by the pause gate upstream;
    /// GameOver keeps the direct exit to the title screen.
    pub(super) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        let launch = ctx.players.just_activated_any(GameAction::Action1, ctx.input);
        let menu = ctx.players.just_activated_any(GameAction::Menu, ctx.input);

        match &self.state {
            GameState::Serving => {
                if launch {
                    self.launch_balls(ctx);
                }
            }
            GameState::GameOver { .. } => {
                if launch {
                    self.start_game(ctx);
                } else if menu {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    pub(super) fn check_win_condition(&mut self, ctx: &mut GameContext) {
        if !matches!(self.state, GameState::Playing | GameState::Serving) { return; }
        if !self.bricks.is_empty() { return; }

        self.destroy_all_balls(ctx.world);
        self.destroy_all_pickups(ctx.world);
        self.wrecking.stop();
        self.unlock_win_achievements(ctx);
        let _ = ctx.scores.submit(score_mode(self.mode), self.score as u64);
        self.state = GameState::GameOver { won: true };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_balls(world);
        self.destroy_all_pickups(world);
        self.clear_transient_visuals(world);
        self.wrecking.stop();
        self.rest_tongs(world);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    /// Put each tong back at rest, closed in the facing it holds — a scowl a quit
    /// interrupted does not resume under the menus.
    pub(crate) fn rest_tongs(&self, world: &mut World) {
        for (tong, side) in [(self.tong, PaddleSide::Bottom), (self.tong_top, PaddleSide::Top)] {
            let Some(tong) = tong else { continue };
            let rest = tong_state(TONG_CLOSED, self.tong_facing[side.index()]);
            if let Some(machine) = world.get_mut::<ClipStateMachine>(tong) {
                let _ = machine.transition_to(&rest);
            }
        }
    }

    /// Give the backdrop grid the chosen chaos mode's colour. The level pick is the only
    /// thing that changes the mode, and it happens after `init()` spawned the backdrop,
    /// so the colour it was born with is the boot mode's.
    pub(crate) fn apply_backdrop_theme(&self, world: &mut World) {
        let color = backdrop_color(&theme_for(self.chaos_mode));
        if let Some(backdrop) = self.backdrop {
            if let Some(grid) = world.get_mut::<GridBackdrop>(backdrop) {
                grid.color = color;
            }
        }
    }

    /// Drop every detached one-shot. An id the world has already dropped — a splash
    /// whose `hurt` ran out — is skipped: the id is generational, so a stale entry can
    /// never remove anything else.
    pub(crate) fn clear_transient_visuals(&mut self, world: &mut World) {
        for entity in self.transient_visuals.drain(..) {
            world.remove_entity(&entity).ok();
        }
    }

    /// Gameplay sprites only render during a match — the menus get the bare counter
    /// and the grid over it, which stay up the way Tong's court does.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = !matches!(
            self.state,
            GameState::TitleScreen { .. } | GameState::LevelSelect { .. } | GameState::Achievements
        );
        let entities = [self.tong, self.tong_top, self.ball].into_iter().flatten()
            .chain(self.extra_balls.iter().copied())
            .chain(self.wall_strips.iter().copied())
            .chain(self.bricks.iter().map(|b| b.entity))
            .chain(self.pickups.entities().collect::<Vec<_>>())
            .chain(self.transient_visuals.iter().copied())
            .collect::<Vec<_>>();
        set_sprites_visible(ctx.world, entities, visible);
    }
}
