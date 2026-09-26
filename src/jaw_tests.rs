//! The tong's jaw through the engine's harness: the real frame, the real physics and
//! the synced tong sheet. Each test states one contract of the bite — what asks, what
//! the body wears, what counts as a hit and what counts as a chomp.

use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};

use crate::constants::*;
use crate::jaw::{drawn_pose, tong_body_collider, JawPose};
use crate::test_support::*;
use crate::types::*;

const THE_VAULT: usize = 1;
const PINATA: usize = 2;
/// How far a speed or an angle read back through physics may sit from the one set.
const TOLERANCE: f32 = 0.01;

fn press(key: KeyCode) -> InputEvent {
    InputEvent::KeyPressed(key)
}

fn release(key: KeyCode) -> InputEvent {
    InputEvent::KeyReleased(key)
}

fn launch(harness: &mut GameHarness<BreakoutGame>) {
    harness.step(FRAME, &[press(KeyCode::Space)]);
    harness.step(FRAME, &[release(KeyCode::Space)]);
    assert_eq!(harness.game().state, GameState::Playing, "the serve launched");
}

fn tong_of(harness: &GameHarness<BreakoutGame>, side: PaddleSide) -> EntityId {
    match side {
        PaddleSide::Bottom => harness.game().tong.expect("the bottom tong"),
        PaddleSide::Top => harness.game().tong_top.expect("co-op's top tong"),
    }
}

fn paddle_of(harness: &GameHarness<BreakoutGame>, side: PaddleSide) -> EntityId {
    match side {
        PaddleSide::Bottom => harness.game().paddle.expect("the bottom paddle"),
        PaddleSide::Top => harness.game().paddle_top.expect("co-op's top paddle"),
    }
}

fn clip_of(harness: &GameHarness<BreakoutGame>, side: PaddleSide) -> String {
    let (clip, _) = Facing::split(&state_of(harness.world(), tong_of(harness, side)))
        .map(|(clip, facing)| (clip.to_string(), facing))
        .expect("a tong state names its facing");
    clip
}

/// The clip and clip-relative frame a tong's animation is drawing.
fn drawn(harness: &GameHarness<BreakoutGame>, side: PaddleSide) -> (String, usize) {
    let animation = harness.world().get::<SpriteAnimation>(tong_of(harness, side)).expect("the tong animates");
    (animation.current_clip.clone().expect("a clip is playing"), animation.current_frame)
}

fn shape_of(harness: &GameHarness<BreakoutGame>, side: PaddleSide) -> ColliderShape {
    harness.world().get::<Collider>(paddle_of(harness, side)).expect("the body collides").shape.clone()
}

/// Step with `events` on the first frame until `side`'s tong rests in `clip`, returning
/// every clip it was in along the way, the resting one last.
fn step_until_resting(
    harness: &mut GameHarness<BreakoutGame>,
    side: PaddleSide,
    clip: &str,
    events: &[InputEvent],
) -> Vec<String> {
    let mut seen = vec![clip_of(harness, side)];
    harness.step(FRAME, events);
    for _ in 0..90 {
        let now = clip_of(harness, side);
        if seen.last() != Some(&now) {
            seen.push(now.clone());
        }
        if now == clip {
            return seen;
        }
        harness.step(FRAME, &[]);
    }
    panic!("{side:?}'s tong never rested {clip}; it went {seen:?}");
}

/// A match in play with no ball on the field, its tongs rested open after the launch:
/// the jaw alone, with nothing to hit.
fn open_and_empty(mode: GameMode) -> GameHarness<BreakoutGame> {
    let mut harness = harness(mode, 0);
    launch(&mut harness);
    harness.context(|game, ctx| game.destroy_all_balls(ctx.world));
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[]);
    harness
}

/// Put `ball` at `offset` from `side`'s paddle, moving at `velocity`.
fn place_ball(harness: &mut GameHarness<BreakoutGame>, ball: EntityId, side: PaddleSide, offset: Vec2, velocity: Vec2) {
    let paddle = position_of(harness.world(), paddle_of(harness, side));
    harness.context(|game, _| {
        game.physics.reset_body(ball, paddle + offset);
        game.physics.set_velocity(ball, velocity, 0.0);
    });
}

