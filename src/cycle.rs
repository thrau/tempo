//! Repeating two-phase countdown. Late ticks preserve the original schedule.
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Focus,
    Break,
}
impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "Focus",
            Self::Break => "Break",
        }
    }
}

pub struct Cycle {
    pub focus: Duration,
    pub break_time: Duration,
    pub phase: Phase,
    remaining: Duration,
    deadline: Option<Instant>,
}
impl Cycle {
    pub fn new(focus: Duration, break_time: Duration) -> Self {
        assert!(!focus.is_zero() && !break_time.is_zero());
        Self {
            focus,
            break_time,
            phase: Phase::Focus,
            remaining: focus,
            deadline: None,
        }
    }
    pub fn running(&self) -> bool {
        self.deadline.is_some()
    }
    pub fn remaining(&self, now: Instant) -> Duration {
        self.deadline
            .map_or(self.remaining, |end| end.saturating_duration_since(now))
    }
    pub fn start(&mut self, now: Instant) {
        if !self.running() {
            self.deadline = Some(now + self.remaining);
        }
    }
    pub fn pause(&mut self, now: Instant) {
        self.remaining = self.remaining(now);
        self.deadline = None;
    }
    pub fn reset(&mut self) {
        self.phase = Phase::Focus;
        self.remaining = self.focus;
        self.deadline = None;
    }
    /// Coalesce missed transitions into a single notification of the current phase.
    pub fn tick(&mut self, now: Instant) -> Option<Phase> {
        let end = self.deadline?;
        if now < end {
            return None;
        }
        let elapsed = now.duration_since(end).as_nanos();
        let period = (self.focus + self.break_time).as_nanos();
        let offset = elapsed % period;
        let (next, first, second) = match self.phase {
            Phase::Focus => (Phase::Break, self.break_time, self.focus),
            Phase::Break => (Phase::Focus, self.focus, self.break_time),
        };
        let remaining = if offset < first.as_nanos() {
            self.phase = next;
            first.as_nanos() - offset
        } else {
            // A complete pair returns to the phase that just ended.
            first.as_nanos() + second.as_nanos() - offset
        };
        self.remaining = Duration::from_nanos(remaining as u64);
        self.deadline = Some(now + self.remaining);
        Some(self.phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_cycles_stay_paused_and_boundaries_are_exact() {
        let now = Instant::now();
        let mut cycle = Cycle::new(Duration::from_secs(2), Duration::from_secs(1));
        assert_eq!(cycle.tick(now + Duration::from_secs(100)), None);
        cycle.start(now);
        assert_eq!(cycle.tick(now + Duration::from_secs(1)), None);
        assert_eq!(cycle.tick(now + Duration::from_secs(2)), Some(Phase::Break));
        assert_eq!(cycle.tick(now + Duration::from_secs(2)), None);
        assert_eq!(cycle.tick(now + Duration::from_secs(3)), Some(Phase::Focus));
        cycle.pause(now + Duration::from_secs(4));
        assert_eq!(cycle.tick(now + Duration::from_secs(100)), None);
        assert_eq!(
            cycle.remaining(now + Duration::from_secs(100)),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn repeats_and_catches_up_without_drift() {
        let now = Instant::now();
        let mut cycle = Cycle::new(Duration::from_secs(25), Duration::from_secs(5));
        cycle.start(now);
        assert_eq!(
            cycle.tick(now + Duration::from_secs(25)),
            Some(Phase::Break)
        );
        assert_eq!(
            cycle.tick(now + Duration::from_secs(30)),
            Some(Phase::Focus)
        );
        assert_eq!(
            cycle.tick(now + Duration::from_secs(118)),
            Some(Phase::Break)
        );
        assert_eq!(
            cycle.remaining(now + Duration::from_secs(118)),
            Duration::from_secs(2)
        );
        cycle.pause(now + Duration::from_secs(119));
        cycle.start(now + Duration::from_secs(200));
        assert_eq!(
            cycle.tick(now + Duration::from_secs(201)),
            Some(Phase::Focus)
        );
        cycle.reset();
        assert_eq!(cycle.phase, Phase::Focus);
        assert!(!cycle.running());
    }
}
