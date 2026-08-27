//! Match state transitions, win detection, and entity visibility.

use engine_core::prelude::*;
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
        ctx.scores.submit(score_mode(self.mode), self.score as u64);
        self.state = GameState::GameOver { won: true };
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_balls(world);
        self.destroy_all_pickups(world);
        self.wrecking.stop();
        self.state = GameState::TitleScreen { selection: 0 };
    }

    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = !matches!(
            self.state,
            GameState::TitleScreen { .. } | GameState::LevelSelect { .. } | GameState::Achievements
        );
        let entities = [self.paddle, self.paddle_top, self.ball].into_iter().flatten()
            .chain(self.extra_balls.iter().copied())
            .chain(self.walls.iter().copied())
            .chain(self.bricks.iter().map(|b| b.entity))
            .chain(self.pickups.entities().collect::<Vec<_>>());
        for entity in entities {
            if let Some(sprite) = ctx.world.get_mut::<Sprite>(entity) {
                sprite.visible = visible;
            }
        }
    }
}
