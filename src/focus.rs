use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ClickEvent {
    pub x: i32,
    pub y: i32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FocusTracker {
    clicks: Arc<Mutex<Vec<ClickEvent>>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            clicks: Arc::new(Mutex::new(Vec::new())),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start_session(&self) {
        let mut clicks = self.clicks.lock().unwrap();
        clicks.clear();
        let mut start = self.start_time.lock().unwrap();
        *start = Some(Instant::now());
        println!("[FocusTracker] Tracking started.");
    }

    #[allow(dead_code)]
    pub fn log_click(&self, x: i32, y: i32) {
        let elapsed = match *self.start_time.lock().unwrap() {
            Some(t) => t.elapsed().as_millis() as u64,
            None => 0,
        };
        let event = ClickEvent {
            x,
            y,
            timestamp_ms: elapsed,
        };
        self.clicks.lock().unwrap().push(event);
        println!("[FocusTracker] Recorded click at ({}, {}) at {}ms", x, y, elapsed);
    }

    pub fn get_clicks(&self) -> Vec<ClickEvent> {
        self.clicks.lock().unwrap().clone()
    }

    /// Builds dynamic FFmpeg pan-and-zoom filter chain based on recorded user clicks
    pub fn build_ffmpeg_zoom_filter(
        &self,
        video_width: u32,
        video_height: u32,
        zoom_factor: f32,
    ) -> Option<String> {
        let clicks = self.get_clicks();
        if clicks.is_empty() {
            return None;
        }

        // Generate zoompan or dynamic crop filter string focusing on the primary / first click target
        let target = clicks[0];
        
        let target_w = (video_width as f32 / zoom_factor) as u32;
        let target_h = (video_height as f32 / zoom_factor) as u32;

        let crop_x = (target.x as u32).saturating_sub(target_w / 2).min(video_width - target_w);
        let crop_y = (target.y as u32).saturating_sub(target_h / 2).min(video_height - target_h);

        // Smooth zoompan filter string for high-end cinematic polish
        // Uses FFmpeg zoompan filter to smoothly zoom into clicked coordinates
        let filter = format!(
            "zoompan=z='if(between(in,30,120),min(zoom+0.015,{}),max(zoom-0.015,1))':x='{}-iw/2/zoom':y='{}-ih/2/zoom':d=120:s={}x{}",
            zoom_factor,
            target.x,
            target.y,
            video_width,
            video_height
        );

        println!("[FocusTracker] Generated FFmpeg Cinematic Zoom filter centered on ({}, {}): {}", crop_x, crop_y, filter);
        Some(filter)
    }
}
