use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct AudioEngine;

impl AudioEngine {
    pub fn list_input_devices() -> Vec<String> {
        let mut devices = vec!["Default".to_string()];
        let host = cpal::default_host();
        if let Ok(input_devs) = host.input_devices() {
            for dev in input_devs {
                if let Ok(name) = dev.name() {
                    if !devices.contains(&name) {
                        devices.push(name);
                    }
                }
            }
        }
        devices
    }

    pub fn list_active_audio_apps() -> Vec<String> {
        let mut apps = Vec::new();
        if cfg!(target_os = "linux") {
            if let Ok(output) = std::process::Command::new("pactl").arg("list").arg("sink-inputs").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("application.name =") || line.contains("media.name =") {
                        let parts: Vec<&str> = line.split('=').collect();
                        if parts.len() == 2 {
                            let app_name = parts[1].trim().trim_matches('"').to_string();
                            if !app_name.is_empty() && !apps.contains(&app_name) {
                                apps.push(app_name);
                            }
                        }
                    }
                }
            }
        }
        if apps.is_empty() {
            apps.push("Discord".to_string());
            apps.push("Spotify".to_string());
            apps.push("Browser Stream".to_string());
        }
        apps
    }

    pub fn start_microphone_recording(
        is_recording_flag: Arc<AtomicBool>,
        mic_device_name: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut audio_path = dirs::audio_dir().unwrap_or_else(|| PathBuf::from("."));
        audio_path.push("WolfRecordings");
        fs::create_dir_all(&audio_path)?;
        let wav_output_path = audio_path.join(format!("temp_audio_{}.wav", chrono::Utc::now().timestamp()));

        let host = cpal::default_host();
        let device = if mic_device_name != "Default" && !mic_device_name.is_empty() {
            host.input_devices()?
                .find(|d| d.name().map(|n| n == mic_device_name).unwrap_or(false))
                .or_else(|| host.default_input_device())
                .ok_or("No matching microphone device found")?
        } else {
            host.default_input_device().ok_or("No default microphone input device found")?
        };

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let wav_path_clone = wav_output_path.clone();
        let wav_path_for_print = wav_output_path.clone();
        
        thread::spawn(move || {
            let spec = hound::WavSpec {
                channels: channels as _,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let writer = match hound::WavWriter::create(&wav_path_clone, spec) {
                Ok(w) => Arc::new(std::sync::Mutex::new(Some(w))),
                Err(e) => {
                    eprintln!("Failed to create WAV writer: {}", e);
                    return;
                }
            };

            let writer_clone = writer.clone();
            let err_fn = move |err| eprintln!("Audio stream error: {}", err);

            // Software volume amplification factor (boosts mic volume 2.5x cleanly)
            let gain = 2.5f32;

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(w) = guard.as_mut() {
                                for &sample in data {
                                    let amplified = sample * gain;
                                    let scaled = (amplified * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                                    let _ = w.write_sample(scaled);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(w) = guard.as_mut() {
                                for &sample in data {
                                    let amplified = (sample as f32) * gain;
                                    let scaled = amplified.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                                    let _ = w.write_sample(scaled);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(w) = guard.as_mut() {
                                for &sample in data {
                                    let shifted = (sample as i32 - 32768) as f32;
                                    let amplified = shifted * gain;
                                    let scaled = amplified.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                                    let _ = w.write_sample(scaled);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                ),
                _ => {
                    eprintln!("Unsupported audio sample format.");
                    return;
                }
            };

            if let Ok(stream) = stream {
                if stream.play().is_ok() {
                    while is_recording_flag.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }

            if let Ok(mut guard) = writer.lock() {
                if let Some(w) = guard.take() {
                    let _ = w.finalize();
                }
            }
            println!("Microphone track saved to {:?}", wav_path_for_print);
        });

        Ok(wav_output_path)
    }

    pub fn start_system_audio_recording(
        is_recording_flag: Arc<AtomicBool>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut audio_path = dirs::audio_dir().unwrap_or_else(|| PathBuf::from("."));
        audio_path.push("WolfRecordings");
        fs::create_dir_all(&audio_path)?;
        let wav_output_path = audio_path.join(format!("temp_system_audio_{}.wav", chrono::Utc::now().timestamp()));
        let wav_path_clone = wav_output_path.clone();

        thread::spawn(move || {
            let pulse_arg = if cfg!(target_os = "macos") {
                vec!["-f", "avfoundation", "-i", ":0"]
            } else if cfg!(target_os = "windows") {
                vec!["-f", "dshow", "-i", "audio=virtual-audio-capturer"]
            } else {
                vec!["-f", "pulse", "-i", "default.monitor"]
            };

            let mut ffmpeg_cmd = std::process::Command::new("ffmpeg");
            ffmpeg_cmd.arg("-y").arg("-fflags").arg("+genpts");
            for arg in pulse_arg {
                ffmpeg_cmd.arg(arg);
            }
            ffmpeg_cmd.args(&[
                "-ac", "2",
                "-ar", "44100",
                "-async", "1",
                wav_path_clone.to_str().unwrap(),
            ]);

            let mut child = match ffmpeg_cmd
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[AudioEngine] Failed to spawn system audio capture: {}", e);
                    return;
                }
            };

            while is_recording_flag.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));
            }

            #[cfg(unix)]
            let _ = std::process::Command::new("kill")
                .arg("-INT")
                .arg(child.id().to_string())
                .status();

            #[cfg(not(unix))]
            let _ = child.kill();

            let _ = child.wait();
            println!("[AudioEngine] System audio track saved to {:?}", wav_path_clone);
        });

        Ok(wav_output_path)
    }
}