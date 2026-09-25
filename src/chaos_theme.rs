//! The Food Pyramid's chaos theming: the engine's shared [`ChaosTheme`] palette
//! with this game's Normal-mode look layered on top. The counter and its rails are
//! art and never tinted, so a theme colours only the backdrop grid, the particles
//! and the menus — `MenuStyle::from_theme` still reads the background and structure
//! colours for its panels. The Insane/Ridiculous/Insiculous palettes are the engine
//! defaults.

use engine_core::prelude::*;

/// The engine palette with breakout's Normal-mode overrides applied.
pub(crate) fn theme_for(mode: ChaosMode) -> ChaosTheme {
    let base = ChaosTheme::for_mode(mode);
    match mode {
        ChaosMode::Normal => ChaosTheme {
            bg_color: Vec4::new(0.0, 0.01, 0.04, 1.0),
            structure_color: Vec4::new(0.35, 0.38, 0.48, 1.0),
            grid_color: Vec4::new(0.12, 0.25, 0.6, 0.5),
            ..base
        },
        _ => base,
    }
}