fn a_fresh_ball(harness: &mut GameHarness<BreakoutGame>) -> EntityId {
    harness.context(|game, ctx| {
        let ball = game.spawn_ball(ctx.world, "Deion", Vec2::new(0.0, 0.0));
        game.ball = Some(ball);
        ball
    })
}

fn velocity_of(harness: &GameHarness<BreakoutGame>, ball: EntityId) -> Vec2 {
    harness.game().physics.get_body_velocity(ball).expect("the ball has a body").0
}

/// How far a pose's field-side collider rises over the paddle's centre at the paddle's
/// centre line, where the arm crosses it: what a ball dropped at the paddle's centre
/// meets first.
fn arm_top_over_centre(pose: JawPose) -> f32 {
    match tong_body_collider(pose, Facing::Up, PaddleSide::Bottom) {
        ColliderShape::CapsuleX { radius, .. } => radius,
        ColliderShape::Compound(parts) => match &parts[0] {
            ColliderShape::Capsule { a, b, radius } => {
                let along = a.x / (a.x - b.x);
                a.y + (b.y - a.y) * along + radius
            }
            other => panic!("an arm is a capsule, not {other:?}"),
        },
        other => panic!("{pose:?} is a compound, not {other:?}"),
    }
}

#[test]
fn through_a_whole_bite_and_a_whole_opening_the_body_wears_the_frame_the_tong_drew() {
    // At every step — the entry, each frame change, the landing — the collider dressed
    // in the frame is the pose the animation was drawing when the frame began.
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    let mut clips = Vec::new();
    for (events, until) in [
        (vec![press(KeyCode::KeyW)], TONG_CLOSED),
        (vec![release(KeyCode::KeyW), press(KeyCode::KeyS)], TONG_OPEN),
    ] {
        let mut first = Some(events);
        for _ in 0..40 {
            let (clip, frame) = drawn(&harness, PaddleSide::Bottom);
            clips.push(clip.clone());
            harness.step(FRAME, &first.take().unwrap_or_default());
            let (pose, facing) = drawn_pose(&clip, frame).expect("a tong frame has a pose");
            assert_eq!(
                shape_of(&harness, PaddleSide::Bottom),
                tong_body_collider(pose, facing, PaddleSide::Bottom),
                "{clip} frame {frame}"
            );
            if clip_of(&harness, PaddleSide::Bottom) == until && drawn(&harness, PaddleSide::Bottom).0.starts_with(until) {
                break;
            }
        }
    }
    for clip in ["open_up", "closing_up", "closed_up", "opening_up"] {
        assert!(clips.iter().any(|drawn| drawn == clip), "the bite drew {clip}: {clips:?}");
    }
}

#[test]
fn a_contact_in_the_frame_a_press_lands_is_not_a_chomp() {
    // The press moves the machine to `closing` at once, but the animation — and so the
    // collider — still draws `open` until the frame tail: the contact is a plain hit.
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    let ball = a_fresh_ball(&mut harness);
    let gap = 3.0;
    place_ball(
        &mut harness, ball, PaddleSide::Bottom,
        Vec2::new(0.0, arm_top_over_centre(JawPose::Open) + BALL_RADIUS + gap),
        Vec2::new(0.0, -BALL_SPEED),
    );
    harness.context(|game, _| game.combo = 4);
    harness.step(FRAME, &[press(KeyCode::KeyW)]);

    assert_eq!(harness.game().combo, 0, "the contact was a hit");
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSING, "the press landed");
    assert!(harness.game().chomped.is_empty(), "but it was not a chomp");
    assert!((velocity_of(&harness, ball).length() - BALL_SPEED).abs() < TOLERANCE);
}

