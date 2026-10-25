//! The Food Pyramid's music through the real game and the engine's harness, whose audio
//! manager plays the committed song on its virtual clock: the sidecar's shape and the bass
//! line's tiling, the six moments and the sources they trigger on, the order within a frame,
//! the stinger's notes, the pause jam keyed to the menu's navigation, the beat pulse, a match
//! start deciding by the song in the slot, and a missing bank leaving the game silent.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};
use music_midi::{BackgroundScore, PartMode, PositionSnapshot, Quantize, SongData};

use crate::constants::{BALL_RADIUS, BALL_SPEED, BRICK_CELL};
use crate::gameplay::bricks_hit_this_frame;
use crate::music::SONG_PATH;
use crate::test_support::*;
use crate::types::*;

const CLASSIC: usize = 0;

fn repository_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn committed_song() -> SongData {
    let bytes = std::fs::read(repository_assets().join(SONG_PATH)).expect("the song is committed");
    SongData::from_smf(&bytes).expect("the song parses").0
}

/// Frames with wall time between them, so the virtual clock renders blocks and the
/// conductor drains what the frames sent.
fn settle(harness: &mut GameHarness<BreakoutGame>) {
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

fn conductor(harness: &GameHarness<BreakoutGame>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("a song is in the slot").conductor
}

fn key(harness: &mut GameHarness<BreakoutGame>, code: KeyCode) {
    harness.step(FRAME, &[InputEvent::KeyPressed(code)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(code)]);
}

/// A CLASSIC match in play, its serve launched and its ball gone, the jaw left a second to
/// come to rest: nothing has triggered but what the test sends.
fn in_play() -> GameHarness<BreakoutGame> {
    let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
    launch(&mut harness);
    harness.context(|game, ctx| game.destroy_all_balls(ctx.world));
    harness.steps(60, FRAME);
    settle(&mut harness);
    harness
}

/// A ball at rest in the middle of the counter, one frame old, so its body is in the
/// physics world and a placement moves it there at once.
fn a_ball(harness: &mut GameHarness<BreakoutGame>) -> EntityId {
    let ball = harness.context(|game, ctx| {
        let ball = game.spawn_ball(ctx.world, "Deion (extra)", Vec2::ZERO);
        game.extra_balls.push(ball);
        ball
    });
    harness.step(FRAME, &[]);
    ball
}

/// The bottom row's bricks, by index into the game's list, lowest x first: nothing lies
/// below them, so a ball can be put against one from underneath.
fn bottom_row(harness: &GameHarness<BreakoutGame>) -> Vec<usize> {
    let game = harness.game();
    let y_of = |index: usize| position_of(harness.world(), game.bricks[index].entity).y;
    let lowest = (0..game.bricks.len()).map(y_of).fold(f32::INFINITY, f32::min);
    let mut row: Vec<usize> = (0..game.bricks.len()).filter(|&index| y_of(index) == lowest).collect();
    row.sort_by(|left, right| {
        let x = |index: usize| position_of(harness.world(), game.bricks[index].entity).x;
        x(*left).total_cmp(&x(*right))
    });
    row
}

/// Put `ball` 3 px under the brick at `index`, moving up, so the contact starts on the
/// next physics step: a ball placed already overlapping a static brick starts no contact.
fn against_brick(harness: &mut GameHarness<BreakoutGame>, ball: EntityId, index: usize) {
    let brick = position_of(harness.world(), harness.game().bricks[index].entity);
    let at = brick - Vec2::new(0.0, BRICK_CELL.y / 2.0 + BALL_RADIUS + 3.0);
    harness.context(|game, _| {
        game.physics.reset_body(ball, at);
        game.physics.set_velocity(ball, Vec2::new(0.0, BALL_SPEED), 0.0);
    });
}

/// Put `ball` already overlapping the bottom tong's body from the field side, moving down.
fn against_paddle(harness: &mut GameHarness<BreakoutGame>, ball: EntityId) {
    place_ball(harness, ball, PaddleSide::Bottom, Vec2::new(0.0, 12.0), Vec2::new(0.0, -BALL_SPEED));
}

/// Step until the frame's trace holds a trigger, and return it.
fn next_trace(harness: &mut GameHarness<BreakoutGame>) -> Vec<&'static str> {
    for _ in 0..10 {
        harness.step(FRAME, &[]);
        let trace = harness.game().music.triggered.clone();
        if !trace.is_empty() {
            return trace;
        }
    }
    panic!("no trigger in ten frames");
}

