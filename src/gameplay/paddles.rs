//! Paddle control (per-player input, mouse takeover), paddle bounces, and the tongs
//! drawn on the paddles: where each lies, which way its mouth points, what its jaw is
//! asked for, and the collider the pose it draws gives its paddle.
//!
//! The jaw is Tong's rule turned a quarter: pushing toward the field closes, pulling
//! away opens, and Player 1's right mouse button bites on its press and opens on its
//! release. Every device asks only on its edge — the axis the frame it leaves the dead
//! zone for a side, the button the frame it goes down or comes up — so whichever asked
//! last wins, and within one frame the button outranks the axis.
//!
//! The paddle body wears the pose of the frame its tong's animation is drawing. The
//! engine advances animations in the frame tail, after the game's update, so that is
//! the frame the player saw last: a lag of one frame, a sixth of the shortest pose.
//! Everything that classifies a contact — a chomp — reads the same drawn frame, never
//! the machine's state, which may already have moved on.

use engine_core::prelude::*;
use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::effects;
use crate::jaw::{drawn_pose, tong_body_collider, JawPose, JAW_AXIS_DEAD_ZONE};
use crate::types::*;
use super::{entity_position, entity_x, ripple_grid, set_clip_state};

/// Direction a ball leaves the bottom paddle, from where it struck.
///
/// `offset_frac` is the hit position relative to the paddle center in
/// half-widths: 0 = dead center, ±1 = the very edge. Center hits return
/// the ball straight up; edge hits deflect up to `PADDLE_MAX_BOUNCE_ANGLE`.
pub(crate) fn paddle_bounce_direction(offset_frac: f32) -> Vec2 {
    let angle = offset_frac.clamp(-1.0, 1.0) * PADDLE_MAX_BOUNCE_ANGLE;
    Vec2::new(angle.sin(), angle.cos())
}

/// Side-aware bounce: a paddle always returns the ball toward the field —
/// up off the bottom paddle, DOWN off the top one. Without the top-side
/// flip, the top paddle would fire balls into itself/the top sensor.
pub(crate) fn paddle_bounce_direction_for(offset_frac: f32, side: PaddleSide) -> Vec2 {
    let dir = paddle_bounce_direction(offset_frac);
    match side {
        PaddleSide::Bottom => dir,
        PaddleSide::Top => Vec2::new(dir.x, -dir.y),
    }
}

/// The jaw a stick leaning `toward_field` asks for, or `None` inside the dead zone.
pub(crate) fn jaw_lean(toward_field: f32) -> Option<Jaw> {
    if toward_field > JAW_AXIS_DEAD_ZONE {
        Some(Jaw::Closed)
    } else if toward_field < -JAW_AXIS_DEAD_ZONE {
        Some(Jaw::Open)
    } else {
        None
    }
}

/// The state a tong's machine should move to this frame to reach `jaw` in `facing`, or
/// `None` to leave it.
///
/// Only a resting tong moves. A `closing`, `opening` or scowl plays out first, so a
/// press released mid-bite does not turn it into an `opening` halfway through and the
/// mouth never turns under a clip — a transition restarts the clip it lands in.
pub(crate) fn tong_target(state: &str, jaw: Jaw, facing: Facing) -> Option<String> {
    let (clip, _) = Facing::split(state)?;
    let resting = match clip {
        TONG_OPEN => Jaw::Open,
        TONG_CLOSED => Jaw::Closed,
        _ => return None,
    };
    let target = match (resting, jaw) {
        (Jaw::Open, Jaw::Closed) => tong_state(TONG_CLOSING, facing),
        (Jaw::Closed, Jaw::Open) => tong_state(TONG_OPENING, facing),
        (rest, _) => tong_state(rest.clip(), facing),
    };
    (target != state).then_some(target)
}

/// Whether a ball at `ball_y` met a paddle at `paddle_y` from the field: above the
/// bottom paddle's centre line, below the top one's. Only such a contact is a hit; a
/// ball behind the line has already beaten the paddle.
pub(crate) fn on_field_side(ball_y: f32, paddle_y: f32, side: PaddleSide) -> bool {
    match side {
        PaddleSide::Bottom => ball_y > paddle_y,
        PaddleSide::Top => ball_y < paddle_y,
    }
}

