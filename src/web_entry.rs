//! Browser entry point: fetch all assets into the engine's VFS, then run
//! the exact same game `main.rs` runs natively.
//!
//! Saves persist to browser localStorage: on wasm the engine treats the
//! `GameConfig` save-path strings as localStorage keys (achievements, high
//! scores, input bindings — see the engine's `docs/WEB_SAVES.md` contract).
//! The site's profile page reads the same keys.
//!
//! With `--features editor` the same library builds a SECOND bundle, served at
//! `/playground/breakout/`, that runs this game inside the engine's scene editor.
//! That bundle writes none of the `beinsiculous.games.breakout.*` keys above — an
//! editor session must not record achievements, scores or bindings the site's
//! boards read — and keeps its camera and panel layout in a slot of its own.

use engine_core::web::{init_web_logging, preload_assets, set_boot_status};
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(not(feature = "editor"))]
use engine_core::prelude::run_game;

#[cfg(feature = "editor")]
use editor_integration::{run_game_with_editor_opts, EditorRunOptions};
#[cfg(feature = "editor")]
use std::path::PathBuf;

/// Where the deployed build serves its assets from; also the VFS key base.
///
/// VERSION-BUMP CHECKLIST — these must all agree (a mismatch 404s every
/// asset at boot with a "not in vfs" message):
/// 1. this constant (`/games/<slug>/v<N>/assets`),
/// 2. `scripts/build_wasm.sh`'s output dir (currently hardcoded `v1`),
/// 3. the site's `src/content/games/<slug>.md` `wasm:` path,
/// 4. the deployed dir `insiculous_web/public/games/<slug>/v<N>/`.
#[cfg(not(feature = "editor"))]
const ASSET_BASE: &str = "/games/breakout/v2/assets";

/// Where the editor bundle serves its assets from; also its VFS key base.
///
/// Its version is INDEPENDENT of the game's own: the two bundles deploy
/// separately. VERSION-BUMP CHECKLIST for this one:
/// 1. this constant (`/playground/<slug>/v<N>/assets`),
/// 2. `scripts/build_wasm.sh --kind editor`'s output dir,
/// 3. the site's `src/content/games/<slug>.md` `editor:` path,
/// 4. the deployed dir `insiculous_web/public/playground/<slug>/v<N>/`.
#[cfg(feature = "editor")]
const EDITOR_ASSET_BASE: &str = "/playground/breakout/v1/assets";

/// The editor's preferences slot for this game.
///
/// On wasm a preferences path is a localStorage key, and the editor's native
/// default (`editor_prefs.json`) would sit outside the site's key contract.
/// Each game gets its own slot because the preferences carry the camera.
#[cfg(feature = "editor")]
const EDITOR_PREFS_SLOT: &str = "beinsiculous.playground.breakout.editor_prefs";

/// The base the boot phase preloads — whichever bundle this build is.
#[cfg(not(feature = "editor"))]
const BOOT_ASSET_BASE: &str = ASSET_BASE;
#[cfg(feature = "editor")]
const BOOT_ASSET_BASE: &str = EDITOR_ASSET_BASE;

#[wasm_bindgen(start)]
pub fn start() {
    init_web_logging();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = preload_assets(BOOT_ASSET_BASE).await {
            log::error!("asset preload failed: {e}");
            set_boot_status(&format!("Failed to load assets: {e}"));
            return;
        }
        if let Err(e) = run_configured_game() {
            log::error!("failed to start game: {e}");
            set_boot_status(&format!("Failed to start: {e}"));
        }
    });
}

#[cfg(not(feature = "editor"))]
fn run_configured_game() -> Result<(), engine_core::EngineError> {
    let config = crate::game_config(ASSET_BASE)
        .with_achievement_save_path("beinsiculous.games.breakout.achievements")
        .with_input_settings_path("beinsiculous.games.breakout.input")
        .with_score_save_path("beinsiculous.games.breakout.scores");
    run_game(crate::BreakoutGame::default(), config)
}

/// The editor session passes NO save paths: with none set the engine keeps
/// achievements, scores and bindings in memory and falls back to the default
/// input bindings, so nothing the site's boards read is written.
#[cfg(feature = "editor")]
fn run_configured_game() -> Result<(), engine_core::EngineError> {
    run_game_with_editor_opts(
        crate::BreakoutGame::default(),
        crate::game_config(EDITOR_ASSET_BASE),
        EditorRunOptions {
            prefs_slot: Some(PathBuf::from(EDITOR_PREFS_SLOT)),
            ..Default::default()
        },
    )
}
