//! Breakout-specific achievement definitions and unlock logic.
//!
//! Registered through `Game::register_achievements` — the engine calls it before the window
//! opens, which is what lets `--achievements-manifest` export the list with no GPU. Win-related achievements unlock from
//! `check_win_condition`; the combo achievement unlocks live from
//! `check_brick_hits` the moment the volley reaches `COMBO_TARGET`.

use engine_core::prelude::*;

use crate::constants::STARTING_LIVES;
use crate::types::BreakoutGame;

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const CLEAR_NORMAL:     &str = "clear_normal";
pub(crate) const CLEAR_INSANE:     &str = "clear_insane";
pub(crate) const CLEAR_RIDICULOUS: &str = "clear_ridiculous";
pub(crate) const CLEAR_INSICULOUS: &str = "clear_insiculous";

pub(crate) const PERFECT_NORMAL:     &str = "perfect_normal";
pub(crate) const PERFECT_INSANE:     &str = "perfect_insane";
pub(crate) const PERFECT_RIDICULOUS: &str = "perfect_ridiculous";
pub(crate) const PERFECT_INSICULOUS: &str = "perfect_insiculous";

pub(crate) const COMBO_VOLLEY: &str = "combo_volley";
pub(crate) const LAST_LIFE:    &str = "last_life";

/// Grouped display order for the achievements page. First tuple element is
/// the section header, second is the list of ids to render under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("Clears",
        &[CLEAR_NORMAL, CLEAR_INSANE, CLEAR_RIDICULOUS, CLEAR_INSICULOUS]),
    ("Perfect Clears",
        &[PERFECT_NORMAL, PERFECT_INSANE, PERFECT_RIDICULOUS, PERFECT_INSICULOUS]),
    ("Skill",
        &[COMBO_VOLLEY, LAST_LIFE]),
];

/// Register every Breakout achievement. Call once from `Game::register_achievements`.
pub(crate) fn register_all(mgr: &mut AchievementManager) {
    mgr.register(Achievement::new(CLEAR_NORMAL,
        "Demolition Crew",
        "Clear every brick in Normal mode."));
    mgr.register(Achievement::new(CLEAR_INSANE,
        "Speed Wrecker",
        "Clear every brick in Insane mode."));
    mgr.register(Achievement::new(CLEAR_RIDICULOUS,
        "Double Demolition",
        "Clear every brick in Ridiculous mode."));
    mgr.register(Achievement::new(CLEAR_INSICULOUS,
        "Insiculous Wrecking Ball",
        "Clear every brick in Insiculous mode."));

    mgr.register(Achievement::new(PERFECT_NORMAL,
        "Flawless Foundation",
        "Clear Normal mode without losing a ball."));
    mgr.register(Achievement::new(PERFECT_INSANE,
        "Flawless Frenzy",
        "Clear Insane mode without losing a ball."));
    mgr.register(Achievement::new(PERFECT_RIDICULOUS,
        "Flawless Juggling",
        "Clear Ridiculous mode without losing a ball."));
    mgr.register(Achievement::new(PERFECT_INSICULOUS,
        "Insiculously Flawless",
        "Clear Insiculous mode without losing a ball."));

    mgr.register(Achievement::new(COMBO_VOLLEY,
        "Wrecking Volley",
        "Destroy 5 bricks in a single volley (no paddle touch)."));
    mgr.register(Achievement::new(LAST_LIFE,
        "Clutch Clear",
        "Clear the board on your very last life."));
}

impl BreakoutGame {
    /// Called from `check_win_condition` when the last brick falls.
    pub(crate) fn unlock_win_achievements(&self, ctx: &mut GameContext) {
        ctx.achievements.unlock(chaos_clear_id(self.chaos_mode));
        if self.lives == STARTING_LIVES {
            ctx.achievements.unlock(chaos_perfect_id(self.chaos_mode));
        }
        if self.lives == 1 {
            ctx.achievements.unlock(LAST_LIFE);
        }
        // COMBO_VOLLEY unlocks live in check_brick_hits, not here.
    }
}

fn chaos_clear_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => CLEAR_NORMAL,
        ChaosMode::Insane     => CLEAR_INSANE,
        ChaosMode::Ridiculous => CLEAR_RIDICULOUS,
        ChaosMode::Insiculous => CLEAR_INSICULOUS,
    }
}

fn chaos_perfect_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => PERFECT_NORMAL,
        ChaosMode::Insane     => PERFECT_INSANE,
        ChaosMode::Ridiculous => PERFECT_RIDICULOUS,
        ChaosMode::Insiculous => PERFECT_INSICULOUS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_all_adds_ten() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        assert_eq!(mgr.total(), 10);
    }

    #[test]
    fn chaos_clear_id_maps_each_mode() {
        assert_eq!(chaos_clear_id(ChaosMode::Normal),     CLEAR_NORMAL);
        assert_eq!(chaos_clear_id(ChaosMode::Insane),     CLEAR_INSANE);
        assert_eq!(chaos_clear_id(ChaosMode::Ridiculous), CLEAR_RIDICULOUS);
        assert_eq!(chaos_clear_id(ChaosMode::Insiculous), CLEAR_INSICULOUS);
    }

    #[test]
    fn chaos_perfect_id_maps_each_mode() {
        assert_eq!(chaos_perfect_id(ChaosMode::Normal),     PERFECT_NORMAL);
        assert_eq!(chaos_perfect_id(ChaosMode::Insane),     PERFECT_INSANE);
        assert_eq!(chaos_perfect_id(ChaosMode::Ridiculous), PERFECT_RIDICULOUS);
        assert_eq!(chaos_perfect_id(ChaosMode::Insiculous), PERFECT_INSICULOUS);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        for id in [
            CLEAR_NORMAL, CLEAR_INSANE, CLEAR_RIDICULOUS, CLEAR_INSICULOUS,
            PERFECT_NORMAL, PERFECT_INSANE, PERFECT_RIDICULOUS, PERFECT_INSICULOUS,
            COMBO_VOLLEY, LAST_LIFE,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }
}