impl BreakoutGame {
    /// Both paddles' worth of side/entity/tong/burst colour, for uniform loops.
    pub(super) fn paddle_roster(&self) -> [(Option<EntityId>, Option<EntityId>, PaddleSide, Vec4); 2] {
        [
            (self.paddle, self.tong, PaddleSide::Bottom, P1_BURST_COLOR),
            (self.paddle_top, self.tong_top, PaddleSide::Top, P2_BURST_COLOR),
        ]
    }

    /// Move the paddles, read the jaws' asks, move each resting tong toward its ask, and
    /// dress each paddle in the pose its tong draws. Runs before the physics step, so the
    /// step's contacts are against the collider dressed here.
    pub(super) fn update_paddles(&mut self, ctx: &mut GameContext) {
        if let Some(paddle) = self.paddle {
            self.update_paddle(ctx, paddle, PlayerId::P1, PADDLE_Y, PaddleSide::Bottom);
        }
        if let Some(paddle) = self.paddle_top {
            self.update_paddle(ctx, paddle, PlayerId::P2, PADDLE_TOP_Y, PaddleSide::Top);
        }
        self.settle_launches(ctx.world);
        self.read_jaw_asks(ctx);
        self.drive_tongs(ctx.world);
        self.dress_paddles(ctx.world);
    }

    /// End the launch's hold on each tong whose `opening` has landed in `open`, and
    /// forget each device's last reading there, so a stick or a button held since before
    /// the launch asks again this very frame — the player launching with the bite held
    /// gets the wind-up, then the bite.
    fn settle_launches(&mut self, world: &World) {
        for (_, tong, side, _) in self.paddle_roster() {
            let Some(tong) = tong else { continue };
            let resting_open = world
                .get::<ClipStateMachine>(tong)
                .and_then(|machine| Facing::split(machine.state()).map(|(clip, _)| clip == TONG_OPEN))
                .unwrap_or(false);
            let control = &mut self.tongs[side.index()];
            if control.launch_opening && resting_open {
                control.launch_opening = false;
                control.axis_lean = None;
                if side == PaddleSide::Bottom {
                    self.bite_button_was_down = false;
                }
            }
        }
    }

    /// Move one paddle from its player's bindings (keys, dpad, or stick) or
    /// the mouse. Mouse takes over whenever it moves and belongs to the
    /// bottom paddle only (the mouse is player 1's device); bound input
    /// takes over whenever it's active. Whatever moved it, the move asks the
    /// paddle's tong to face the way it went.
    fn update_paddle(&mut self, ctx: &GameContext, paddle: EntityId, player: PlayerId, y: f32, side: PaddleSide) {
        let x = entity_x(ctx.world, paddle);

        // Solo: the lone paddle listens to both players' devices, so WASD,
        // arrows, and either pad all work.
        let axis = match self.mode {
            GameMode::SinglePlayer => (ctx.players.move_x(PlayerId::P1, ctx.input)
                + ctx.players.move_x(PlayerId::P2, ctx.input))
            .clamp(-1.0, 1.0),
            GameMode::TwoPlayerCoop => ctx.players.move_x(player, ctx.input),
        };

        let mouse_moved =
            player == PlayerId::P1 && ctx.input.mouse_movement_delta().0.abs() > 0.0;
        let new_x = if axis != 0.0 {
            x + axis * PADDLE_SPEED * ctx.delta_time
        } else if mouse_moved {
            // Window pixels (origin top-left) → world (origin center).
            ctx.input.mouse_position().x - ctx.window_size.x / 2.0
        } else {
            x
        };

        let new_x = new_x.clamp(-PADDLE_MAX_X, PADDLE_MAX_X);
        if let Some(facing) = Facing::from_displacement(new_x - x) {
            self.tongs[side.index()].facing = facing;
        }
        self.physics.set_kinematic_target(paddle, Vec2::new(new_x, y), 0.0);
    }

    /// How far a paddle's player leans the jaw axis toward the field: up for the bottom
    /// paddle, down for the top one. Solo merges both slots' axes, as it merges their
    /// movement — ↑ is a Player 2 key, and a solo arrow-key player bites too.
    fn jaw_axis(&self, ctx: &GameContext, side: PaddleSide) -> f32 {
        let up = match self.mode {
            GameMode::SinglePlayer => (ctx.players.move_y(PlayerId::P1, ctx.input)
                + ctx.players.move_y(PlayerId::P2, ctx.input))
            .clamp(-1.0, 1.0),
            GameMode::TwoPlayerCoop => ctx.players.move_y(
                match side {
                    PaddleSide::Bottom => PlayerId::P1,
                    PaddleSide::Top => PlayerId::P2,
                },
                ctx.input,
            ),
        };
        match side {
            PaddleSide::Bottom => up,
            PaddleSide::Top => -up,
        }
    }