#[test]
fn a_contact_during_a_drawn_bite_is_a_chomp_and_a_plain_hit_after_it_ends_it() {
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    let ball = a_fresh_ball(&mut harness);
    harness.step(FRAME, &[press(KeyCode::KeyW)]);
    assert_eq!(drawn(&harness, PaddleSide::Bottom).0, tong_state(TONG_CLOSING, Facing::Up), "the tong draws the bite");

    // Near the hinge end, where the plain aim would leave at about 46 degrees.
    let offset_x = 30.0;
    let arm_there = match tong_body_collider(JawPose::Wide, Facing::Up, PaddleSide::Bottom) {
        ColliderShape::Compound(parts) => match &parts[0] {
            ColliderShape::Capsule { a, b, radius } => a.y + (b.y - a.y) * (a.x - offset_x) / (a.x - b.x) + radius,
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    };
    place_ball(
        &mut harness, ball, PaddleSide::Bottom,
        Vec2::new(offset_x, arm_there + BALL_RADIUS + 2.0),
        Vec2::new(0.0, -BALL_SPEED),
    );
    harness.step(FRAME, &[]);

    assert_eq!(harness.game().chomped, vec![ball], "the chomp is recorded");
    let velocity = velocity_of(&harness, ball);
    let chomp_speed = BALL_SPEED * CHOMP_SPEED_GAIN;
    assert!((velocity.length() - chomp_speed).abs() < TOLERANCE, "faster: {velocity:?}");
    let angle = velocity.x.atan2(velocity.y);
    assert!(angle > 0.0 && angle <= PADDLE_MAX_BOUNCE_ANGLE * CHOMP_ANGLE_FACTOR + TOLERANCE,
        "steeper: {} degrees", angle.to_degrees());
    harness.steps(10, FRAME);
    assert!((velocity_of(&harness, ball).length() - chomp_speed).abs() < TOLERANCE, "and held there");

    // The jaw now rests shut: a plain contact on it restores the match's speed.
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[]);
    place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, PADDLE_H / 2.0 + BALL_RADIUS + 2.0), Vec2::new(0.0, -BALL_SPEED));
    harness.step(FRAME, &[]);
    assert!(harness.game().chomped.is_empty(), "a plain hit ends the chomp");
    assert!((velocity_of(&harness, ball).length() - BALL_SPEED).abs() < TOLERANCE);
}

#[test]
fn a_loss_and_a_restart_each_forget_a_chomp() {
    for forget in ["loss", "restart"] {
        let mut harness = open_and_empty(GameMode::SinglePlayer);
        let ball = a_fresh_ball(&mut harness);
        harness.context(|game, _| game.chomped.push(ball));
        match forget {
            "loss" => {
                place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, -40.0), Vec2::new(0.0, -BALL_SPEED));
                harness.steps(30, FRAME);
            }
            _ => harness.context(|game, ctx| game.start_game(ctx)),
        }
        assert!(harness.game().chomped.is_empty(), "a {forget} forgets the chomp");
    }
}

#[test]
fn a_pull_back_made_mid_bite_opens_the_jaw_once_the_bite_lands() {
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    harness.step(FRAME, &[press(KeyCode::KeyW)]);
    harness.step(FRAME, &[release(KeyCode::KeyW), press(KeyCode::KeyS)]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSING, "the bite plays out");
    harness.step(FRAME, &[release(KeyCode::KeyS)]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSING, "released before the bite ends");

    let seen = step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[]);
    assert_eq!(seen, [TONG_CLOSING, TONG_CLOSED, TONG_OPENING, TONG_OPEN], "the pull-back landed after the bite");
}

#[test]
fn a_ball_held_against_the_jaw_through_a_whole_bite_counts_one_hit() {
    // The pose changes three times under the ball and the collider is rebuilt each
    // time; the engine keys a contact's start by the entity pair, so it starts once.
    let mut harness = harness(GameMode::SinglePlayer, THE_VAULT);
    launch(&mut harness);
    let ball = harness.game().ball.expect("ball");
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[]);
    harness.context(|game, _| game.speed_mult = 1.0);

    // Held: before every step the ball is seated half a pixel into the pose the body will
    // wear in that step, at rest, so it touches the jaw in every step however the pose
    // moves under it.
    let seat = |harness: &mut GameHarness<BreakoutGame>| {
        let (clip, frame) = drawn(harness, PaddleSide::Bottom);
        let (pose, _) = drawn_pose(&clip, frame).expect("a tong frame has a pose");
        let height = arm_top_over_centre(pose) + BALL_RADIUS - 0.5;
        place_ball(harness, ball, PaddleSide::Bottom, Vec2::new(0.0, height), Vec2::ZERO);
    };
    let mut events = vec![press(KeyCode::KeyW)];
    let mut poses = Vec::new();
    for _ in 0..30 {
        seat(&mut harness);
        poses.push(drawn(&harness, PaddleSide::Bottom));
        harness.step(FRAME, &std::mem::take(&mut events));
        if clip_of(&harness, PaddleSide::Bottom) == TONG_CLOSED {
            break;
        }
    }
    let worn: std::collections::BTreeSet<String> = poses.iter().map(|(clip, frame)| format!("{clip} {frame}")).collect();
    assert!(worn.len() >= 4, "the ball was held through open and the three bite frames: {worn:?}");
    seat(&mut harness);
    harness.step(FRAME, &[]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSED, "the whole bite played");
    assert!((harness.game().speed_mult - INSANE_SPEED_GAIN).abs() < 1e-5, "one hit, not {}", harness.game().speed_mult.ln() / INSANE_SPEED_GAIN.ln());
}

