# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run all 74 tests (see "Where the tests live" below)
cargo test <test_name>        # run a single test
cargo clippy                  # must stay clean
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root points to the shared art repo (`../deion_assets`) — read-only reference, never write through it.

## Architecture

This is a single-crate game (`insiculous_breakout`) built on the in-house `insiculous_2d` ECS engine. `BreakoutGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` loads the sheets and builds the persistent scaffolding (the counter, the backdrop grid, the solo playfield), `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`, and at `/playground/breakout/` in the browser (the same feature, built by the engine's `build_wasm.sh --kind editor` and served from the site); no game code changes between the modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: the menu states (`TitleScreen`, `LevelSelect`, `Achievements`) dispatch to handlers in `menu.rs`; everything else (`Serving`, `Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{paddles,balls,bricks,flow}.rs`. Flow is Title → Level Select (picking a level also picks its chaos mode) → Serving ↔ Playing → GameOver. `update_entity_visibility()` (flow.rs) hides all gameplay sprites while a menu state is active — entities persist, only `Sprite.visible` flips; the counter and the grid over it stay up, as Tong's court does.

**Module layout:**

- `main.rs` — `Game` impl, `GameConfig` (800×600, 60 FPS, asset/save paths anchored via `game_root!()`), editor wiring
- `constants.rs` — the sheets block (one `SheetSpec` per sheet), the `FOODS` table (`Food`, its sheet, its burst colour, its depth) and `food_for_row`, and ALL gameplay tuning (sizes derived from the measured art, speeds, brick grid, pickup timing, grid impulses, burst colours, the draw depths). Values tuned in the editor inspector must be copied back here to persist
- `types.rs` — `BreakoutGame` state, `Sheets`, `GameState`, `GameMode` (SinglePlayer / TwoPlayerCoop), `PaddleSide`, `Facing` and `tong_state`, the clip names (the contract with the sidecars), `Brick`, `PickupKind`
- `spawning.rs` — the clip machines (tong, ball, candy, one-shot), paddle bodies (kinematic capsule-x colliders) and the tongs drawn on them, walls and their edge strips, loss sensors, the counter, the backdrop, ball (dynamic, CCD, restitution 1.0), one-shot effects, generated fallback brick grids, `rebuild_playfield()`
- `levels.rs` — level rosters (`LEVELS` / `LEVELS_2P`), scene loading, brick-name/tag parsing (see below)
- `gameplay/` — `mod.rs` (frame orchestration + pause gate + the F1 collider overlay), `paddles.rs` (input + offset-based bounce aim, the tongs' placement and facing), `balls.rs` (serve glue, launch, velocity maintenance, loss, the splash and the scowl), `bricks.rs` (hit resolution + `brick_bounce_velocity`, the foil tearing), `flow.rs` (state transitions, win detection, visibility, the backdrop's theme, the one-shots' drain)
- `power_ups.rs` — what candies DO (multiball / wrecking / insiculous), the collect one-shot, the wrecking sheet swap; tracking mechanics come from the engine's `Pickups<K>` / `EffectTimer`
- `menu.rs` — menu input handlers + `start_game()` (the match-start reset)
- `drawing.rs` — all UI (MenuPanel menus, HUD, game-over panel, pause overlay)
- `effects.rs` — particle burst configs; `chaos_theme.rs` — the Normal-mode palette over the engine's `ChaosTheme` (the grid colour, and the menu panels' background and border)
- `scripts/regenerate_levels.py` — the one-off generator that wrote the eight scenes at the food-brick pitch; `--check` proves they still equal a fresh generation
- `achievements.rs` — definitions, registration (through `register_achievements()`, which the engine calls before the window opens; `cargo run -- --achievements-manifest <path>` exports the list), win-unlock logic

**The game steps physics itself.** `BreakoutGame` owns a `PhysicsSystem` (`PhysicsConfig::top_down()`, no gravity) and calls `self.physics.update(&mut ctx.world, ctx.delta_time)` inside `update_gameplay()`. Collision events are drained ONCE per frame via `take_collision_events()` into an owned `Vec<CollisionData>`, and every consumer (paddle hits, brick hits, pickup catches, missed pickups, ball loss) reads that shared slice — a second take in the same frame would return empty. The pause gate sits above the physics step: while `PauseMenu` is active the frame ends early (no physics, no input; `ctx.time_scale` freezes the engine's backdrop grid and particles, and only the F1 overlay is still drawn by the game).

**THE footgun — destroy-on-contact cancels rapier's impulse.** Bricks are static bodies destroyed the frame a ball contact starts. Destroying the body cancels rapier's contact impulse, so on corner hits or in the gap between two bricks (horizontal contact normal) nothing pushes the ball back — it would plough straight through the grid. The fix is `brick_bounce_velocity()` in `gameplay/bricks.rs`: the game supplies the reflection itself, pushing the ball's velocity away from the destroyed brick on the dominant contact axis (normalized by brick half-extents; ties go vertical). It is direction-agnostic (handles balls arriving downward off the top paddle) and idempotent (if rapier already reflected, it changes nothing). Armored bricks that merely take damage SURVIVE the contact, so rapier's natural reflection lands and no correction is applied. Regression tests for this live in `gameplay_tests.rs` and in the engine at `crates/physics/tests/ball_brick_bounce.rs`.

**Level scene system.** Brick layouts are authored as engine scene RON files in `assets/scenes/` — `level{1..4}.scene.ron` (solo) and `level{1..4}_2p.scene.ron` (co-op middle-band layouts), one per chaos mode, rostered in `levels.rs` (`LEVELS` / `LEVELS_2P` consts; chaos mode is a property of the level, not a separate menu). On every `start_game()` the selected scene is loaded fresh via `SceneLoader::load_from_file` (raw filesystem path — it does NOT go through `asset_base_path`, so `level_scene_path()` anchors it explicitly) and instantiated; missing/broken/empty scenes warn and fall back to the generated grid in `spawning.rs`, so the worst case is the classic layout, never a brickless game. Conventions inside the scenes:

- Entity names `brick_r{row}_c{col}` — every entity named `brick*` becomes a `Brick`; the row digit drives the score payout (`brick_value_from_name`, graceful minimum for renamed bricks)
- `EntityTag` with `+`-joined tokens: `armored{N}` (2..=9 hits) and `drop_multiball` / `drop_wrecking` / `drop_insiculous` (pickup drops). Unknown tokens warn and are skipped
- A brick's food is the sheet its `SpriteAnimation` plays (`levels::brick_food`), falling back to its row's tier with a warning; the burst colour is the food's. Armored bricks autoplay `armored` from their prefab — the scene is the one mechanism for the starting frame
- Prefabs in the RON are authoring sugar; the editor's save pipeline flattens them

**Co-op (2P) restructures the playfield.** `rebuild_playfield()` (spawning.rs) tears down and respawns walls/sensors/paddles every match start: solo keeps the classic solid top wall; co-op REMOVES it (destroying the collider matters — a hidden static wall would still block) and adds a top loss sensor plus P2's paddle at `PADDLE_TOP_Y`. Score and lives are shared; the serve goes to whichever side lost the last ball (`serve_side_after_loss`). Paddle bounces are side-aware (`paddle_bounce_direction_for` — a paddle always returns the ball toward the field). Input: P1 = bottom paddle + mouse, P2 = top; solo's lone paddle listens to BOTH player slots so WASD, arrows, and either pad all work.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why sprite scales are `size / 80.0` (`SheetSpec::scale()`, and the `(0.8, 0.4)` brick scales in the scene files).
- Collider shapes use **absolute pixels** and IGNORE `Transform2D.scale` entirely. Sprites and colliders are sized through different paths, so they can silently diverge — the scene files keep `scale (0.8, 0.4) ↔ half_extents (32.0, 16.0)` in sync, and the generator writes both. `F1` in-game (or `C` in the editor) overlays collider outlines to check; `levels_tests.rs` asserts every shipped level's colliders match the brick dimensions.

**Chaos modes** are bound to levels: CLASSIC=Normal, THE VAULT=Insane (global `speed_mult` grows `INSANE_SPEED_GAIN` per paddle hit), PINATA=Ridiculous (two-ball serves), THE GAUNTLET=Insiculous (both). `ctx.chaos_mode` is mirrored from the level pick so engine readers agree. `chaos_theme.rs` overrides only the Normal palette; `apply_backdrop_theme()` gives the backdrop the picked mode's colour at match start — the level pick is the only thing that changes the mode.

**Power-ups**: the insiculous trio — Multiball (+1 ball, cap `MAX_EXTRA_BALLS`), Wrecking (10s one-hit-kill, refresh-not-stack, Deion frozen), Insiculous (both). Dropped by `drop_*`-tagged bricks as falling candies — sensors the size of the candy's drawn body — caught with either tong; a catch unwraps where the candy was. Drop-bricks pulse their emissive channel (`pulse_drop_bricks`); armor damage plays the foil frames — the two never fight.

**Where the 74 tests live** (all headless, `cargo test`):
- `src/flow_tests.rs` (11) — the match through the engine's `GameHarness` over the synced art: the tong on its unturned paddle and its facing, a fresh serve drawn on the serving paddle, the scowl only for the last ball, the splash from the sheet the ball wore, restart and quit leaving nothing, the wrecking sheet swap, the collect where the candy was (two kinds in one frame included), the backdrop's colour, every level's bricks on the loaded sheets
- `src/gameplay_tests.rs` (19) — bounce math, `brick_bounce_velocity` corner/gap cases, armored-brick physics integration, candy catch and miss, min-vertical enforcement, serve sides
- `src/levels_tests.rs` (19) — every shipped scene parses, bricks valid, fits the playfield, every collider is the cell, row N is row N's food, each scene's brick count and tag multiset, armor autoplays foil, one depth per food sheet
- `src/constants.rs` (8) — every sheet read back against its synced sidecar (grids, clip names, one-shots play once), the specs and the derived numbers, the depth ladder
- `src/spawning.rs` (8) — grid geometry, payouts, co-op playfield swap, no tong or strip outliving a rebuild, the counter
- `src/achievements.rs` (5), `src/power_ups.rs` (2), `src/gameplay/paddles.rs` (2 — the tong's facing waits out a scowl)

`src/test_support.rs` holds the shared fixtures: the sheets read back through the engine's GPU-free load path, and the whole game driven through the engine's `GameHarness`.

**Paths:** assets and saves resolve through `engine_core::game_root!()` (exe directory if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Achievements persist to `saves/breakout_achievements.json`, input bindings to `saves/input_settings.json`.

**Editor naming:** spawned entities that appear in the hierarchy get `Name` components — "Paddle P1" (the body) and "Tong P1" (its art), "Deion", "Deion (extra)", "Candy (Wrecking Ball)", "Splash", "Court", "Wall left 0", "Grid Backdrop"; the scenes' `brick_r{row}_c{col}` names are frozen (the row parse hangs off them) — keep this when adding new entities.

## The Food Pyramid (landed 2026-09-25)

Breakout is the **third of the six Phase G Deion re-skins**, after Tong and Chicken Coop. In the
re-skinned build the paddle is **Tong's tong** lying on the counter, the ball is **Deion** as a 16 px
water ball, the wall is **six tiers of food** — donut, steak, cheese wheel, baguette, broccoli,
watermelon from the top row down, the pyramid from its peak to its base — armored bricks are
**foil-wrapped**, and the power-ups are **wrapped candies**. The game is played on Tong's kitchen
counter. In-game it is **THE FOOD PYRAMID** (the title menu and the window title). The crate is still
`insiculous_breakout`, `BreakoutGame` is still the game type, the site still lists it as *Insiculous
Breakout* and its slug is still `breakout` — those are frozen with the ten achievement ids and the
save keys.

- **Art enters only through the sync.** `assets/sprites/sync.list` pins the `deion_assets` commit
  and lists the fifteen sources — the six bricks, the three candies, the two balls, both tongs, the
  court tile and the court edge; `python3 deion_assets/scripts/sync_sprites.py .` copies each PNG and
  its `.sheet.ron` sidecar in, and `--check` hashes the copies against the pinned blobs. Never
  hand-copy or hand-edit a file there — fix the master in `deion_assets` and re-sync. The working
  set's `scripts/check-sprite-sync.sh` runs that check over every game.
- **The sheets block** in `src/constants.rs` names each sheet once as the engine's `SheetSpec`: its
  path, its cell, and the opaque box of the reference frame, **measured** from the synced PNG
  (bottom-right one past the last opaque pixel). The tongs, the court and the edge are the bytes
  Tong measured.

  | subject | cell | box | what it drives |
  |---|---|---|---|
  | six food bricks | 64×32 | **the whole cell** (stated) | collider, sprite, pitch 68 × 36 |
  | tong, closed | 64×96 | 22×78 at (21, 9), centred | the paddle capsule `capsule_x(78, 11)` |
  | Deion, water and frozen | 16×16 | the body, mohawk excluded, equal on both | the ball's circle and anchor |
  | multiball / wrecking / insiculous candy | 48×48 | 39×21 / 39×29 / 39×27 (`idle` union) | each candy's sensor |
  | court tile / court edge | 64×64 / 64×16 | the whole cell | the counter, and the 16 px walls |

- **Every brick's collider is its cell, not its food** — the stated exception to "the drawn thing
  covers the real thing": the smaller foods sit inside their cells with margins (the cheese wheel
  7 px a side, the baguette 7 px above and 6 below), and a ball bounces off that margin. Each food's
  own box would open gaps a 16 px ball threads. The polish batch may draw the foods closer to their
  cells' edges; no code changes when it does.
- **The tong is art beside its body.** The paddle is a kinematic capsule that never turns — physics
  turns a collider with its body, and the F1 overlay draws outlines unrotated — and the tong is a
  sprite-only entity turned a quarter (`TONG_ROTATION`), placed on its paddle after every physics
  step and spawned and drained with it in `rebuild_playfield`. The mouth points the way the paddle
  last moved (`_up` left, `_down` right); a facing change lands only at rest. The tong rests
  `closed` — its arms are parallel, so the capsule is exactly the art — and scowls (`scored_on`)
  when the last live ball is lost, never while one is in flight, because the scowl's arms shudder
  wider than the capsule.
- **One depth per sheet, never per entity.** The batcher draws a texture's batch whole, ordered by
  its lowest depth, and the sprite pipeline writes depth for transparent texels too — so a sheet with
  entities under and over another sheet's depth would punch holes in it. The ladder in
  `constants.rs` nests the counter, the strips, the six food sheets, the tongs, the two ball sheets
  (a splash at its ball's) and the three candy sheets (a collect at its candy's); a renderer that
  discards transparent texels is `insiculous_2d#154`.
- **1×, pixel-snapped** (`GameConfig::with_pixel_snap(true)`): one art pixel per window pixel at
  `RENDER_UNIT = 80`, nearest filtering, no faked scale. Every art sprite is drawn white with
  emissive 0; the chaos themes colour only the backdrop grid, the particles and the menus.
- **The wall is regenerated, not hand-placed.** `scripts/regenerate_levels.py` wrote the eight
  scenes from their own brick names and tags at the new pitch, one prefab per food and an armored
  twin; it re-parses its output and asserts the same names, cells and tags, and `levels_tests.rs`
  holds the scenes to `constants.rs`.
- **Next: the pyramid roster** (`breakout#3`) — the six-level pyramid climbed from the base, its
  level-select screen, its gates and the unlock save, Jesse's tier ordering and the
  both-or-either question for the finale. It is a batch of its own; today's four-level roster is
  what ships.
- **Style SSOT:** `deion_assets/DEION_STYLE.md` via the `deion_assets -> ../../deion_assets`
  symlink. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) — tiered ship rule
  (DEION_STYLE.md §6, Aug 19 2026): may ship in FREE web builds, never in paid/marketplace builds.
  **The Food Pyramid is free-tier only until Jesse's cleanup pass**: the synced copies keep their
  `ai_` prefix, so `deion_assets/scripts/check_no_ai_assets.sh assets` fails on a paid build, as it
  must. The exit is a hand-cleaned master outside `deion_assets/ai/`, re-exported without the
  prefix, with the `sync.list` line moved to it.
- Sheet clip names (`.sheet.ron` sidecars) are the stable API between art and code; `types.rs`
  spells each one once.

## Work tracking

Open work lives on the **Studio Board** (https://github.com/orgs/beinsiculous/projects/1)
as issues in this repo. **Always pass `-R beinsiculous/breakout`** — a bare `gh` command
resolves against the session's working directory, which is often the working-set root, so
it lists and files against the wrong repository.

```sh
gh issue list -R beinsiculous/breakout
gh api repos/beinsiculous/breakout/milestones --jq '.[] | "\(.title): \(.description)"'
```

Issues are grouped into **sprint milestones**; each description records the batch's
internal order and its gates. Take the next unblocked issue in a sprint, not an arbitrary
one. Claim by assigning yourself; close with `fixes beinsiculous/breakout#N` in the commit.

**Unfinished work becomes an issue.** Anything you don't finish — work you deferred, debt
you created, a follow-up you spotted — is filed before you report done. Never buried in a
doc, never left as a bare `TODO:`, never dropped. The `file-issue` skill carries the shape;
`sprint-planning` groups issues into shippable batches.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/adversarial-review/`; prompt templates in `prompts/`.
- Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation (a PostToolUse hook on ExitPlanMode reminds about this).
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` (PreToolUse hook on Bash): prefix with `ADV_REVIEWED=1` only after a code-mode review was adjudicated with the user, or the user explicitly skipped review.
- `review/` is gitignored transients (keep `review/.gitkeep`).
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies — the canonical ones live in the working-set root, not in `insiculous_2d`. Never edit a copy: fix the root's and re-copy, and `scripts/check-skill-parity.sh` there reports any repo that drifted.
