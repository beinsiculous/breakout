//! Movement routing through real physics, plus the score's harmonic and cursor contracts.
use super::*;

#[test]
fn movement_onsets_reversals_and_clamping_follow_actual_displacement() {
    let mut harness = harness(GameMode::TwoPlayerCoop, CLASSIC);
    harness.step(FRAME, &[]);
    harness.step(
        FRAME,
        &[
            InputEvent::KeyPressed(KeyCode::KeyD),
            InputEvent::KeyPressed(KeyCode::ArrowRight),
        ],
    );
    assert_eq!(harness.game().music.counts.moves, 2);
    for paddle in [harness.game().paddle, harness.game().paddle_top] {
        assert!(harness.sound_sources().source_of(paddle.unwrap()).is_some());
    }
    harness.steps(12, FRAME);
    assert_eq!(
        harness.game().music.counts.moves,
        2,
        "steady hold stays silent after cooldown"
    );
    harness.step(
        FRAME,
        &[
            InputEvent::KeyReleased(KeyCode::KeyD),
            InputEvent::KeyPressed(KeyCode::KeyA),
        ],
    );
    assert_eq!(
        harness.game().music.counts.moves,
        3,
        "bottom reversal sounds"
    );
    harness.step(
        FRAME,
        &[
            InputEvent::KeyReleased(KeyCode::KeyA),
            InputEvent::KeyPressed(KeyCode::KeyD),
        ],
    );
    harness.steps(70, FRAME);
    assert_eq!(
        harness.game().music.counts.moves,
        3,
        "suppressed reversal never replays; walls do not chatter"
    );
    for paddle in [harness.game().paddle, harness.game().paddle_top] {
        assert!(
            (position_of(harness.world(), paddle.unwrap()).x - crate::constants::PADDLE_MAX_X)
                .abs()
                < 0.01
        );
    }
    harness.step(
        FRAME,
        &[
            InputEvent::KeyReleased(KeyCode::KeyD),
            InputEvent::KeyReleased(KeyCode::ArrowRight),
        ],
    );
    harness.steps(20, FRAME);
    assert_eq!(harness.game().music.counts.moves, 3, "stationary");
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyD)]);
    assert_eq!(
        harness.game().music.counts.moves,
        3,
        "onset input at a wall is not displacement"
    );
    harness.step(
        FRAME,
        &[
            InputEvent::KeyReleased(KeyCode::KeyD),
            InputEvent::KeyPressed(KeyCode::KeyA),
        ],
    );
    assert_eq!(harness.game().music.counts.moves, 4, "leaving wall");
}

#[test]
fn speed_threshold_is_consistent_across_common_refresh_rates() {
    for rate in [30.0, 60.0, 75.0, 90.0, 100.0, 120.0, 144.0, 165.0, 240.0] {
        for speed in [10.0, 14.0, 16.0, 20.0] {
            let mut harness = harness(GameMode::SinglePlayer, CLASSIC);
            let seconds = 1.0 / rate;
            // Set the pointer at the center before testing actual small displacements.
            harness.step(seconds, &[InputEvent::MouseMoved(400.0, 300.0)]);
            for frame in 1..=120 {
                harness.step(
                    seconds,
                    &[InputEvent::MouseMoved(
                        400.0 + frame as f32 * speed * seconds,
                        300.0,
                    )],
                );
            }
            assert_eq!(
                harness.game().music.counts.moves,
                u64::from(speed > 15.0),
                "speed={speed}, rate={rate}"
            );
        }
    }
}

#[test]
fn pause_title_and_restart_reset_movement_and_end_replaced_sources() {
    let mut harness = harness(GameMode::TwoPlayerCoop, CLASSIC);
    harness.step(FRAME, &[]);
    harness.step(
        FRAME,
        &[
            InputEvent::KeyPressed(KeyCode::KeyD),
            InputEvent::KeyPressed(KeyCode::ArrowRight),
        ],
    );
    let old = [
        harness.game().paddle.unwrap(),
        harness.game().paddle_top.unwrap(),
    ];
    assert_eq!(harness.game().music.counts.moves, 2);
    start_match(&mut harness, GameMode::TwoPlayerCoop, CLASSIC);
    harness.steps(2, FRAME);
    assert_eq!(
        harness.game().music.counts.moves,
        4,
        "restart clears both cooldowns"
    );
    for paddle in old {
        assert!(harness.sound_sources().source_of(paddle).is_none());
    }
    key(&mut harness, KeyCode::Escape);
    harness.step(
        FRAME,
        &[
            InputEvent::KeyReleased(KeyCode::KeyD),
            InputEvent::KeyReleased(KeyCode::ArrowRight),
            InputEvent::KeyPressed(KeyCode::KeyA),
        ],
    );
    harness.steps(20, FRAME);
    assert_eq!(harness.game().music.counts.moves, 4, "paused");
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::KeyA)]);
    key(&mut harness, KeyCode::Escape);
    harness.steps(20, FRAME);
    assert_eq!(harness.game().music.counts.moves, 4, "no stale menu motion");
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyD)]);
    assert_eq!(harness.game().music.counts.moves, 5);
    harness.context(|game, ctx| game.reset_to_title(ctx.world));
    harness.steps(20, FRAME);
    assert_eq!(harness.game().music.counts.moves, 5, "title is silent");
}

