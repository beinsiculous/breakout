//! The tong's jaw, as the art draws it and as The Food Pyramid's paddle wears it: the
//! five poses the tong's frames draw, and the collider a paddle body wears for each,
//! Tong's upright jaw turned a quarter onto the counter.
//!
//! The block between the two `shared tong jaw` markers is Tong's measured jaw, held
//! byte-identical with `games/pong/src/jaw.rs` — the two games sync one tong sheet, so
//! they read one set of numbers; the working set's `scripts/check-tong-jaw-sync.sh`
//! diffs the two. Change it in both files or in neither. What follows the block is
//! this game's own: the quarter turn, and the field-side arm the body keeps.

use engine_core::prelude::*;
use crate::constants::{PADDLE_H, PADDLE_W};
use crate::types::{
    Facing, PaddleSide, TONG_CLOSED, TONG_CLOSING, TONG_OPEN, TONG_OPENING, TONG_SCORED_ON,
};

// ==== shared tong jaw: begin ====
// --- the jaw ---
// The five poses the tong's fourteen `_up` frames draw, measured from the synced PNG
// cell by cell. The numbers below are world units with the cell's centre as origin
// and Y up; every one of them
// was read as a cell pixel and converted, `x - 32` and `48 - y` for a pixel's centre.

/// Where a tong's two arms meet: the hinge both arms run into, on the cell's mirror
/// axis. Its drawn base is the cell's rows 81-86; the two arms are already one run at
/// row 81, and the base's run there is a pixel short of the arms' own union a row
/// above, so the hinge sits on the axis rather than on that run's centre. The arms
/// being mirror images of each other is what a proper jaw needs, and it is what the
/// tip measurements below are stated against.
pub(crate) const TONG_HINGE: Vec2 = Vec2::new(0.0, -33.5);

/// Half the arm's thickness at mid-length: the arms are drawn seven pixels thick
/// between the hinge and the tip pads.
pub(crate) const TONG_ARM_RADIUS: f32 = 3.5;

/// The drawn width of a tip pad: eleven pixels across, in every one of the five poses,
/// so the jaw's mouth is the whole of what opens and shuts. It is also the pad's cap
/// radius: the pad is drawn as a box with a semicircular top of this radius.
pub(crate) const TONG_PAD_HALF_WIDTH: f32 = 5.5;

/// The height every pose holds its tips at: where the arm meets its pad.
pub(crate) const TONG_TIP_Y: f32 = 27.5;

/// The pad's straight run, hinge end to tip end: the pad is drawn from row 26 up to
/// row 9 — its bottom edge at 21 here, its top at 39, level with the closed tong's
/// own top — and a capsule of the pad's half-width between these two covers exactly
/// that, its top cap the drawn dome.
pub(crate) const TONG_PAD_INNER_Y: f32 = 26.5;
pub(crate) const TONG_PAD_OUTER_Y: f32 = 33.5;

/// A pose's gripping tips, for a tong whose tips point up: half its mouth plus half a
/// pad out from the axis, both sides of it.
const fn tong_tips(mouth_gap: f32) -> Vec2 {
    Vec2::new(mouth_gap / 2.0 + TONG_PAD_HALF_WIDTH, TONG_TIP_Y)
}

/// The faces a pose's two tip pads present to each other, pad to pad: how far open the
/// jaw is, and the whole of what closes on the way to `Closed`. Every one of them is
/// narrower than the meatball, so no closing jaw can take a ball in — the bite is a
/// tip deflection. The arm capsules come a little closer still, their inner faces
/// 28 px apart when `Open`, and that is under the ball too.
pub(crate) const TONG_MOUTH_GAP: f32 = 24.0;
const TONG_MOUTH_GAP_WIDE: f32 = 18.0;
const TONG_MOUTH_GAP_NARROW: f32 = 10.0;
const TONG_MOUTH_GAP_TWITCH: f32 = 4.0;

