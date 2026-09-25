//! The match driven through the engine's harness: the real frame, the real physics and
//! the synced art, with no window. The loss sensor is real physics here, so a lost ball
//! is a ball sent past the paddle and stepped until the sensor fires.

use engine_core::prelude::*;
use engine_core::test_support::InputEvent;

use crate::chaos_theme::theme_for;
use crate::constants::*;
use crate::spawning::backdrop_color;
use crate::test_support::*;
use crate::types::*;

/// How far a position read back through physics may sit from the one it was set to: a
/// body's round trip through the physics world costs a few ulps.
const POSITION_TOLERANCE: f32 = 0.001;

fn launch(harness: &mut engine_core::test_support::GameHarness<BreakoutGame>) {
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Space)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Space)]);
    assert_eq!(harness.game().state, GameState::Playing, "the serve launched");
}

/// Send `ball` downward from just under the bottom paddle, into the loss sensor.
fn send_past_the_bottom(harness: &mut engine_core::test_support::GameHarness<BreakoutGame>, ball: EntityId) {
    harness.context(|game, _| {
        game.physics.reset_body(ball, Vec2::new(0.0, PADDLE_Y - 40.0));
        game.physics.set_velocity(ball, Vec2::new(0.0, -BALL_SPEED), 0.0);
    });
}

/// Step until the life count drops, and say how many frames that took.
fn step_until_a_life_is_lost(harness: &mut engine_core::test_support::GameHarness<BreakoutGame>) -> usize {
    let lives = harness.game().lives;
    for frame in 1..=120 {
        harness.step(FRAME, &[]);
        if harness.game().lives < lives {
            return frame;
        }
    }
    panic!("no life was lost in two seconds");
}

#[test]
fn a_tong_lies_on_its_unturned_paddle_after_every_step_and_faces_the_way_it_moves() {
    let mut harness = harness(GameMode::SinglePlayer, 0);
    let (paddle, tong) = (harness.game().paddle.expect("paddle"), harness.game().tong.expect("tong"));

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::ArrowRight)]);
    for _ in 0..20 {
        harness.step(FRAME, &[]);
        let world = harness.world();
        assert_eq!(position_of(world, tong), position_of(world, paddle), "the tong rides its paddle");
        assert_eq!(world.get::<Transform2D>(paddle).expect("body").rotation, 0.0, "the body never turns");
        assert_eq!(world.get::<Transform2D>(tong).expect("art").rotation, TONG_ROTATION);
    }
    assert!(position_of(harness.world(), paddle).x > 0.0, "the paddle moved right");
    assert_eq!(state_of(harness.world(), tong), tong_state(TONG_CLOSED, Facing::Down), "mouth leads right");

    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::ArrowRight)]);
    harness.steps(5, FRAME);
    assert_eq!(state_of(harness.world(), tong), tong_state(TONG_CLOSED, Facing::Down), "a stop keeps it");

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::ArrowLeft)]);
    harness.steps(3, FRAME);
    assert_eq!(state_of(harness.world(), tong), tong_state(TONG_CLOSED, Facing::Up), "mouth leads left");
}

#[test]
fn losing_one_ball_of_two_plays_no_scowl_and_losing_the_last_does() {
    let mut harness = harness(GameMode::SinglePlayer, 0);
    launch(&mut harness);
    let tong = harness.game().tong.expect("tong");
    let first = harness.game().ball.expect("ball");
    // A second ball resting mid-field, where nothing reaches it.
    let second = harness.context(|game, ctx| {
        let extra = game.spawn_ball(ctx.world, "Deion (extra)", Vec2::new(200.0, 0.0));
        game.extra_balls.push(extra);
        extra
    });

    send_past_the_bottom(&mut harness, first);
    harness.steps(30, FRAME);
    assert_eq!(harness.game().ball, Some(second), "the other ball plays on");
    assert_eq!(harness.game().lives, STARTING_LIVES);
    assert!(state_of(harness.world(), tong).starts_with(TONG_CLOSED), "no scowl while a ball is in flight");

    send_past_the_bottom(&mut harness, second);
    step_until_a_life_is_lost(&mut harness);
    assert!(
        state_of(harness.world(), tong).starts_with(TONG_SCORED_ON),
        "the last ball lost makes the tong scowl, got {}",
        state_of(harness.world(), tong)
    );
    harness.steps(60, FRAME);
    assert!(state_of(harness.world(), tong).starts_with(TONG_CLOSED), "and the scowl ends at rest");
}

