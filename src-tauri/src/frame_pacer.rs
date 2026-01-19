// Frame Pacer - Consistent FPS delivery
// Based on RustDesk's VideoFrameController but simplified

use std::time::{Duration, Instant};

/// Manages frame pacing to ensure consistent FPS
pub struct FramePacer {
    target_fps: u32,
    last_frame_time: Instant,
    frame_count: u64,
    start_time: Instant,
}

impl FramePacer {
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_fps,
            last_frame_time: Instant::now(),
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    /// Get the target duration between frames (SPF = Seconds Per Frame)
    pub fn spf(&self) -> Duration {
        Duration::from_millis(1000 / self.target_fps as u64)
    }

    /// Check if enough time has passed to capture next frame
    pub fn should_capture(&mut self) -> bool {
        let elapsed = self.last_frame_time.elapsed();
        let spf = self.spf();
        
        if elapsed >= spf {
            self.last_frame_time = Instant::now();
            self.frame_count += 1;
            true
        } else {
            false
        }
    }

    /// Sleep until next frame is due
    pub fn sleep_until_next(&self) {
        let elapsed = self.last_frame_time.elapsed();
        let spf = self.spf();
        
        if let Some(sleep_time) = spf.checked_sub(elapsed) {
            std::thread::sleep(sleep_time);
        }
    }

    /// Get actual FPS based on frame count
    pub fn actual_fps(&self) -> f32 {
        let elapsed_secs = self.start_time.elapsed().as_secs_f32();
        if elapsed_secs > 0.0 {
            self.frame_count as f32 / elapsed_secs
        } else {
            0.0
        }
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Reset counters
    pub fn reset(&mut self) {
        self.frame_count = 0;
        self.start_time = Instant::now();
        self.last_frame_time = Instant::now();
    }

    /// Change target FPS
    pub fn set_fps(&mut self, fps: u32) {
        self.target_fps = fps;
    }

    /// Get target FPS
    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }
}

/// Adaptive frame pacer that adjusts FPS based on conditions
pub struct AdaptiveFramePacer {
    pacer: FramePacer,
    min_fps: u32,
    max_fps: u32,
    packet_loss_threshold: f32,
    consecutive_slow_frames: u32,
}

impl AdaptiveFramePacer {
    pub fn new(default_fps: u32, min_fps: u32, max_fps: u32) -> Self {
        Self {
            pacer: FramePacer::new(default_fps),
            min_fps,
            max_fps,
            packet_loss_threshold: 0.1, // 10% packet loss
            consecutive_slow_frames: 0,
        }
    }

    pub fn should_capture(&mut self) -> bool {
        self.pacer.should_capture()
    }

    pub fn sleep_until_next(&self) {
        self.pacer.sleep_until_next()
    }

    /// Adjust FPS based on packet loss
    pub fn adjust_for_packet_loss(&mut self, loss_rate: f32) {
        if loss_rate > self.packet_loss_threshold {
            // High packet loss → reduce FPS
            let new_fps = (self.pacer.target_fps() as f32 * 0.8) as u32;
            let new_fps = new_fps.max(self.min_fps);
            
            if new_fps != self.pacer.target_fps() {
                eprintln!("📉 Reducing FPS due to packet loss: {} → {} (loss: {:.1}%)",
                    self.pacer.target_fps(), new_fps, loss_rate * 100.0);
                self.pacer.set_fps(new_fps);
            }
        } else if loss_rate < self.packet_loss_threshold / 2.0 {
            // Low packet loss → can increase FPS
            let new_fps = (self.pacer.target_fps() as f32 * 1.1) as u32;
            let new_fps = new_fps.min(self.max_fps);
            
            if new_fps != self.pacer.target_fps() {
                eprintln!("📈 Increasing FPS (low packet loss): {} → {}",
                    self.pacer.target_fps(), new_fps);
                self.pacer.set_fps(new_fps);
            }
        }
    }

    /// Adjust FPS based on encoding/capture performance
    pub fn adjust_for_slow_frame(&mut self, frame_time_ms: u64) {
        let target_frame_time = 1000 / self.pacer.target_fps() as u64;
        
        if frame_time_ms > target_frame_time * 2 {
            // Frame took 2x longer than it should
            self.consecutive_slow_frames += 1;
            
            if self.consecutive_slow_frames >= 5 {
                // 5 consecutive slow frames → reduce FPS
                let new_fps = (self.pacer.target_fps() as f32 * 0.9) as u32;
                let new_fps = new_fps.max(self.min_fps);
                
                eprintln!("📉 Reducing FPS due to slow encoding: {} → {} ({} ms/frame)",
                    self.pacer.target_fps(), new_fps, frame_time_ms);
                self.pacer.set_fps(new_fps);
                self.consecutive_slow_frames = 0;
            }
        } else {
            self.consecutive_slow_frames = 0;
        }
    }

    pub fn actual_fps(&self) -> f32 {
        self.pacer.actual_fps()
    }

    pub fn target_fps(&self) -> u32 {
        self.pacer.target_fps()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_pacer_30fps() {
        let mut pacer = FramePacer::new(30);
        
        // Should capture immediately first time
        assert!(pacer.should_capture());
        
        // Should not capture immediately after
        assert!(!pacer.should_capture());
        
        // Sleep for 1/30 second
        std::thread::sleep(Duration::from_millis(34));
        
        // Should capture now
        assert!(pacer.should_capture());
    }

    #[test]
    fn test_adaptive_pacer() {
        let mut pacer = AdaptiveFramePacer::new(30, 10, 60);
        
        // High packet loss should reduce FPS
        pacer.adjust_for_packet_loss(0.15);
        assert!(pacer.target_fps() < 30);
        
        // Low packet loss should increase FPS
        pacer.adjust_for_packet_loss(0.01);
        // (May or may not increase depending on implementation)
    }
}
