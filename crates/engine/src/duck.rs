//! Ducking: an input (music, game) gets quieter while another input (the
//! microphone, Discord) is loud.
//!
//! Every input publishes the peak of its last callback in [`Levels`]. A
//! ducked input reads the level of its trigger in its own callback and
//! smooths its gain ([`Ducker`]). Everything is atomics, nothing allocates or
//! locks.

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

use crate::meter::to_db;

/// No trigger.
const NONE: u32 = u32::MAX;

/// Ducking settings of one input. The default does nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuckSettings {
    /// Input slot whose level lowers this input; `None` = off.
    pub trigger: Option<usize>,
    /// Level of the trigger above which ducking starts, dBFS.
    pub threshold_db: f32,
    /// How far this input drops, dB (positive).
    pub amount_db: f32,
    /// Time constant of the drop, ms.
    pub attack_ms: f32,
    /// Time constant of the return, ms.
    pub release_ms: f32,
}

impl Default for DuckSettings {
    fn default() -> Self {
        Self {
            trigger: None,
            threshold_db: -40.0,
            amount_db: 12.0,
            attack_ms: 30.0,
            release_ms: 600.0,
        }
    }
}

impl DuckSettings {
    /// Clamps to sane ranges; non-finite values fall back to the default.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let clamp = |v: f32, def: f32, lo: f32, hi: f32| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                def
            }
        };
        Self {
            trigger: self.trigger,
            threshold_db: clamp(self.threshold_db, d.threshold_db, -80.0, -10.0),
            amount_db: clamp(self.amount_db, d.amount_db, 0.0, 40.0),
            attack_ms: clamp(self.attack_ms, d.attack_ms, 1.0, 500.0),
            release_ms: clamp(self.release_ms, d.release_ms, 20.0, 3000.0),
        }
    }
}

/// Settings shared between the control thread and an input callback.
#[derive(Debug)]
pub struct InputDuck {
    trigger: AtomicU32,
    threshold_db: AtomicU32,
    amount_db: AtomicU32,
    attack_ms: AtomicU32,
    release_ms: AtomicU32,
}

impl Default for InputDuck {
    fn default() -> Self {
        let duck = Self {
            trigger: AtomicU32::new(NONE),
            threshold_db: AtomicU32::new(0),
            amount_db: AtomicU32::new(0),
            attack_ms: AtomicU32::new(0),
            release_ms: AtomicU32::new(0),
        };
        duck.set(DuckSettings::default());
        duck
    }
}

impl InputDuck {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, s: DuckSettings) {
        let s = s.sanitized();
        let trigger = s
            .trigger
            .and_then(|t| u32::try_from(t).ok())
            .filter(|t| *t != NONE)
            .unwrap_or(NONE);
        self.trigger.store(trigger, Relaxed);
        self.threshold_db.store(s.threshold_db.to_bits(), Relaxed);
        self.amount_db.store(s.amount_db.to_bits(), Relaxed);
        self.attack_ms.store(s.attack_ms.to_bits(), Relaxed);
        self.release_ms.store(s.release_ms.to_bits(), Relaxed);
    }

    pub fn get(&self) -> DuckSettings {
        let f = |a: &AtomicU32| f32::from_bits(a.load(Relaxed));
        let trigger = self.trigger.load(Relaxed);
        DuckSettings {
            trigger: (trigger != NONE).then_some(trigger as usize),
            threshold_db: f(&self.threshold_db),
            amount_db: f(&self.amount_db),
            attack_ms: f(&self.attack_ms),
            release_ms: f(&self.release_ms),
        }
    }
}

/// Peak (linear) of the last callback of every input slot.
#[derive(Debug)]
pub struct Levels(Vec<AtomicU32>);

impl Levels {
    pub fn new(inputs: usize) -> Self {
        Self((0..inputs).map(|_| AtomicU32::new(0)).collect())
    }

    /// Records the level of `input`. Real-time safe.
    pub fn set(&self, input: usize, linear: f32) {
        if let Some(slot) = self.0.get(input) {
            let v = if linear.is_finite() {
                linear.max(0.0)
            } else {
                0.0
            };
            slot.store(v.to_bits(), Relaxed);
        }
    }

    pub fn get(&self, input: usize) -> f32 {
        self.0
            .get(input)
            .map_or(0.0, |s| f32::from_bits(s.load(Relaxed)))
    }
}

