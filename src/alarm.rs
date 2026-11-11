//! One-shot local-time alarms. Re-enabling schedules the next occurrence.
#[derive(Debug)]
pub struct Alarm {
    pub hour: i32,
    pub minute: i32,
    pub due: i64,
    pub enabled: bool,
}
impl Alarm {
    pub fn new(hour: i32, minute: i32, now: &glib::DateTime) -> Result<Self, glib::BoolError> {
        Ok(Self {
            hour,
            minute,
            due: next_occurrence(hour, minute, now)?,
            enabled: true,
        })
    }
    pub fn enable(&mut self, now: &glib::DateTime) -> Result<(), glib::BoolError> {
        self.due = next_occurrence(self.hour, self.minute, now)?;
        self.enabled = true;
        Ok(())
    }
    pub fn tick(&mut self, now: i64) -> bool {
        if self.enabled && now >= self.due {
            self.enabled = false;
            true
        } else {
            false
        }
    }
}

pub fn next_occurrence(
    hour: i32,
    minute: i32,
    now: &glib::DateTime,
) -> Result<i64, glib::BoolError> {
    for day in 0..3 {
        let date = now.add_days(day)?;
        let (year, month, day) = date.ymd();
        // Use the current local timezone, including its daylight-saving rules.
        if let Ok(candidate) =
            glib::DateTime::new(&now.timezone(), year, month, day, hour, minute, 0.0)
        {
            if candidate.to_unix() > now.to_unix() {
                return Ok(candidate.to_unix());
            }
        }
    }
    Err(glib::bool_error!(
        "This time is not available in your timezone"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schedules_today_and_reenables_for_tomorrow() {
        let now = glib::DateTime::new(&glib::TimeZone::utc(), 2025, 1, 1, 10, 0, 0.0).unwrap();
        let mut alarm = Alarm::new(10, 1, &now).unwrap();
        assert_eq!(alarm.due - now.to_unix(), 60);
        alarm.tick(alarm.due);
        alarm.enable(&now.add_minutes(1).unwrap()).unwrap();
        assert_eq!(alarm.due - now.to_unix(), 24 * 3600 + 60);
        assert!(Alarm::new(25, 0, &now).is_err());
    }

    #[test]
    fn next_day_and_fire_once() {
        let zone = glib::TimeZone::utc();
        let now = glib::DateTime::new(&zone, 2025, 1, 1, 12, 0, 0.0).unwrap();
        let mut alarm = Alarm::new(11, 0, &now).unwrap();
        assert_eq!(alarm.due - now.to_unix(), 23 * 3600);
        assert!(!alarm.tick(alarm.due - 1));
        assert!(alarm.tick(alarm.due));
        assert!(!alarm.tick(alarm.due + 1));
    }
}
