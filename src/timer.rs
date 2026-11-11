//! Monotonic countdowns, independent of the UI and wall-clock changes.
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Timer {
    pub duration: Duration,
    remaining: Duration,
    deadline: Option<Instant>,
    pub finished: bool,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        assert!(!duration.is_zero());
        Self {
            duration,
            remaining: duration,
            deadline: None,
            finished: false,
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
        if self.finished {
            self.reset();
        }
        if !self.running() {
            self.deadline = Some(now + self.remaining);
        }
    }

    pub fn pause(&mut self, now: Instant) {
        self.remaining = self.remaining(now);
        self.deadline = None;
    }

    pub fn reset(&mut self) {
        self.remaining = self.duration;
        self.deadline = None;
        self.finished = false;
    }

    /// Returns true exactly once per completion, including after a delayed tick.
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.deadline.is_some_and(|end| now >= end) {
            self.deadline = None;
            self.remaining = Duration::ZERO;
            self.finished = true;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_cancels_a_running_deadline() {
        let now = Instant::now();
        let mut timer = Timer::new(Duration::from_secs(1));
        timer.start(now);
        timer.reset();
        assert!(!timer.tick(now + Duration::from_secs(10)));
        assert_eq!(timer.remaining(now), Duration::from_secs(1));
        assert!(!timer.finished);
    }

    #[test]
    fn pause_resume_finish_restart() {
        let now = Instant::now();
        let mut timer = Timer::new(Duration::from_secs(10));
        timer.start(now);
        timer.pause(now + Duration::from_secs(3));
        assert_eq!(
            timer.remaining(now + Duration::from_secs(100)),
            Duration::from_secs(7)
        );
        timer.start(now + Duration::from_secs(100));
        assert!(!timer.tick(now + Duration::from_secs(106)));
        assert!(timer.tick(now + Duration::from_secs(110)));
        assert!(!timer.tick(now + Duration::from_secs(111)));
        timer.start(now + Duration::from_secs(120));
        assert_eq!(
            timer.remaining(now + Duration::from_secs(120)),
            Duration::from_secs(10)
        );
        timer.reset();
        assert!(!timer.running());
    }
}
