# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run all 47 tests (see "Where the tests live" below)
cargo test <test_name>        # run a single test
cargo clippy                  # must stay clean
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root points to the shared art repo (`../deion_assets`) — read-only reference, never write through it.

## Architecture

This is a single-crate game (`insiculous_breakout`) built on the in-house `insiculous_2d` ECS engine. `BreakoutGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` builds the persistent scaffolding (background, solo playfield, deforming grid), `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`; no game code changes between the two modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: the menu states (`TitleScreen`, `LevelSelect`, `Achievements`) dispatch to handlers in `menu.rs`; everything else (`Serving`, `Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{paddles,balls,bricks,flow}.rs`. Flow is Title → Level Select (picking a level also picks its chaos mode) → Serving ↔ Playing → GameOver. `update_entity_visibility()` (flow.rs) hides all gameplay sprites while a menu state is active — entities persist, only `Sprite.visible` flips.

**Module layout:**

- `main.rs` — `Game` impl, `GameConfig` (800×600, 60 FPS, asset/save paths anchored via `game_root!()`), editor wiring
- `constants.rs` — ALL gameplay tuning (sizes, speeds, brick grid, pickup timing, grid impulses, colors). Values tuned in the editor inspector must be copied back here to persist
- `types.rs` — `BreakoutGame` state, `GameState`, `GameMode` (SinglePlayer / TwoPlayerCoop), `PaddleSide`, `Brick`, `PickupKind`
- `spawning.rs` — paddles (kinematic capsule-x colliders), walls, loss sensors, ball (dynamic, CCD, restitution 1.0), generated fallback brick grids, `rebuild_playfield()`
- `levels.rs` — level rosters (`LEVELS` / `LEVELS_2P`), scene loading, brick-name/tag parsing (see below)
- `gameplay/` — `mod.rs` (frame orchestration + pause gate), `paddles.rs` (input + offset-based bounce aim), `balls.rs` (serve glue, launch, velocity maintenance, loss), `bricks.rs` (hit resolution + `brick_bounce_velocity`), `flow.rs` (state transitions, win detection, visibility)
- `power_ups.rs` — what pickups DO (multiball / wrecking / insiculous); tracking mechanics come from the engine's `Pickups<K>` / `EffectTimer`
- `menu.rs` — menu input handlers + `start_game()` (the match-start reset)
- `drawing.rs` — all UI (MenuPanel menus, HUD, game-over panel, pause overlay)
- `effects.rs` — particle burst configs; `chaos_theme.rs` — Normal-mode palette overrides on the engine's `ChaosTheme`
- `achievements.rs` — definitions, registration, win-unlock logic

**The game steps physics itself.** `BreakoutGame` owns a `PhysicsSystem` (`PhysicsConfig::top_down()`, no gravity) and calls `self.physics.update(&mut ctx.world, ctx.delta_time)` inside `update_gameplay()`. Collision events are drained ONCE per frame via `take_collision_events()` into an owned `Vec<CollisionData>`, and every consumer (paddle hits, brick hits, pickup catches, missed pickups, ball loss) reads that shared slice — a second take in the same frame would return empty. The pause gate sits above the physics step: while `PauseMenu` is active the frame ends early (no physics, no input, grid re-emitted with dt 0 so the frozen scene stays visible).

**THE footgun — destroy-on-contact cancels rapier's impulse.** Bricks are static bodies destroyed the frame a ball contact starts. Destroying the body cancels rapier's contact impulse, so on corner hits or in the gap between two bricks (horizontal contact normal) nothing pushes the ball back — it would plough straight through the grid. The fix is `brick_bounce_velocity()` in `gameplay/bricks.rs`: the game supplies the reflection itself, pushing the ball's velocity away from the destroyed brick on the dominant contact axis (normalized by brick half-extents; ties go vertical). It is direction-agnostic (handles balls arriving downward off the top paddle) and idempotent (if rapier already reflected, it changes nothing). Armored bricks that merely take damage SURVIVE the contact, so rapier's natural reflection lands and no correction is applied. Regression tests for this live in `gameplay_tests.rs` and in the engine at `crates/physics/tests/ball_brick_bounce.rs`.

**Level scene system.** Brick layouts are authored as engine scene RON files in `assets/scenes/` — `level{1..4}.scene.ron` (solo) and `level{1..4}_2p.scene.ron` (co-op middle-band layouts), one per chaos mode, rostered in `levels.rs` (`LEVELS` / `LEVELS_2P` consts; chaos mode is a property of the level, not a separate menu). On every `start_game()` the selected scene is loaded fresh via `SceneLoader::load_from_file` (raw filesystem path — it does NOT go through `asset_base_path`, so `level_scene_path()` anchors it explicitly) and instantiated; missing/broken/empty scenes warn and fall back to the generated grid in `spawning.rs`, so the worst case is the classic layout, never a brickless game. Conventions inside the scenes:

- Entity names `brick_r{row}_c{col}` — every entity named `brick*` becomes a `Brick`; the row digit drives the score payout (`brick_value_from_name`, graceful minimum for renamed bricks)
- `EntityTag` with `+`-joined tokens: `armored{N}` (2..=9 hits) and `drop_multiball` / `drop_wrecking` / `drop_insiculous` (pickup drops). Unknown tokens warn and are skipped
- Particle-burst color is read from the entity's live `Sprite`, so bricks retinted in the editor keep matching effects
- Prefabs in the RON are authoring sugar; the editor's save pipeline flattens them

**Co-op (2P) restructures the playfield.** `rebuild_playfield()` (spawning.rs) tears down and respawns walls/sensors/paddles every match start: solo keeps the classic solid top wall; co-op REMOVES it (destroying the collider matters — a hidden static wall would still block) and adds a top loss sensor plus P2's paddle at `PADDLE_TOP_Y`. Score and lives are shared; the serve goes to whichever side lost the last ball (`serve_side_after_loss`). Paddle bounces are side-aware (`paddle_bounce_direction_for` — a paddle always returns the ball toward the field). Input: P1 = bottom paddle + mouse, P2 = top; solo's lone paddle listens to BOTH player slots so WASD, arrows, and either pad all work.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why sprite scales are `size / 80.0` (see `PADDLE_SCALE`, `BALL_SCALE`, and the `(0.875, 0.3)` brick scales in the scene files).
- Collider shapes use **absolute pixels** and IGNORE `Transform2D.scale` entirely. Sprites and colliders are sized through different paths, so they can silently diverge — the scene files keep `scale (0.875, 0.3) ↔ half_extents (35.0, 12.0)` in sync by hand. `F1` in-game (or `C` in the editor) overlays collider outlines to check; `levels_tests.rs` asserts every shipped level's colliders match the brick dimensions.

**Chaos modes** are bound to levels: CLASSIC=Normal, THE VAULT=Insane (global `speed_mult` grows `INSANE_SPEED_GAIN` per paddle hit), PINATA=Ridiculous (two-ball serves), THE GAUNTLET=Insiculous (both). `ctx.chaos_mode` is mirrored from the level pick so engine readers agree. `chaos_theme.rs` overrides only the Normal palette; themes apply via `apply_theme()` at match start.

**Power-ups**: the insiculous trio — Multiball (+1 ball, cap `MAX_EXTRA_BALLS`), Wrecking (10s one-hit-kill, refresh-not-stack, red-hot ball visuals), Insiculous (both). Dropped by `drop_*`-tagged bricks as falling sensor capsules, caught with either paddle. Drop-bricks pulse their emissive channel (`pulse_drop_bricks`); armor damage dims the color channel — the two never fight.

**Where the ~47 tests live** (all headless, `cargo test`):
- `src/gameplay_tests.rs` (19) — bounce math, `brick_bounce_velocity` corner/gap cases, armored-brick physics integration, min-vertical enforcement, serve sides
- `src/levels_tests.rs` (14) — every shipped scene parses, bricks valid, fits the playfield, colliders match sprite dimensions
- `src/spawning.rs` (6) — grid geometry, payouts, co-op playfield swap
- `src/achievements.rs` (5), `src/power_ups.rs` (3)

**Paths:** assets and saves resolve through `engine_core::game_root!()` (exe directory if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Achievements persist to `saves/breakout_achievements.json`, input bindings to `saves/input_settings.json`.

**Editor naming:** spawned entities that appear in the hierarchy get `Name` components (e.g. pickups: "Pickup (Wrecking Ball)") — keep this when adding new entities.

## The Deion Re-skin (Phase G): The Food Pyramid

Planned identity for this game under the Deion pivot (see the engine's `PROJECT_ROADMAP.md`). **The game is still neon today** — nothing below is implemented; it is the agreed theme direction for Phase G.

**Level select becomes a food pyramid** — the 1992 USDA food pyramid silhouette, with Jesse's ordering:

- **Base — Fruits & Veggies**: L1 Fruits, L2 Veggies
- **Layer 2 — Bread/Grains**: L3
- **Layer 3 — Dairy (L4) and Meats/Proteins (L5), side by side**
- **Top — Sweets & Fats**: L6, the finale

**Progression gates**: clear L1 + L2 → unlock L3; clearing L3 opens a CHOICE of L4 or L5; L6 is the finale. Whether BOTH L4 and L5 must be cleared before L6 (or either one suffices) is an open question.

**Per-level brick themes**: fruit bricks, veggie bricks, baguette/grain bricks, cheese-wheel bricks, steak/protein bricks, donut/candy bricks. Proposal: the Sweets & Fats finale is **Dr. Maxwell's territory** — his devil's-food fortress relocates to the pyramid top.

**Power-ups re-theme per level where a fun fit exists** — an OPEN design space, deliberately loose; don't force a mapping where none is fun.

**Characters**: the paddle becomes a **tong character** — living kitchen tongs with a face, whose rounded grip doubles as the rounded paddle surface (art/design shared with Pong's "Tong" re-skin). The ball is Deion himself; the wrecking-ball mode stays as his frozen form (matches his universal Ridiculous ice form).

**New scope vs today**: the pyramid level-select screen and unlock persistence don't exist in the current linear 4-level roster — **design TBD at re-skin time**. Nothing is committed beyond the theme; today's `LEVELS`/`LEVELS_2P` arrays, `LevelSelect` state, and chaos-mode-per-level binding are what actually ship.

**Asset rules (settled, engine-wide):**
- Style SSOT: `deion_assets/DEION_STYLE.md` via the root symlink (the symlink assumes the standard side-by-side checkout — the same requirement the Cargo path dependency already imposes).
- Metrics: 16px base cell, nearest filtering, 5× integer scale to `RENDER_UNIT = 80`. Never fake footprints via `Transform2D.scale` (colliders ignore it).
- Runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/` (F2 tooling, not yet built) — never symlink art in, never hand-copy.
- AI art is quarantined: `ai_` prefix, lives only in `deion_assets/ai/`, ships in FREE web builds only, never in paid/marketplace builds (tiered rule, DEION_STYLE.md §6, Aug 19 2026). `deion_assets/scripts/check_no_ai_assets.sh` must pass on any paid release's asset tree.
- Sheet clip names (`.sheet.ron` sidecars) are the stable API between art and code.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/adversarial-review/`; prompt templates in `prompts/`.
- Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation (a PostToolUse hook on ExitPlanMode reminds about this).
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` (PreToolUse hook on Bash): prefix with `ADV_REVIEWED=1` only after a code-mode review was adjudicated with the user, or the user explicitly skipped review.
- `review/` is gitignored transients (keep `review/.gitkeep`).
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies of `../../insiculous_2d/scripts/*` — re-copy when the engine master changes.
