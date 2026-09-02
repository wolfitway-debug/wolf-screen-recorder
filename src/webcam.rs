use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct WebcamEngine;

impl WebcamEngine {
    pub fn start_feed(enabled_flag: Arc<AtomicBool>) {
        thread::spawn(move || {
            println!("[WebcamEngine] Floating PIP webcam stream worker initialized...");
            let mut frame_count = 0;
            while enabled_flag.load(Ordering::Relaxed) {
                frame_count += 1;
                if frame_count % 30 == 0 {
                    println!("[WebcamEngine] Webcam PIP live feed streaming frame {}", frame_count);
                }
                thread::sleep(std::time::Duration::from_millis(33));
            }
            println!("[WebcamEngine] Webcam PIP stream worker shut down.");
        });
    }
}