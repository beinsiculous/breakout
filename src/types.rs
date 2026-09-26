use engine_core::prelude::*;

use crate::constants::{Food, FOODS};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    LevelSelect { selection: u8 },
    Achievements,
    /// Ball rests on the paddle waiting for launch.
    Serving,
    Playing,
    GameOver { won: bool },
}

/// Solo classic, or two-paddle co-op (P1 bottom, P2 top, shared score/lives).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameMode {
    SinglePlayer,
    TwoPlayerCoop,
}

/// Which edge a paddle guards. Determines bounce direction (a paddle always
/// returns the ball toward the field) and who serves after a lost ball.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaddleSide {
    Bottom,
    Top,
}

impl PaddleSide {
    /// The slot this side's tong facing is kept in.
    pub(crate) fn index(self) -> usize {
        match self {
            PaddleSide::Bottom => 0,
            PaddleSide::Top => 1,
        }
    }
}

/// Which way a tong's mouth points as it lies on the counter. The clip suffix is
/// Tong's: `_up` is the pictured tong, and the quarter turn that lays it down points
/// its mouth left, so a tong moving left asks for `Up` and one moving right for
/// `Down` — the mouth leads, as it does in Tong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Facing {
    Up,
    Down,
}

impl Facing {
    /// The suffix this facing appends to a tong clip's name.
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Facing::Up => "up",
            Facing::Down => "down",
        }
    }

    /// The facing a tong's displacement this frame asks for, or `None` for a tong that
    /// did not move — a stop keeps the last facing.
    pub(crate) fn from_displacement(delta_x: f32) -> Option<Facing> {
        if delta_x < 0.0 {
            Some(Facing::Up)
        } else if delta_x > 0.0 {
            Some(Facing::Down)
        } else {
            None
        }
    }

    /// Split a tong state's name into its clip and its facing, or `None` for a name
    /// that carries no facing suffix.
    pub(crate) fn split(state: &str) -> Option<(&str, Facing)> {
        let (clip, facing) = state.rsplit_once('_')?;
        match facing {
            "up" => Some((clip, Facing::Up)),
            "down" => Some((clip, Facing::Down)),
            _ => None,
        }
    }
}

/// A tong clip's name for one facing — `closed` + `up` is `closed_up`. The names a
/// tong's machine states and its sheet's clips share, spelled in one place.
pub(crate) fn tong_state(clip: &str, facing: Facing) -> String {
    format!("{clip}_{}", facing.suffix())
}

/// What a tong's jaw is asked for. A player's ask is held until a device asks the other
/// way, so this is the jaw's intent rather than a per-frame pulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Jaw {
    Open,
    Closed,
}

impl Jaw {
    /// The clip a jaw resting in this position plays.
    pub(crate) fn clip(self) -> &'static str {
        match self {
            Jaw::Open => TONG_OPEN,
            Jaw::Closed => TONG_CLOSED,
        }
    }
}

/// One tong's control state, kept by `PaddleSide::index`: which way its mouth is asked
/// to point, what its jaw is asked for, and what the frame's gameplay read off it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TongControl {
    /// The facing the tong was last asked for. A tong mid-clip takes it at its next rest.
    pub(crate) facing: Facing,
    /// The jaw the player last asked for. Recorded whether or not the machine can act on
    /// it, and taken at the machine's next rest.
    pub(crate) jaw: Jaw,
    /// The launch's `opening`, still to land: it outranks the player's ask, and until
    /// the jaw rests open no ask is read.
    pub(crate) launch_opening: bool,
    /// Where the jaw axis leaned last frame — toward the field, away, or inside the dead
    /// zone. An axis asks only on the frame it leaves for a side, never again while held.
    pub(crate) axis_lean: Option<Jaw>,
    /// Whether the tong's animation was drawing a `closing` frame when this frame's
    /// collider was dressed: what makes a contact a chomp.
    pub(crate) drawing_closing: bool,
}

impl TongControl {
    const fn new(facing: Facing) -> Self {
        Self { facing, jaw: Jaw::Closed, launch_opening: false, axis_lean: None, drawing_closing: false }
    }
}

/// Both tongs' controls as a match starts them: each tong facing the way Tong draws it,
/// the left tong's `_up` and the right tong's `_down`, its jaw shut.
pub(crate) const DEFAULT_TONGS: [TongControl; 2] = [TongControl::new(Facing::Up), TongControl::new(Facing::Down)];

