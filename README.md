# 🐺 Wolf Screen Recorder

> **High-Performance Hardware-Accelerated Screen Capture Engine with Studio Pre-Processing & Cinematic Focus Modes**

Wolf Screen Recorder is an advanced, ultra-lightweight screen recording application built in Rust and Slint UI. Designed for instant adaptability across Linux, macOS, and Windows, it features native GPU auto-detection, vector annotations, cinematic cursor tracking, and bilingual zero-overhead UI state.

---

## 🔒 License & Legal Notice

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**.

### 🛑 The Fork-Only Upstream Rule
**Notice to Contributors and Corporate Users:**
To maintain absolute intellectual property integrity and technical auditability, **WOLFITWAYAGENCY SRL maintains 100% sole copyright ownership of the primary upstream repository**.
* **Zero Direct Pull Requests**: Direct pull requests to this repository are disabled and will not be merged.
* **Fork-Only Modifications**: External developers, enterprises, and organizations are required to fork the repository for custom needs, internal adaptations, or specific feature extensions in accordance with GPLv3.

For enterprise licensing, corporate sponsorship, or commercial custom builds, please access the Paddle merchant checkout link inside the application's **"Support the Dev" / "Susține Dezvoltatorul"** interface or contact `WOLFITWAYAGENCY SRL`.

---

## ✨ Features & Architecture

### ⚡ Milestone 1: Native Hardware Auto-Detection Engine
* **GPU & Encoder Profiling**: Queries system capabilities at boot via `sysinfo` and FFmpeg hardware capability scanning to route video encoding directly through hardware pipelines (`NVENC` for NVIDIA, `VideoToolbox` for macOS, `VA-API` for Linux, or `QSV` for Intel).
* **Fallback Logic**: Seamlessly falls back to optimized software encoding (`libx264` with dynamic preset tuning) if specialized hardware encoders are missing or fail.
* **Dynamic Resource Allocation**: Auto-tunes thread allocation, frame buffer lengths, and FFmpeg parameter limits based on system core count and memory availability.

### 🌐 Milestone 2: Bilingual Interface & Micro-Interaction Layer (EN/RO)
* **Zero-Overhead Dictionary**: Instant language swapping between English (`EN`) and Romanian (`RO`) inside the Slint UI state.
* **Micro-Interactions**: Hover pulses on the record button, smooth opacity sliders for preferences, live status notifications, and hardware badge indicators (e.g. `[VA-API]`, `[NVENC]`, `[SW x264]`).

### 🎨 Milestone 3: Product Showcase Pre-Processing & Annotations
* **Vector Overlay Engine**: Integrates Rust `image` and `imageproc` crates for programmatically stamping bounding boxes, highlighted click radii, drop shadows, and clean typography onto captured snapshot frames.
* **Automated Watermarking**: Burns brand watermarks ("WOLFITWAY") or custom creator metadata onto exported frames automatically for professional showcases.

### 🎬 Milestone 4: Cinematic Zoom & Dynamic Focus Modes
* **Cursor-Tracking Focus**: Logs mouse click coordinates and timestamps during recording sessions.
* **FFmpeg Pan-and-Zoom Filters**: Programmatically injects dynamic `crop` and `zoompan` filter chains during post-processing to smoothly zoom in on clicked UI elements and product features.

### 💳 Milestone 5: GPLv3 Distribution & Enterprise Monetization
* **Corporate Paddle Checkout**: Integrated post-recording modal connecting users directly to corporate sponsorship and enterprise licensing via Paddle checkout rails (`https://buy.paddle.com/placeholder-wolfitway`).

---

## 🛠️ Building & Running

### Prerequisites
* **Rust**: 1.75+ (Cargo toolchain)
* **FFmpeg**: Installed and available in PATH (with `libx264`, `vaapi`, `nvenc`, or `qsv` support depending on GPU)

### Build Commands
```bash
# Check compilation
cargo check

# Run in debug mode
cargo run

# Build release binary
cargo build --release
```

---

*Copyright © WOLFITWAYAGENCY SRL. All Rights Reserved.*
