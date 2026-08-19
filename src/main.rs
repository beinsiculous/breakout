use insiculous_breakout::{game_config, BreakoutGame};

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = game_config(&root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/breakout_achievements.json").to_string_lossy())
        .with_input_settings_path(root.join("saves/input_settings.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop, collider overlay);
    // without it the game runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(BreakoutGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    engine_core::prelude::run_game(BreakoutGame::default(), config).unwrap();
}