/// Step with wall time between frames until `done` holds, failing past `limit`.
fn poll(harness: &mut GameHarness<BreakoutGame>, limit: Duration, what: &str, done: impl Fn(&PositionSnapshot) -> bool) {
    let started = Instant::now();
    while !done(&conductor(harness)) {
        assert!(started.elapsed() < limit, "{what} within {limit:?}");
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

#[test]
fn the_committed_sidecar_binds_six_moments_on_unquantized_parts_with_two_instruments_on_the_march() {
    let text = std::fs::read_to_string(repository_assets().join(format!("{SONG_PATH}.ron"))).expect("the sidecar");
    let file = music_midi::parse_song_file("ai_food_pyramid.mid.ron", &text).expect("the sidecar parses");
    let binding = |action: &str| file.bindings.iter().find(|binding| binding.action == action).expect(action).clone();
    let part = |name: &str| file.parts.iter().find(|part| part.name == name).expect(name).clone();

    let (brick, armor) = (binding("brick"), binding("armor"));
    assert_eq!(brick.part, armor.part, "a foil brick echoes the tune the bricks walk");
    assert_ne!(brick.instrument, armor.instrument, "on its own instrument");
    assert_eq!(part(&brick.part).mode, PartMode::Loop);
    assert_eq!(part(&binding("paddle").part).mode, PartMode::AtSongPosition);
    for action in ["loss", "clear", "menu"] {
        assert_eq!(part(&binding(action).part).mode, PartMode::Loop, "{action}");
    }
    assert!(file.parts.iter().all(|part| part.quantize == Quantize::Off), "no trigger is parked on the loop seam");
    assert!(file.provenance.is_some());
    assert_eq!(file.soundfont, "soundfonts/food_pyramid.sf2");
}

#[test]
fn the_bass_line_tiles_the_loop_so_a_tong_hit_never_lands_in_a_rest() {
    let song = committed_song();
    let bass = song.tracks().iter().find(|track| track.name() == Some("bass")).expect("the bass track");
    let units = bass.units();
    assert_eq!(units[0].tick, 0, "it starts on the first tick");
    let end_of = |unit: &music_midi::MusicalUnit| unit.tick + unit.notes.iter().map(|note| note.duration_ticks).max().unwrap_or(0);
    for pair in units.windows(2) {
        assert_eq!(end_of(&pair[0]), pair[1].tick, "a unit ends where the next begins, at {}", pair[1].tick);
    }
    let loop_end = BackgroundScore::new(&song, &Default::default()).end_tick();
    assert_eq!(end_of(units.last().expect("a unit")), loop_end, "the last ends on the loop's last tick");
}

#[test]
fn the_song_plays_after_init() {
    let mut harness = title_harness();
    assert!(!harness.game().music.game.song_failed());
    let before = conductor(&harness).blocks_rendered;
    settle(&mut harness);
    assert!(conductor(&harness).blocks_rendered > before, "the virtual clock renders the song");
    let own = format!("{}/{SONG_PATH}", repository_assets().display());
    assert_eq!(harness.audio().song_label(), Some(own.as_str()));
}

#[test]
fn a_destroyed_brick_rings_on_the_games_source_and_a_foil_bricks_note_ends_with_it() {
    let mut harness = in_play();
    let row = bottom_row(&harness);
    let ball = a_ball(&mut harness);
    let accepted = conductor(&harness).accepted_triggers;

    let plain = harness.game().bricks[row[0]].entity;
    against_brick(&mut harness, ball, row[0]);
    assert_eq!(next_trace(&mut harness), ["brick"]);
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1);
    assert_eq!(harness.sound_sources().source_of(plain), None, "the brick had no source of its own");
    assert_eq!(conductor(&harness).live_sources, 1, "the game's source, which the tail does not end");

    let foil_index = bottom_row(&harness)[0];
    let foil = harness.game().bricks[foil_index].entity;
    harness.context(|game, _| game.bricks[foil_index].hits_left = 2);
    against_brick(&mut harness, ball, foil_index);
    assert_eq!(next_trace(&mut harness), ["armor"]);
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 2);
    assert!(harness.sound_sources().source_of(foil).is_some(), "the survivor's note is on its own source");
    let foil_index = harness.game().bricks.iter().position(|brick| brick.entity == foil).expect("it survived");
    against_brick(&mut harness, ball, foil_index);
    assert_eq!(next_trace(&mut harness), ["brick"]);
    assert_eq!(harness.sound_sources().source_of(foil), None, "its death ends its source in that tail");
    assert_eq!(harness.game().music.counts.armor_hits, 1);
    assert_eq!(harness.game().music.counts.bricks, 2);
}