#[test]
fn movement_coexists_with_paddle_brick_and_clear_in_the_same_physics_frame() {
    let mut harness = in_play();
    let last = bottom_row(&harness)[0];
    harness.context(|game, ctx| {
        let keep = game.bricks.remove(last);
        for brick in game.bricks.drain(..) {
            game.physics.destroy_entity(ctx.world, brick.entity);
        }
        game.bricks.push(keep);
    });
    let (hitter, breaker) = (a_ball(&mut harness), a_ball(&mut harness));
    against_paddle(&mut harness, hitter);
    against_brick(&mut harness, breaker, 0);
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyD)]);
    assert_eq!(
        harness.game().music.triggered,
        ["move", "paddle", "brick", "clear"]
    );
    assert_eq!(harness.game().state, GameState::GameOver { won: true });
}

#[test]
fn original_score_harmony_loop_end_and_independent_movement_cursor() {
    let assets = repository_assets();
    let file = music_midi::parse_song_file(
        "Shelf Breaker",
        &std::fs::read_to_string(assets.join(format!("{SONG_PATH}.ron"))).unwrap(),
    )
    .unwrap();
    let bank = std::fs::read(assets.join(&file.soundfont)).unwrap();
    let song = std::sync::Arc::new(
        music_midi::Song::new(
            file.clone(),
            committed_song(),
            &[(&file.soundfont, &bank)],
            &[],
        )
        .unwrap(),
    );
    assert_eq!(song.background().end_tick(), 61440);
    assert_eq!(song.data().ticks_per_quarter(), 480);
    assert!(bank.len() < 4 * 1024 * 1024);
    assert_eq!(file.instruments.len(), 5);
    let actions: std::collections::BTreeSet<_> = file
        .bindings
        .iter()
        .map(|binding| binding.action.as_str())
        .collect();
    assert_eq!(
        actions,
        ["brick", "armor", "paddle", "loss", "clear", "menu", "move"]
            .into_iter()
            .collect()
    );
    let mut backing = std::collections::BTreeSet::new();
    let mut cues = std::collections::BTreeSet::new();
    for track in song.data().tracks() {
        let cue = matches!(
            track.name(),
            Some("melody" | "bass" | "dirge" | "stinger" | "jam" | "movement")
        );
        for unit in track.units() {
            for note in &unit.notes {
                assert!(unit.tick + note.duration_ticks <= 61440);
                if track.channel() != Some(9) {
                    if cue {
                        cues.insert(note.key % 12);
                    } else {
                        backing.insert(note.key % 12);
                    }
                }
            }
        }
    }
    assert_eq!(backing, [2, 4, 7, 11].into_iter().collect());
    assert_eq!(cues, backing);
    for cue in &cues {
        for background in &backing {
            assert!(![1, 6, 11].contains(&((cue + 12 - background) % 12)));
        }
    }
    let march = file
        .parts
        .iter()
        .position(|part| part.name == "march")
        .unwrap();
    let movement = file
        .parts
        .iter()
        .position(|part| part.name == "movement")
        .unwrap();
    let (mut conductor, handle) =
        music_midi::Conductor::new(music_midi::ConductorConfig::new(44100)).unwrap();
    conductor.load(song.clone()).unwrap();
    handle.send(music_midi::ConductorCommand::Play).unwrap();
    let start = Instant::now();
    for (index, action) in ["brick", "move", "armor"].into_iter().enumerate() {
        let stamp = start + Duration::from_millis(index as u64 * 10);
        handle
            .send(music_midi::ConductorCommand::Trigger {
                binding: song.binding_index(action).unwrap(),
                source: music_midi::SourceId {
                    id: 1,
                    generation: 0,
                    epoch: 1,
                },
                stamp,
            })
            .unwrap();
        conductor.render_at(
            stamp,
            &mut [0.0; 64],
            &mut [0.0; 64],
            music_midi::BusGains {
                music: 1.0,
                sfx: 1.0,
            },
        );
        assert_eq!(
            conductor.part_cursor(march),
            Some(if action == "armor" { 2 } else { 1 })
        );
        assert_eq!(
            conductor.part_cursor(movement),
            Some(if action == "brick" { 0 } else { 1 })
        );
    }
    assert_eq!(handle.position().snapshot().accepted_triggers, 3);
}