// --- the clip names -----------------------------------------------------------------
// The contract with the sheets' sidecars (`assets/sprites/*.sheet.ron`): a rename there
// is a rename here, or the machine warns and holds its ground.

pub(crate) const TONG_OPEN: &str = "open";
pub(crate) const TONG_CLOSING: &str = "closing";
pub(crate) const TONG_CLOSED: &str = "closed";
pub(crate) const TONG_OPENING: &str = "opening";
pub(crate) const TONG_SCORED_ON: &str = "scored_on";
pub(crate) const BALL_IDLE: &str = "idle";
pub(crate) const BALL_HURT: &str = "hurt";
pub(crate) const BRICK_INTACT: &str = "intact";
/// Played by the scenes' armored prefabs through `autoplay`, never by code; the levels
/// tests hold every armored brick to it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const BRICK_ARMORED: &str = "armored";
pub(crate) const BRICK_ARMOR_DAMAGED: &str = "armor_damaged";
pub(crate) const CANDY_IDLE: &str = "idle";
pub(crate) const CANDY_COLLECT: &str = "collect";

/// A live brick: its entity, the food it is made of, the score it pays out when
/// destroyed, remaining hits (armored bricks take several), and the candy it drops.
pub(crate) struct Brick {
    pub(crate) entity: EntityId,
    pub(crate) value: u32,
    pub(crate) food: Food,
    /// Hits left to destroy it (1 = plain brick).
    pub(crate) hits_left: u32,
    /// Candy dropped when destroyed, if any.
    pub(crate) drop: Option<PickupKind>,
}

/// Power-up candies dropped by special bricks (the insiculous trio: two
/// base powers plus one that grants both).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickupKind {
    /// Grants an extra ball.
    Multiball,
    /// All balls one-hit-kill any brick for a while.
    Wrecking,
    /// Both at once.
    Insiculous,
}

/// Every sheet the game draws with, loaded once in `init()`.
///
/// Each sheet's path, cell and measured anchor live in `constants.rs`; each PNG and its
/// `.sheet.ron` sidecar is a synced copy of the deion_assets master
/// (`assets/sprites/sync.list`), so no art here is hand-authored and none is loaded
/// from anywhere else.
pub(crate) struct Sheets {
    /// White 1x1 texture for the particle bursts.
    pub(crate) white: u32,
    /// The six food bricks, in `FOODS` order.
    pub(crate) foods: [SpriteSheet; 6],
    pub(crate) ball_water: SpriteSheet,
    pub(crate) ball_ice: SpriteSheet,
    pub(crate) candy_multiball: SpriteSheet,
    pub(crate) candy_wrecking: SpriteSheet,
    pub(crate) candy_insiculous: SpriteSheet,
    pub(crate) tong_left: SpriteSheet,
    pub(crate) tong_right: SpriteSheet,
    pub(crate) court: SpriteSheet,
    pub(crate) court_edge: SpriteSheet,
}

impl Sheets {
    /// The loaded sheet a food's bricks wear.
    pub(crate) fn food(&self, food: Food) -> &SpriteSheet {
        let index = FOODS
            .iter()
            .position(|spec| spec.food == food)
            .expect("every food has a row in FOODS");
        &self.foods[index]
    }
}

/// A sheet with no texture, one cell and no clips. `Sheets::default` holds these until
/// `init()` loads the real ones: the engine builds the game with `Default` and calls
/// `init` on the first frame, before any entity that could draw exists.
pub(crate) fn placeholder_sheet() -> SpriteSheet {
    SpriteSheet {
        texture: TextureHandle { id: 0 },
        grid: SheetGrid::new(1, 1),
        clips: Vec::new(),
        path: String::new(),
    }
}

impl Default for Sheets {
    fn default() -> Self {
        Self {
            white: 0,
            foods: std::array::from_fn(|_| placeholder_sheet()),
            ball_water: placeholder_sheet(),
            ball_ice: placeholder_sheet(),
            candy_multiball: placeholder_sheet(),
            candy_wrecking: placeholder_sheet(),
            candy_insiculous: placeholder_sheet(),
            tong_left: placeholder_sheet(),
            tong_right: placeholder_sheet(),
            court: placeholder_sheet(),
            court_edge: placeholder_sheet(),
        }
    }
}

