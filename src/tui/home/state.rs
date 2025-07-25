use std::time::{Duration, Instant};

pub struct AnimationState {
    pub frame_index: usize,
    pub last_frame_time: Instant,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            frame_index: 0,
            last_frame_time: Instant::now(),
        }
    }

    pub fn update(&mut self, frame_count: usize, frame_duration: Duration) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        if elapsed >= frame_duration {
            self.frame_index = (self.frame_index + 1) % frame_count;
            self.last_frame_time = now;
        }
    }
}