    /// Record what each jaw's devices asked for this frame. The edges are tracked every
    /// frame, but an ask is kept only in play and only once the launch's `opening` has
    /// landed: the serve holds the jaw shut, and the launch's wind-up outranks a held bite.
    fn read_jaw_asks(&mut self, ctx: &GameContext) {
        let button_down = ctx.input.is_mouse_button_pressed(MouseButton::Right);
        let button_ask = match (button_down, self.bite_button_was_down) {
            (true, false) => Some(Jaw::Closed),
            (false, true) => Some(Jaw::Open),
            _ => None,
        };
        self.bite_button_was_down = button_down;

        let listening = self.state == GameState::Playing;
        for (paddle, _, side, _) in self.paddle_roster() {
            if paddle.is_none() {
                continue;
            }
            let lean = jaw_lean(self.jaw_axis(ctx, side));
            let control = &mut self.tongs[side.index()];
            let axis_ask = if lean != control.axis_lean { lean } else { None };
            control.axis_lean = lean;
            // The mouse is Player 1's device: it works the bottom tong only.
            let button = if side == PaddleSide::Bottom { button_ask } else { None };
            if listening && !control.launch_opening {
                if let Some(jaw) = button.or(axis_ask) {
                    control.jaw = jaw;
                }
            }
        }
    }

    /// Move each resting tong toward its jaw and its facing; a launch still pending asks
    /// for the wind-up instead.
    pub(super) fn drive_tongs(&mut self, world: &mut World) {
        for (_, tong, side, _) in self.paddle_roster() {
            let Some(tong) = tong else { continue };
            let Some(state) = world.get::<ClipStateMachine>(tong).map(|machine| machine.state().to_string()) else {
                continue;
            };
            let control = self.tongs[side.index()];
            let jaw = if control.launch_opening { Jaw::Open } else { control.jaw };
            if let Some(target) = tong_target(&state, jaw, control.facing) {
                set_clip_state(world, tong, &target);
            }
        }
    }

    /// Dress each paddle body in the pose its tong's animation is drawing, and note
    /// whether that frame is a `closing` one. While serving the body wears the closed
    /// capsule whatever the tong draws: the glued ball rests 2 px over it, and a scowl's
    /// arms rising into the resting ball are art only.
    ///
    /// The write is skipped when the shape already is the wanted one: a pose held over
    /// several frames is one collider, and a rebuild is spent only where the drawn jaw
    /// changed.
    fn dress_paddles(&mut self, world: &mut World) {
        let serving = self.state == GameState::Serving;
        for (paddle, tong, side, _) in self.paddle_roster() {
            let (Some(paddle), Some(tong)) = (paddle, tong) else { continue };
            let drawn = world
                .get::<SpriteAnimation>(tong)
                .and_then(|animation| Some((animation.current_clip.clone()?, animation.current_frame)));
            self.tongs[side.index()].drawing_closing = drawn
                .as_ref()
                .and_then(|(clip, _)| Facing::split(clip))
                .is_some_and(|(clip, _)| clip == TONG_CLOSING);

            let wanted = if serving {
                tong_body_collider(JawPose::Closed, Facing::Up, side)
            } else {
                let Some((pose, facing)) = drawn.and_then(|(clip, frame)| drawn_pose(&clip, frame)) else {
                    continue;
                };
                tong_body_collider(pose, facing, side)
            };
            if let Some(collider) = world.get_mut::<Collider>(paddle) {
                if collider.shape != wanted {
                    collider.shape = wanted;
                }
            }
        }
    }

    /// Ask every jaw for shut and drop any launch still pending: a serve, a match start
    /// and a quit all rest the tongs closed.
    pub(crate) fn hold_jaws_shut(&mut self) {
        for control in &mut self.tongs {
            control.jaw = Jaw::Closed;
            control.launch_opening = false;
        }
    }

