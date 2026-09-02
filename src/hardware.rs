use std::process::Command;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEncoder {
    Nvenc,
    VideoToolbox,
    Vaapi,
    Qsv,
    Libx264,
}

impl HwEncoder {
    pub fn codec_name(&self) -> &'static str {
        match self {
            HwEncoder::Nvenc => "h264_nvenc",
            HwEncoder::VideoToolbox => "h264_videotoolbox",
            HwEncoder::Vaapi => "h264_vaapi",
            HwEncoder::Qsv => "h264_qsv",
            HwEncoder::Libx264 => "libx264",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            HwEncoder::Nvenc => "NVIDIA NVENC",
            HwEncoder::VideoToolbox => "Apple VideoToolbox",
            HwEncoder::Vaapi => "Linux VA-API",
            HwEncoder::Qsv => "Intel QuickSync (QSV)",
            HwEncoder::Libx264 => "Software (libx264)",
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            HwEncoder::Nvenc => "[NVENC]",
            HwEncoder::VideoToolbox => "[VT]",
            HwEncoder::Vaapi => "[VA-API]",
            HwEncoder::Qsv => "[QSV]",
            HwEncoder::Libx264 => "[SW x264]",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HardwareProfile {
    pub encoder: HwEncoder,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub recommended_threads: usize,
    pub frame_buffer_capacity: usize,
    pub preset: &'static str,
}

impl HardwareProfile {
    pub fn detect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len().max(1);
        let total_memory_mb = sys.total_memory() / (1024 * 1024);
        let available_memory_mb = sys.available_memory() / (1024 * 1024);

        let encoder = Self::detect_best_encoder();
        
        // Dynamic resource allocation based on cores & memory
        let recommended_threads = match cpu_cores {
            1..=2 => 2,
            3..=8 => cpu_cores - 1,
            _ => 8,
        };

        // Buffer capacity scales with free RAM (e.g. 60 to 300 frames)
        let frame_buffer_capacity = if available_memory_mb > 16384 {
            300
        } else if available_memory_mb > 8192 {
            180
        } else if available_memory_mb > 4096 {
            120
        } else {
            60
        };

        let preset = match encoder {
            HwEncoder::Libx264 => {
                if cpu_cores >= 8 {
                    "veryfast"
                } else if cpu_cores >= 4 {
                    "superfast"
                } else {
                    "ultrafast"
                }
            }
            HwEncoder::Nvenc => "p4", // Medium speed/quality preset for NVENC
            HwEncoder::Qsv => "veryfast",
            HwEncoder::Vaapi | HwEncoder::VideoToolbox => "default",
        };

        let profile = HardwareProfile {
            encoder,
            cpu_cores,
            total_memory_mb,
            available_memory_mb,
            recommended_threads,
            frame_buffer_capacity,
            preset,
        };

        println!(
            "[HardwareEngine] Detected {} Cores, {} MB RAM (Available: {} MB). Selected Encoder: {} {}",
            profile.cpu_cores,
            profile.total_memory_mb,
            profile.available_memory_mb,
            profile.encoder.display_name(),
            profile.encoder.tag()
        );

        profile
    }

    fn detect_best_encoder() -> HwEncoder {
        // Query FFmpeg encoder availability
        let ffmpeg_encoders = match Command::new("ffmpeg").arg("-encoders").output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(_) => String::new(),
        };

        // macOS check
        if cfg!(target_os = "macos") && ffmpeg_encoders.contains("h264_videotoolbox") {
            return HwEncoder::VideoToolbox;
        }

        // Test NVENC first if FFmpeg lists h264_nvenc
        if ffmpeg_encoders.contains("h264_nvenc") && Self::test_ffmpeg_encoder("h264_nvenc") {
            return HwEncoder::Nvenc;
        }

        // Test VA-API next on Linux
        if cfg!(target_os = "linux") && ffmpeg_encoders.contains("h264_vaapi") && Self::test_ffmpeg_encoder("h264_vaapi") {
            return HwEncoder::Vaapi;
        }

        // Test QSV
        if ffmpeg_encoders.contains("h264_qsv") && Self::test_ffmpeg_encoder("h264_qsv") {
            return HwEncoder::Qsv;
        }

        // Fallback to libx264 software encoder
        HwEncoder::Libx264
    }

    fn test_ffmpeg_encoder(codec: &str) -> bool {
        // Run a tiny test encode to verify hardware support on current GPU drivers
        let status = Command::new("ffmpeg")
            .args(&[
                "-y",
                "-f", "lavfi",
                "-i", "color=c=black:s=64x64:d=0.1",
                "-c:v", codec,
                "-f", "null",
                "-",
            ])
            .output();

        match status {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    pub fn get_ffmpeg_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.push("-c:v".to_string());
        args.push(self.encoder.codec_name().to_string());

        match self.encoder {
            HwEncoder::Libx264 => {
                args.push("-preset".to_string());
                args.push(self.preset.to_string());
                args.push("-crf".to_string());
                args.push("23".to_string());
                args.push("-threads".to_string());
                args.push(self.recommended_threads.to_string());
            }
            HwEncoder::Nvenc => {
                args.push("-preset".to_string());
                args.push(self.preset.to_string());
                args.push("-cq".to_string());
                args.push("23".to_string());
            }
            HwEncoder::Qsv => {
                args.push("-preset".to_string());
                args.push(self.preset.to_string());
                args.push("-global_quality".to_string());
                args.push("23".to_string());
            }
            HwEncoder::Vaapi => {
                args.push("-vaapi_device".to_string());
                args.push("/dev/dri/renderD128".to_string());
                args.push("-vf".to_string());
                args.push("format=nv12,hwupload".to_string());
            }
            HwEncoder::VideoToolbox => {
                args.push("-realtime".to_string());
                args.push("true".to_string());
            }
        }

        args
    }
}
