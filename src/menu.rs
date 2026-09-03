use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::spawning;
use crate::types::*;

/// Title-screen menu entries. Both halves (input hit-testing here, drawing
/// in `drawing.rs`) derive rows from `TITLE_ITEMS`, so keyboard, mouse, and
/// labels can never desync when the roster changes per target.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleItem {
    OnePlayer,
    TwoPlayerCoop,
    Achievements,
    Exit,
}

/// The web build hides "Achievements" — the site's game page shows the same
/// unlocks (read from localStorage) beside the canvas instead.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::OnePlayer,
    TitleItem::TwoPlayerCoop,
    TitleItem::Achievements,
    TitleItem::Exit,
];
#[cfg(target_arch = "wasm32")]
pub(crate) const TITLE_ITEMS: &[TitleItem] =
    &[TitleItem::OnePlayer, TitleItem::TwoPlayerCoop, TitleItem::Exit];

/// Row index of `item` in the current target's roster (0 if absent).
pub(crate) fn title_index(item: TitleItem) -> u8 {
    TITLE_ITEMS.iter().position(|i| *i == item).unwrap_or(0) as u8
}

pub(crate) fn title_label(item: TitleItem) -> &'static str {
    match item {
        TitleItem::OnePlayer => "1 Player",
        TitleItem::TwoPlayerCoop => "2 Player Co-op",
        TitleItem::Achievements => "Achievements",
        TitleItem::Exit => "Exit",
    }
}

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`drawing.rs`) — the geometry must match or clicks land
/// beside the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 380.0, TITLE_ITEMS.len())
}
/// Row count follows the mode's roster, so the hit-tested rows can never
/// drift from the drawn ones when a level is added.
pub(crate) fn level_select_panel(title: &str, window_size: Vec2, mode: GameMode) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 420.0, crate::levels::roster(mode).len())
}
pub(crate) fn achievements_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, window_size.x - 120.0, 15)
}

impl BreakoutGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = title_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        // An out-of-range stored selection (e.g. the shorter wasm menu)
        // clamps instead of panicking at the dispatch index below.
        let selection = selection.min(TITLE_ITEMS.len() as u8 - 1);
        let mut selection = input.navigate(selection, TITLE_ITEMS.len() as u8);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match TITLE_ITEMS[selection as usize] {
                TitleItem::OnePlayer => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::LevelSelect { selection: 0 };
                }
                TitleItem::TwoPlayerCoop => {
                    self.mode = GameMode::TwoPlayerCoop;
                    self.state = GameState::LevelSelect { selection: 0 };
                }
                TitleItem::Achievements => self.state = GameState::Achievements,
                TitleItem::Exit => ctx.request_exit(),
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // The page is one big non-selectable list: any click on it dismisses,
        // same as confirm/back.
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen {
                selection: title_index(TitleItem::Achievements),
            };
        }
    }

    pub(crate) fn update_level_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let levels = crate::levels::roster(self.mode);
        let mouse = level_select_panel("", ctx.window_size, self.mode).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, levels.len() as u8);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::LevelSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
            let index = selection as usize;
            self.selected_level = index;
            // Chaos mode is a property of the level now, not a menu choice.
            self.chaos_mode = levels[index].mode;
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx);
        }
    }

    /// Reset score/lives, rebuild the playfield for the selected mode,
    /// rebuild the brick grid, and put a fresh ball on the serving paddle.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.speed_mult = 1.0;
        self.combo = 0;
        self.serving_side = PaddleSide::Bottom;

        self.destroy_all_balls(ctx.world);
        self.destroy_all_pickups(ctx.world);
        self.wrecking.stop();
        for brick in self.bricks.drain(..) {
            self.physics.destroy_entity(ctx.world, brick.entity);
        }
        // Walls/sensors/paddles differ per mode (co-op opens the top edge),
        // so the playfield structure is rebuilt every match start.
        self.rebuild_playfield(ctx.world, self.mode);
        self.bricks = self.spawn_level_bricks(ctx);

        let ball = self.spawn_ball(ctx.world);
        self.ball = Some(ball);

        self.apply_theme(ctx.world);
        if let Some(paddle) = self.paddle {
            self.physics.set_kinematic_target(paddle, Vec2::new(0.0, PADDLE_Y), 0.0);
        }
        if let Some(paddle) = self.paddle_top {
            self.physics.set_kinematic_target(paddle, Vec2::new(0.0, PADDLE_TOP_Y), 0.0);
        }
        self.state = GameState::Serving;
    }

    /// Spawn the selected level's brick layout from its scene, falling back
    /// to the generated grid when the scene is missing, broken, or empty —
    /// worst case is always the classic layout, never a brickless game.
    fn spawn_level_bricks(&mut self, ctx: &mut GameContext) -> Vec<Brick> {
        // Owned copy: the scene load below borrows `ctx.assets` mutably.
        let asset_base = ctx.assets.base_path().to_string();
        if let Some(level) =
            crate::levels::load_level_data(&asset_base, self.mode, self.selected_level)
        {
            match crate::levels::spawn_bricks_from_scene(&level, ctx.world, ctx.assets) {
                Ok(bricks) if !bricks.is_empty() => return bricks,
                Ok(_) => eprintln!(
                    "breakout: level scene has no bricks; using generated brick grid"
                ),
                Err(e) => eprintln!(
                    "breakout: failed to instantiate level scene: {e}; using generated brick grid"
                ),
            }
        }
        match self.mode {
            GameMode::SinglePlayer => spawning::spawn_bricks(ctx.world, self.tex_id),
            GameMode::TwoPlayerCoop => spawning::spawn_bricks_2p(ctx.world, self.tex_id),
        }
    }

    /// Push the current `chaos_mode`'s look onto the live entities:
    /// background tint, wall color, ball color, and grid color.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = theme_for(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) { s.color = theme.bg_color; }
        }
        for &w in &self.walls {
            if let Some(s) = world.get_mut::<Sprite>(w) { s.color = theme.structure_color; }
        }
        for ball in self.ball.into_iter().chain(self.extra_balls.iter().copied()) {
            if let Some(s) = world.get_mut::<Sprite>(ball) { s.color = theme.accent_color; }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }
}
