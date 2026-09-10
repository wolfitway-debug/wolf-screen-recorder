use xcap::Monitor;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::process::{Command, Stdio};
use std::io::Write;

use crate::hardware::HardwareProfile;
use crate::focus::FocusTracker;
use crate::annotations::AnnotationEngine;

pub struct CaptureEngine;

impl CaptureEngine {
    pub fn save_screenshot(
        watermark_text: Option<&str>,
        logo_path: Option<&PathBuf>,
        position: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let monitors = Monitor::all()?;
        let primary_monitor = monitors.first().ok_or("No primary monitor found")?;
        
        let image = primary_monitor.capture_image()?;
        
        let mut path = dirs::picture_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("WolfRecorder");
        fs::create_dir_all(&path)?;
        
        let filename = format!("snapshot_{}.png", chrono::Utc::now().timestamp());
        path.push(filename);
        
        let mut dyn_img = image::DynamicImage::ImageRgba8(image);

        if let Some(r) = crate::region::get_active_region() {
            if r.width > 0 && r.height > 0 && (r.x as u32) + r.width <= dyn_img.width() && (r.y as u32) + r.height <= dyn_img.height() {
                dyn_img = dyn_img.crop_imm(r.x as u32, r.y as u32, r.width, r.height);
            }
        }

        if watermark_text.is_some() || logo_path.is_some() {
            AnnotationEngine::process_snapshot_with_watermark(&mut dyn_img, watermark_text, logo_path, position);
        }

        dyn_img.save(&path)?;
        Ok(path)
    }

    pub fn start_video_recording(
        is_recording_flag: Arc<AtomicBool>,
        hw_profile: HardwareProfile,
        focus_tracker: FocusTracker,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut session_path = dirs::video_dir().unwrap_or_else(|| PathBuf::from("."));
        session_path.push("WolfRecordings");
        fs::create_dir_all(&session_path)?;

        let temp_video_mp4 = session_path.join(format!("temp_video_{}.mp4", chrono::Utc::now().timestamp()));
        let temp_video_clone = temp_video_mp4.clone();

        focus_tracker.start_session();

        thread::spawn(move || {
            let monitors = match Monitor::all() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to fetch monitors for recording: {}", e);
                    return;
                }
            };
            let monitor = match monitors.first() {
                Some(m) => m,
                None => {
                    eprintln!("No monitors found for recording.");
                    return;
                }
            };

            let width = monitor.width();
            let height = monitor.height();

            // Construct FFmpeg command with Hardware Auto-Detection args
            let mut ffmpeg_args = vec![
                "-y".to_string(),
                "-f".to_string(), "rawvideo".to_string(),
                "-vcodec".to_string(), "rawvideo".to_string(),
                "-s".to_string(), format!("{}x{}", width, height),
                "-pix_fmt".to_string(), "rgba".to_string(),
                "-r".to_string(), "30".to_string(),
                "-i".to_string(), "-".to_string(),
            ];

            // Add profile-specific encoder & acceleration flags
            ffmpeg_args.extend(hw_profile.get_ffmpeg_args());
            ffmpeg_args.push("-pix_fmt".to_string());
            ffmpeg_args.push("yuv420p".to_string());
            ffmpeg_args.push(temp_video_clone.to_str().unwrap().to_string());

            println!("[CaptureEngine] Spawning FFmpeg with args: {:?}", ffmpeg_args);

            let mut ffmpeg_child = match Command::new("ffmpeg")
                .args(&ffmpeg_args)
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("Failed to spawn ffmpeg hardware encoder: {}. Retrying with software fallback libx264...", e);
                    // Fallback to libx264
                    Command::new("ffmpeg")
                        .args(&[
                            "-y", "-f", "rawvideo", "-vcodec", "rawvideo",
                            "-s", &format!("{}x{}", width, height),
                            "-pix_fmt", "rgba", "-r", "30", "-i", "-",
                            "-c:v", "libx264", "-preset", "ultrafast", "-crf", "23",
                            "-pix_fmt", "yuv420p", temp_video_clone.to_str().unwrap()
                        ])
                        .stdin(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .expect("Failed to spawn ffmpeg software fallback")
                }
            };

            let mut stdin = match ffmpeg_child.stdin.take() {
                Some(stream) => stream,
                None => {
                    eprintln!("Failed to open stdin pipe to ffmpeg.");
                    return;
                }
            };

            let frame_duration = Duration::from_nanos(1_000_000_000 / 30);
            let recording_start = std::time::Instant::now();
            let mut frame_count: u64 = 0;

            while is_recording_flag.load(Ordering::Relaxed) {
                if let Ok(image) = monitor.capture_image() {
                    let rgba_data = image.as_raw();
                    if let Err(e) = stdin.write_all(rgba_data) {
                        eprintln!("Failed to write frame to ffmpeg stdin: {}", e);
                        break;
                    }
                    let _ = stdin.flush();
                    frame_count += 1;
                }

                let target_time = recording_start + frame_duration * (frame_count as u32);
                let now = std::time::Instant::now();
                if target_time > now {
                    thread::sleep(target_time - now);
                }
            }

            drop(stdin);

            let output = ffmpeg_child.wait_with_output();
            match output {
                Ok(o) if o.status.success() => {
                    println!("[CaptureEngine] Video capture successfully encoded using {}: {:?}", hw_profile.encoder.display_name(), temp_video_clone);
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    eprintln!("[CaptureEngine] Hardware encoder output stderr:\n{}", stderr);
                }
                Err(e) => eprintln!("Failed to wait for ffmpeg process: {}", e),
            }
        });

        Ok(temp_video_mp4)
    }
}