#[test]
fn a_contact_while_serving_is_not_a_paddle_hit() {
    let mut harness = harness(GameMode::SinglePlayer, THE_VAULT);
    let extra = harness.context(|game, ctx| {
        game.combo = 3;
        // Clear of the glued ball beside it, over the shut jaw's end.
        let extra = game.spawn_ball(ctx.world, "Deion (extra)", Vec2::new(-45.0, PADDLE_Y + 60.0));
        game.extra_balls.push(extra);
        extra
    });
    harness.context(|game, _| game.physics.set_velocity(extra, Vec2::new(0.0, -BALL_SPEED), 0.0));
    harness.steps(20, FRAME);
    assert_eq!(harness.game().state, GameState::Serving);
    assert!(velocity_of(&harness, extra).y > 0.0, "the ball bounced off the shut jaw");
    assert_eq!(harness.game().combo, 3, "but no hit was counted");
    assert_eq!(harness.game().speed_mult, 1.0, "and no speed gained");
}

#[test]
fn a_ball_that_has_beaten_the_open_jaw_falls_through_its_trailing_arm_to_the_loss() {
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    let ball = a_fresh_ball(&mut harness);
    // Where the trailing arm is drawn, under the paddle's centre line.
    place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, -20.0), Vec2::new(0.0, -BALL_SPEED));
    harness.step(FRAME, &[]);
    assert!(velocity_of(&harness, ball).y < 0.0, "nothing behind the jaw to bounce off");
    let lives = harness.game().lives;
    harness.steps(20, FRAME);
    assert_eq!(harness.game().lives, lives - 1, "the ball reached the loss sensor");
}

#[test]
fn after_a_loss_the_scowl_plays_out_on_a_shut_body_under_the_glued_ball() {
    let mut harness = harness(GameMode::SinglePlayer, 0);
    launch(&mut harness);
    let ball = harness.game().ball.expect("ball");
    place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, -40.0), Vec2::new(0.0, -BALL_SPEED));
    let lives = harness.game().lives;
    for _ in 0..30 {
        harness.step(FRAME, &[]);
        if harness.game().lives < lives {
            break;
        }
    }
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_SCORED_ON);

    let closed = tong_body_collider(JawPose::Closed, Facing::Up, PaddleSide::Bottom);
    for _ in 0..40 {
        harness.step(FRAME, &[]);
        assert_eq!(shape_of(&harness, PaddleSide::Bottom), closed, "the body stays shut under {:?}", drawn(&harness, PaddleSide::Bottom));
        let glued = harness.game().ball.expect("a fresh serve");
        let above = position_of(harness.world(), glued).y - position_of(harness.world(), paddle_of(&harness, PaddleSide::Bottom)).y;
        assert!(above - BALL_RADIUS > PADDLE_H / 2.0, "the glued ball clears the shut jaw: {above}");
    }
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSED, "the scowl lands shut");
}