/// Where each pose's gripping tips stand. The closed jaw's is not here: it is one flat
/// body rather than two arms, and it keeps the capsule the tong has always had.
pub(crate) const TONG_TIP_OPEN: Vec2 = tong_tips(TONG_MOUTH_GAP);
pub(crate) const TONG_TIP_WIDE: Vec2 = tong_tips(TONG_MOUTH_GAP_WIDE);
pub(crate) const TONG_TIP_NARROW: Vec2 = tong_tips(TONG_MOUTH_GAP_NARROW);
pub(crate) const TONG_TIP_TWITCH: Vec2 = tong_tips(TONG_MOUTH_GAP_TWITCH);

/// One of the five jaw poses the tong's frames draw. Its fourteen cells are these
/// five repeated; `clip_poses` says which pose a given frame draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JawPose {
    Open,
    Wide,
    Narrow,
    Twitch,
    Closed,
}

impl JawPose {
    /// Where this pose's gripping tips are, for a tong whose tips point up. `None`
    /// for the closed jaw, which is one flat body rather than two arms.
    pub(crate) fn tips(self) -> Option<Vec2> {
        match self {
            JawPose::Open => Some(TONG_TIP_OPEN),
            JawPose::Wide => Some(TONG_TIP_WIDE),
            JawPose::Narrow => Some(TONG_TIP_NARROW),
            JawPose::Twitch => Some(TONG_TIP_TWITCH),
            JawPose::Closed => None,
        }
    }
}

/// The poses a clip draws, one per frame, indexed by the animation's
/// **clip-relative** `current_frame` — a position within the clip, never a sheet cell
/// index, so a facing appended to the sheet's later rows reads the same table.
///
/// A name outside the five yields nothing, and the collider is then left as it is: a
/// clip the game has no poses for is a sheet the game has not been told about, and
/// holding the jaw's last outline beats inventing one.
pub(crate) fn clip_poses(clip: &str) -> &'static [JawPose] {
    match clip {
        TONG_OPEN => &[JawPose::Open, JawPose::Open],
        TONG_CLOSING => &[JawPose::Wide, JawPose::Narrow, JawPose::Closed],
        TONG_CLOSED => &[JawPose::Closed, JawPose::Closed],
        TONG_OPENING => &[JawPose::Narrow, JawPose::Wide, JawPose::Open],
        TONG_SCORED_ON => &[
            JawPose::Twitch,
            JawPose::Narrow,
            JawPose::Twitch,
            JawPose::Narrow,
        ],
        _ => &[],
    }
}

/// How far a stick is pushed toward the field before it asks a tong's jaw to move: a
/// stick leaning inside this band leaves the intent where it was, which is what makes
/// the jaw stick rather than chatter. The bite feels the same in every game that has it.
pub(crate) const JAW_AXIS_DEAD_ZONE: f32 = 0.5;
// ==== shared tong jaw: end ====

/// How far above the closed line an open jaw's field-side pad reaches: the open tip's
/// distance from the tong's mirror axis plus the pad's half-width. No pose reaches
/// further toward the field, so a ball placed beyond this plus its radius touches none.
pub(crate) const TONG_OPEN_FIELD_REACH: f32 = TONG_TIP_OPEN.x + TONG_PAD_HALF_WIDTH;

/// A point of the upright tong — Tong's frame, tips up for `_up`, the cell's centre at
/// the origin — as the paddle body sees it: the facing's mirror, then the quarter turn
/// counter-clockwise that lays the tong on the counter, `(x, y)` to `(-y, x)`.
pub(crate) fn lay_on_counter(facing: Facing, point: Vec2) -> Vec2 {
    let oriented = match facing {
        Facing::Up => point,
        Facing::Down => Vec2::new(point.x, -point.y),
    };
    Vec2::new(-oriented.y, oriented.x)
}