#[test]
fn a_tong_hit_plays_on_the_paddles_source_and_a_match_start_ends_it_with_the_paddle() {
    let mut harness = in_play();
    let ball = a_ball(&mut harness);
    let paddle = harness.game().paddle.expect("the paddle");
    let accepted = conductor(&harness).accepted_triggers;
    against_paddle(&mut harness, ball);
    assert_eq!(next_trace(&mut harness), ["paddle"]);
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1);
    assert!(harness.sound_sources().source_of(paddle).is_some());

    start_match(&mut harness, GameMode::SinglePlayer, CLASSIC);
    harness.step(FRAME, &[]);
    assert_ne!(harness.game().paddle, Some(paddle), "the playfield was rebuilt");
    assert_eq!(harness.sound_sources().source_of(paddle), None, "the old paddle's source ended with it");
}

#[test]
fn a_lost_ball_is_one_trigger() {
    let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
    launch(&mut harness);
    let ball = harness.game().ball.expect("Deion");
    let accepted = conductor(&harness).accepted_triggers;
    send_past_the_bottom(&mut harness, ball);
    assert_eq!(next_trace(&mut harness), ["loss"]);
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1);
    assert_eq!(harness.game().music.counts.losses, 1);
}

#[test]
fn two_bricks_hit_in_one_frame_resolve_and_trigger_in_descending_index() {
    let mut harness = in_play();
    let row = bottom_row(&harness);
    let (low, high) = (row[0].min(row[2]), row[0].max(row[2]));
    let (first, second) = (a_ball(&mut harness), a_ball(&mut harness));
    harness.context(|game, _| game.bricks[low].hits_left = 2);
    against_brick(&mut harness, first, low);
    against_brick(&mut harness, second, high);
    assert_eq!(next_trace(&mut harness), ["brick", "armor"], "the higher index, plain, before the lower, foil");
}

#[test]
fn the_bricks_a_frame_hit_are_each_listed_once_in_descending_index() {
    let harness = harness(GameMode::SinglePlayer, CLASSIC);
    let bricks = &harness.game().bricks[..3];
    let (first, second) = (EntityId::new(), EntityId::new());
    let contact = |ball: EntityId, brick: usize, started: bool| CollisionData {
        event: CollisionEvent { entity_a: ball, entity_b: bricks[brick].entity, started, stopped: !started },
        contacts: Vec::new(),
    };
    let collisions = [contact(first, 0, true), contact(second, 2, true), contact(first, 2, true), contact(second, 1, false)];
    assert_eq!(bricks_hit_this_frame(&collisions, &[first, second], bricks), [2, 0], "a contact that only stopped is no hit");
}

