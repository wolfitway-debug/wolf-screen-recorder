use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::process::{Command, Stdio};
use std::io::Read;

pub struct WebcamEngine;

impl WebcamEngine {
    pub fn list_video_devices() -> Vec<String> {
        let mut devices = Vec::new();
        if cfg!(target_os = "linux") {
            for i in 0..8 {
                let dev_path = format!("/dev/video{}", i);
                if std::path::Path::new(&dev_path).exists() {
                    devices.push(format!("Camera {} ({})", i, dev_path));
                }
            }
        }
        if devices.is_empty() {
            devices.push("Auto (/dev/video0)".to_string());
        }
        devices
    }

    pub fn start_feed(
        enabled_flag: Arc<AtomicBool>,
        pip_window: slint::Weak<crate::WebcamPipWindow>,
        selected_device: &str,
    ) {
        let selected_device_str = selected_device.to_string();
        thread::spawn(move || {
            println!("[WebcamEngine] Initializing hardware camera stream for: {}", selected_device_str);

            let dev_args = if cfg!(target_os = "macos") {
                vec!["-f".to_string(), "avfoundation".to_string(), "-i".to_string(), "0".to_string()]
            } else if cfg!(target_os = "windows") {
                vec!["-f".to_string(), "dshow".to_string(), "-i".to_string(), "video=Integrated Camera".to_string()]
            } else {
                let dev_path = if selected_device_str.contains("/dev/video") {
                    let idx = selected_device_str.find("/dev/video").unwrap();
                    selected_device_str[idx..].trim_end_matches(')').to_string()
                } else if std::path::Path::new("/dev/video0").exists() {
                    "/dev/video0".to_string()
                } else if std::path::Path::new("/dev/video1").exists() {
                    "/dev/video1".to_string()
                } else {
                    "/dev/video0".to_string()
                };
                vec!["-f".to_string(), "v4l2".to_string(), "-i".to_string(), dev_path]
            };

            let width = 320u32;
            let height = 240u32;
            let frame_size = (width * height * 4) as usize;

            let mut ffmpeg_cmd = Command::new("ffmpeg");
            ffmpeg_cmd.arg("-y");
            for arg in dev_args {
                ffmpeg_cmd.arg(arg);
            }
            ffmpeg_cmd.args(&[
                "-video_size", &format!("{}x{}", width, height),
                "-f", "rawvideo",
                "-pix_fmt", "rgba",
                "-r", "30",
                "-",
            ]);

            let mut child = match ffmpeg_cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[WebcamEngine] Failed to spawn FFmpeg camera pipeline: {}", e);
                    return;
                }
            };

            let mut stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    eprintln!("[WebcamEngine] Failed to acquire FFmpeg stdout stream.");
                    let _ = child.kill();
                    return;
                }
            };

            let mut buffer = vec![0u8; frame_size];
            let mut first_frame = true;

            while enabled_flag.load(Ordering::Relaxed) {
                match stdout.read_exact(&mut buffer) {
                    Ok(_) => {
                        let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                            &buffer,
                            width,
                            height,
                        );

                        let pip_weak = pip_window.clone();
                        let is_first = first_frame;
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(pip) = pip_weak.upgrade() {
                                let slint_img = slint::Image::from_rgba8(pixel_buffer);
                                pip.set_webcam_frame(slint_img);
                                if is_first {
                                    pip.set_has_frame(true);
                                }
                            }
                        });
                        first_frame = false;
                    }
                    Err(e) => {
                        eprintln!("[WebcamEngine] Stream read error or camera disconnected: {}", e);
                        break;
                    }
                }
            }

            let _ = child.kill();
            let _ = child.wait();

            let pip_weak = pip_window.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(pip) = pip_weak.upgrade() {
                    pip.set_has_frame(false);
                }
            });

            println!("[WebcamEngine] Webcam PIP hardware stream shut down.");
        });
    }
}