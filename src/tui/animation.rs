use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum AnimationType {
    SlideIn {
        start_y: i32,
        end_y: i32,
        start_x: i32,
        end_x: i32,
        start_color: [u8; 3],
        end_color: [u8; 3],
    },
    SlideDown {
        start_y: i32,
        end_y: i32,
    },
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub animation_type: AnimationType,
    pub start_time: Instant,
    pub duration: Duration,
}

impl Animation {
    pub fn new(animation_type: AnimationType, duration: Duration) -> Self {
        Self {
            animation_type,
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }

    pub fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }

    pub fn get_current_color(&self) -> Option<[u8; 3]> {
        if let AnimationType::SlideIn { start_color, end_color, .. } = self.animation_type {
            let progress = self.progress();
            let r = (start_color[0] as f32 + (end_color[0] as f32 - start_color[0] as f32) * progress) as u8;
            let g = (start_color[1] as f32 + (end_color[1] as f32 - start_color[1] as f32) * progress) as u8;
            let b = (start_color[2] as f32 + (end_color[2] as f32 - start_color[2] as f32) * progress) as u8;
            Some([r, g, b])
        } else {
            None
        }
    }
}