    /// The launch opens every tong's jaw through `opening`, the wind-up, before any ask
    /// is read again.
    pub(super) fn open_jaws_for_launch(&mut self) {
        for (_, tong, side, _) in self.paddle_roster() {
            if tong.is_some() {
                let control = &mut self.tongs[side.index()];
                control.jaw = Jaw::Open;
                control.launch_opening = true;
            }
        }
    }

    /// Put each tong where physics left its paddle this frame.
    pub(super) fn place_tongs(&self, world: &mut World) {
        for (paddle, tong, _, _) in self.paddle_roster() {
            let (Some(paddle), Some(tong)) = (paddle, tong) else { continue };
            let Some(position) = entity_position(world, paddle) else { continue };
            if let Some(transform) = world.get_mut::<Transform2D>(tong) {
                transform.position = position;
            }
        }
    }

    /// The speed a ball is held at: the match's, and a chomp's gain on a chomped ball.
    pub(crate) fn ball_target_speed(&self, ball: EntityId) -> f32 {
        let gain = if self.chomped.contains(&ball) { CHOMP_SPEED_GAIN } else { 1.0 };
        (BALL_SPEED * self.speed_mult * gain).min(BALL_MAX_SPEED)
    }

    /// Paddle bounces, in play only and from the field side only: aim the ball by hit
    /// offset (toward the field, per side), reset the combo, apply the Insane speed gain,
    /// spray particles. A contact while the tong draws a `closing` frame is a chomp: the
    /// aim's spread narrows and the ball runs faster until its next plain paddle hit.
    pub(super) fn check_paddle_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        if self.state != GameState::Playing { return; }
        let theme = theme_for(self.chaos_mode);

        for (paddle, _, side, color) in self.paddle_roster() {
            let Some(paddle) = paddle else { continue };
            let Some(paddle_position) = entity_position(ctx.world, paddle) else { continue };
            let chomp = self.tongs[side.index()].drawing_closing;

            for &ball in &self.all_balls() {
                let hit = collisions.iter()
                    .any(|c| c.event.started && c.event.involves(ball, paddle));
                if !hit { continue; }
                let Some(pos) = entity_position(ctx.world, ball) else { continue };
                if !on_field_side(pos.y, paddle_position.y, side) { continue; }

                self.combo = 0;
                if self.chaos_mode.is_insane() {
                    self.speed_mult *= INSANE_SPEED_GAIN;
                }
                if chomp {
                    if !self.chomped.contains(&ball) {
                        self.chomped.push(ball);
                    }
                } else {
                    self.chomped.retain(|&chomped| chomped != ball);
                }

                // Override the physical reflection with offset-based aim —
                // this is what makes Breakout controllable rather than
                // deterministic.
                let offset = ((pos.x - paddle_position.x) / (PADDLE_W / 2.0)).clamp(-1.0, 1.0);
                let aim = if chomp { offset * CHOMP_ANGLE_FACTOR } else { offset };
                let dir = paddle_bounce_direction_for(aim, side);
                self.physics.set_velocity(ball, dir * self.ball_target_speed(ball), 0.0);

                let (burst_color, strength, radius) = if chomp {
                    (CHOMP_BURST_COLOR, GRID_IMPULSE_CHOMP_STRENGTH, GRID_IMPULSE_CHOMP_RADIUS)
                } else {
                    (color, GRID_IMPULSE_PADDLE_HIT_STRENGTH, GRID_IMPULSE_PADDLE_HIT_RADIUS)
                };
                ctx.particles.spawn_burst(pos, &effects::paddle_hit_burst(burst_color, &theme, self.sheets.white));
                ripple_grid(ctx.world, pos, strength, radius);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::spawn_tong;

    /// A sheet carrying the tong's clip names in the synced sidecar's shape: `open` and
    /// `closed` two frames looping, `closing` and `opening` three and `scored_on` four,
    /// played once, all at 10 fps.
    fn tong_sheet() -> SpriteSheet {
        let mut clips = Vec::new();
        for facing in [Facing::Up, Facing::Down] {
            clips.push((tong_state(TONG_OPEN, facing), AnimationClip::new(vec![0, 1], 10.0)));
            clips.push((tong_state(TONG_CLOSING, facing), AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false)));
            clips.push((tong_state(TONG_CLOSED, facing), AnimationClip::new(vec![0, 1], 10.0)));
            clips.push((tong_state(TONG_OPENING, facing), AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false)));
            clips.push((
                tong_state(TONG_SCORED_ON, facing),
                AnimationClip::new(vec![0, 1, 2, 3], 10.0).with_looping(false),
            ));
        }
        SpriteSheet { texture: TextureHandle { id: 1 }, grid: SheetGrid::new(4, 1), clips, path: String::new() }
    }

