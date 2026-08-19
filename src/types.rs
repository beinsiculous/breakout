use engine_core::prelude::*;

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

/// A live brick: its entity plus the score it pays out when destroyed,
/// remaining hits (armored bricks take several), and the pickup it drops.
pub(crate) struct Brick {
    pub(crate) entity: EntityId,
    pub(crate) value: u32,
    pub(crate) color: Vec4,
    /// Hits left to destroy it (1 = plain brick).
    pub(crate) hits_left: u32,
    /// Pickup dropped when destroyed, if any.
    pub(crate) drop: Option<PickupKind>,
}

/// Power-up pickups dropped by special bricks (the insiculous trio: two
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

pub struct BreakoutGame {
    pub(crate) physics: PhysicsSystem,

    pub(crate) paddle: Option<EntityId>,
    /// Player 2's paddle guarding the top edge. Present only in co-op.
    pub(crate) paddle_top: Option<EntityId>,
    pub(crate) ball: Option<EntityId>,
    pub(crate) extra_balls: Vec<EntityId>,
    pub(crate) bricks: Vec<Brick>,
    /// Index into the active roster (`levels::LEVELS` / `LEVELS_2P`) of the
    /// level being played. The scene is loaded fresh on every match start
    /// (missing file → generated grid).
    pub(crate) selected_level: usize,
    pub(crate) mode: GameMode,
    /// Which paddle serves the next ball: starts at the bottom, then flips
    /// to whichever edge the last ball was lost past (that player redeems).
    pub(crate) serving_side: PaddleSide,
    pub(crate) walls: Vec<EntityId>,
    pub(crate) bottom_sensor: Option<EntityId>,
    /// Ball-loss sensor above the top edge. Present only in co-op (solo
    /// keeps the classic solid top wall).
    pub(crate) top_sensor: Option<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture for paddle, bricks, walls, background, particles.
    pub(crate) tex_id: u32,
    /// PNG texture for the circular ball sprite.
    pub(crate) ball_tex_id: u32,

    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Falling power-up pickups currently in flight (engine-tracked).
    pub(crate) pickups: Pickups<PickupKind>,
    /// Wrecking-ball countdown; while active every ball one-hit-kills.
    pub(crate) wrecking: EffectTimer,

    /// Global ball speed multiplier. Insane mode grows it on every paddle
    /// hit; reset on life loss and at match start.
    pub(crate) speed_mult: f32,
    /// Bricks destroyed since the ball last touched the paddle.
    pub(crate) combo: u32,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the sprites.
    pub(crate) debug_colliders: bool,
    /// Engine pause menu (Esc/Start toggles during a match).
    pub(crate) pause: PauseMenu,
}

impl Default for BreakoutGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::top_down()),
            paddle: None,
            paddle_top: None,
            ball: None,
            extra_balls: Vec::new(),
            bricks: Vec::new(),
            selected_level: 0,
            mode: GameMode::SinglePlayer,
            serving_side: PaddleSide::Bottom,
            walls: Vec::new(),
            bottom_sensor: None,
            top_sensor: None,
            background: None,
            tex_id: 0,
            ball_tex_id: 0,
            score: 0,
            lives: crate::constants::STARTING_LIVES,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            pickups: Pickups::new(),
            wrecking: EffectTimer::default(),
            speed_mult: 1.0,
            combo: 0,
            grid: None,
            debug_colliders: false,
            pause: PauseMenu::new(),
        }
    }
}
