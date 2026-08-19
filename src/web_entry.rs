//! Browser entry point: fetch all assets into the engine's VFS, then run
//! the exact same game `main.rs` runs natively.
//!
//! No save paths are configured on the web, so the engine's existing
//! fallbacks apply: in-memory achievements and default two-player input
//! bindings (full browser persistence is the deferred H6 work).

use engine_core::prelude::run_game;
use engine_core::web::{init_web_logging, preload_assets, set_boot_status};
use wasm_bindgen::prelude::wasm_bindgen;

/// Where the deployed build serves its assets from; also the VFS key base.
///
/// VERSION-BUMP CHECKLIST — these must all agree (a mismatch 404s every
/// asset at boot with a "not in vfs" message):
/// 1. this constant (`/games/<slug>/v<N>/assets`),
/// 2. `scripts/build_wasm.sh`'s output dir (currently hardcoded `v1`),
/// 3. the site's `src/content/games/<slug>.md` `wasm:` path,
/// 4. the deployed dir `insiculous_web/public/games/<slug>/v<N>/`.
const ASSET_BASE: &str = "/games/breakout/v1/assets";

#[wasm_bindgen(start)]
pub fn start() {
    init_web_logging();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = preload_assets(ASSET_BASE).await {
            log::error!("asset preload failed: {e}");
            set_boot_status(&format!("Failed to load assets: {e}"));
            return;
        }
        if let Err(e) = run_game(crate::BreakoutGame::default(), crate::game_config(ASSET_BASE)) {
            log::error!("failed to start game: {e}");
            set_boot_status(&format!("Failed to start: {e}"));
        }
    });
}
