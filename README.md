# Insiculous Breakout

Neon Breakout built on the [insiculous_2d](../../insiculous_2d) engine — a
rainbow brick wall, bloom-heavy Geometry-Wars look, a spring-mass-deforming
grid background, scene-authored levels, power-ups, achievements, 2-player
co-op, and the engine's signature chaos modes.

## Running

The game depends on the engine by path (`../../insiculous_2d`), so keep both
checkouts side by side:

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's visual editor
cargo test                    # 47 headless tests
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
drop power-ups: **Multiball** (extra ball), **Wrecking** (10s of one-hit-kill
red-hot balls), or **Insiculous** (both at once) — catch the falling capsule
with your paddle.

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
| THE VAULT | Insane | Armored fortress; ball speeds up per paddle hit |
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
├── constants.rs      # All gameplay tuning values (sizes, speeds, layout)
├── types.rs          # BreakoutGame state, GameState, GameMode, Brick, PickupKind
├── spawning.rs       # Entity creation (paddles, ball, walls, sensors, fallback grid)
├── levels.rs         # Level rosters, scene loading, brick name/tag parsing
├── gameplay/         # Match loop: mod (orchestration+pause), paddles, balls,
│                     #   bricks (brick_bounce_velocity), flow
├── power_ups.rs      # What Multiball / Wrecking / Insiculous pickups do
├── menu.rs           # Title / level select / achievements input, start_game
├── effects.rs        # Deforming grid background, particle bursts
├── chaos_theme.rs    # Per-chaos-mode color themes
├── achievements.rs   # Achievement definitions
├── drawing.rs        # UI drawing (menus, HUD, pause overlay)
├── gameplay_tests.rs # Gameplay rule + physics regression tests
└── levels_tests.rs   # Shipped-scene validation tests
assets/scenes/        # level1-4 + level1-4_2p scene RON files
```

## The Deion Pivot: The Food Pyramid

The engine-wide **Deion pivot** re-skins every game into the world of Deion
the Insiculous (see `deion_assets/DEION_STYLE.md` via the repo's symlink).
Breakout's planned identity: **climb the food pyramid**. The game is still
neon today — this is the design note for the Phase G re-skin.

The level select becomes the 1992 USDA food pyramid, climbed from the base:

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
- **Brick themes per level**: fruit bricks, veggie bricks, baguette/grain
  bricks, cheese-wheel bricks, steak/protein bricks, donut/candy bricks.
- **Cast**: the paddle becomes a **tong character** — living kitchen tongs
  with a face, whose rounded grip is the rounded paddle surface (shared
  with Pong's re-skin). The ball is **Deion** himself; the wrecking-ball
  power stays as his frozen ice form.
- **Power-ups** re-theme per level where a fun fit exists — an open design
  space, kept deliberately loose.
- The pyramid select screen and unlock persistence are new scope vs
  today's linear roster — design lands at re-skin time; only the theme is
  committed.

### Open questions

- Must BOTH L4 and L5 be cleared before L6, or does either one unlock the
  finale?
- Which per-level power-up themes actually earn their keep?
- Brick durability mapping — e.g. foil-wrapped armored bricks?
- How do the 2P co-op scenes map onto the pyramid?

Answered questions move up into the theme spec and get DELETED from this
list (live-docs convention).
