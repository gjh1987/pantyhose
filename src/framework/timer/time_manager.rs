use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::framework::{data::MinHeap, timer};
use super::timer::{Timer, TimerId, TimerCallback};

pub struct TimeManager {
    timers: MinHeap<Timer>,
    timer_map: HashMap<TimerId, bool>,
    next_id: TimerId,
    now:u64
}

impl TimeManager {
    // ========== new methods ==========
    pub fn new() -> Self {
        Self {
            timers: MinHeap::new(),
            timer_map: HashMap::new(),
            next_id: 1,
            now:0
        }
    }

    // ========== init methods ==========
    pub fn start(&mut self){
        self.now = Instant::now().elapsed().as_millis() as u64
    }

    // ========== get/set methods ==========
    pub fn timer_count(&self) -> usize {
        self.timer_map.len()
    }

    fn get_next_id(&mut self) -> TimerId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ========== other methods ==========
    pub fn add_timer(
        &mut self,
        delay_time: u64,
        callback: TimerCallback,
        repeat_time: u8
    ) -> TimerId {
        let id = self.get_next_id();
        let timer = Timer::new(id, delay_time, repeat_time, self.now, callback);
        
        self.timers.insert(timer);
        self.timer_map.insert(id, true);
        
        id
    }

    pub fn remove_timer(&mut self, timer_id: TimerId) -> bool {
        self.timer_map.remove(&timer_id).is_some()
    }

    pub fn first_time_wait(&mut self) -> u64 {
        // 刷新时间
        self.now = Instant::now().elapsed().as_millis() as u64;

        if let Some(timer) = self.timers.peek() {
            let o = timer.next_trigger;
           return  o - self.now;
        } else {
            1000
        }
    }

    pub fn tick(&mut self) {
        let mut ready_timers = Vec::new();
        while let Some(timer) = self.timers.peek() {
            if timer.is_ready(self.now) {
                if let Some(timer) = self.timers.pop() {
                    ready_timers.push(timer);
                }
            } else {
                break;
            }
        }

        for mut timer in ready_timers {
            if self.timer_map.contains_key(&timer.id) {
                timer.execute();
                
                if timer.repeat_time < 0 {
                    timer.reset_next_trigger(self.now);
                    self.timers.insert(timer);
                } else {
                    timer.repeat_time -= 1;
                    if timer.repeat_time > 0 {
                        timer.reset_next_trigger(self.now);
                        self.timers.insert(timer);
                    }else{
                        self.timer_map.remove(&timer.id).is_some();
                    }
                }
            }
        }
    }

    pub fn clear_all_timers(&mut self) {
        self.timers.clear();
        self.timer_map.clear();
    }
}

impl Default for TimeManager {
    fn default() -> Self {
        Self::new()
    }
}