#[test]
fn a_lost_ball_splashes_once_at_the_windows_edge_from_the_sheet_it_wore() {
    for frozen in [false, true] {
        let mut harness = harness(GameMode::SinglePlayer, 0);
        launch(&mut harness);
        if frozen {
            harness.context(|game, ctx| {
                game.wrecking.start(WRECKING_DURATION);
                game.apply_ball_visuals(ctx.world);
            });
        }
        let ball = harness.game().ball.expect("ball");
        send_past_the_bottom(&mut harness, ball);
        step_until_a_life_is_lost(&mut harness);

        let world = harness.world();
        let splashes = named(world, "Splash");
        assert_eq!(splashes.len(), 1, "exactly one splash");
        let position = position_of(world, splashes[0]);
        assert_eq!(position.y, -(WIN_H / 2.0 - SPLASH_EDGE_INSET), "at the edge it was lost past");
        assert!(position.x.abs() < WIN_W / 2.0, "inside the window");
        let (spec, depth) = if frozen { (&BALL_ICE, BALL_ICE_DEPTH) } else { (&BALL_WATER, BALL_WATER_DEPTH) };
        let animation = world.get::<SpriteAnimation>(splashes[0]).expect("the splash animates");
        assert_eq!(animation.sheet.as_deref(), Some(spec.path), "frozen: {frozen}");
        assert_eq!(animation.current_clip.as_deref(), Some(BALL_HURT));
        assert_eq!(world.get::<Sprite>(splashes[0]).expect("sprite").depth, depth, "at the ball's depth");

        harness.steps(60, FRAME);
        assert!(named(harness.world(), "Splash").is_empty(), "and it ends itself");
    }
}

#[test]
fn a_restart_and_a_quit_leave_no_one_shot_and_no_candy() {
    let leftovers = |harness: &mut engine_core::test_support::GameHarness<BreakoutGame>| {
        launch(harness);
        let ball = harness.game().ball.expect("ball");
        send_past_the_bottom(harness, ball);
        step_until_a_life_is_lost(harness);
        harness.context(|game, ctx| game.spawn_pickup(ctx.world, PickupKind::Multiball, Vec2::new(0.0, 200.0)));
        harness.step(FRAME, &[]);
        assert_eq!(named(harness.world(), "Splash").len(), 1);
        assert_eq!(named(harness.world(), "Candy (Multiball)").len(), 1);
    };
    let assert_clean = |harness: &engine_core::test_support::GameHarness<BreakoutGame>, after: &str| {
        assert!(named(harness.world(), "Splash").is_empty(), "a splash outlived the {after}");
        assert!(named(harness.world(), "Candy (Multiball)").is_empty(), "a candy outlived the {after}");
        assert!(harness.game().transient_visuals.is_empty() && harness.game().pickups.is_empty());
    };

    let mut harness = harness(GameMode::SinglePlayer, 0);
    leftovers(&mut harness);
    harness.context(|game, ctx| game.start_game(ctx));
    assert_clean(&harness, "restart");

    leftovers(&mut harness);
    harness.context(|game, ctx| game.reset_to_title(ctx.world));
    assert_clean(&harness, "quit");
}

