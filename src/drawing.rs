use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::chaos_theme::theme_for;
use crate::menu::{achievements_panel, level_select_panel, title_label, title_panel, TITLE_ITEMS};
use crate::types::*;

/// How far below the window's centre the serve prompt's first line sits, in window
/// pixels: clear of the food wall in both modes, and its three lines end above the crest
/// of a Deion resting on the bottom tong.
const SERVE_PROMPT_BELOW_CENTER: f32 = 124.0;
/// The gap between the serve prompt's lines, in window pixels.
const SERVE_PROMPT_LINE_GAP: f32 = 26.0;

impl BreakoutGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&theme_for(self.chaos_mode))
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::LevelSelect { selection } => self.draw_level_select(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = title_panel("THE FOOD PYRAMID", ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, item) in TITLE_ITEMS.iter().enumerate() {
            y = panel.item(ctx.ui, y, title_label(*item), i as u8 == selection, &style);
        }
        panel.hint(
            ctx.ui,
            "W/S or D-Pad navigate - SPACE/ENTER, (A), or click confirm",
            &style,
        );
    }

    fn draw_level_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let roster = crate::levels::roster(self.mode);
        let panel = level_select_panel("SELECT LEVEL", ctx.window_size, self.mode);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, level) in roster.iter().enumerate() {
            // Each entry glows in its chaos mode's banner color.
            let c = theme_for(level.mode).banner_color;
            y = panel.item_colored(
                ctx.ui,
                y,
                &format!("{} - {}", level.title, level.mode.label()),
                c,
                i as u8 == selection,
                &style,
            );
        }
        panel.hint(
            ctx.ui,
            crate::levels::level_hint(self.mode, selection as usize),
            &style,
        );
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        // Tall window; the section list draws left-aligned inside it.
        let panel = achievements_panel("ACHIEVEMENTS", ctx.window_size);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} unlocked"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section, ids) in DISPLAY_SECTIONS {
            ctx.ui.label_styled(section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                // Registry always has entries for these ids (registered in `register_achievements`).
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                ctx.ui.label_styled(
                    &format!("{marker} {}", ach.name),
                    Vec2::new(left + 8.0, y),
                    name_color,
                    14.0,
                );
                ctx.ui.label_styled(&ach.description, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, "ESC, SPACE, or click to go back", &style);
    }

    fn draw_gameplay(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        // Solo's top wall is drawn by the pale counter-edge rail, and white text on it
        // loses its contrast: the HUD sits just below the rail there. Co-op has no top
        // rail, and lower down its label would run into the top tong.
        let hud_y = match self.mode {
            GameMode::SinglePlayer => crate::constants::WALL_THICKNESS + 8.0,
            GameMode::TwoPlayerCoop => 16.0,
        };
        ctx.ui.label(&format!("SCORE {}", self.score), Vec2::new(40.0, hud_y));
        let lives_text = format!("LIVES {}", "* ".repeat(self.lives as usize).trim_end());
        ctx.ui.label(&lives_text, Vec2::new(ctx.window_size.x - 140.0, hud_y));
        if self.mode == GameMode::TwoPlayerCoop {
            ctx.ui.label_centered("CO-OP", Vec2::new(cx, hud_y));
        }

        let theme = theme_for(self.chaos_mode);
        if let Some(banner) = theme.banner_text {
            let color = Color::new(theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w);
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 24.0), color, 16.0);
        }

        if self.combo >= 3 {
            ctx.ui.label_centered(&format!("COMBO x{}", self.combo), Vec2::new(cx, 48.0));
        }

        if self.wrecking_active() {
            let c = crate::constants::WRECKING_LABEL_COLOR;
            ctx.ui.label_centered_styled(
                &format!("WRECKING {:.1}s", self.wrecking.remaining()),
                Vec2::new(cx, 72.0),
                Color::new(c.x, c.y, c.z, c.w),
                16.0,
            );
        }

        match &self.state {
            GameState::Serving => {
                let server = match (self.mode, self.serving_side) {
                    (GameMode::SinglePlayer, _) => "SPACE, ENTER, or CLICK to launch",
                    (_, PaddleSide::Bottom) => "P1 SERVES - SPACE, CLICK, or (A) to launch",
                    (_, PaddleSide::Top) => "P2 SERVES - ENTER or (A) to launch",
                };
                // The second line teaches the jaw: toward the field bites, away opens, and
                // a bite timed as Deion lands is the chomp shot.
                let bite = match self.mode {
                    GameMode::SinglePlayer => "W/UP bite - S/DOWN open - RIGHT-CLICK bite - bite as it lands to smash",
                    GameMode::TwoPlayerCoop => "P1: W bite, S open, RIGHT-CLICK bite - P2: DOWN bite, UP open",
                };
                // Between the wall and the bottom paddle: the food wall reaches 26 px above
                // the centre solo, and co-op's band 106 px below it.
                let lines = [server, bite, "A/D, Arrows, stick, or mouse to move - ESC to pause"];
                for (index, line) in lines.into_iter().enumerate() {
                    let below = SERVE_PROMPT_BELOW_CENTER + index as f32 * SERVE_PROMPT_LINE_GAP;
                    ctx.ui.label_centered(line, Vec2::new(cx, cy + below));
                }
            }
            GameState::GameOver { won } => {
                let msg = if *won { "BOARD CLEARED!" } else { "GAME OVER" };
                let style = self.menu_style();
                let panel = MenuPanel::new(msg, Vec2::new(cx, cy), 340.0, 2);
                let mut y = panel.begin(ctx.ui, &style);
                y = panel.line(ctx.ui, y, &format!("Final score: {}", self.score), &style);
                panel.line(ctx.ui, y, "SPACE or ENTER to play again", &style);
                panel.hint(ctx.ui, "ESC for title screen", &style);
            }
            _ => {}
        }

        if self.pause.is_active() {
            let style = self.menu_style();
            self.pause.draw(ctx.ui, ctx.window_size, &style);
        }
    }
}