pub struct BreakoutGame {
    pub(crate) physics: PhysicsSystem,
    pub(crate) sheets: Sheets,

    /// Player 1's paddle body: a kinematic capsule the size of the closed tong.
    pub(crate) paddle: Option<EntityId>,
    /// Player 2's paddle body guarding the top edge. Present only in co-op.
    pub(crate) paddle_top: Option<EntityId>,
    /// The tong drawn on Player 1's paddle: art only, placed on the body every frame.
    pub(crate) tong: Option<EntityId>,
    /// The tong drawn on Player 2's paddle. Present only in co-op.
    pub(crate) tong_top: Option<EntityId>,
    /// Each tong's facing and jaw, by `PaddleSide::index`.
    pub(crate) tongs: [TongControl; 2],
    /// Whether the right mouse button — Player 1's bite — was down last frame: the
    /// input reports a press, not a release, so the release is this going false.
    pub(crate) bite_button_was_down: bool,
    pub(crate) ball: Option<EntityId>,
    pub(crate) extra_balls: Vec<EntityId>,
    /// Balls a chomp sent out, running faster until their next plain paddle hit or
    /// their loss.
    pub(crate) chomped: Vec<EntityId>,
    pub(crate) bricks: Vec<Brick>,
    /// Index into the active roster (`levels::LEVELS` / `LEVELS_2P`) of the
    /// level being played. The scene is loaded fresh on every match start
    /// (missing file → generated grid).
    pub(crate) selected_level: usize,
    pub(crate) mode: GameMode,
    /// Which paddle serves the next ball: starts at the bottom, then flips
    /// to whichever edge the last ball was lost past (that player redeems).
    pub(crate) serving_side: PaddleSide,
    /// The walls' colliders — one continuous box each, drawn by `wall_strips`.
    pub(crate) walls: Vec<EntityId>,
    /// The counter edge strips drawn on the walls: art only, spawned and drained with
    /// the walls, so co-op's open top edge draws no rail.
    pub(crate) wall_strips: Vec<EntityId>,
    pub(crate) bottom_sensor: Option<EntityId>,
    /// Ball-loss sensor above the top edge. Present only in co-op (solo
    /// keeps the classic solid top wall).
    pub(crate) top_sensor: Option<EntityId>,
    /// The kitchen counter the game is played on, spawned once in `init()`.
    pub(crate) court: Option<EntityId>,
    /// The deforming grid drawn over the counter, spawned once in `init()`.
    pub(crate) backdrop: Option<EntityId>,
    /// Detached one-shots — a lost ball's splash, a caught candy's collect — that end
    /// themselves. A match start and a quit drain whatever is left.
    pub(crate) transient_visuals: Vec<EntityId>,

    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Falling candies currently in flight (engine-tracked).
    pub(crate) pickups: Pickups<PickupKind>,
    /// Wrecking-ball countdown; while active every ball one-hit-kills.
    pub(crate) wrecking: EffectTimer,

    /// Global ball speed multiplier. Insane mode grows it on every paddle
    /// hit; reset on life loss and at match start.
    pub(crate) speed_mult: f32,
    /// Bricks destroyed since the ball last touched the paddle.
    pub(crate) combo: u32,

    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
    /// Engine pause menu (Esc/Start toggles during a match).
    pub(crate) pause: PauseMenu,
}

impl Default for BreakoutGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::top_down()),
            sheets: Sheets::default(),
            paddle: None,
            paddle_top: None,
            tong: None,
            tong_top: None,
            tongs: DEFAULT_TONGS,
            bite_button_was_down: false,
            ball: None,
            extra_balls: Vec::new(),
            chomped: Vec::new(),
            bricks: Vec::new(),
            selected_level: 0,
            mode: GameMode::SinglePlayer,
            serving_side: PaddleSide::Bottom,
            walls: Vec::new(),
            wall_strips: Vec::new(),
            bottom_sensor: None,
            top_sensor: None,
            court: None,
            backdrop: None,
            transient_visuals: Vec::new(),
            score: 0,
            lives: crate::constants::STARTING_LIVES,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            pickups: Pickups::new(),
            wrecking: EffectTimer::default(),
            speed_mult: 1.0,
            combo: 0,
            debug_colliders: false,
            pause: PauseMenu::new(),
        }
    }
}