#[test]
fn wrecking_freezes_deion_and_its_end_thaws_him_where_he_stands() {
    // Serving: the ball rests glued to the paddle, so any movement would be the swap's.
    let mut harness = harness(GameMode::SinglePlayer, 0);
    let ball = harness.game().ball.expect("ball");
    let resting = position_of(harness.world(), ball);
    let offset = harness.world().get::<Sprite>(ball).expect("sprite").offset;

    harness.context(|game, ctx| {
        game.spawn_pickup(ctx.world, PickupKind::Wrecking, Vec2::new(0.0, PADDLE_Y + 60.0));
    });
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        if harness.game().pickups.is_empty() {
            break;
        }
    }
    assert!(harness.game().wrecking_active(), "the wrecking candy was caught");
    let world = harness.world();
    assert_eq!(world.get::<SpriteAnimation>(ball).expect("anim").sheet.as_deref(), Some(BALL_ICE.path));
    assert_eq!(world.get::<Sprite>(ball).expect("sprite").texture_handle, harness.game().sheets.ball_ice.texture.id);
    assert_eq!(world.get::<Sprite>(ball).expect("sprite").color, Vec4::ONE, "a sheet, never a tint");
    assert!(position_of(world, ball).distance(resting) < POSITION_TOLERANCE, "the freeze moves nothing");
    assert_eq!(world.get::<Sprite>(ball).expect("sprite").offset, offset);

    harness.context(|game, _| game.wrecking.start(FRAME / 2.0));
    harness.steps(2, FRAME);
    let world = harness.world();
    assert_eq!(world.get::<SpriteAnimation>(ball).expect("anim").sheet.as_deref(), Some(BALL_WATER.path));
    assert_eq!(world.get::<Sprite>(ball).expect("sprite").depth, BALL_WATER_DEPTH);
    assert!(position_of(world, ball).distance(resting) < POSITION_TOLERANCE, "the thaw moves nothing");
}

#[test]
fn a_caught_candy_unwraps_where_it_was_caught_not_where_the_paddle_is() {
    let mut harness = harness(GameMode::SinglePlayer, 0);
    harness.context(|game, ctx| {
        game.spawn_pickup(ctx.world, PickupKind::Multiball, Vec2::new(30.0, PADDLE_Y + 60.0));
    });
    let mut falling_at = Vec2::ZERO;
    for _ in 0..60 {
        if let Some(candy) = harness.game().pickups.entities().next() {
            falling_at = position_of(harness.world(), candy);
        }
        harness.step(FRAME, &[]);
        if harness.game().pickups.is_empty() {
            break;
        }
    }
    let collects = named(harness.world(), "Candy Collect");
    assert_eq!(collects.len(), 1, "one collect for one catch");
    let position = position_of(harness.world(), collects[0]);
    assert!((position.x - 30.0).abs() < POSITION_TOLERANCE, "at the candy's x, not the paddle's 0: {position:?}");
    assert!((position.y - falling_at.y).abs() <= PICKUP_FALL_SPEED * FRAME + 0.01, "where it was falling");
    assert_eq!(state_of(harness.world(), collects[0]), CANDY_COLLECT);
    assert_eq!(harness.world().get::<Sprite>(collects[0]).expect("sprite").depth, CANDY_MULTIBALL_DEPTH);
}

#[test]
fn starting_a_match_gives_the_backdrop_the_chosen_modes_colour() {
    let mut harness = title_harness();
    let backdrop = harness.game().backdrop.expect("init spawns the backdrop");
    let born_with = harness.world().get::<GridBackdrop>(backdrop).expect("grid").color;

    start_match(&mut harness, GameMode::SinglePlayer, 1);
    let grid = harness.world().get::<GridBackdrop>(backdrop).expect("grid");
    assert_eq!(grid.color, backdrop_color(&theme_for(ChaosMode::Insane)), "THE VAULT's lattice");
    assert_ne!(grid.color, born_with, "and not the boot mode's colour");
}

#[test]
fn a_restart_and_a_quit_rest_both_tongs_closed() {
    let scowl_both = |harness: &mut engine_core::test_support::GameHarness<BreakoutGame>| {
        harness.context(|game, ctx| {
            for tong in [game.tong, game.tong_top].into_iter().flatten() {
                let facing = Facing::split(ctx.world.get::<ClipStateMachine>(tong).expect("m").state())
                    .expect("a tong state")
                    .1;
                crate::gameplay::set_clip_state(ctx.world, tong, &tong_state(TONG_SCORED_ON, facing));
            }
        });
    };
    let assert_rested = |harness: &engine_core::test_support::GameHarness<BreakoutGame>| {
        let game = harness.game();
        for (tong, side) in [(game.tong, PaddleSide::Bottom), (game.tong_top, PaddleSide::Top)] {
            let tong = tong.expect("co-op has both tongs");
            assert_eq!(
                state_of(harness.world(), tong),
                tong_state(TONG_CLOSED, game.tong_facing[side.index()])
            );
        }
    };

    let mut harness = harness(GameMode::TwoPlayerCoop, 0);
    scowl_both(&mut harness);
    harness.context(|game, ctx| game.start_game(ctx));
    assert_rested(&harness);

    scowl_both(&mut harness);
    harness.context(|game, ctx| game.reset_to_title(ctx.world));
    assert_rested(&harness);
}