#[test]
fn a_frame_with_a_tong_hit_and_a_destroyed_brick_triggers_the_paddle_before_the_brick() {
    let mut harness = in_play();
    let row = bottom_row(&harness);
    let (hitter, breaker) = (a_ball(&mut harness), a_ball(&mut harness));
    against_paddle(&mut harness, hitter);
    against_brick(&mut harness, breaker, row[0]);
    assert_eq!(next_trace(&mut harness), ["paddle", "brick"], "the paddle pass runs before the brick pass");
}

#[test]
fn the_last_brick_triggers_the_brick_then_the_clear_whose_chord_sounds_and_ends() {
    let mut harness = in_play();
    let last = bottom_row(&harness)[0];
    harness.context(|game, ctx| {
        let keep = game.bricks.remove(last);
        for brick in game.bricks.drain(..) {
            game.physics.destroy_entity(ctx.world, brick.entity);
        }
        game.bricks.push(keep);
    });
    let ball = a_ball(&mut harness);
    against_brick(&mut harness, ball, 0);
    assert_eq!(next_trace(&mut harness), ["brick", "clear"], "the win is decided on the frame the last brick breaks");
    assert_eq!(harness.game().state, GameState::GameOver { won: true });
}

#[test]
fn a_cleared_board_is_one_accepted_trigger_whose_notes_are_held_and_then_released() {
    let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
    settle(&mut harness);
    let before = conductor(&harness);
    assert_eq!(before.pending_events, 0, "nothing else has triggered");
    harness.context(|game, ctx| {
        for brick in game.bricks.drain(..) {
            game.physics.destroy_entity(ctx.world, brick.entity);
        }
    });
    harness.step(FRAME, &[]);
    assert_eq!(harness.game().music.counts.clears, 1);

    poll(&mut harness, Duration::from_secs(2), "the stinger accepted and its notes held", |now| {
        now.accepted_triggers == before.accepted_triggers + 1 && now.pending_events > 0
    });
    assert_eq!(conductor(&harness).silent_triggers, before.silent_triggers, "the unit had notes to play");
    let song = committed_song();
    let stinger = song.tracks().iter().find(|track| track.name() == Some("stinger")).expect("the stinger track");
    let tempo = song.tempo_map();
    let longest = stinger.units().iter().flat_map(|unit| unit.notes.iter().map(move |note| (unit.tick, note.duration_ticks)))
        .map(|(tick, duration)| tempo.seconds_at_tick(tick + duration) - tempo.seconds_at_tick(tick))
        .fold(0.0, f64::max);
    poll(&mut harness, Duration::from_secs_f64(longest + 2.0), "every held note-off sent", |now| now.pending_events == 0);
}

/// Where the pause menu drew `label` in the last frame.
fn drawn_at(harness: &GameHarness<BreakoutGame>, label: &str) -> Vec2 {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text.contains(label) => Some(data.position + Vec2::new(4.0, data.height / 2.0)),
        _ => None,
    }).unwrap_or_else(|| panic!("'{label}' was drawn"))
}

#[test]
fn the_pause_menu_jams_one_unit_a_move_and_nothing_on_open_confirm_the_pointer_or_a_reopen() {
    let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
    let jams = |harness: &GameHarness<BreakoutGame>| harness.game().music.game.jam_units();
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyW)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 0, "the frame it opens jams nothing, a move on it included");

    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(jams(&harness), 1, "one move, one unit");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1, "the song plays on under the pause menu");

    let (resume, restart) = (drawn_at(&harness, "Resume"), drawn_at(&harness, "Restart"));
    for at in [restart, resume, restart, resume] {
        harness.step(FRAME, &[InputEvent::MouseMoved(at.x, at.y)]);
    }
    assert_eq!(jams(&harness), 1, "a pointer swept across the rows jams nothing");
    harness.step(FRAME, &[InputEvent::MouseMoved(resume.x, resume.y)]);
    key(&mut harness, KeyCode::Enter);
    assert!(!harness.game().pause.is_active(), "confirm on Resume resumes");
    assert_eq!(jams(&harness), 1, "and confirm jams nothing");

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyS)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyS)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "a reopen jams nothing on its opening frame");
    assert!(harness.ui_commands().iter().any(|command| matches!(command,
        DrawCommand::Text { data, .. } if data.text.contains("move to jam"))), "the hint says a move plays");
}

