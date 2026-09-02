use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct WebcamEngine;

impl WebcamEngine {
    pub fn start_feed(enabled_flag: Arc<AtomicBool>) {
        thread::spawn(move || {
            println!("Webcam stream worker initialized...");
            while enabled_flag.load(Ordering::Relaxed) {
                // TODO: Pull frames from video4linux / native camera capture here
                thread::sleep(std::time::Duration::from_millis(33));
            }
            println!("Webcam stream worker shut down.");
        });
    }
}