/// The smoothed gain of one ducked input, owned by its callback.
#[derive(Debug, Default)]
pub struct Ducker {
    /// Current drop in dB, 0 or negative.
    drop_db: f32,
}

impl Ducker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the gain towards its target for a callback of `dt` seconds and
    /// returns the linear gain at its start and its end.
    pub fn advance(
        &mut self,
        own: usize,
        duck: &InputDuck,
        levels: &Levels,
        dt: f32,
    ) -> (f32, f32) {
        let before = self.drop_db;
        let s = duck.get();
        let target = match s.trigger {
            Some(t) if t != own && to_db(levels.get(t)) > s.threshold_db => -s.amount_db,
            _ => 0.0,
        };
        if self.drop_db != target {
            let ms = if target < self.drop_db {
                s.attack_ms
            } else {
                s.release_ms
            };
            let k = 1.0 - (-dt * 1000.0 / ms.max(1.0)).exp();
            self.drop_db += (target - self.drop_db) * k;
            // Close enough: land exactly so the fast path can take over.
            if (target - self.drop_db).abs() < 0.02 {
                self.drop_db = target;
            }
        }
        (db_to_linear(before), db_to_linear(self.drop_db))
    }
}

fn db_to_linear(db: f32) -> f32 {
    if db == 0.0 {
        1.0
    } else {
        10f32.powf(db / 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(level: f32) -> (InputDuck, Levels, Ducker) {
        let duck = InputDuck::new();
        duck.set(DuckSettings {
            trigger: Some(1),
            threshold_db: -30.0,
            amount_db: 12.0,
            attack_ms: 20.0,
            release_ms: 200.0,
        });
        let levels = Levels::new(2);
        levels.set(1, level);
        (duck, levels, Ducker::new())
    }

    fn run(d: &mut Ducker, duck: &InputDuck, levels: &Levels, ms: u32) -> f32 {
        let mut g = 1.0;
        for _ in 0..ms / 10 {
            g = d.advance(0, duck, levels, 0.01).1;
        }
        g
    }

    #[test]
    fn off_without_trigger() {
        let (duck, levels, mut d) = setup(1.0);
        duck.set(DuckSettings::default());
        assert_eq!(run(&mut d, &duck, &levels, 1000), 1.0);
    }

    #[test]
    fn drops_when_trigger_is_loud_and_returns() {
        let (duck, levels, mut d) = setup(0.5);
        let g = run(&mut d, &duck, &levels, 500);
        assert!((g - 10f32.powf(-12.0 / 20.0)).abs() < 0.01, "{g}");
        levels.set(1, 0.0);
        assert_eq!(run(&mut d, &duck, &levels, 3000), 1.0);
    }

    #[test]
    fn quiet_trigger_does_nothing() {
        let (duck, levels, mut d) = setup(0.001);
        assert_eq!(run(&mut d, &duck, &levels, 500), 1.0);
    }

    #[test]
    fn attack_is_faster_than_release() {
        let (duck, levels, mut d) = setup(0.5);
        let down = run(&mut d, &duck, &levels, 50);
        assert!(down < 0.7, "{down}");
        levels.set(1, 0.0);
        let up = run(&mut d, &duck, &levels, 50);
        assert!(up < 0.9, "{up}");
    }

    #[test]
    fn own_slot_and_unknown_slot_are_ignored() {
        let (duck, levels, mut d) = setup(0.5);
        duck.set(DuckSettings {
            trigger: Some(0),
            ..duck.get()
        });
        levels.set(0, 1.0);
        assert_eq!(run(&mut d, &duck, &levels, 500), 1.0);
        duck.set(DuckSettings {
            trigger: Some(9),
            ..duck.get()
        });
        assert_eq!(run(&mut d, &duck, &levels, 500), 1.0);
    }

    #[test]
    fn settings_round_trip_and_clamp() {
        let duck = InputDuck::new();
        duck.set(DuckSettings {
            trigger: Some(3),
            threshold_db: f32::NAN,
            amount_db: 99.0,
            attack_ms: 0.0,
            release_ms: 1e9,
        });
        let s = duck.get();
        assert_eq!(s.trigger, Some(3));
        assert_eq!(s.threshold_db, -40.0);
        assert_eq!(s.amount_db, 40.0);
        assert_eq!(s.attack_ms, 1.0);
        assert_eq!(s.release_ms, 3000.0);
    }
}
