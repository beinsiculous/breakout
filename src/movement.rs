//! Quiet feedback follows gesture speed, with actual travel required for an onset.
use crate::constants::{MOVEMENT_SOUND_COOLDOWN, MOVEMENT_SOUND_MIN_SPEED};

const POINTER_IDLE_GRACE_SECONDS: f32 = 1.0 / 30.0;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MovementCue {
    direction: i8,
    cooldown: f32,
    idle_seconds: f32,
}

impl MovementCue {
    pub(crate) fn observe(&mut self, displacement: f32, seconds: f32, requested_speed: f32) -> bool {
        if !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        self.cooldown = (self.cooldown - seconds).max(0.0);
        // Pointer events can arrive slower than rendering. Brief event gaps are
        // part of the same drag, and its delta spans that whole event interval.
        let gesture_seconds = (seconds + self.idle_seconds).min(POINTER_IDLE_GRACE_SECONDS).max(seconds);
        if requested_speed == 0.0 {
            self.idle_seconds += seconds;
            if self.idle_seconds <= POINTER_IDLE_GRACE_SECONDS { return false; }
        } else {
            self.idle_seconds = 0.0;
        }
        let gesture_speed = requested_speed * seconds / gesture_seconds;
        let direction = if gesture_speed.abs() > MOVEMENT_SOUND_MIN_SPEED {
            if requested_speed > 0.0 { 1 } else { -1 }
        } else { 0 };
        // Physics-free frames preserve held motion and wait to consume a new
        // direction until actual travel, rather than treating each gap as a stop.
        if direction != 0 && displacement * direction as f32 <= 0.0 {
            return false;
        }
        let onset = direction != 0 && direction != self.direction;
        // Suppressed actual reversals are consumed, not replayed on a steady hold.
        self.direction = direction;
        if onset && self.cooldown == 0.0 {
            self.cooldown = MOVEMENT_SOUND_COOLDOWN;
            true
        } else { false }
    }
}