/// The collider a paddle body wears while its tong draws `pose`: the closed tong's flat
/// capsule when the jaw is shut, and otherwise the one arm and pad that face the field.
///
/// The other arm and its pad are art only, in every split pose. They hang past the
/// paddle toward the edge it guards, where a ball has already beaten the paddle, and a
/// ball that bounced off them would come back into play off the drawn jaw's back.
///
/// Upright, the arms stand either side of the tong's mirror axis; laid down, the arm on
/// the axis's positive side lies above the body and the other below. So the bottom
/// paddle keeps the positive arm and the top one, whose field is below it, the negative.
pub(crate) fn tong_body_collider(pose: JawPose, facing: Facing, side: PaddleSide) -> ColliderShape {
    let Some(tips) = pose.tips() else {
        return ColliderShape::capsule_x(PADDLE_W, PADDLE_H / 2.0);
    };
    let field_tip_x = match side {
        PaddleSide::Bottom => tips.x,
        PaddleSide::Top => -tips.x,
    };
    let laid = |point: Vec2| lay_on_counter(facing, point);
    ColliderShape::compound(vec![
        ColliderShape::capsule(laid(TONG_HINGE), laid(Vec2::new(field_tip_x, tips.y)), TONG_ARM_RADIUS),
        ColliderShape::capsule(
            laid(Vec2::new(field_tip_x, TONG_PAD_INNER_Y)),
            laid(Vec2::new(field_tip_x, TONG_PAD_OUTER_Y)),
            TONG_PAD_HALF_WIDTH,
        ),
    ])
}

