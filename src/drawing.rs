use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::chaos_theme::theme_for;
use crate::menu::{achievements_panel, level_select_panel, title_panel};
use crate::types::*;

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
        let panel = title_panel("INSICULOUS BREAKOUT", ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        let items = ["1 Player", "2 Player Co-op", "Achievements", "Exit"];
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
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
                // Registry always has entries for these ids (registered in init).
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

        ctx.ui.label(&format!("SCORE {}", self.score), Vec2::new(40.0, 16.0));
        let lives_text = format!("LIVES {}", "* ".repeat(self.lives as usize).trim_end());
        ctx.ui.label(&lives_text, Vec2::new(ctx.window_size.x - 140.0, 16.0));
        if self.mode == GameMode::TwoPlayerCoop {
            ctx.ui.label_centered("CO-OP", Vec2::new(cx, 16.0));
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
            let c = crate::constants::WRECKING_BALL_COLOR;
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
                ctx.ui.label_centered(server, Vec2::new(cx, cy - 50.0));
                ctx.ui.label_centered("A/D, Arrows, stick, or mouse to move - ESC to pause", Vec2::new(cx, cy - 24.0));
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