#[test]
fn a_beat_ripples_under_the_tongs_while_the_match_runs_and_never_under_the_pause_menu() {
    let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
    let pulsed = |harness: &GameHarness<BreakoutGame>| harness.game().music.game.pulsed();
    let before = pulsed(&harness);
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(600));
        harness.step(FRAME, &[]);
    }
    assert!(pulsed(&harness) > before, "a beat every 0.54 s at 112 BPM, while serving");

    key(&mut harness, KeyCode::Escape);
    let paused_at = pulsed(&harness);
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(600));
        harness.step(FRAME, &[]);
    }
    assert!(harness.game().pause.is_active());
    assert_eq!(pulsed(&harness), paused_at, "the frozen grid would discard the impulse");
}

#[test]
fn a_match_start_requests_the_song_into_an_empty_slot_and_takes_its_own_back_to_the_top() {
    let mut harness = title_harness();
    let own = format!("{}/{SONG_PATH}", repository_assets().display());
    harness.audio_mut().stop_song();
    start_match(&mut harness, GameMode::SinglePlayer, CLASSIC);
    assert_eq!(harness.audio().song_label(), Some(own.as_str()), "an empty slot is requested into");

    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(400));
        harness.step(FRAME, &[]);
    }
    assert!(harness.audio().song_position().expect("playing").beat >= 1);
    start_match(&mut harness, GameMode::SinglePlayer, CLASSIC);
    settle(&mut harness);
    let position = harness.audio().song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (0, 0), "a match begins at the top of the march");
}

/// An asset folder holding every directory the game reads but `soundfonts/`, each a link to
/// the repository's own, removed however the test ends.
#[cfg(unix)]
struct AssetsWithoutBank {
    root: PathBuf,
}

#[cfg(unix)]
impl AssetsWithoutBank {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("food-pyramid-missing-bank-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("ai")).expect("a scratch asset folder");
        for directory in ["fonts", "scenes", "sprites", "ai/music"] {
            std::os::unix::fs::symlink(repository_assets().join(directory), assets.join(directory)).expect("linked");
        }
        Self { root }
    }
}

#[cfg(unix)]
impl Drop for AssetsWithoutBank {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn without_its_bank_the_game_runs_silent_and_counts_every_trigger_it_refuses() {
    let scratch = AssetsWithoutBank::new();
    let assets = scratch.root.join("assets");
    let config = crate::game_config(assets.to_str().expect("UTF-8"));
    let mut harness = GameHarness::new(BreakoutGame::default(), config);
    harness.step(FRAME, &[]);
    start_match(&mut harness, GameMode::SinglePlayer, CLASSIC);
    assert!(harness.game().music.game.song_failed(), "the song failed at init");
    assert_eq!(harness.game().music.game.failures(), 2, "and again at the match start");
    assert_eq!(harness.audio().song_label(), None);

    launch(&mut harness);
    let ball = harness.game().ball.expect("Deion");
    let refused = harness.sound_sources().refused_triggers();
    send_past_the_bottom(&mut harness, ball);
    assert_eq!(next_trace(&mut harness), ["loss"]);
    assert_eq!(harness.sound_sources().refused_triggers(), refused + 1, "refused, and counted");
    assert!(matches!(harness.game().state, GameState::Serving), "and the game runs on");
}