/// The pose a tong's animation is drawing for its clip-relative `frame`, with the
/// facing its clip names, or `None` for a clip the jaw has no poses for — a name
/// without a facing suffix, or a frame past the clip's table.
pub(crate) fn drawn_pose(clip: &str, frame: usize) -> Option<(JawPose, Facing)> {
    let (base, facing) = Facing::split(clip)?;
    let pose = *clip_poses(base).get(frame)?;
    Some((pose, facing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{BALL_RADIUS, TONG_LEFT, TONG_RIGHT};
    use crate::test_support::sidecar;

    const SPLIT_POSES: [JawPose; 4] = [JawPose::Open, JawPose::Wide, JawPose::Narrow, JawPose::Twitch];
    const FACINGS: [Facing; 2] = [Facing::Up, Facing::Down];
    const SIDES: [PaddleSide; 2] = [PaddleSide::Bottom, PaddleSide::Top];

    /// Tong's upright collider for a split pose, built the way `games/pong` builds it —
    /// arm, pad, arm, pad, the positive side first — as `(start, end, radius)` segments.
    fn upright_parts(pose: JawPose, facing: Facing) -> Vec<(Vec2, Vec2, f32)> {
        let tips = pose.tips().expect("a split pose has tips");
        let oriented = |point: Vec2| match facing {
            Facing::Up => point,
            Facing::Down => Vec2::new(point.x, -point.y),
        };
        let mut parts = Vec::new();
        for mirror in [1.0, -1.0] {
            let tip_x = mirror * tips.x;
            parts.push((oriented(TONG_HINGE), oriented(Vec2::new(tip_x, tips.y)), TONG_ARM_RADIUS));
            parts.push((
                oriented(Vec2::new(tip_x, TONG_PAD_INNER_Y)),
                oriented(Vec2::new(tip_x, TONG_PAD_OUTER_Y)),
                TONG_PAD_HALF_WIDTH,
            ));
        }
        parts
    }

    fn quarter_turn(point: Vec2) -> Vec2 {
        Vec2::new(-point.y, point.x)
    }

    fn parts_of(shape: &ColliderShape) -> Vec<(Vec2, Vec2, f32)> {
        match shape {
            ColliderShape::Compound(parts) => parts
                .iter()
                .map(|part| match part {
                    ColliderShape::Capsule { a, b, radius } => (*a, *b, *radius),
                    other => panic!("a jaw's part is a capsule, not {other:?}"),
                })
                .collect(),
            other => panic!("a split jaw is a compound, not {other:?}"),
        }
    }

    #[test]
    fn every_tong_clip_draws_as_many_poses_as_its_sheet_plays_frames() {
        // The pose tables are indexed by a clip-relative frame: a table a frame short
        // reads past its end, one too long never reaches its tail.
        for spec in [&TONG_LEFT, &TONG_RIGHT] {
            let prepared = sidecar(spec);
            for (name, clip) in &prepared.sheet.clips {
                let (base, _) = Facing::split(name).unwrap_or_else(|| panic!("{name} names its facing"));
                assert_eq!(clip_poses(base).len(), clip.frame_indices.len(), "{}: '{name}'", spec.path);
            }
        }
    }

    #[test]
    fn the_closed_jaw_laid_down_is_the_paddle_capsule_it_always_was() {
        for facing in FACINGS {
            for side in SIDES {
                assert_eq!(
                    tong_body_collider(JawPose::Closed, facing, side),
                    ColliderShape::capsule_x(78.0, 11.0),
                    "{facing:?} {side:?}"
                );
            }
        }
    }

    #[test]
    fn a_split_pose_is_tongs_field_side_arm_and_pad_turned_point_by_point() {
        for pose in SPLIT_POSES {
            for facing in FACINGS {
                let upright = upright_parts(pose, facing);
                for (side, kept) in [(PaddleSide::Bottom, &upright[0..2]), (PaddleSide::Top, &upright[2..4])] {
                    let turned: Vec<_> = kept
                        .iter()
                        .map(|&(start, end, radius)| (quarter_turn(start), quarter_turn(end), radius))
                        .collect();
                    assert_eq!(
                        parts_of(&tong_body_collider(pose, facing, side)),
                        turned,
                        "{pose:?} {facing:?} {side:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_split_pose_keeps_only_the_arm_and_pad_on_the_field_side() {
        // The body's centre line is y = 0: the bottom paddle's field is above it, the
        // top one's below. Every part of every split pose must stand on its field's side
        // of the line or on it — the hinge end sits on the line — and none past it.
        for pose in SPLIT_POSES {
            for facing in FACINGS {
                for side in SIDES {
                    let parts = parts_of(&tong_body_collider(pose, facing, side));
                    assert_eq!(parts.len(), 2, "{pose:?}: one arm and one pad");
                    let toward_field = match side {
                        PaddleSide::Bottom => 1.0,
                        PaddleSide::Top => -1.0,
                    };
                    for (start, end, _) in parts {
                        assert!(start.y * toward_field >= 0.0 && end.y * toward_field >= 0.0,
                            "{pose:?} {facing:?} {side:?}: a part at {start:?}-{end:?} trails the paddle");
                    }
                }
            }
        }
    }

    #[test]
    fn an_up_tong_leads_with_its_tips_to_the_left_and_a_down_one_to_the_right() {
        // The sprite turned a quarter counter-clockwise points `_up`'s tips left; the
        // collider must lean the same way, pads at the leading end.
        let (_, pad_up, _) = parts_of(&tong_body_collider(JawPose::Open, Facing::Up, PaddleSide::Bottom))[1];
        let (_, pad_down, _) = parts_of(&tong_body_collider(JawPose::Open, Facing::Down, PaddleSide::Bottom))[1];
        assert!(pad_up.x < 0.0 && pad_down.x > 0.0, "{pad_up:?} {pad_down:?}");
    }

    #[test]
    fn the_open_reach_is_the_highest_any_pose_rises_and_deion_cannot_fit_the_mouth() {
        let highest = |shape: &ColliderShape| {
            parts_of(shape).iter().map(|&(start, end, radius)| start.y.max(end.y) + radius).fold(f32::MIN, f32::max)
        };
        for pose in SPLIT_POSES {
            for facing in FACINGS {
                let reach = highest(&tong_body_collider(pose, facing, PaddleSide::Bottom));
                assert!(reach <= TONG_OPEN_FIELD_REACH, "{pose:?} reaches {reach}");
            }
        }
        assert_eq!(TONG_OPEN_FIELD_REACH, 23.0, "the open pad's dome, 23 px over the closed line");
        // The open arms are 28 px apart at their inner faces: Deion's body must not fit.
        let arm_inner_gap = 2.0 * (TONG_TIP_OPEN.x - TONG_ARM_RADIUS);
        assert_eq!(arm_inner_gap, 28.0);
        assert!(2.0 * BALL_RADIUS > arm_inner_gap, "a {} px Deion would fit a {arm_inner_gap} px mouth", 2.0 * BALL_RADIUS);
    }

    #[test]
    fn a_drawn_frame_names_its_pose_and_its_facing() {
        assert_eq!(drawn_pose("closing_up", 0), Some((JawPose::Wide, Facing::Up)));
        assert_eq!(drawn_pose("opening_down", 2), Some((JawPose::Open, Facing::Down)));
        assert_eq!(drawn_pose("closing_up", 3), None, "past the clip's table");
        assert_eq!(drawn_pose("idle", 0), None, "no facing");
    }
}