#[test]
fn the_launch_opens_through_opening_and_no_launched_ball_touches_the_jaw() {
    for (mode, level, side) in [
        (GameMode::SinglePlayer, 0, PaddleSide::Bottom),
        (GameMode::SinglePlayer, PINATA, PaddleSide::Bottom),
        (GameMode::TwoPlayerCoop, 0, PaddleSide::Bottom),
        (GameMode::TwoPlayerCoop, 0, PaddleSide::Top),
    ] {
        let mut harness = harness(mode, level);
        harness.context(|game, _| game.serving_side = side);
        harness.step(FRAME, &[]);
        harness.step(FRAME, &[press(KeyCode::Space)]);
        assert_eq!(harness.game().state, GameState::Playing);
        let mut seen = vec![clip_of(&harness, side)];
        // A ball spawned this frame reads its velocity back once its body has synced.
        harness.step(FRAME, &[release(KeyCode::Space)]);
        let balls: Vec<EntityId> =
            harness.game().ball.into_iter().chain(harness.game().extra_balls.iter().copied()).collect();
        assert_eq!(balls.len(), if level == PINATA { 2 } else { 1 }, "{mode:?} {level}");
        let launched: Vec<Vec2> = balls.iter().map(|&ball| velocity_of(&harness, ball)).collect();
        harness.context(|game, _| game.combo = 7);

        // Every pose the jaw draws is within reach of a launched ball only for its first
        // frames: by the twelfth it has risen 72 px, three times the open pad's reach, and
        // no brick is near enough yet to turn it.
        for step in 0..12 {
            harness.step(FRAME, &[]);
            for (ball, velocity) in balls.iter().zip(&launched) {
                assert!(velocity_of(&harness, *ball).distance(*velocity) < TOLERANCE,
                    "{mode:?} {level} {side:?}: a ball touched the jaw on step {step}");
            }
            let now = clip_of(&harness, side);
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
        for clip in step_until_resting(&mut harness, side, TONG_OPEN, &[]) {
            if seen.last() != Some(&clip) {
                seen.push(clip);
            }
        }
        assert_eq!(seen, [TONG_CLOSED, TONG_OPENING, TONG_OPEN], "{mode:?} {level} {side:?}: never a jump to open");
        assert_ne!(harness.game().combo, 0, "{mode:?} {level} {side:?}: no paddle hit");
    }
}

#[test]
fn the_launchs_wind_up_outranks_a_held_bite_and_the_bite_follows_it() {
    // With the bite held through the serve, and again launched mid-scowl.
    let mut harness = harness(GameMode::SinglePlayer, 0);
    harness.step(FRAME, &[press(KeyCode::KeyW)]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSED, "the serve holds the jaw shut");
    launch(&mut harness);
    let seen = step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[]);
    assert_eq!(seen, [TONG_OPENING, TONG_OPEN, TONG_CLOSING, TONG_CLOSED], "wind-up, then the bite");

    let ball = harness.game().ball.expect("ball");
    place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, -40.0), Vec2::new(0.0, -BALL_SPEED));
    let lives = harness.game().lives;
    for _ in 0..30 {
        harness.step(FRAME, &[]);
        if harness.game().lives < lives {
            break;
        }
    }
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_SCORED_ON);
    launch(&mut harness);
    let mut seen = vec![clip_of(&harness, PaddleSide::Bottom)];
    for _ in 0..90 {
        harness.step(FRAME, &[]);
        let now = clip_of(&harness, PaddleSide::Bottom);
        if seen.last() != Some(&now) {
            seen.push(now);
        }
    }
    assert_eq!(seen, [TONG_SCORED_ON, TONG_CLOSED, TONG_OPENING, TONG_OPEN, TONG_CLOSING, TONG_CLOSED],
        "the scowl ends, the wind-up plays, then the held bite");
}

#[test]
fn toward_the_field_bites_away_opens_and_a_resting_stick_holds() {
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[press(KeyCode::KeyW)]);
    harness.step(FRAME, &[release(KeyCode::KeyW)]);
    harness.steps(20, FRAME);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSED, "a released key leaves the bite standing");
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[press(KeyCode::KeyS)]);
    harness.step(FRAME, &[release(KeyCode::KeyS)]);
    harness.steps(20, FRAME);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_OPEN);

    // Solo merges both slots: the up arrow is a Player 2 key, and it bites too.
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[press(KeyCode::ArrowUp)]);
}

#[test]
fn player_twos_field_is_down_and_the_mouse_works_only_the_bottom_tong() {
    let mut harness = open_and_empty(GameMode::TwoPlayerCoop);
    step_until_resting(&mut harness, PaddleSide::Top, TONG_OPEN, &[]);
    step_until_resting(&mut harness, PaddleSide::Top, TONG_CLOSED, &[press(KeyCode::ArrowDown)]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_OPEN, "Player 2's key works Player 2's tong");
    harness.step(FRAME, &[release(KeyCode::ArrowDown)]);
    step_until_resting(&mut harness, PaddleSide::Top, TONG_OPEN, &[press(KeyCode::ArrowUp)]);
    harness.step(FRAME, &[release(KeyCode::ArrowUp)]);

    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[InputEvent::MouseButtonPressed(MouseButton::Right)]);
    assert_eq!(clip_of(&harness, PaddleSide::Top), TONG_OPEN, "the mouse is Player 1's");
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[InputEvent::MouseButtonReleased(MouseButton::Right)]);
}

