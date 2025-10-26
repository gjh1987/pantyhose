use std::cmp::Ordering;

pub type TimerId = u64;
pub type TimerCallback = Box<dyn FnMut() + Send>;

pub struct Timer {
    pub id: TimerId,
    pub delay_time: u64,
    pub repeat_time: u8,
    pub next_trigger: u64,
    pub callback: TimerCallback,
}

impl Timer {
    pub fn new(
        id: TimerId,
        delay_time: u64,
        repeat_time: u8,
        now:u64,
        callback: TimerCallback,
    ) -> Self {
        Self {
            id,
            delay_time,
            repeat_time: repeat_time,
            next_trigger: now + delay_time,
            callback
        }
    }

    pub fn is_ready(&self, now: u64) -> bool {
        now >= self.next_trigger
    }

    pub fn execute(&mut self) {
        (self.callback)();
    }

    pub fn reset_next_trigger(&mut self, now:u64) {
            self.next_trigger = now + self.delay_time;
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.next_trigger == other.next_trigger
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        other.next_trigger.cmp(&self.next_trigger)
    }
}