#[test]
fn high_refresh_steady_holds_and_clamped_walls_never_retrigger() {
    for rate in [75.0, 90.0, 120.0, 144.0, 165.0, 240.0] {
        let seconds = 1.0 / rate;
        let mut harness = harness(GameMode::TwoPlayerCoop, CLASSIC);
        harness.steps(8, seconds);
        harness.step(
            seconds,
            &[
                InputEvent::KeyPressed(KeyCode::KeyD),
                InputEvent::KeyPressed(KeyCode::ArrowRight),
            ],
        );
        // Both tongs move for multiple cooldowns and reach their clamps, including
        // all zero-step render frames between the actual physics steps.
        let paddles = [
            harness.game().paddle.unwrap(),
            harness.game().paddle_top.unwrap(),
        ];
        let mut saw_zero_displacement = [false; 2];
        for frame in 0..(rate * 4.0) as usize {
            let previous = paddles.map(|paddle| position_of(harness.world(), paddle).x);
            harness.step(seconds, &[]);
            for (index, paddle) in paddles.into_iter().enumerate() {
                let position = position_of(harness.world(), paddle).x;
                if position == previous[index]
                    && position.abs() < crate::constants::PADDLE_MAX_X - 1.0
                {
                    saw_zero_displacement[index] = true;
                }
            }
            if frame == (rate * 0.5) as usize - 1 {
                assert_eq!(
                    harness.game().music.counts.moves,
                    2,
                    "exactly one onset per tong after 0.5 seconds at {rate} Hz"
                );
            }
        }
        assert!(
            saw_zero_displacement.into_iter().all(|seen| seen),
            "both moving tongs had zero-step frames before reaching a wall at {rate} Hz"
        );
        assert_eq!(
            harness.game().music.counts.moves,
            2,
            "one onset per tong at {rate} Hz"
        );
        for paddle in [harness.game().paddle, harness.game().paddle_top] {
            assert!(
                (position_of(harness.world(), paddle.unwrap()).x - crate::constants::PADDLE_MAX_X)
                    .abs()
                    < 0.01
            );
        }
        harness.steps(rate as usize, seconds);
        assert_eq!(
            harness.game().music.counts.moves,
            2,
            "clamped hold at {rate} Hz"
        );
        harness.step(
            seconds,
            &[
                InputEvent::KeyReleased(KeyCode::KeyD),
                InputEvent::KeyReleased(KeyCode::ArrowRight),
            ],
        );
        harness.steps(8, seconds);
        harness.step(
            seconds,
            &[
                InputEvent::KeyPressed(KeyCode::KeyD),
                InputEvent::KeyPressed(KeyCode::ArrowRight),
            ],
        );
        harness.steps(8, seconds);
        assert_eq!(
            harness.game().music.counts.moves,
            2,
            "new outward input at a wall is silent at {rate} Hz"
        );
        harness.step(
            seconds,
            &[
                InputEvent::KeyReleased(KeyCode::KeyD),
                InputEvent::KeyReleased(KeyCode::ArrowRight),
                InputEvent::KeyPressed(KeyCode::KeyA),
                InputEvent::KeyPressed(KeyCode::ArrowLeft),
            ],
        );
        harness.steps((rate * 0.5) as usize, seconds);
        assert_eq!(
            harness.game().music.counts.moves,
            4,
            "leaving the wall sounds once per tong at {rate} Hz"
        );
    }
}

#[test]
fn lower_rate_pointer_events_do_not_restart_a_continuous_drag() {
    for rate in [144.0, 240.0] {
        for period in [2, 4] {
            for speed in [10.0, 20.0] {
                let seconds = 1.0 / rate;
                let mut harness = harness(GameMode::TwoPlayerCoop, CLASSIC);
                harness.step(FRAME, &[InputEvent::MouseMoved(400.0, 300.0)]);
                harness.steps(4, FRAME);
                for frame in 1..=rate as usize {
                    if frame % period == 0 {
                        harness.step(seconds, &[InputEvent::MouseMoved(400.0 + frame as f32 * speed * seconds, 300.0)]);
                    } else {
                        harness.step(seconds, &[]);
                    }
                }
                assert_eq!(harness.game().music.counts.moves, u64::from(speed > 15.0), "rate={rate} period={period} speed={speed}");
            }
        }
    }
}