#[test]
fn the_right_button_bites_on_its_press_opens_on_its_release_and_outranks_the_axis() {
    let mut harness = open_and_empty(GameMode::SinglePlayer);
    let right = MouseButton::Right;

    // Same frame, open jaw: the axis asks open, the button asks shut. The button wins.
    let seen = step_until_resting(
        &mut harness, PaddleSide::Bottom, TONG_CLOSED,
        &[press(KeyCode::KeyS), InputEvent::MouseButtonPressed(right)],
    );
    assert_eq!(seen, [TONG_OPEN, TONG_CLOSING, TONG_CLOSED]);
    harness.step(FRAME, &[release(KeyCode::KeyS)]);

    // Same frame, shut jaw: the axis asks shut, the button's release asks open.
    let seen = step_until_resting(
        &mut harness, PaddleSide::Bottom, TONG_OPEN,
        &[press(KeyCode::KeyW), InputEvent::MouseButtonReleased(right)],
    );
    assert_eq!(seen, [TONG_CLOSED, TONG_OPENING, TONG_OPEN]);
    harness.step(FRAME, &[release(KeyCode::KeyW)]);

    // A key's bite, with the button never touched, stands.
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_CLOSED, &[press(KeyCode::KeyW)]);
    harness.step(FRAME, &[release(KeyCode::KeyW)]);
    harness.steps(20, FRAME);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_CLOSED, "an unpressed button asks for nothing");
}

#[test]
fn a_multiball_ball_spawns_clear_of_the_open_jaw() {
    let mut harness = harness(GameMode::SinglePlayer, THE_VAULT);
    launch(&mut harness);
    harness.context(|game, ctx| game.destroy_all_balls(ctx.world));
    step_until_resting(&mut harness, PaddleSide::Bottom, TONG_OPEN, &[]);
    harness.context(|game, ctx| {
        game.combo = 5;
        game.spawn_pickup(ctx.world, PickupKind::Multiball, Vec2::new(0.0, PADDLE_Y + 60.0));
    });
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        if !harness.game().extra_balls.is_empty() {
            break;
        }
    }
    let extra = *harness.game().extra_balls.first().expect("the candy spawned a ball");
    harness.step(FRAME, &[]);
    let launched = velocity_of(&harness, extra);
    harness.steps(2, FRAME);
    assert_eq!(harness.game().combo, 5, "no paddle hit on its first steps");
    assert_eq!(harness.game().speed_mult, 1.0);
    assert!(velocity_of(&harness, extra).distance(launched) < TOLERANCE, "it touched nothing");
}

#[test]
fn deion_is_a_radius_fifteen_circle_on_both_sheets_and_his_splash_stays_in_the_window() {
    let mut harness = harness(GameMode::SinglePlayer, 0);
    let ball = harness.game().ball.expect("ball");
    assert_eq!(
        harness.world().get::<Collider>(ball).expect("collider").shape,
        ColliderShape::Circle { radius: 15.0 }
    );
    assert_eq!(harness.world().get::<Sprite>(ball).expect("sprite").offset, BALL_WATER.sprite_offset());
    assert_eq!(BALL_ICE.sprite_offset(), BALL_WATER.sprite_offset());

    launch(&mut harness);
    place_ball(&mut harness, ball, PaddleSide::Bottom, Vec2::new(0.0, -40.0), Vec2::new(0.0, -BALL_SPEED));
    let lives = harness.game().lives;
    for _ in 0..30 {
        harness.step(FRAME, &[]);
        if harness.game().lives < lives {
            break;
        }
    }
    let splash = named(harness.world(), "Splash");
    assert_eq!(splash.len(), 1);
    let body_bottom = position_of(harness.world(), splash[0]).y - BALL_RADIUS;
    assert!(body_bottom >= -WIN_H / 2.0, "the splash's body is inside the window: {body_bottom}");
}