    fn state(world: &World, tong: EntityId) -> String {
        world.get::<ClipStateMachine>(tong).expect("machine").state().to_string()
    }

    /// The frame tail as the engine runs it after `update`, for the one tong: its clip
    /// advances, then the machine applies a finished clip's `Next`.
    fn tail(world: &mut World, tong: EntityId, seconds: f32) {
        if let Some(animation) = world.get_mut::<SpriteAnimation>(tong) {
            animation.update(seconds);
        }
        ClipStateMachineSystem.update(world, seconds);
    }

    #[test]
    fn a_facing_asked_for_during_a_scowl_lands_when_the_scowl_ends_and_not_before() {
        let mut world = World::new();
        let mut game = BreakoutGame::default();
        let tong = spawn_tong(
            &mut world, "Tong P1", &TONG_LEFT, &tong_sheet(), TONG_LEFT_DEPTH, Vec2::ZERO, Facing::Up);
        game.tong = Some(tong);
        tail(&mut world, tong, 0.0);

        set_clip_state(&mut world, tong, &tong_state(TONG_SCORED_ON, Facing::Up));
        tail(&mut world, tong, 0.0);
        game.tongs[PaddleSide::Bottom.index()].facing = Facing::Down;

        for _ in 0..3 {
            game.drive_tongs(&mut world);
            assert_eq!(state(&world, tong), tong_state(TONG_SCORED_ON, Facing::Up), "the scowl plays out");
            tail(&mut world, tong, 0.1);
        }
        tail(&mut world, tong, 0.2);
        assert_eq!(state(&world, tong), tong_state(TONG_CLOSED, Facing::Up), "it ends in its own facing");

        game.drive_tongs(&mut world);
        assert_eq!(state(&world, tong), tong_state(TONG_CLOSED, Facing::Down), "then the pending facing lands");
    }

    #[test]
    fn a_resting_tong_moves_toward_its_jaw_and_facing_and_a_moving_one_waits() {
        assert_eq!(tong_target("closed_up", Jaw::Closed, Facing::Down), Some("closed_down".to_string()));
        assert_eq!(tong_target("closed_up", Jaw::Closed, Facing::Up), None);
        assert_eq!(tong_target("closed_up", Jaw::Open, Facing::Up), Some("opening_up".to_string()));
        assert_eq!(tong_target("open_down", Jaw::Closed, Facing::Down), Some("closing_down".to_string()));
        assert_eq!(tong_target("open_down", Jaw::Open, Facing::Up), Some("open_up".to_string()));
        for moving in ["closing_up", "opening_down", "scored_on_up"] {
            assert_eq!(tong_target(moving, Jaw::Open, Facing::Down), None, "{moving} plays out");
            assert_eq!(tong_target(moving, Jaw::Closed, Facing::Down), None, "{moving} plays out");
        }
        assert_eq!(Facing::from_displacement(-3.0), Some(Facing::Up), "moving left, the mouth leads left");
        assert_eq!(Facing::from_displacement(3.0), Some(Facing::Down));
        assert_eq!(Facing::from_displacement(0.0), None, "a stop keeps the last facing");
    }

    #[test]
    fn a_stick_leaning_past_the_dead_zone_asks_and_a_resting_one_does_not() {
        assert_eq!(jaw_lean(1.0), Some(Jaw::Closed), "toward the field bites");
        assert_eq!(jaw_lean(-1.0), Some(Jaw::Open), "away opens");
        assert_eq!(jaw_lean(JAW_AXIS_DEAD_ZONE), None, "the dead zone's edge is inside it");
        assert_eq!(jaw_lean(0.2), None);
        assert!(on_field_side(1.0, 0.0, PaddleSide::Bottom) && !on_field_side(-1.0, 0.0, PaddleSide::Bottom));
        assert!(on_field_side(-1.0, 0.0, PaddleSide::Top) && !on_field_side(1.0, 0.0, PaddleSide::Top));
    }
}
