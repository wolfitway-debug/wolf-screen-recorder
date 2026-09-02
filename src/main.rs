mod capture;
mod audio;
mod webcam;
mod hardware;
mod annotations;
mod focus;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Command;

use hardware::HardwareProfile;
use focus::FocusTracker;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;

    // Milestone 1: Hardware Auto-Detection Engine initialization at boot
    let hw_profile = HardwareProfile::detect();
    ui.set_hw_encoder_tag(hw_profile.encoder.tag().into());
    ui.set_status_message(format!("Ready ({})", hw_profile.encoder.display_name()).into());

    let focus_tracker = FocusTracker::new();

    // Window position dragging handler
    let ui_handle = ui.as_weak();
    ui.on_move_window(move |delta_x, delta_y| {
        let ui = ui_handle.unwrap();
        let current_pos = ui.window().position();
        let new_x = current_pos.x + (delta_x as i32);
        let new_y = current_pos.y + (delta_y as i32);
        ui.window().set_position(slint::PhysicalPosition::new(new_x, new_y));
    });

    let recording_flag = Arc::new(AtomicBool::new(false));
    let audio_enabled_flag = Arc::new(AtomicBool::new(true));

    // Milestone 2: Language Toggle Handler (EN <-> RO)
    let ui_handle = ui.as_weak();
    ui.on_toggle_language(move || {
        let ui = ui_handle.unwrap();
        let current_lang = ui.get_lang();
        let next_lang = if current_lang == "en" { "ro" } else { "en" };
        ui.set_lang(next_lang.into());
        println!("[UI] Swapped language to: {}", next_lang);
    });

    // Milestone 5: Corporate Paddle Checkout Rails handler
    ui.on_open_paddle_checkout(move || {
        let paddle_url = "https://buy.paddle.com/placeholder-wolfitway";
        println!("[Paddle] Opening corporate checkout rails: {}", paddle_url);
        if let Err(e) = open::that(paddle_url) {
            eprintln!("[Paddle] Failed to open browser link: {}", e);
        }
    });

    // Milestone 1, 3, 4: Recording Engine Activation Handler
    let ui_handle = ui.as_weak();
    let flag_clone = recording_flag.clone();
    let audio_flag_clone = audio_enabled_flag.clone();
    let hw_profile_clone = hw_profile.clone();
    let focus_tracker_clone = focus_tracker.clone();

    ui.on_toggle_recording(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_is_recording();
        let next_state = !current;
        ui.set_is_recording(next_state);

        flag_clone.store(next_state, Ordering::Relaxed);

        let lang = ui.get_lang();
        if next_state {
            let status_msg = if lang == "ro" {
                format!("ÎNREGISTRARE ACTIVĂ {}", hw_profile_clone.encoder.tag())
            } else {
                format!("RECORDING ACTIVE {}", hw_profile_clone.encoder.tag())
            };
            ui.set_status_message(status_msg.into());

            let audio_path_result = if audio_flag_clone.load(Ordering::Relaxed) {
                audio::AudioEngine::start_microphone_recording(flag_clone.clone()).ok()
            } else {
                None
            };

            let recording_flag_for_thread = flag_clone.clone();
            let hw_profile_thread = hw_profile_clone.clone();
            let focus_tracker_thread = focus_tracker_clone.clone();
            let ui_handle_thread = ui.as_weak();
            let cinematic_zoom_enabled = ui.get_cinematic_zoom_enabled();

            std::thread::spawn(move || {
                let temp_video_path = match capture::CaptureEngine::start_video_recording(
                    recording_flag_for_thread.clone(),
                    hw_profile_thread,
                    focus_tracker_thread.clone(),
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Failed to start video recording: {}", e);
                        return;
                    }
                };

                // Wait for stop signal
                while recording_flag_for_thread.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                // Give stream writer time to finalize file
                std::thread::sleep(std::time::Duration::from_millis(300));

                let final_muxed_path = temp_video_path.with_file_name(format!(
                    "recording_{}.mp4",
                    chrono::Utc::now().timestamp()
                ));

                println!("[CaptureEngine] Muxing video and audio tracks into final MP4...");

                // Milestone 4: Apply Cinematic Pan-and-Zoom FFmpeg filter if click events were logged
                let mut ffmpeg_cmd = Command::new("ffmpeg");
                ffmpeg_cmd.arg("-y").arg("-i").arg(temp_video_path.to_str().unwrap());

                if let Some(audio_path) = &audio_path_result {
                    ffmpeg_cmd.arg("-i").arg(audio_path.to_str().unwrap());
                }

                if cinematic_zoom_enabled {
                    if let Some(vf_zoom) = focus_tracker_thread.build_ffmpeg_zoom_filter(1920, 1080, 1.3) {
                        ffmpeg_cmd.arg("-vf").arg(vf_zoom);
                    }
                }

                ffmpeg_cmd.args(&[
                    "-c:v", "libx264",
                    "-preset", "fast",
                    "-c:a", "aac",
                    "-b:a", "192k",
                    "-af", "aresample=async=1",
                    final_muxed_path.to_str().unwrap(),
                ]);

                let status = ffmpeg_cmd.status();

                match status {
                    Ok(s) if s.success() => {
                        println!("[CaptureEngine] Successfully created final muxed recording: {:?}", final_muxed_path);
                        let _ = std::fs::remove_file(temp_video_path);
                        if let Some(audio_path) = audio_path_result {
                            let _ = std::fs::remove_file(audio_path);
                        }

                        // Update UI status & trigger support prompt modal
                        let ui_weak = ui_handle_thread.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                let lang = ui.get_lang();
                                let msg = if lang == "ro" { "Export finalizat cu succes!" } else { "Export complete!" };
                                ui.set_status_message(msg.into());
                                ui.set_show_support_modal(true);
                            }
                        });
                    }
                    _ => eprintln!("[CaptureEngine] FFmpeg post-process muxing failed."),
                }
            });

            println!("Recording session initiated.");
        } else {
            let status_msg = if lang == "ro" { "Finalizare procesare..." } else { "Muxing & finalizing..." };
            ui.set_status_message(status_msg.into());
            println!("Stop signal sent to recording threads.");
        }
    });

    // Milestone 3: Snapshot with Optional Watermark
    let ui_handle = ui.as_weak();
    ui.on_take_snapshot(move || {
        let ui = ui_handle.unwrap();
        let wm = if ui.get_watermark_enabled() { Some("WOLFITWAY") } else { None };
        match capture::CaptureEngine::save_screenshot(wm) {
            Ok(path) => {
                println!("Saved screenshot with watermark to: {:?}", path);
                let lang = ui.get_lang();
                let msg = if lang == "ro" { "Instantaneu salvat!" } else { "Snapshot saved!" };
                ui.set_status_message(msg.into());
            }
            Err(e) => eprintln!("Failed to capture snapshot: {}", e),
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_select_mode(move |mode| {
        let ui = ui_handle.unwrap();
        ui.set_selected_mode(mode);
    });

    let ui_handle = ui.as_weak();
    ui.on_select_resolution(move |res| {
        let ui = ui_handle.unwrap();
        ui.set_resolution(res);
    });

    let audio_flag_main = audio_enabled_flag.clone();
    let ui_handle = ui.as_weak();
    ui.on_toggle_audio(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_audio_enabled();
        let next = !current;
        ui.set_audio_enabled(next);
        audio_flag_main.store(next, Ordering::Relaxed);
    });

    let webcam_flag = Arc::new(AtomicBool::new(false));
    let webcam_flag_clone = webcam_flag.clone();
    let ui_handle = ui.as_weak();
    ui.on_toggle_webcam(move || {
        let ui = ui_handle.unwrap();
        let current = ui.get_webcam_enabled();
        let next = !current;
        ui.set_webcam_enabled(next);
        webcam_flag_clone.store(next, Ordering::Relaxed);
        if next {
            webcam::WebcamEngine::start_feed(webcam_flag_clone.clone());
        }
    });

    ui.run()?;
    Ok(())
}