/// The field-side surface of `pose`'s arm over `x`, for the tong on `side` facing `facing`:
/// where a ball dropped onto the arm there meets it.
fn arm_surface_at(pose: JawPose, facing: Facing, side: PaddleSide, x: f32) -> f32 {
    match tong_body_collider(pose, facing, side) {
        ColliderShape::Compound(parts) => match &parts[0] {
            ColliderShape::Capsule { a, b, radius } => {
                let centre = a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
                match side {
                    PaddleSide::Bottom => centre + radius,
                    PaddleSide::Top => centre - radius,
                }
            }
            other => panic!("an arm is a capsule, not {other:?}"),
        },
        other => panic!("{pose:?} is a compound, not {other:?}"),
    }
}

#[test]
fn player_twos_chomp_sends_the_ball_down_steeper_and_faster() {
    let mut harness = open_and_empty(GameMode::TwoPlayerCoop);
    step_until_resting(&mut harness, PaddleSide::Top, TONG_OPEN, &[]);
    let ball = a_fresh_ball(&mut harness);
    harness.step(FRAME, &[press(KeyCode::ArrowDown)]);
    assert_eq!(drawn(&harness, PaddleSide::Top).0, tong_state(TONG_CLOSING, Facing::Down), "the top tong draws the bite");

    let offset_x = -30.0;
    let surface = arm_surface_at(JawPose::Wide, Facing::Down, PaddleSide::Top, offset_x);
    place_ball(
        &mut harness, ball, PaddleSide::Top,
        Vec2::new(offset_x, surface - BALL_RADIUS - 2.0),
        Vec2::new(0.0, BALL_SPEED),
    );
    harness.step(FRAME, &[]);

    assert_eq!(harness.game().chomped, vec![ball], "the chomp is recorded");
    let velocity = velocity_of(&harness, ball);
    assert!((velocity.length() - BALL_SPEED * CHOMP_SPEED_GAIN).abs() < TOLERANCE, "faster: {velocity:?}");
    assert!(velocity.y < 0.0, "toward the field, below the top tong: {velocity:?}");
    let off_straight_down = velocity.x.atan2(-velocity.y);
    assert!(off_straight_down.abs() <= PADDLE_MAX_BOUNCE_ANGLE * CHOMP_ANGLE_FACTOR + TOLERANCE,
        "steeper: {} degrees", off_straight_down.to_degrees());
}

#[test]
fn ridiculouss_second_ball_leaves_from_the_paddle_the_mouse_just_moved_even_mid_scowl() {
    // The served ball's transform lags the paddle by a frame; the second ball must not.
    let mut harness = harness(GameMode::SinglePlayer, PINATA);
    harness.context(|game, ctx| {
        let tong = game.tong.expect("tong");
        crate::gameplay::set_clip_state(ctx.world, tong, &tong_state(TONG_SCORED_ON, Facing::Up));
        game.combo = 7;
    });
    harness.step(FRAME, &[]);
    assert_eq!(clip_of(&harness, PaddleSide::Bottom), TONG_SCORED_ON, "a scowl in flight");

    // The engine's first pointer update only records where the pointer is; the move that
    // follows is the one the paddle sees.
    let window_centre_x = 400.0;
    harness.step(FRAME, &[InputEvent::MouseMoved(window_centre_x, 300.0)]);
    harness.step(FRAME, &[InputEvent::MouseMoved(window_centre_x + 30.0, 300.0), press(KeyCode::Space)]);
    assert_eq!(harness.game().state, GameState::Playing);
    let paddle_x = position_of(harness.world(), paddle_of(&harness, PaddleSide::Bottom)).x;
    assert!((paddle_x - 30.0).abs() < TOLERANCE, "the mouse moved the paddle: {paddle_x}");
    let extra = *harness.game().extra_balls.first().expect("Ridiculous serves two");
    let spawned = position_of(harness.world(), extra);
    assert!((spawned.x - paddle_x).abs() < TOLERANCE, "the second ball leaves from the paddle, not {spawned:?}");
    assert_eq!(spawned.y, PADDLE_Y + SERVE_OFFSET_Y);

    harness.step(FRAME, &[release(KeyCode::Space)]);
    for step in 0..12 {
        harness.step(FRAME, &[]);
        assert_ne!(harness.game().combo, 0, "no paddle hit on step {step}");
    }
}