#[test]
fn every_shipped_level_spawns_its_bricks_on_the_loaded_food_sheets() {
    // The scene loader resolves each brick's texture through the asset base: the handle
    // it gets must be the one `init` loaded, or a sheet's bricks would draw as two
    // batches.
    for mode in [GameMode::SinglePlayer, GameMode::TwoPlayerCoop] {
        for level in 0..crate::levels::roster(mode).len() {
            let harness = harness(mode, level);
            let game = harness.game();
            assert!(!game.bricks.is_empty());
            for brick in &game.bricks {
                let world = harness.world();
                let sprite = world.get::<Sprite>(brick.entity).expect("sprite");
                assert_eq!(sprite.texture_handle, game.sheets.food(brick.food).texture.id);
                let clip = world.get::<SpriteAnimation>(brick.entity).and_then(|a| a.current_clip.clone());
                let wanted = if brick.hits_left > 1 { BRICK_ARMORED } else { BRICK_INTACT };
                assert_eq!(clip.as_deref(), Some(wanted), "{mode:?} level {level}");
            }
        }
    }
}

#[test]
fn a_fresh_serve_is_drawn_on_the_serving_paddle_the_frame_it_spawns() {
    // After a loss past the top in co-op, the top tong serves: the fresh ball's first
    // rendered frame must already sit on it, not at the bottom paddle it used to spawn on.
    let mut harness = harness(GameMode::TwoPlayerCoop, 0);
    launch(&mut harness);
    let ball = harness.game().ball.expect("ball");
    harness.context(|game, _| {
        game.physics.reset_body(ball, Vec2::new(0.0, PADDLE_TOP_Y + 40.0));
        game.physics.set_velocity(ball, Vec2::new(0.0, BALL_SPEED), 0.0);
    });
    step_until_a_life_is_lost(&mut harness);

    let game = harness.game();
    assert_eq!(game.serving_side, PaddleSide::Top);
    let fresh = game.ball.expect("a fresh ball is served");
    let top_paddle_x = position_of(harness.world(), game.paddle_top.expect("co-op top paddle")).x;
    assert_eq!(
        position_of(harness.world(), fresh),
        Vec2::new(top_paddle_x, PADDLE_TOP_Y - SERVE_OFFSET_Y),
        "drawn on the top tong in the frame it was spawned"
    );
}

#[test]
fn two_candies_caught_in_one_frame_each_unwrap_their_own_kind_where_they_were() {
    // The unwraps are paired to their candies by the order `Pickups::collect` reports
    // catches in; two kinds caught in the same frame are the case that order decides.
    let mut harness = harness(GameMode::SinglePlayer, 0);
    harness.context(|game, ctx| {
        game.spawn_pickup(ctx.world, PickupKind::Wrecking, Vec2::new(-20.0, PADDLE_Y + 60.0));
        game.spawn_pickup(ctx.world, PickupKind::Multiball, Vec2::new(20.0, PADDLE_Y + 60.0));
    });
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        if harness.game().pickups.is_empty() {
            break;
        }
    }
    let collects = named(harness.world(), "Candy Collect");
    assert_eq!(collects.len(), 2, "both candies caught");
    for collect in collects {
        let world = harness.world();
        let sheet = world.get::<SpriteAnimation>(collect).and_then(|a| a.sheet.clone());
        let x = position_of(world, collect).x;
        let wanted = if x < 0.0 { CANDY_WRECKING.path } else { CANDY_MULTIBALL.path };
        assert_eq!(sheet.as_deref(), Some(wanted), "the unwrap at x {x} plays its own candy's sheet");
        assert!((x.abs() - 20.0).abs() < POSITION_TOLERANCE, "where its candy was: {x}");
    }
}
