# The Food Pyramid (Insiculous Breakout)

Breakout in the Deion world, built on the [insiculous_2d](../../insiculous_2d)
engine: Tong's tong as the paddle, Deion the water ball as the ball, a wall of
six food tiers on a kitchen counter, a deforming grid over it, scene-authored
levels, candy power-ups, achievements, 2-player co-op, and the engine's
signature chaos modes.

## Running

The game depends on the engine by path (`../../insiculous_2d`), so keep both
checkouts side by side:

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's visual editor
cargo test                    # 74 headless tests
```

Assets and saves resolve relative to the executable (falling back to the crate
directory), so `cargo run` works from any working directory. Achievements
persist to `saves/breakout_achievements.json`; input bindings to
`saves/input_settings.json`.

## How to Play

Clear every brick. You have 3 lives; one is spent each time every live ball
is gone. Top rows pay more points than bottom rows, and where the ball
strikes the paddle controls the bounce — center returns it straight up,
edges deflect it up to 60°. Special bricks take multiple hits (armored) or
drop candies: **Multiball** (extra ball), **Wrecking** (10s of one-hit-kill
frozen Deion), or **Insiculous** (both at once) — catch the falling candy
with your tong.

| Input | Action |
|-------|--------|
| `←`/`→` or `A`/`D`, d-pad, or stick | Move paddle |
| Mouse | Move paddle (P1 only; takes over when the mouse moves) |
| `Space` / `Enter` / Left click / pad (A) | Launch ball |
| `Escape` / pad Start | Pause (Resume / Restart / Quit to Title / Exit) |
| `F1` | Toggle collider debug overlay |

**Menus** — `W`/`S` or `↑`/`↓` (or d-pad) to navigate, `Space`/`Enter`/(A)
to confirm, `Escape` to go back.

**2-Player Co-op** — P2 (arrows/Enter or gamepad 1) guards a second paddle
at the TOP edge: the top wall is gone, the brick band moves to the middle,
and score/lives are shared. Whoever lost the last ball serves the next one.

## Levels & Chaos Modes

Levels are authored as engine scene files (`assets/scenes/level*.scene.ron`,
editable in the visual editor) with dedicated `_2p` layouts for co-op. Each
level is bound to a chaos mode — picking the level picks the chaos:

| Level | Mode | Effect |
|-------|------|--------|
| CLASSIC | Normal | The classic wall, a taste of armor |
| THE VAULT | Insane | A fortress of foil; ball speeds up per paddle hit |
| PINATA | Ridiculous | Rains power-ups; every serve launches two balls |
| THE GAUNTLET | Insiculous | Everything at once |

## What This Game Demonstrates

The second entry in the engine's 20-games challenge (after Pong). New
patterns exercised here:

- **Scene-authored levels** — brick layouts live in editor-editable RON
  scenes; names (`brick_r{row}_c{col}`) drive scoring and `EntityTag`
  tokens (`armored{N}`, `drop_*`) drive behavior, with a generated-grid
  fallback if a scene is missing.
- **Dynamic entity despawning from collision events** — bricks are static
  bodies destroyed the frame a ball touches them, with score/combo payout
  (and a game-side bounce fix, since destroying a body cancels rapier's
  contact impulse).
- **Lives system** — loss sensors + escape safety net spend a life when the
  last live ball is gone.
- **Offset-based paddle control** — gameplay overrides the physical
  reflection so the player aims the ball by where it lands on the paddle.
- **Falling power-up pickups** — engine `Pickups`/`EffectTimer` tracking,
  game-defined effects.
- **Co-op that restructures the playfield** — 2P removes the top wall and
  mirrors the paddle/sensor pair.
- **Mouse + keyboard + gamepad on one control** — whichever moved last wins.

## Editor Mode

`cargo run --features editor` opens the exact same game inside the engine's
scene editor — hierarchy, inspector with undo/redo, play/pause/stop
(`F5`/`Ctrl+P`/`Ctrl+Shift+P`), and the collider outline overlay (`C`). All
gameplay tuning constants live in `src/constants.rs`; entities are spawned
from those values in `src/spawning.rs`; brick layouts are edited directly in
the level scenes.

The same build runs in the browser at [beinsiculous.com/playground/breakout/](https://beinsiculous.com/playground/breakout/): the game inside the editor, layout only — the rules are compiled in and nothing you change there persists.

## Project Layout

```
src/
├── main.rs           # Game trait impl, window/config setup, editor wiring
├── constants.rs      # The sheets, the six foods, all tuning values, draw depths
├── types.rs          # BreakoutGame state, Sheets, GameState, Facing, clip names, Brick
├── spawning.rs       # Entity creation (paddles and tongs, ball, walls and strips,
│                     #   counter, backdrop, one-shots, fallback grid)
├── levels.rs         # Level rosters, scene loading, brick name/tag parsing
├── gameplay/         # Match loop: mod (orchestration+pause), paddles, balls,
│                     #   bricks (brick_bounce_velocity), flow
├── power_ups.rs      # What Multiball / Wrecking / Insiculous pickups do
├── menu.rs           # Title / level select / achievements input, start_game
├── effects.rs        # Particle bursts
├── chaos_theme.rs    # Per-chaos-mode color themes
├── achievements.rs   # Achievement definitions
├── drawing.rs        # UI drawing (menus, HUD, pause overlay)
├── gameplay_tests.rs # Gameplay rule + physics regression tests
├── levels_tests.rs   # Shipped-scene validation tests
├── flow_tests.rs     # The match through the engine's headless harness
└── test_support.rs   # Shared fixtures: synced sheets, the harness
assets/scenes/        # level1-4 + level1-4_2p scene RON files
assets/sprites/       # Synced art: sync.list and its PNG + .sheet.ron copies
scripts/regenerate_levels.py  # One-off generator of the eight scenes (--check)
```

## The Deion Pivot: The Food Pyramid

The engine-wide **Deion pivot** re-skins every game into the world of Deion
the Insiculous (see `deion_assets/DEION_STYLE.md` via the repo's symlink).
Breakout's re-skin **landed 2026-09-25** on today's four-level roster:

- **The wall is the pyramid, tier by tier**: donut on the top row, then
  steak, cheese wheel, baguette, broccoli, and watermelon at the base — the
  top row still pays the most. Armored bricks are foil-wrapped and the foil
  tears at their last hit.
- **Cast**: the paddle is **Tong's tong** lying on the counter, its mouth
  leading the way it moves, scowling when a life is lost; the ball is
  **Deion** as a water ball, frozen solid while the wrecking candy runs.
- **Power-ups** are wrapped candies that unwrap where they are caught.
- The game is played on Tong's kitchen counter, the same court and rails.

The next batch climbs the pyramid (`breakout#3`): the level select becomes
the 1992 USDA food pyramid, climbed from the base.

```
                      ▲
                     /6\        Sweets & Fats — THE FINALE
                    /___\         (Dr. Maxwell's devil's-food fortress?)
                   / 4|5 \      Dairy (L4) | Meats & Proteins (L5)
                  /___|___\       — side by side, your choice
                 /    3    \    Bread & Grains (L3)
                /___________\
               /   1  |  2   \  Fruits (L1) | Veggies (L2)
              /_______|_______\   — the base
```

- **Progression**: clear L1 + L2 to unlock L3; clearing L3 opens a choice
  of L4 or L5; L6 crowns the run.
- **Power-ups** may re-theme per level where a fun fit exists — an open
  design space, kept deliberately loose.
- The pyramid select screen and unlock persistence are new scope vs
  today's linear roster.

### Open questions

- Must BOTH L4 and L5 be cleared before L6, or does either one unlock the
  finale?
- Which per-level power-up themes actually earn their keep?
- How do the 2P co-op scenes map onto the pyramid?

Answered questions move up into the theme spec and get DELETED from this
list (live-docs convention).
